// OpenAPI v3.1.0 Specification
//
// OpenAPI inside OpenAPI
//
// The version of the OpenAPI document: 3.1.0
//
// Generated by: https://openapi-generator.tech

use crate::v31;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerVariable {
  #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
  pub r#enum: Option<Vec<String>>,
  #[serde(rename = "default")]
  pub default: String,
  #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
}

impl ServerVariable {
  pub fn new(default: String) -> ServerVariable {
    ServerVariable { r#enum: None, default, description: None }
  }
}
