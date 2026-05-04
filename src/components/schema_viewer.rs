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
  let node = to_node(value, components, expanding);
  let mut blocks = Vec::new();
  let mut buf = String::new();
  emit_node(&node, indent, &mut blocks, &mut buf);
  flush_yaml(indent, &mut buf, &mut blocks);
  blocks
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

/// Intermediate tree built from the source schema with refs resolved and
/// composition keywords replaced by sentinel nodes. We do not stuff Markers
/// or Variants into `serde_json::Value` because they cannot be rendered as
/// valid YAML; instead we keep our own enum and emit YAML for the plain
/// branches piecewise.
#[derive(Debug, Clone)]
enum Node {
  Scalar(serde_json::Value),
  Object(Vec<(String, Node)>),
  Array(Vec<Node>),
  Marker(String),
}

/// Walks `value`, following refs and producing a `Node` tree. Composition
/// (`allOf` / `anyOf` / `oneOf`) is handled in later tasks; for now a
/// composition keyword is treated as an ordinary object key.
fn to_node(
  value: &serde_json::Value,
  components: &HashMap<String, serde_json::Value>,
  expanding: &mut HashSet<String>,
) -> Node {
  if let Some(name) = ref_target_name(value) {
    if expanding.contains(name) {
      return Node::Marker(format!("[Recursive {name}]"));
    }
    if let Some(target) = components.get(name) {
      expanding.insert(name.to_string());
      let node = to_node(target, components, expanding);
      expanding.remove(name);
      return node;
    }
  }

  match value {
    serde_json::Value::Object(map) => {
      let mut pairs = Vec::with_capacity(map.len());
      for (k, v) in map {
        pairs.push((k.clone(), to_node(v, components, expanding)));
      }
      Node::Object(pairs)
    },
    serde_json::Value::Array(items) => Node::Array(items.iter().map(|v| to_node(v, components, expanding)).collect()),
    _ => Node::Scalar(value.clone()),
  }
}

/// Walks a Node tree and produces RenderBlocks. Pure branches (no markers)
/// are coalesced into single Yaml blocks via the shared `buf` so they
/// render through one syntect highlighter pass at render time.
fn emit_node(node: &Node, indent: usize, out: &mut Vec<RenderBlock>, buf: &mut String) {
  match node {
    Node::Marker(text) => {
      flush_yaml(indent, buf, out);
      out.push(RenderBlock::Marker { indent, text: text.clone() });
    },
    Node::Scalar(_) | Node::Array(_) => {
      // A bare scalar or array at this entry point is unusual — schemas are
      // objects in practice. Render via the lossy JSON view.
      let json = node_to_json_lossy(node);
      if let Ok(yaml) = serde_yaml::to_string(&json) {
        buf.push_str(&yaml);
      }
    },
    Node::Object(pairs) => {
      for (key, value) in pairs {
        if contains_marker(value) {
          flush_yaml(indent, buf, out);
          // Emit the key line on its own (already indented for the Yaml block).
          out.push(RenderBlock::Yaml(format!("{}{key}:\n", " ".repeat(indent))));
          // Recurse into the value at indent + 2.
          emit_node(value, indent + 2, out, buf);
          flush_yaml(indent + 2, buf, out);
        } else {
          // Pure subtree — serialize the {key: value} pair as YAML and
          // append. Trailing newline from serde_yaml is preserved.
          let mut one = serde_json::Map::new();
          one.insert(key.clone(), node_to_json_lossy(value));
          if let Ok(yaml) = serde_yaml::to_string(&serde_json::Value::Object(one)) {
            buf.push_str(&yaml);
          }
        }
      }
    },
  }
}

fn flush_yaml(indent: usize, buf: &mut String, out: &mut Vec<RenderBlock>) {
  if buf.is_empty() {
    return;
  }
  let text = std::mem::take(buf);
  out.push(RenderBlock::Yaml(indent_lines(&text, indent)));
}

fn contains_marker(node: &Node) -> bool {
  match node {
    Node::Marker(_) => true,
    Node::Scalar(_) => false,
    Node::Array(items) => items.iter().any(contains_marker),
    Node::Object(pairs) => pairs.iter().any(|(_, v)| contains_marker(v)),
  }
}

/// Lossy JSON view of a Node: Markers become string literals. Only called
/// from branches where `contains_marker` returned false, so the lossiness
/// is unreachable in practice.
fn node_to_json_lossy(node: &Node) -> serde_json::Value {
  match node {
    Node::Marker(t) => serde_json::Value::String(t.clone()),
    Node::Scalar(v) => v.clone(),
    Node::Array(items) => serde_json::Value::Array(items.iter().map(node_to_json_lossy).collect()),
    Node::Object(pairs) => {
      let mut m = serde_json::Map::new();
      for (k, v) in pairs {
        m.insert(k.clone(), node_to_json_lossy(v));
      }
      serde_json::Value::Object(m)
    },
  }
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

  #[test]
  fn recursive_ref_emits_marker() {
    let mut components = HashMap::new();
    components.insert("Loop".to_string(), json!({ "$ref": "#/components/schemas/Loop" }));

    let blocks = walk(json!({ "$ref": "#/components/schemas/Loop" }), components);

    assert_eq!(blocks.len(), 1, "expected exactly one block: {blocks:#?}");
    match &blocks[0] {
      RenderBlock::Marker { text, indent: _ } => assert_eq!(text, "[Recursive Loop]"),
      other => panic!("expected Marker, got {other:?}"),
    }
  }

  #[test]
  fn nested_ref_inside_properties_is_resolved() {
    let mut components = HashMap::new();
    components.insert("Address".to_string(), json!({ "type": "object", "x-custom": "addr" }));

    let value = json!({
      "type": "object",
      "properties": {
        "address": { "$ref": "#/components/schemas/Address" },
      },
    });

    let blocks = walk(value, components);

    let yaml = blocks
      .iter()
      .filter_map(|b| {
        match b {
          RenderBlock::Yaml(s) => Some(s.as_str()),
          _ => None,
        }
      })
      .collect::<String>();

    assert!(yaml.contains("x-custom: addr"), "nested ref was not resolved; full yaml:\n{yaml}");
    assert!(!yaml.contains("$ref"), "yaml still contains literal $ref:\n{yaml}");
  }
}
