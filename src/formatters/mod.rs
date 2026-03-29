pub mod json;

pub trait Formatter: Send + Sync {
  fn can_format(&self, content_type: &str) -> bool;
  fn format(&self, input: &str) -> String;
  fn syntax_name(&self) -> Option<&'static str> {
    None
  }
}

pub struct FormatterRegistry {
  formatters: Vec<Box<dyn Formatter>>,
}

impl Default for FormatterRegistry {
  fn default() -> Self {
    Self::new()
  }
}

impl FormatterRegistry {
  pub fn new() -> Self {
    Self { formatters: vec![] }
  }

  pub fn register(&mut self, formatter: Box<dyn Formatter>) {
    self.formatters.push(formatter);
  }

  pub fn format(&self, content_type: &str, input: &str) -> String {
    self
      .formatters
      .iter()
      .find(|f| f.can_format(content_type))
      .map(|f| f.format(input))
      .unwrap_or_else(|| input.to_string())
  }

  pub fn syntax_name(&self, content_type: &str) -> Option<&'static str> {
    self.formatters.iter().find(|f| f.can_format(content_type)).and_then(|f| f.syntax_name())
  }
}
