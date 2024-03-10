use std::{
  collections::BTreeMap,
  sync::{Arc, RwLock},
};

use color_eyre::eyre::Result;
use oas3::Schema;
use ratatui::{prelude::*, widgets::*};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings};

use crate::pages::home::State;

const SYNTAX_THEME: &str = "Solarized (dark)";

#[derive(Default)]
pub struct SchemaViewer {
  components: BTreeMap<String, Schema>,
  styles: Vec<Vec<(Style, String)>>,
  line_offset: usize,

  name_history: Vec<String>,
  line_offset_history: Vec<usize>,

  highlighter_syntax_set: SyntaxSet,
  highlighter_theme_set: ThemeSet,
}

impl SchemaViewer {
  pub fn new(components: BTreeMap<String, Schema>) -> Self {
    Self {
      components,
      styles: Vec::default(),
      line_offset: 0,
      name_history: Vec::default(),
      line_offset_history: Vec::default(),
      highlighter_syntax_set: SyntaxSet::load_defaults_newlines(),
      highlighter_theme_set: ThemeSet::load_defaults(),
    }
  }

  pub fn from(state: Arc<RwLock<State>>) -> Self {
    let state_reader = state.read().unwrap();
    Self::new(BTreeMap::from_iter(
      state_reader
        .openapi_spec
        .components
        .as_ref()
        .unwrap()
        .schemas
        .iter()
        .filter_map(|(key, value)| value.resolve(&state_reader.openapi_spec).ok().map(|schema| (key.clone(), schema))),
    ))
  }

  pub fn set(&mut self, schema: Schema) -> Result<()> {
    self.line_offset = 0;
    self.name_history = vec![];
    self.line_offset_history = vec![];
    self.set_styles(schema)
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

  pub fn back(&mut self, schema: Schema) -> Result<()> {
    log::info!("{:?}", self.line_offset_history);
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
    self.line_offset = self.line_offset.saturating_add(1).min(self.styles.len() - 1);
  }

  pub fn up(&mut self) {
    self.line_offset = self.line_offset.saturating_sub(1);
  }

  pub fn schema_path(&self) -> Vec<String> {
    self.name_history.clone()
  }

  pub fn render_widget(&self, frame: &mut Frame<'_>, area: Rect) {
    let lines = self.styles.iter().map(|items| {
      return Line::from(
        items
          .iter()
          .map(|item| {
            return Span::styled(&item.1, item.0.bg(Color::Reset));
          })
          .collect::<Vec<_>>(),
      );
    });
    let mut list_state = ListState::default().with_selected(Some(self.line_offset));

    frame.render_stateful_widget(
      List::new(lines).highlight_symbol(symbols::scrollbar::HORIZONTAL.end).highlight_spacing(HighlightSpacing::Always),
      area,
      &mut list_state,
    );
  }

  fn set_styles(&mut self, schema: Schema) -> Result<()> {
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
    let schema = self.components.get(schema_name.as_str()).unwrap();
    self.set_styles(schema.clone())
  }
}