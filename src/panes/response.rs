use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use oas3::Schema;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};
use syntect::{
  easy::HighlightLines,
  highlighting::ThemeSet,
  parsing::{SyntaxReference, SyntaxSet},
  util::LinesWithEndings,
};

use crate::{
  action::Action,
  pages::home::State,
  panes::Pane,
  tui::{EventResponse, Frame},
};

pub struct ResponseType {
  status: String,
  media_type: String,
  schema: Schema,
}

#[derive(Default)]
pub struct ResponsePane {
  focused: bool,
  focused_border_style: Style,
  state: Arc<RwLock<State>>,
  response_schemas: Vec<ResponseType>,
  response_schemas_index: usize,
  response_schemas_styles: Vec<Vec<(Style, String)>>,
  response_schema_line_offset: usize,
  highlighter_syntax_set: SyntaxSet,
  highlighter_theme_set: ThemeSet,
}

impl ResponsePane {
  pub fn new(state: Arc<RwLock<State>>, focused: bool, focused_border_style: Style) -> Self {
    Self {
      state,
      focused,
      focused_border_style,
      response_schemas: Vec::default(),
      response_schemas_index: 0,
      response_schemas_styles: Vec::default(),
      response_schema_line_offset: 0,
      highlighter_syntax_set: SyntaxSet::load_defaults_newlines(),
      highlighter_theme_set: ThemeSet::load_defaults(),
    }
  }

  fn yaml_syntax(&self) -> &SyntaxReference {
    return self.highlighter_syntax_set.find_syntax_by_extension("yaml").unwrap();
  }

  fn border_style(&self) -> Style {
    match self.focused {
      true => self.focused_border_style,
      false => Style::default(),
    }
  }

  fn border_type(&self) -> BorderType {
    match self.focused {
      true => BorderType::Thick,
      false => BorderType::Plain,
    }
  }

  fn status_color(&self, status: &str) -> Color {
    if status.starts_with('2') || status.starts_with("default") {
      return Color::LightCyan;
    }
    if status.starts_with('3') {
      return Color::LightBlue;
    }
    if status.starts_with('4') {
      return Color::LightYellow;
    }
    if status.starts_with('5') {
      return Color::LightRed;
    }
    Color::default()
  }

  fn set_response_schema_styles(&mut self) -> Result<()> {
    self.response_schemas_styles = Vec::default();
    if let Some(response_type) = self.response_schemas.get(self.response_schemas_index) {
      let yaml_schema = serde_yaml::to_string(&response_type.schema)?;
      let mut highlighter =
        HighlightLines::new(self.yaml_syntax(), &self.highlighter_theme_set.themes["Solarized (dark)"]);
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
        self.response_schemas_styles.push(line_styles);
      }
    }
    Ok(())
  }

  fn init_response_schema(&mut self) -> Result<()> {
    {
      self.response_schemas_styles = vec![];
      self.response_schema_line_offset = 0;
      let state = self.state.read().unwrap();
      if let Some((_path, _method, operation)) = state.active_operation() {
        self.response_schemas = operation
          .responses
          .iter()
          .filter_map(|(key, value)| value.resolve(&state.openapi_spec).map_or(None, |v| Some((key.to_string(), v))))
          .flat_map(|(key, response)| {
            response
              .content
              .iter()
              .filter_map(|(media_type, media)| {
                media.schema(&state.openapi_spec).map_or(None, |schema| {
                  Some(ResponseType { status: key.clone(), media_type: media_type.to_string(), schema })
                })
              })
              .collect::<Vec<ResponseType>>()
          })
          .collect();
      }
    }
    self.set_response_schema_styles()?;
    Ok(())
  }
}
impl Pane for ResponsePane {
  fn init(&mut self) -> Result<()> {
    self.init_response_schema()?;
    Ok(())
  }

  fn focus(&mut self) -> Result<()> {
    self.focused = true;
    Ok(())
  }

  fn unfocus(&mut self) -> Result<()> {
    self.focused = false;
    Ok(())
  }

  fn handle_key_events(&mut self, _key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  #[allow(unused_variables)]
  fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::Update => {
        self.response_schemas_index = 0;
        self.init_response_schema()?;
      },
      Action::Down => {
        self.response_schema_line_offset =
          self.response_schema_line_offset.saturating_add(1).min(self.response_schemas_styles.len() - 1);
      },
      Action::Up => {
        self.response_schema_line_offset = self.response_schema_line_offset.saturating_sub(1);
      },
      Action::Tab(index) if index < self.response_schemas.len().try_into()? => {
        self.response_schemas_index = index.try_into()?;
        self.init_response_schema()?;
      },
      Action::Submit => {},
      _ => {},
    }

    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let inner_margin: Margin = Margin { horizontal: 1, vertical: 1 };

    let inner = area.inner(&inner_margin);
    frame.render_widget(
      Tabs::new(self.response_schemas.iter().map(|resp| {
        Span::styled(
          format!("{} [{}]", resp.status, resp.media_type),
          Style::default().fg(self.status_color(resp.status.as_str())).dim(),
        )
      }))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).not_dim())
      .select(self.response_schemas_index),
      inner,
    );

    let inner_margin: Margin = Margin { horizontal: 1, vertical: 1 };
    let mut inner = inner.inner(&inner_margin);
    inner.height = inner.height.saturating_add(1);
    let lines = self.response_schemas_styles.iter().map(|items| {
      return Line::from(
        items
          .iter()
          .map(|item| {
            return Span::styled(&item.1, item.0.bg(Color::Reset));
          })
          .collect::<Vec<_>>(),
      );
    });
    let mut list_state = ListState::default().with_selected(Some(self.response_schema_line_offset));

    frame.render_stateful_widget(
      List::new(lines).highlight_symbol(symbols::scrollbar::HORIZONTAL.end).highlight_spacing(HighlightSpacing::Always),
      inner,
      &mut list_state,
    );
    frame.render_widget(
      Block::default()
        .title("Responses")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type()),
      area,
    );
    Ok(())
  }
}
