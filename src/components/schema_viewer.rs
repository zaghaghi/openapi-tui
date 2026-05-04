use std::{
  collections::{HashMap, HashSet},
  ops::Range,
};

use color_eyre::eyre::Result;
use ratatui::{prelude::*, widgets::*};
use syntect::{
  easy::HighlightLines,
  highlighting::{FontStyle as SyntectFontStyle, ThemeSet},
  parsing::SyntaxSet,
  util::LinesWithEndings,
};

use crate::state::State;

pub type NodeId = String;

#[derive(Debug, Clone)]
pub enum RenderBlock {
  /// A chunk of YAML text already indented to its target column.
  Yaml(String),
  /// A bracketed annotation rendered with a fixed marker style.
  Marker { indent: usize, text: String },
  /// A tab strip plus the body of the currently-selected variant.
  Variants { id: NodeId, indent: usize, choices: Vec<String>, selected: usize, body_blocks: Vec<RenderBlock> },
}

#[derive(Debug, Clone)]
pub struct VariantScope {
  pub line_range: Range<usize>,
  pub id: NodeId,
  pub choice_count: usize,
}

/// Resolves OpenAPI composition (`$ref`, `allOf`, `anyOf`, `oneOf`) into a
/// flat `Vec<RenderBlock>` that the renderer can consume. Pure: takes the
/// components map and the user's per-strip selections explicitly.
#[allow(dead_code)]
fn resolve_walk(
  value: &serde_json::Value,
  _parent_path: &str,
  indent: usize,
  components: &HashMap<String, serde_json::Value>,
  _variant_selection: &HashMap<NodeId, usize>,
  expanding: &mut HashSet<String>,
) -> Vec<RenderBlock> {
  if let Some(name) = ref_target_name(value) {
    if let Some(target) = components.get(name) {
      expanding.insert(name.to_string());
      let blocks = resolve_walk(target, _parent_path, indent, components, _variant_selection, expanding);
      expanding.remove(name);
      return blocks;
    }
    // Unknown component: fall through and emit the literal $ref.
  }

  let yaml = match serde_yaml::to_string(value) {
    Ok(s) => s,
    Err(_) => return Vec::new(),
  };
  vec![RenderBlock::Yaml(indent_lines(&yaml, indent))]
}

/// Returns the bare component name if `value` is exactly `{"$ref":
/// "#/components/schemas/<name>"}`, else None. We deliberately reject other
/// `$ref` shapes (parameters, responses, external) — those stay literal.
#[allow(dead_code)]
fn ref_target_name(value: &serde_json::Value) -> Option<&str> {
  let obj = value.as_object()?;
  if obj.len() != 1 {
    return None;
  }
  let s = obj.get("$ref")?.as_str()?;
  s.strip_prefix("#/components/schemas/")
}

/// Prepend `n` spaces to each non-empty line.
#[allow(dead_code)]
fn indent_lines(s: &str, n: usize) -> String {
  if n == 0 {
    return s.to_string();
  }
  let pad = " ".repeat(n);
  let mut out = String::with_capacity(s.len() + n * s.lines().count());
  for line in s.lines() {
    if !line.is_empty() {
      out.push_str(&pad);
    }
    out.push_str(line);
    out.push('\n');
  }
  out
}

const SYNTAX_THEME: &str = "Solarized (dark)";

pub struct SchemaViewer {
  components: HashMap<String, serde_json::Value>,
  styles: Vec<Vec<(Style, String)>>,
  line_offset: usize,

  name_history: Vec<String>,
  line_offset_history: Vec<usize>,

  highlighter_syntax_set: SyntaxSet,
  highlighter_theme_set: ThemeSet,
}

impl Default for SchemaViewer {
  fn default() -> Self {
    Self {
      components: HashMap::default(),
      styles: Vec::default(),
      line_offset: 0,
      name_history: Vec::default(),
      line_offset_history: Vec::default(),
      highlighter_syntax_set: SyntaxSet::load_defaults_newlines(),
      highlighter_theme_set: ThemeSet::load_defaults(),
    }
  }
}

impl SchemaViewer {
  pub fn set_components(&mut self, state: &State) {
    self.components = HashMap::default();
    if let Some(components) = &state.openapi_spec.components {
      if let Some(schemas) = &components.schemas {
        self.components = HashMap::from_iter(schemas.clone());
      }
    }
  }

  pub fn clear(&mut self) {
    self.line_offset = 0;
    self.name_history = vec![];
    self.line_offset_history = vec![];
    self.styles = vec![];
  }

  pub fn set(&mut self, schema: serde_json::Value) -> Result<()> {
    self.line_offset = 0;
    self.name_history = vec![];
    self.line_offset_history = vec![];
    self.set_styles(schema)?;
    self.go()
  }

  pub fn go(&mut self) -> Result<()> {
    if let Some(line_styles) = self.styles.get(self.line_offset) {
      let line: Vec<String> = line_styles
        .iter()
        .filter_map(|item| {
          if item.1.eq("$ref") || item.1.starts_with("#/components/schemas/") {
            return Some(item.1.clone());
          }
          None
        })
        .collect();
      if line.len() != 2 {
        return Ok(());
      }
      if !line[0].eq("$ref") || !line[1].starts_with("#/components/schemas/") {
        return Ok(());
      }

      let (_, schema_name) = line[1].split_at(21);

      self.line_offset_history.push(self.line_offset);
      self.line_offset = 0;
      self.name_history.push(schema_name.to_string());

      self.set_styles_by_name(schema_name.to_string())
    } else {
      Ok(())
    }
  }

  pub fn back(&mut self, schema: serde_json::Value) -> Result<()> {
    if let Some(line_offset) = self.line_offset_history.pop() {
      self.line_offset = line_offset;
    } else {
      self.line_offset = 0;
    }

    if self.name_history.is_empty() {
      self.set(schema)
    } else if self.name_history.len() < 2 {
      self.name_history = vec![];
      self.set_styles(schema)
    } else {
      self.name_history.pop();
      let schema_name = self.name_history.last().expect("empty nested schema vector");
      self.set_styles_by_name(schema_name.clone())
    }
  }

  pub fn down(&mut self) {
    self.line_offset = self.line_offset.saturating_add(1).min(self.styles.len().saturating_sub(1));
  }

  pub fn up(&mut self) {
    self.line_offset = self.line_offset.saturating_sub(1);
  }

  pub fn schema_path(&self) -> Vec<String> {
    self.name_history.clone()
  }

  pub fn render_widget(&self, frame: &mut Frame<'_>, area: Rect) {
    let lines = self.styles.iter().map(|items| {
      Line::from(items.iter().map(|item| Span::styled(&item.1, item.0.bg(Color::Reset))).collect::<Vec<_>>())
    });
    let mut list_state = ListState::default().with_selected(Some(self.line_offset));

    frame.render_stateful_widget(
      List::new(lines).highlight_symbol(symbols::scrollbar::HORIZONTAL.end).highlight_spacing(HighlightSpacing::Always),
      area,
      &mut list_state,
    );
  }

  fn set_styles(&mut self, schema: serde_json::Value) -> Result<()> {
    self.styles = vec![];
    let yaml_schema = serde_yaml::to_string(&schema)?;
    let mut highlighter = HighlightLines::new(
      self.highlighter_syntax_set.find_syntax_by_extension("yaml").expect("yaml syntax highlighter not found"),
      &self.highlighter_theme_set.themes[SYNTAX_THEME],
    );
    for (line_num, line) in LinesWithEndings::from(yaml_schema.as_str()).enumerate() {
      let mut line_styles: Vec<(Style, String)> = highlighter
        .highlight_line(line, &self.highlighter_syntax_set)?
        .into_iter()
        .map(|segment| {
          let fg = match segment.0.foreground {
            syntect::highlighting::Color { r, g, b, a } if a > 0 => Some(Color::Rgb(r, g, b)),
            _ => None,
          };
          let fs = segment.0.font_style;
          let mut modifier = Modifier::empty();
          if fs.contains(SyntectFontStyle::BOLD) {
            modifier |= Modifier::BOLD;
          }
          if fs.contains(SyntectFontStyle::ITALIC) {
            modifier |= Modifier::ITALIC;
          }
          if fs.contains(SyntectFontStyle::UNDERLINE) {
            modifier |= Modifier::UNDERLINED;
          }
          let mut style = Style::default().add_modifier(modifier).underline_color(Color::Reset).bg(Color::Reset);
          if let Some(fg) = fg {
            style = style.fg(fg);
          }
          (style, segment.1.to_string())
        })
        .collect();
      line_styles.insert(0, (Style::default().dim(), format!(" {:<3} ", line_num + 1)));
      self.styles.push(line_styles);
    }
    Ok(())
  }

  fn set_styles_by_name(&mut self, schema_name: String) -> Result<()> {
    if let Some(schema) = self.components.get(schema_name.as_str()) {
      self.set_styles(schema.clone())
    } else {
      Ok(())
    }
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn walk(value: serde_json::Value, components: HashMap<String, serde_json::Value>) -> Vec<RenderBlock> {
    let selection = HashMap::new();
    let mut expanding = HashSet::new();
    resolve_walk(&value, "", 0, &components, &selection, &mut expanding)
  }

  #[test]
  fn plain_object_yields_single_yaml_block() {
    let blocks = walk(json!({ "type": "object" }), HashMap::new());
    assert_eq!(blocks.len(), 1, "expected exactly one block");
    match &blocks[0] {
      RenderBlock::Yaml(s) => {
        assert!(s.contains("type: object"), "yaml block did not contain 'type: object': {s}")
      },
      other => panic!("expected Yaml block, got {other:?}"),
    }
  }

  #[test]
  fn ref_to_component_resolves_inline() {
    let mut components = HashMap::new();
    components.insert("Foo".to_string(), json!({ "type": "object", "x-custom": "marker" }));

    let blocks = walk(json!({ "$ref": "#/components/schemas/Foo" }), components);

    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
      RenderBlock::Yaml(s) => {
        assert!(s.contains("type: object"));
        assert!(s.contains("x-custom: marker"));
        assert!(!s.contains("$ref"), "resolved output should not contain $ref");
      },
      other => panic!("expected Yaml, got {other:?}"),
    }
  }
}
