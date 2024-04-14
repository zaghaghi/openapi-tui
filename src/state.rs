use std::collections::HashMap;

use color_eyre::eyre::Result;
use openapi_31::v31::{Openapi, Operation};

use crate::response::Response;

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
  async fn from_path(openapi_path: String) -> Result<Self> {
    let openapi_spec = tokio::fs::read_to_string(&openapi_path)
      .await
      .map(|content| serde_yaml::from_str::<Openapi>(content.as_str()))??;

    let openapi_operations = openapi_spec
      .into_operations()
      .map(|(path, method, operation)| {
        if path.starts_with('/') {
          OperationItem { path, method, operation, r#type: OperationItemType::Path }
        } else {
          OperationItem { path, method, operation, r#type: OperationItemType::Webhook }
        }
      })
      .collect::<Vec<_>>();
    Ok(Self {
      openapi_spec,
      openapi_input_source: openapi_path,
      openapi_operations,
      active_operation_index: 0,
      active_tag_name: None,
      active_filter: String::default(),
      input_mode: InputMode::Normal,
      responses: HashMap::default(),
    })
  }

  async fn from_url(openapi_url: reqwest::Url) -> Result<Self> {
    let resp: String = reqwest::get(openapi_url.clone()).await?.text().await?;
    let mut openapi_spec = serde_yaml::from_str::<Openapi>(resp.as_str())?;
    if openapi_spec.servers.is_none() {
      let origin = openapi_url.origin().ascii_serialization();
      openapi_spec.servers = Some(vec![openapi_31::v31::Server::new(format!("{}/", origin))]);
    }

    let openapi_operations = openapi_spec
      .into_operations()
      .map(|(path, method, operation)| {
        if path.starts_with('/') {
          OperationItem { path, method, operation, r#type: OperationItemType::Path }
        } else {
          OperationItem { path, method, operation, r#type: OperationItemType::Webhook }
        }
      })
      .collect::<Vec<_>>();
    Ok(Self {
      openapi_spec,
      openapi_input_source: openapi_url.to_string(),
      openapi_operations,
      active_operation_index: 0,
      active_tag_name: None,
      active_filter: String::default(),
      input_mode: InputMode::Normal,
      responses: HashMap::default(),
    })
  }

  pub async fn from_input(input: String) -> Result<Self> {
    if let Ok(url) = reqwest::Url::parse(input.as_str()) {
      State::from_url(url).await
    } else {
      State::from_path(input).await
    }
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
}

impl OperationItem {
  pub fn has_tag(&self, tag: &String) -> bool {
    self.operation.tags.as_ref().map_or(false, |tags| tags.contains(tag))
  }
}
