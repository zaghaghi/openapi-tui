// OpenAPI v3.1.0 Specification
//
// OpenAPI inside OpenAPI
//
// The version of the OpenAPI document: 3.1.0
//
// Generated by: https://openapi-generator.tech

use super::reference::Resolve;
use crate::v31;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
  #[serde(rename = "name")]
  pub name: String,
  #[serde(rename = "in")]
  pub r#in: In,
  #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(rename = "required", skip_serializing_if = "Option::is_none")]
  pub required: Option<bool>,
  #[serde(rename = "deprecated", skip_serializing_if = "Option::is_none")]
  pub deprecated: Option<bool>,
  #[serde(rename = "schema", default, skip_serializing_if = "Option::is_none")]
  pub schema: Option<serde_json::Value>,
  #[serde(rename = "content", skip_serializing_if = "Option::is_none")]
  pub content: Option<std::collections::BTreeMap<String, v31::MediaType>>,
}

impl Parameter {
  pub fn new(name: String, r#in: In) -> Parameter {
    Parameter { name, r#in, description: None, required: None, deprecated: None, schema: None, content: None }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum In {
  #[serde(rename = "query")]
  Query,
  #[serde(rename = "header")]
  Header,
  #[serde(rename = "path")]
  Path,
  #[serde(rename = "cookie")]
  Cookie,
}

impl Default for In {
  fn default() -> In {
    Self::Query
  }
}
