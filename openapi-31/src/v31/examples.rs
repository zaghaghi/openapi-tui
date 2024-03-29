// OpenAPI v3.1.0 Specification
//
// OpenAPI inside OpenAPI
//
// The version of the OpenAPI document: 3.1.0
//
// Generated by: https://openapi-generator.tech

use crate::v31;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Examples {
  #[serde(rename = "example", default, skip_serializing_if = "Option::is_none")]
  pub example: Option<serde_json::Value>,
  #[serde(rename = "examples", skip_serializing_if = "Option::is_none")]
  pub examples: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

impl Examples {
  pub fn new() -> Examples {
    Examples { example: None, examples: None }
  }
}
