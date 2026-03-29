use super::Formatter;

pub struct JsonFormatter;

impl Formatter for JsonFormatter {
  fn can_format(&self, content_type: &str) -> bool {
    content_type.contains("json")
  }

  fn syntax_name(&self) -> Option<&'static str> {
    Some("json")
  }

  fn format(&self, input: &str) -> String {
    serde_json::from_str::<serde_json::Value>(input)
      .and_then(|v| serde_json::to_string_pretty(&v))
      .unwrap_or_else(|_| input.to_string())
  }
}
