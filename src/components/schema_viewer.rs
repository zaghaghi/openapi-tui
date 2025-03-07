use std::collections::HashMap;

use color_eyre::eyre::Result;
use ratatui::{prelude::*, widgets::*};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings};

use crate::state::State;

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
          (
            syntect_tui::translate_style(segment.0)
              .ok()
              .unwrap_or_default()
              .underline_color(Color::Reset)
              .bg(Color::Reset),
            segment.1.to_string(),
          )
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
