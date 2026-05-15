use std::{
  collections::{BTreeMap, HashMap, HashSet},
  env,
  io::IsTerminal,
};

use color_eyre::eyre::{eyre, Result};
use openapi_31::v31::{Openapi, Operation, Server};
use tokio::io::AsyncReadExt;

use crate::{
  auth::{self, AuthScheme},
  response::Response,
};

#[derive(Default)]
pub struct State {
  pub openapi_input_source: String,
  pub openapi_spec: Openapi,
  pub openapi_operations: Vec<OperationItem>,
  pub active_operation_index: usize,
  pub active_tag_name: Option<String>,
  pub active_filter: String,
  pub input_mode: InputMode,
  pub responses: HashMap<String, Response>,
  pub pending_operations: HashSet<String>,
  pub spinner_frame: usize,
  pub global_headers: Vec<(String, String)>,
  pub auth_schemes: Vec<AuthScheme>,
  pub auth_values: HashMap<String, String>,
  pub global_security: Option<Vec<BTreeMap<String, Vec<String>>>>,
}

#[derive(Debug, Default, Clone)]
pub enum OperationItemType {
  #[default]
  Path,
  Webhook,
}

#[derive(Debug, Default, Clone)]
pub struct OperationItem {
  pub path: String,
  pub method: String,
  pub operation: Operation,
  pub r#type: OperationItemType,
}

#[derive(Default, PartialEq)]
pub enum InputMode {
  #[default]
  Normal,
  Insert,
  Command,
}

impl State {
  fn build(openapi_spec: Openapi, raw: &serde_yaml::Value, openapi_input_source: String) -> Self {
    let openapi_operations = openapi_spec
      .clone()
      .into_operations()
      .map(|(path, method, operation)| {
        if path.starts_with('/') {
          OperationItem { path, method, operation, r#type: OperationItemType::Path }
        } else {
          OperationItem { path, method, operation, r#type: OperationItemType::Webhook }
        }
      })
      .collect::<Vec<_>>();

    let auth_schemes = auth::parse_security_schemes(raw);
    let global_security = raw.get("security").and_then(auth::parse_security_requirements);

    Self {
      openapi_spec,
      openapi_input_source,
      openapi_operations,
      active_operation_index: 0,
      active_tag_name: None,
      active_filter: String::default(),
      input_mode: InputMode::Normal,
      responses: HashMap::default(),
      pending_operations: HashSet::default(),
      spinner_frame: 0,
      global_headers: Vec::new(),
      auth_schemes,
      auth_values: HashMap::default(),
      global_security,
    }
  }

  async fn from_path(openapi_path: String) -> Result<Self> {
    let content = tokio::fs::read_to_string(&openapi_path).await?;
    let openapi_spec = serde_yaml::from_str::<Openapi>(content.as_str())?;
    let raw: serde_yaml::Value = serde_yaml::from_str(content.as_str())?;
    Ok(Self::build(openapi_spec, &raw, openapi_path))
  }

  async fn from_url(openapi_url: reqwest::Url) -> Result<Self> {
    let resp: String = reqwest::get(openapi_url.clone()).await?.text().await?;
    let mut openapi_spec = serde_yaml::from_str::<Openapi>(resp.as_str())?;
    let raw: serde_yaml::Value = serde_yaml::from_str(resp.as_str())?;
    if openapi_spec.servers.is_none() {
      let origin = openapi_url.origin().ascii_serialization();
      openapi_spec.servers = Some(vec![openapi_31::v31::Server::new(format!("{}/", origin))]);
    }
    Ok(Self::build(openapi_spec, &raw, openapi_url.to_string()))
  }

  async fn from_stdin() -> Result<Self> {
    // Guard against the user typing `openapi-tui --input -` without piping
    // anything; without this the process silently waits for them to type a
    // full spec + Ctrl-D, which looks like a hang.
    if std::io::stdin().is_terminal() {
      return Err(eyre!("--input - expects an OpenAPI spec piped to stdin (got a TTY)"));
    }

    let mut buffer = String::new();
    tokio::io::stdin().read_to_string(&mut buffer).await?;

    if buffer.trim().is_empty() {
      return Err(eyre!("--input - received no data on stdin"));
    }

    let openapi_spec = serde_yaml::from_str::<Openapi>(buffer.as_str())?;
    let raw: serde_yaml::Value = serde_yaml::from_str(buffer.as_str())?;
    Ok(Self::build(openapi_spec, &raw, "<stdin>".to_string()))
  }

  pub async fn from_input(input: String, global_headers: Vec<(String, String)>) -> Result<Self> {
    let mut state = if input == "-" {
      State::from_stdin().await?
    } else if let Ok(url) = reqwest::Url::parse(input.as_str()) {
      State::from_url(url).await?
    } else {
      State::from_path(input).await?
    };
    state.global_headers = global_headers;
    Ok(state)
  }

  pub fn effective_security(&self, operation: &Operation) -> Option<Vec<BTreeMap<String, Vec<String>>>> {
    if let Some(per_op) = &operation.security {
      Some(auth::parse_security_requirements_json(per_op))
    } else {
      self.global_security.clone()
    }
  }

  pub fn auth_scheme(&self, name: &str) -> Option<&AuthScheme> {
    self.auth_schemes.iter().find(|s| s.name == name)
  }

  pub fn get_operation(&self, operation_id: Option<String>) -> Option<&OperationItem> {
    self.openapi_operations.iter().find(|operation_item| operation_item.operation.operation_id.eq(&operation_id))
  }

  pub fn active_operation(&self) -> Option<&OperationItem> {
    if let Some(active_tag) = &self.active_tag_name {
      self
        .openapi_operations
        .iter()
        .filter(|flat_operation| {
          flat_operation.has_tag(active_tag) && flat_operation.path.contains(self.active_filter.as_str())
        })
        .nth(self.active_operation_index)
    } else {
      self
        .openapi_operations
        .iter()
        .filter(|flat_operation| flat_operation.path.contains(self.active_filter.as_str()))
        .nth(self.active_operation_index)
    }
  }

  pub fn operations_len(&self) -> usize {
    if let Some(active_tag) = &self.active_tag_name {
      self
        .openapi_operations
        .iter()
        .filter(|item| item.has_tag(active_tag) && item.path.contains(self.active_filter.as_str()))
        .count()
    } else {
      self
        .openapi_operations
        .iter()
        .filter(|flat_operation| flat_operation.path.contains(self.active_filter.as_str()))
        .count()
    }
  }

  fn default_url(server: &Server) -> String {
    let mut url = server.url.clone();
    if let Some(variables) = &server.variables {
      for (k, v) in variables {
        url = url.replace(format!("{{{}}}", k).as_str(), &v.default);
      }
    }
    url.trim_end_matches('/').to_string()
  }

  pub fn default_server_urls(&self, extra_servers: &Option<Vec<Server>>) -> Vec<String> {
    let mut result = vec![];
    if let Ok(url) = env::var("OPENAPI_TUI_DEFAULT_SERVER") {
      result.push(url.trim_end_matches('/').to_string());
    }

    extra_servers.iter().flatten().for_each(|server| {
      result.push(State::default_url(server));
    });

    self.openapi_spec.servers.iter().flatten().for_each(|server| {
      result.push(State::default_url(server));
    });

    if result.is_empty() {
      result.push("http://localhost".to_string());
    }
    result
  }
}

impl OperationItem {
  pub fn has_tag(&self, tag: &String) -> bool {
    self.operation.tags.as_ref().is_some_and(|tags| tags.contains(tag))
  }
}
