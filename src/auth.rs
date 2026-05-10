use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION, COOKIE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyLocation {
  Header,
  Query,
  Cookie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthKind {
  ApiKey { name: String, location: ApiKeyLocation },
  HttpBearer,
  HttpBasic,
  Unsupported(String),
}

#[derive(Debug, Clone)]
pub struct AuthScheme {
  pub name: String,
  pub kind: AuthKind,
}

impl AuthKind {
  pub fn label(&self) -> String {
    match self {
      AuthKind::ApiKey { name, location } => {
        let loc = match location {
          ApiKeyLocation::Header => "header",
          ApiKeyLocation::Query => "query",
          ApiKeyLocation::Cookie => "cookie",
        };
        format!("apiKey ({loc} {name})")
      },
      AuthKind::HttpBearer => "http bearer".to_string(),
      AuthKind::HttpBasic => "http basic (user:pass)".to_string(),
      AuthKind::Unsupported(s) => format!("unsupported: {s}"),
    }
  }

  pub fn is_supported(&self) -> bool {
    !matches!(self, AuthKind::Unsupported(_))
  }
}

pub fn parse_security_schemes(raw: &serde_yaml::Value) -> Vec<AuthScheme> {
  let Some(map) = raw.get("components").and_then(|c| c.get("securitySchemes")).and_then(|s| s.as_mapping()) else {
    return Vec::new();
  };

  let mut out = Vec::new();
  for (k, v) in map {
    let Some(name) = k.as_str() else { continue };
    let Some(obj) = v.as_mapping() else { continue };
    let ty = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let kind = match ty {
      "apiKey" => {
        let key_name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        let location = match obj.get("in").and_then(|n| n.as_str()).unwrap_or("header") {
          "query" => ApiKeyLocation::Query,
          "cookie" => ApiKeyLocation::Cookie,
          _ => ApiKeyLocation::Header,
        };
        AuthKind::ApiKey { name: key_name, location }
      },
      "http" => {
        let scheme = obj.get("scheme").and_then(|n| n.as_str()).unwrap_or("").to_ascii_lowercase();
        match scheme.as_str() {
          "bearer" => AuthKind::HttpBearer,
          "basic" => AuthKind::HttpBasic,
          other => AuthKind::Unsupported(format!("http {other}")),
        }
      },
      "oauth2" | "openIdConnect" | "mutualTLS" => AuthKind::Unsupported(ty.to_string()),
      other => AuthKind::Unsupported(other.to_string()),
    };
    out.push(AuthScheme { name: name.to_string(), kind });
  }
  out
}

/// Parse a `security` node (top-level or per-operation) into the list of OR-ed
/// requirement options. Each option is a map of `scheme_name -> scopes`.
pub fn parse_security_requirements(value: &serde_yaml::Value) -> Option<Vec<BTreeMap<String, Vec<String>>>> {
  let arr = value.as_sequence()?;
  let mut out: Vec<BTreeMap<String, Vec<String>>> = Vec::with_capacity(arr.len());
  for entry in arr {
    let Some(m) = entry.as_mapping() else { continue };
    let mut req = BTreeMap::new();
    for (k, v) in m {
      if let Some(name) = k.as_str() {
        let scopes = v
          .as_sequence()
          .map(|seq| seq.iter().filter_map(|s| s.as_str().map(String::from)).collect())
          .unwrap_or_default();
        req.insert(name.to_string(), scopes);
      }
    }
    out.push(req);
  }
  Some(out)
}

/// Parse `security` from `serde_json::Value` (used for per-operation entries
/// that openapi-31 already deserialized).
pub fn parse_security_requirements_json(
  value: &[BTreeMap<String, serde_json::Value>],
) -> Vec<BTreeMap<String, Vec<String>>> {
  value
    .iter()
    .map(|m| {
      m.iter()
        .map(|(k, v)| {
          let scopes = v
            .as_array()
            .map(|seq| seq.iter().filter_map(|s| s.as_str().map(String::from)).collect())
            .unwrap_or_default();
          (k.clone(), scopes)
        })
        .collect()
    })
    .collect()
}

/// Pick the first option whose schemes are ALL present in `values`. Returns the
/// list of (scheme_name, value) pairs to apply, or `None` if no option matches.
pub fn select_satisfied_option<'a>(
  options: &'a [BTreeMap<String, Vec<String>>],
  values: &std::collections::HashMap<String, String>,
) -> Option<Vec<&'a String>> {
  for opt in options {
    if opt.is_empty() {
      // Empty requirement = explicit no-auth; treat as satisfied with nothing.
      return Some(Vec::new());
    }
    if opt.keys().all(|name| values.get(name).is_some_and(|v| !v.is_empty())) {
      return Some(opt.keys().collect());
    }
  }
  None
}

/// Apply a single resolved scheme to a `reqwest::RequestBuilder`.
pub fn apply_scheme(request: reqwest::RequestBuilder, scheme: &AuthScheme, value: &str) -> reqwest::RequestBuilder {
  match &scheme.kind {
    AuthKind::ApiKey { name, location } => match location {
      ApiKeyLocation::Header => match (HeaderName::try_from(name.as_str()), HeaderValue::from_str(value)) {
        (Ok(n), Ok(v)) => request.header(n, v),
        _ => request,
      },
      ApiKeyLocation::Query => request.query(&[(name.as_str(), value)]),
      ApiKeyLocation::Cookie => match HeaderValue::from_str(&format!("{name}={value}")) {
        Ok(v) => request.header(COOKIE, v),
        Err(_) => request,
      },
    },
    AuthKind::HttpBearer => match HeaderValue::from_str(&format!("Bearer {value}")) {
      Ok(v) => request.header(AUTHORIZATION, v),
      Err(_) => request,
    },
    AuthKind::HttpBasic => {
      let encoded = BASE64.encode(value.as_bytes());
      match HeaderValue::from_str(&format!("Basic {encoded}")) {
        Ok(v) => request.header(AUTHORIZATION, v),
        Err(_) => request,
      }
    },
    AuthKind::Unsupported(_) => request,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn yaml(src: &str) -> serde_yaml::Value {
    serde_yaml::from_str(src).unwrap()
  }

  #[test]
  fn parses_apikey_and_http_schemes() {
    let v = yaml(
      r#"
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
    basicAuth:
      type: http
      scheme: basic
    apiKeyHeader:
      type: apiKey
      in: header
      name: X-API-Key
    apiKeyQuery:
      type: apiKey
      in: query
      name: api_key
    oauth:
      type: oauth2
"#,
    );
    let schemes = parse_security_schemes(&v);
    assert_eq!(schemes.len(), 5);
    let by_name: std::collections::HashMap<_, _> = schemes.iter().map(|s| (s.name.as_str(), &s.kind)).collect();
    assert_eq!(by_name["bearerAuth"], &AuthKind::HttpBearer);
    assert_eq!(by_name["basicAuth"], &AuthKind::HttpBasic);
    assert!(matches!(by_name["apiKeyHeader"], AuthKind::ApiKey { location: ApiKeyLocation::Header, .. }));
    assert!(matches!(by_name["apiKeyQuery"], AuthKind::ApiKey { location: ApiKeyLocation::Query, .. }));
    assert!(matches!(by_name["oauth"], AuthKind::Unsupported(_)));
  }

  #[test]
  fn select_satisfied_picks_first_complete_option() {
    let options = vec![
      [("a".to_string(), vec![]), ("b".to_string(), vec![])].into_iter().collect(),
      [("c".to_string(), vec![])].into_iter().collect(),
    ];
    let mut values = std::collections::HashMap::new();
    values.insert("c".to_string(), "x".to_string());
    let picked = select_satisfied_option(&options, &values).unwrap();
    assert_eq!(picked, vec![&"c".to_string()]);
  }

  #[test]
  fn empty_requirement_is_explicit_no_auth() {
    let options: Vec<BTreeMap<String, Vec<String>>> = vec![BTreeMap::new()];
    let values = std::collections::HashMap::new();
    let picked = select_satisfied_option(&options, &values).unwrap();
    assert!(picked.is_empty());
  }
}
