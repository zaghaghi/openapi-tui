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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
  #[default]
  Annotated,
  Yaml,
}

#[derive(Debug, Clone)]
pub enum RenderBlock {
  /// A chunk of YAML text already indented to its target column.
  Yaml(String),
  /// A bracketed annotation rendered with a fixed marker style.
  Marker { indent: usize, text: String },
  /// A tab strip plus the body of the currently-selected variant.
  Variants { id: NodeId, indent: usize, choices: Vec<String>, selected: usize, body_blocks: Vec<RenderBlock> },
  /// A single annotated-mode field line plus optional detail line. The
  /// detail is spliced into `styles` only when the cursor's logical
  /// position lands on this field.
  AnnotatedField { indent: usize, field_line: Vec<(Style, String)>, detail: Option<Vec<(Style, String)>> },
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
  parent_path: &str,
  indent: usize,
  components: &HashMap<String, serde_json::Value>,
  variant_selection: &HashMap<NodeId, usize>,
  expanding: &mut HashSet<String>,
  view_mode: ViewMode,
) -> Vec<RenderBlock> {
  let _ = view_mode; // dispatched in Task 4
  let node = to_node(value, parent_path, components, variant_selection, expanding);
  let mut blocks = Vec::new();
  let mut buf = String::new();
  emit_node(&node, indent, &mut blocks, &mut buf);
  flush_yaml(indent, &mut buf, &mut blocks);
  blocks
}

/// Returns the bare component name if `value` is exactly `{"$ref":
/// "#/components/schemas/<name>"}`, else None. We deliberately reject other
/// `$ref` shapes (parameters, responses, external) — those stay literal.
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
  /// A node that emits a leading block, then a body. Used to attach
  /// `[All of: ...]` markers to merged objects without polluting the
  /// object's key namespace.
  Composed {
    leading: Box<Node>,
    body: Box<Node>,
  },
  Variants {
    id: NodeId,
    choices: Vec<String>,
    selected: usize,
    body: Box<Node>,
  },
}

/// Walks `value`, following refs and producing a `Node` tree.
fn to_node(
  value: &serde_json::Value,
  parent_path: &str,
  components: &HashMap<String, serde_json::Value>,
  variant_selection: &HashMap<NodeId, usize>,
  expanding: &mut HashSet<String>,
) -> Node {
  if let Some(name) = ref_target_name(value) {
    if expanding.contains(name) {
      return Node::Marker(format!("[Recursive {name}]"));
    }
    if let Some(target) = components.get(name) {
      expanding.insert(name.to_string());
      let node = to_node(target, parent_path, components, variant_selection, expanding);
      expanding.remove(name);
      return node;
    }
  }

  if let Some(members) = all_of_members(value) {
    let sources = all_of_sources(members);
    let merged = merge_all_of(members, parent_path, components, variant_selection, expanding);
    return Node::Composed { leading: Box::new(Node::Marker(format!("[All of: {sources}]"))), body: Box::new(merged) };
  }

  if let Some((kind, members)) = composition_members(value) {
    let id = format!("{parent_path}/{kind}");
    let choices = members.iter().enumerate().map(|(i, m)| variant_label(m, i)).collect::<Vec<_>>();
    let max_index = choices.len().saturating_sub(1);
    let selected = variant_selection.get(&id).copied().unwrap_or(0).min(max_index);
    let chosen = members.get(selected).cloned().unwrap_or(serde_json::Value::Null);
    let chosen_path = format!("{id}/{selected}");
    let body = to_node(&chosen, &chosen_path, components, variant_selection, expanding);

    let variants_node = Node::Variants { id, choices, selected, body: Box::new(body) };

    // Sibling keys (anything that is not the composition keyword itself).
    let sibling_pairs: Vec<(String, Node)> = value
      .as_object()
      .map(|m| {
        m.iter()
          .filter(|(k, _)| k.as_str() != kind && k.as_str() != "allOf")
          .map(|(k, v)| {
            let child_path = format!("{parent_path}/{k}");
            (k.clone(), to_node(v, &child_path, components, variant_selection, expanding))
          })
          .collect()
      })
      .unwrap_or_default();

    if sibling_pairs.is_empty() {
      return variants_node;
    }
    return Node::Composed { leading: Box::new(variants_node), body: Box::new(Node::Object(sibling_pairs)) };
  }

  match value {
    serde_json::Value::Object(map) => {
      let mut pairs = Vec::with_capacity(map.len());
      for (k, v) in map {
        let child_path = format!("{parent_path}/{k}");
        pairs.push((k.clone(), to_node(v, &child_path, components, variant_selection, expanding)));
      }
      Node::Object(pairs)
    },
    serde_json::Value::Array(items) => {
      Node::Array(
        items
          .iter()
          .enumerate()
          .map(|(i, v)| {
            let child_path = format!("{parent_path}/{i}");
            to_node(v, &child_path, components, variant_selection, expanding)
          })
          .collect(),
      )
    },
    _ => Node::Scalar(value.clone()),
  }
}

fn all_of_members(value: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
  value.as_object()?.get("allOf")?.as_array()
}

fn all_of_sources(members: &[serde_json::Value]) -> String {
  members
    .iter()
    .map(|m| {
      match ref_target_name(m) {
        Some(name) => name.to_string(),
        None => "<inline>".to_string(),
      }
    })
    .collect::<Vec<_>>()
    .join(", ")
}

fn merge_all_of(
  members: &[serde_json::Value],
  parent_path: &str,
  components: &HashMap<String, serde_json::Value>,
  variant_selection: &HashMap<NodeId, usize>,
  expanding: &mut HashSet<String>,
) -> Node {
  let mut acc: Vec<(String, Node)> = Vec::new();
  let mut seen_keys: HashMap<String, usize> = HashMap::new();
  let mut markers: Vec<Node> = Vec::new();

  for (i, member) in members.iter().enumerate() {
    let member_path = format!("{parent_path}/allOf/{i}");
    let resolved = to_node(member, &member_path, components, variant_selection, expanding);

    // Walk through any nested Composed wrappers so members that resolve to
    // `Composed { Marker, Object }` (allOf-of-allOf via $ref) contribute
    // their underlying pairs to the merge instead of being silently dropped.
    let mut node = resolved;
    let pairs = loop {
      match node {
        Node::Object(pairs) => break Some(pairs),
        Node::Composed { leading, body } => {
          if let Node::Marker(t) = *leading {
            markers.push(Node::Marker(t));
          }
          node = *body;
        },
        Node::Marker(t) => {
          markers.push(Node::Marker(t));
          break None;
        },
        // Scalars/arrays/Variants at allOf member top level are not
        // meaningful for object merging — drop silently.
        _ => break None,
      }
    };

    let Some(pairs) = pairs else { continue };
    for (k, v) in pairs {
      if k == "allOf" {
        continue;
      }
      if let Some(&idx) = seen_keys.get(&k) {
        let prev = std::mem::replace(&mut acc[idx].1, Node::Scalar(serde_json::Value::Null));
        acc[idx].1 = merge_nodes(prev, v);
      } else {
        seen_keys.insert(k.clone(), acc.len());
        acc.push((k, v));
      }
    }
  }

  let body = Node::Object(acc);
  // Prepend collected markers (e.g. from recursive sub-refs) so the outer
  // view shows them above the merged content rather than smuggling them
  // into the object's key namespace under synthetic keys.
  markers.into_iter().rev().fold(body, |acc, marker| Node::Composed { leading: Box::new(marker), body: Box::new(acc) })
}

/// Deep-merge two Nodes. Object + Object recurses by key. Array + Array
/// concatenates with naive (JSON-equality) dedupe. Anything else: source
/// wins (the later allOf member overrides).
fn merge_nodes(target: Node, source: Node) -> Node {
  match (target, source) {
    (Node::Object(mut t), Node::Object(s)) => {
      for (k, v) in s {
        if let Some(pos) = t.iter().position(|(tk, _)| tk == &k) {
          let existing = std::mem::replace(&mut t[pos].1, Node::Scalar(serde_json::Value::Null));
          t[pos].1 = merge_nodes(existing, v);
        } else {
          t.push((k, v));
        }
      }
      Node::Object(t)
    },
    (Node::Array(mut t), Node::Array(s)) => {
      for item in s {
        let key = serde_json::to_string(&node_to_json_lossy(&item)).unwrap_or_default();
        let already = t.iter().any(|e| serde_json::to_string(&node_to_json_lossy(e)).unwrap_or_default() == key);
        if !already {
          t.push(item);
        }
      }
      Node::Array(t)
    },
    (_, source) => source,
  }
}

fn composition_members(value: &serde_json::Value) -> Option<(&'static str, &Vec<serde_json::Value>)> {
  let obj = value.as_object()?;
  if let Some(arr) = obj.get("anyOf").and_then(|v| v.as_array()) {
    return Some(("anyOf", arr));
  }
  if let Some(arr) = obj.get("oneOf").and_then(|v| v.as_array()) {
    return Some(("oneOf", arr));
  }
  None
}

fn variant_label(member: &serde_json::Value, index: usize) -> String {
  if let Some(name) = ref_target_name(member) {
    return name.to_string();
  }
  if let Some(title) = member.as_object().and_then(|o| o.get("title")).and_then(|v| v.as_str()) {
    return title.to_string();
  }
  format!("Variant {}", index + 1)
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
    Node::Composed { leading, body } => {
      emit_node(leading, indent, out, buf);
      flush_yaml(indent, buf, out);
      emit_node(body, indent, out, buf);
    },
    Node::Variants { id, choices, selected, body } => {
      flush_yaml(indent, buf, out);
      let mut body_blocks = Vec::new();
      let mut body_buf = String::new();
      emit_node(body, indent, &mut body_blocks, &mut body_buf);
      flush_yaml(indent, &mut body_buf, &mut body_blocks);
      out.push(RenderBlock::Variants {
        id: id.clone(),
        indent,
        choices: choices.clone(),
        selected: *selected,
        body_blocks,
      });
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
    Node::Variants { .. } => true,
    Node::Scalar(_) => false,
    Node::Array(items) => items.iter().any(contains_marker),
    Node::Object(pairs) => pairs.iter().any(|(_, v)| contains_marker(v)),
    Node::Composed { leading, body } => contains_marker(leading) || contains_marker(body),
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
    Node::Composed { body, .. } => node_to_json_lossy(body),
    Node::Variants { body, .. } => node_to_json_lossy(body),
  }
}

/// Prepend `n` spaces to each non-empty line.
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
  visible_to_logical: Vec<usize>,
  line_offset: usize,

  name_history: Vec<String>,
  line_offset_history: Vec<usize>,

  variant_selection: HashMap<NodeId, usize>,
  variant_scopes: Vec<VariantScope>,
  cached_blocks: Vec<RenderBlock>,
  view_mode: ViewMode,

  highlighter_syntax_set: SyntaxSet,
  highlighter_theme_set: ThemeSet,
}

impl Default for SchemaViewer {
  fn default() -> Self {
    Self {
      components: HashMap::default(),
      styles: Vec::default(),
      visible_to_logical: Vec::default(),
      line_offset: 0,
      name_history: Vec::default(),
      line_offset_history: Vec::default(),
      variant_selection: HashMap::default(),
      variant_scopes: Vec::default(),
      cached_blocks: Vec::default(),
      view_mode: ViewMode::default(),
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
    self.visible_to_logical = vec![];
    self.variant_selection.clear();
    self.variant_scopes.clear();
    self.cached_blocks.clear();
  }

  pub fn set(&mut self, schema: serde_json::Value) -> Result<()> {
    self.line_offset = 0;
    self.name_history = vec![];
    self.line_offset_history = vec![];
    self.variant_selection.clear();
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
    // With $refs auto-resolved inline, the rendered output never contains
    // literal $ref lines for `go()` to land on, so name_history /
    // line_offset_history stay empty in practice. In that case treat
    // `back` as a no-op rather than calling `set()`, which would clear
    // variant_selection and lose the user's anyOf/oneOf choices.
    if self.line_offset_history.is_empty() && self.name_history.is_empty() {
      return Ok(());
    }

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

  pub fn next_variant(&mut self, schema: &serde_json::Value) -> Result<()> {
    self.step_variant(schema, 1)
  }

  pub fn prev_variant(&mut self, schema: &serde_json::Value) -> Result<()> {
    self.step_variant(schema, -1)
  }

  fn step_variant(&mut self, schema: &serde_json::Value, delta: i32) -> Result<()> {
    // Find the innermost (last-pushed, smallest range) scope containing the cursor.
    let cursor = self.line_offset;
    let scope = self
      .variant_scopes
      .iter()
      .filter(|s| s.line_range.contains(&cursor))
      .min_by_key(|s| s.line_range.end - s.line_range.start)
      .cloned();

    let Some(scope) = scope else {
      return Ok(());
    };
    if scope.choice_count == 0 {
      return Ok(());
    }

    let current = self.variant_selection.get(&scope.id).copied().unwrap_or(0);
    let count = scope.choice_count as i32;
    let next = ((current as i32 + delta).rem_euclid(count)) as usize;
    self.variant_selection.insert(scope.id, next);

    self.set_styles(schema.clone())?;
    // The new selected variant may have a shorter body than the previous
    // one; clamp the cursor so it stays inside the new line range and the
    // user can keep pressing `,` / `.` without first scrolling.
    self.line_offset = self.line_offset.min(self.styles.len().saturating_sub(1));
    Ok(())
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
    self.visible_to_logical = vec![];
    self.variant_scopes = vec![];

    let mut expanding = HashSet::new();
    self.cached_blocks =
      resolve_walk(&schema, "", 0, &self.components, &self.variant_selection, &mut expanding, self.view_mode);

    let blocks = std::mem::take(&mut self.cached_blocks);
    let mut logical = 0usize;
    let result = self.render_blocks(&blocks, &mut logical);
    self.cached_blocks = blocks;
    result?;
    Ok(())
  }

  fn render_blocks(&mut self, blocks: &[RenderBlock], logical: &mut usize) -> Result<()> {
    for block in blocks {
      match block {
        RenderBlock::Yaml(text) => self.render_yaml_block(text, logical)?,
        RenderBlock::Marker { indent, text } => self.render_marker(*indent, text, logical),
        RenderBlock::Variants { id, indent, choices, selected, body_blocks } => {
          let start = *logical;
          self.render_variant_strip(*indent, choices, *selected, logical);
          self.render_blocks(body_blocks, logical)?;
          let end = *logical;
          self.variant_scopes.push(VariantScope {
            line_range: start..end,
            id: id.clone(),
            choice_count: choices.len(),
          });
        },
        RenderBlock::AnnotatedField { indent: _, field_line, detail } => {
          self.push_styled_line(&mut field_line.clone(), *logical);
          if let Some(detail_line) = detail {
            if *logical == self.line_offset {
              self.push_styled_line(&mut detail_line.clone(), *logical);
            }
          }
          *logical += 1;
        },
      }
    }
    Ok(())
  }

  fn render_yaml_block(&mut self, text: &str, logical: &mut usize) -> Result<()> {
    let mut highlighter = HighlightLines::new(
      self.highlighter_syntax_set.find_syntax_by_extension("yaml").expect("yaml syntax highlighter not found"),
      &self.highlighter_theme_set.themes[SYNTAX_THEME],
    );
    for line in LinesWithEndings::from(text) {
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
      let line_num = self.styles.len() + 1;
      line_styles.insert(0, (Style::default().dim(), format!(" {:<3} ", line_num)));
      self.styles.push(line_styles);
      self.visible_to_logical.push(*logical);
      *logical += 1;
    }
    Ok(())
  }

  fn render_marker(&mut self, indent: usize, text: &str, logical: &mut usize) {
    let mut line_styles = vec![
      (Style::default(), " ".repeat(indent)),
      (Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC).add_modifier(Modifier::DIM), text.to_string()),
    ];
    self.push_styled_line(&mut line_styles, *logical);
    *logical += 1;
  }

  fn render_variant_strip(&mut self, indent: usize, choices: &[String], selected: usize, logical: &mut usize) {
    let mut line_styles: Vec<(Style, String)> = vec![(Style::default(), " ".repeat(indent))];
    for (i, choice) in choices.iter().enumerate() {
      if i > 0 {
        line_styles.push((Style::default().add_modifier(Modifier::DIM), " · ".to_string()));
      }
      if i == selected {
        line_styles.push((Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED), format!("[{choice}]")));
      } else {
        line_styles.push((Style::default().add_modifier(Modifier::DIM), choice.clone()));
      }
    }
    self.push_styled_line(&mut line_styles, *logical);
    *logical += 1;
  }

  fn push_styled_line(&mut self, line_styles: &mut Vec<(Style, String)>, logical: usize) {
    let line_num = self.styles.len() + 1;
    line_styles.insert(0, (Style::default().dim(), format!(" {:<3} ", line_num)));
    self.styles.push(std::mem::take(line_styles));
    self.visible_to_logical.push(logical);
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
    resolve_walk(&value, "", 0, &components, &selection, &mut expanding, ViewMode::Yaml)
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

  #[test]
  fn any_of_produces_variants_block() {
    let mut components = HashMap::new();
    components.insert("A".to_string(), json!({ "type": "object", "x-tag": "a" }));
    components.insert("B".to_string(), json!({ "type": "object", "x-tag": "b" }));

    let value = json!({
      "anyOf": [
        { "$ref": "#/components/schemas/A" },
        { "$ref": "#/components/schemas/B" }
      ]
    });

    let blocks = walk(value, components);

    let variants = blocks
      .iter()
      .find_map(|b| {
        match b {
          RenderBlock::Variants { choices, selected, body_blocks, .. } => Some((choices, *selected, body_blocks)),
          _ => None,
        }
      })
      .expect("expected a Variants block");

    assert_eq!(variants.0, &vec!["A".to_string(), "B".to_string()]);
    assert_eq!(variants.1, 0);
    // Body should contain x-tag: a, not x-tag: b.
    let body_yaml: String = variants
      .2
      .iter()
      .filter_map(|b| {
        match b {
          RenderBlock::Yaml(s) => Some(s.as_str()),
          _ => None,
        }
      })
      .collect();
    assert!(body_yaml.contains("x-tag: a"), "body did not have selected variant: {body_yaml}");
    assert!(!body_yaml.contains("x-tag: b"), "body leaked unselected variant: {body_yaml}");
  }

  #[test]
  fn all_of_emits_marker_and_merged_yaml() {
    let mut components = HashMap::new();
    components.insert("Pet".to_string(), json!({ "type": "object", "properties": { "name": { "type": "string" } } }));

    let value = json!({
      "allOf": [
        { "$ref": "#/components/schemas/Pet" },
        { "type": "object", "properties": { "bark": { "type": "string" } } }
      ]
    });

    let blocks = walk(value, components);

    // Find the marker
    let marker_idx = blocks
      .iter()
      .position(|b| matches!(b, RenderBlock::Marker { text, .. } if text == "[All of: Pet, <inline>]"))
      .expect("missing [All of: Pet, <inline>] marker");
    // After the marker, there should be at least one Yaml block containing
    // both "name" and "bark" properties.
    let after_yaml: String = blocks
      .iter()
      .skip(marker_idx + 1)
      .filter_map(|b| {
        match b {
          RenderBlock::Yaml(s) => Some(s.as_str()),
          _ => None,
        }
      })
      .collect();
    assert!(after_yaml.contains("name:"), "merged yaml missing 'name': {after_yaml}");
    assert!(after_yaml.contains("bark:"), "merged yaml missing 'bark': {after_yaml}");
  }

  #[test]
  fn any_of_with_sibling_keys_preserves_them() {
    let value = json!({
      "anyOf": [{ "type": "object", "title": "X" }],
      "description": "hello",
      "nullable": true,
    });

    let blocks = walk(value, HashMap::new());

    let mut saw_variants = false;
    let mut saw_sibling_yaml = false;
    for b in &blocks {
      match b {
        RenderBlock::Variants { .. } => saw_variants = true,
        RenderBlock::Yaml(s) if s.contains("description: hello") && s.contains("nullable: true") => {
          saw_sibling_yaml = true;
        },
        _ => {},
      }
    }
    assert!(saw_variants, "expected a Variants block");
    assert!(saw_sibling_yaml, "expected a Yaml block with sibling keys: {blocks:#?}");
  }

  #[test]
  fn variants_have_pointer_path_ids() {
    let value = json!({
      "type": "object",
      "properties": {
        "field": {
          "anyOf": [{ "type": "string" }, { "type": "integer" }]
        }
      }
    });

    let blocks = walk(value, HashMap::new());

    // Find the Variants block by recursing into the IR.
    fn find_variants(blocks: &[RenderBlock]) -> Option<&RenderBlock> {
      for b in blocks {
        if let RenderBlock::Variants { body_blocks, .. } = b {
          return Some(b).or_else(|| find_variants(body_blocks));
        }
      }
      None
    }
    let v = find_variants(&blocks).expect("expected Variants somewhere");
    match v {
      RenderBlock::Variants { id, .. } => {
        assert_eq!(id, "/properties/field/anyOf");
      },
      _ => unreachable!(),
    }
  }

  #[test]
  fn variant_selection_is_honored() {
    let value = json!({
      "anyOf": [
        { "type": "object", "x-tag": "first" },
        { "type": "object", "x-tag": "second" }
      ]
    });

    let mut selection = HashMap::new();
    selection.insert("/anyOf".to_string(), 1);

    let mut expanding = HashSet::new();
    let blocks = resolve_walk(&value, "", 0, &HashMap::new(), &selection, &mut expanding, ViewMode::Yaml);

    let v = blocks
      .iter()
      .find_map(|b| {
        match b {
          RenderBlock::Variants { selected, body_blocks, .. } => Some((*selected, body_blocks)),
          _ => None,
        }
      })
      .expect("expected Variants");
    assert_eq!(v.0, 1);
    let body_yaml: String = v
      .1
      .iter()
      .filter_map(|b| {
        match b {
          RenderBlock::Yaml(s) => Some(s.as_str()),
          _ => None,
        }
      })
      .collect();
    assert!(body_yaml.contains("x-tag: second"), "expected second variant in body: {body_yaml}");
  }

  /// Walk every component schema in a real OpenAPI spec through `resolve_walk`
  /// and assert the resolution pipeline doesn't panic, return errors, or
  /// produce empty output for non-empty inputs. Used as an end-to-end smoke
  /// test against `examples/petstore.json` and `examples/stripe/spec.yml`.
  fn smoke_test_spec(path: &str) {
    let raw = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    let spec: serde_json::Value = if path.ends_with(".yml") || path.ends_with(".yaml") {
      serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {path} as YAML: {e}"))
    } else {
      serde_json::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {path} as JSON: {e}"))
    };

    let components: HashMap<String, serde_json::Value> = spec
      .get("components")
      .and_then(|c| c.get("schemas"))
      .and_then(|s| s.as_object())
      .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
      .unwrap_or_default();

    assert!(!components.is_empty(), "{path} has no components.schemas — wrong spec?");

    let selection: HashMap<NodeId, usize> = HashMap::new();
    let mut walked = 0usize;
    for (name, schema) in &components {
      let mut expanding = HashSet::new();
      let blocks = resolve_walk(schema, "", 0, &components, &selection, &mut expanding, ViewMode::Yaml);
      assert!(
        !blocks.is_empty(),
        "schema `{name}` from {path} produced 0 RenderBlocks (likely a serialization error swallowed somewhere)"
      );
      assert!(expanding.is_empty(), "schema `{name}` from {path} left state in `expanding`: {expanding:?}");
      walked += 1;
    }
    assert!(walked > 0);
    eprintln!("[smoke] walked {walked} schemas from {path} without panic");
  }

  #[test]
  fn all_of_with_self_referential_member_does_not_emit_synthetic_key() {
    // Regression: previously `merge_all_of` smuggled Markers from
    // recursive members into the merged Object as `__marker_N` keys,
    // which leaked into the rendered YAML as literal `__marker_0:` lines.
    let mut components = HashMap::new();
    components.insert(
      "Self".to_string(),
      json!({
        "allOf": [
          { "$ref": "#/components/schemas/Self" },
          { "type": "object", "properties": { "name": { "type": "string" } } }
        ]
      }),
    );

    let blocks = walk(json!({ "$ref": "#/components/schemas/Self" }), components);

    let yaml: String = blocks
      .iter()
      .filter_map(|b| {
        match b {
          RenderBlock::Yaml(s) => Some(s.as_str()),
          _ => None,
        }
      })
      .collect();

    assert!(!yaml.contains("__marker_"), "synthetic __marker_N key leaked into rendered yaml:\n{yaml}");

    // The recursion marker should still be present (just not attached to
    // a fake key).
    let has_recursive_marker =
      blocks.iter().any(|b| matches!(b, RenderBlock::Marker { text, .. } if text == "[Recursive Self]"));
    assert!(has_recursive_marker, "expected [Recursive Self] marker in blocks: {blocks:#?}");
  }

  #[test]
  fn all_of_member_that_resolves_to_another_all_of_is_merged() {
    // Regression: previously a `Composed { Marker, Object }` member
    // (i.e. an allOf member whose $ref target is itself an allOf schema)
    // was silently dropped from the merge. The fix walks Composed
    // wrappers down to the underlying Object.
    let mut components = HashMap::new();
    components.insert(
      "Inner".to_string(),
      json!({
        "allOf": [
          { "type": "object", "properties": { "inner_field": { "type": "string" } } }
        ]
      }),
    );

    let value = json!({
      "allOf": [
        { "$ref": "#/components/schemas/Inner" },
        { "type": "object", "properties": { "outer_field": { "type": "integer" } } }
      ]
    });

    let blocks = walk(value, components);
    let yaml: String = blocks
      .iter()
      .filter_map(|b| {
        match b {
          RenderBlock::Yaml(s) => Some(s.as_str()),
          _ => None,
        }
      })
      .collect();

    assert!(yaml.contains("inner_field"), "inner_field from nested allOf was dropped:\n{yaml}");
    assert!(yaml.contains("outer_field"), "outer_field from outer allOf is missing:\n{yaml}");
  }

  #[test]
  fn e2e_petstore_all_schemas_resolve() {
    smoke_test_spec("examples/petstore.json");
  }

  /// Stripe's `account_business_profile` schema is the canonical
  /// sibling-keys-with-anyOf pattern: properties like `support_address`
  /// are `anyOf: [$ref Address]` plus sibling `description` and
  /// `nullable`. Confirm the resolved tree contains a Variants block
  /// with `address` as a choice for `support_address`. We walk only
  /// this schema (not the much larger `account` schema) because Stripe
  /// has dense cross-references that produce O(branch^depth) blowup
  /// without memoization — see follow-up note in the design doc risks.
  #[test]
  fn e2e_stripe_account_business_profile_shape() {
    let raw = std::fs::read_to_string("examples/stripe/spec.yml").expect("read stripe spec");
    let spec: serde_json::Value = serde_yaml::from_str(&raw).expect("parse stripe spec");
    let components: HashMap<String, serde_json::Value> = spec
      .get("components")
      .and_then(|c| c.get("schemas"))
      .and_then(|s| s.as_object())
      .expect("components.schemas")
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect();

    let abp = components.get("account_business_profile").expect("stripe components.schemas.account_business_profile");
    let selection = HashMap::new();
    let mut expanding = HashSet::new();
    let blocks = resolve_walk(abp, "", 0, &components, &selection, &mut expanding, ViewMode::Yaml);

    fn find_variants_with_choice<'a>(blocks: &'a [RenderBlock], choice: &str) -> Option<&'a RenderBlock> {
      for b in blocks {
        if let RenderBlock::Variants { choices, body_blocks, .. } = b {
          if choices.iter().any(|c| c == choice) {
            return Some(b);
          }
          if let Some(inner) = find_variants_with_choice(body_blocks, choice) {
            return Some(inner);
          }
        }
      }
      None
    }

    assert!(
      find_variants_with_choice(&blocks, "address").is_some(),
      "expected Variants block with choice `address` (from support_address.anyOf)"
    );
    assert!(
      find_variants_with_choice(&blocks, "account_annual_revenue").is_some(),
      "expected Variants block with choice `account_annual_revenue` (from annual_revenue.anyOf)"
    );
  }

  /// Walks a curated set of small Stripe component schemas as a smoke
  /// test against real-world OpenAPI shapes. We avoid `account` and
  /// other massive schemas because the resolution pipeline has no
  /// memoization and exhibits exponential blowup on highly
  /// cross-referenced trees.
  #[test]
  fn e2e_stripe_sample_schemas_resolve() {
    let raw = std::fs::read_to_string("examples/stripe/spec.yml").expect("read stripe spec");
    let spec: serde_json::Value = serde_yaml::from_str(&raw).expect("parse stripe spec");
    let components: HashMap<String, serde_json::Value> = spec
      .get("components")
      .and_then(|c| c.get("schemas"))
      .and_then(|s| s.as_object())
      .expect("components.schemas")
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect();

    let sample = ["address", "account_annual_revenue", "account_monthly_estimated_revenue", "account_business_profile"];

    for name in &sample {
      let schema = components.get(*name).unwrap_or_else(|| panic!("missing stripe schema `{name}`"));
      let selection = HashMap::new();
      let mut expanding = HashSet::new();
      let blocks = resolve_walk(schema, "", 0, &components, &selection, &mut expanding, ViewMode::Yaml);
      assert!(!blocks.is_empty(), "stripe schema `{name}` produced 0 blocks");
      assert!(expanding.is_empty(), "stripe schema `{name}` left state in expanding: {expanding:?}");
    }
  }

  #[test]
  fn annotated_field_detail_appears_only_when_cursor_on_field() {
    use ratatui::style::Style;
    let blocks = vec![
      RenderBlock::AnnotatedField {
        indent: 0,
        field_line: vec![(Style::default(), "id (integer)?: 10".to_string())],
        detail: Some(vec![(Style::default(), "  format: int64".to_string())]),
      },
      RenderBlock::AnnotatedField {
        indent: 0,
        field_line: vec![(Style::default(), "name (string): \"doggie\"".to_string())],
        detail: None,
      },
    ];
    let mut viewer = SchemaViewer { cached_blocks: blocks, ..Default::default() };

    // Cursor on first field (logical 0) → detail visible (3 styled lines: field, detail, field)
    viewer.line_offset = 0;
    viewer.styles = vec![];
    viewer.visible_to_logical = vec![];
    viewer.variant_scopes = vec![];
    let blocks = std::mem::take(&mut viewer.cached_blocks);
    let mut logical = 0usize;
    viewer.render_blocks(&blocks, &mut logical).unwrap();
    viewer.cached_blocks = blocks;
    assert_eq!(viewer.styles.len(), 3, "cursor on field 0 should splice detail");
    assert_eq!(viewer.visible_to_logical, vec![0, 0, 1]);

    // Cursor on second field (logical 1) → no detail (2 styled lines: field, field)
    viewer.line_offset = 1;
    viewer.styles = vec![];
    viewer.visible_to_logical = vec![];
    let blocks = std::mem::take(&mut viewer.cached_blocks);
    let mut logical = 0usize;
    viewer.render_blocks(&blocks, &mut logical).unwrap();
    viewer.cached_blocks = blocks;
    assert_eq!(viewer.styles.len(), 2, "cursor on field 1, no detail field");
    assert_eq!(viewer.visible_to_logical, vec![0, 1]);
  }
}
