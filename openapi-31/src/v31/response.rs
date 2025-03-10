// OpenAPI v3.1.0 Specification
//
// OpenAPI inside OpenAPI
//
// The version of the OpenAPI document: 3.1.0
//
// Generated by: https://openapi-generator.tech

use crate::v31;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
  #[serde(rename = "description")]
  pub description: String,
  #[serde(rename = "headers", skip_serializing_if = "Option::is_none")]
  pub headers: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "content", skip_serializing_if = "Option::is_none")]
  pub content: Option<std::collections::BTreeMap<String, v31::MediaType>>,
  #[serde(rename = "links", skip_serializing_if = "Option::is_none")]
  pub links: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

impl Response {
  pub fn new(description: String) -> Response {
    Response { description, headers: None, content: None, links: None }
  }
}
