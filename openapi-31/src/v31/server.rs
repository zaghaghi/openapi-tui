// OpenAPI v3.1.0 Specification
//
// OpenAPI inside OpenAPI
//
// The version of the OpenAPI document: 3.1.0
//
// Generated by: https://openapi-generator.tech

use crate::v31;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Server {
  #[serde(rename = "url")]
  pub url: String,
  #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(rename = "variables", skip_serializing_if = "Option::is_none")]
  pub variables: Option<std::collections::BTreeMap<String, v31::ServerVariable>>,
}

impl Server {
  pub fn new(url: String) -> Server {
    Server { url, description: None, variables: None }
  }
}
