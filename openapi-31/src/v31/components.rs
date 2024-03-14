// OpenAPI v3.1.0 Specification
//
// OpenAPI inside OpenAPI
//
// The version of the OpenAPI document: 3.1.0
//
// Generated by: https://openapi-generator.tech

use crate::v31;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Components {
  #[serde(rename = "schemas", skip_serializing_if = "Option::is_none")]
  pub schemas: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "responses", skip_serializing_if = "Option::is_none")]
  pub responses: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
  pub parameters: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "examples", skip_serializing_if = "Option::is_none")]
  pub examples: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "requestBodies", skip_serializing_if = "Option::is_none")]
  pub request_bodies: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "headers", skip_serializing_if = "Option::is_none")]
  pub headers: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "securitySchemes", skip_serializing_if = "Option::is_none")]
  pub security_schemes: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "links", skip_serializing_if = "Option::is_none")]
  pub links: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "callbacks", skip_serializing_if = "Option::is_none")]
  pub callbacks: Option<std::collections::BTreeMap<String, serde_json::Value>>,
  #[serde(rename = "pathItems", skip_serializing_if = "Option::is_none")]
  pub path_items: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

impl Components {
  pub fn new() -> Components {
    Components {
      schemas: None,
      responses: None,
      parameters: None,
      examples: None,
      request_bodies: None,
      headers: None,
      security_schemes: None,
      links: None,
      callbacks: None,
      path_items: None,
    }
  }
}
