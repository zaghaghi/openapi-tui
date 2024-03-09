use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use oas3::{spec::RequestBody, Schema};
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

pub struct RequestType {
  location: String,
  media_type: String,
  schema: Schema,
  title: String,
}

#[derive(Default)]
pub struct RequestPane {
  focused: bool,
  focused_border_style: Style,
  state: Arc<RwLock<State>>,

  request_schemas: Vec<RequestType>,
  request_schemas_index: usize,
  request_schemas_styles: Vec<Vec<(Style, String)>>,
  request_schema_line_offset: usize,

  highlighter_syntax_set: SyntaxSet,
  highlighter_theme_set: ThemeSet,
}

impl RequestPane {
  pub fn new(state: Arc<RwLock<State>>, focused: bool, focused_border_style: Style) -> Self {
    Self {
      state,
      focused,
      focused_border_style,
      request_schemas: Vec::default(),
      request_schemas_index: 0,
      request_schemas_styles: Vec::default(),
      request_schema_line_offset: 0,
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

  fn location_color(&self, status: &str) -> Color {
    if status.starts_with("header") {
      return Color::LightCyan;
    }
    if status.starts_with("path") {
      return Color::LightBlue;
    }
    if status.starts_with("query") {
      return Color::LightMagenta;
    }
    if status.starts_with("body") {
      return Color::LightYellow;
    }
    Color::default()
  }

  fn set_request_schema_styles(&mut self) -> Result<()> {
    self.request_schemas_styles = Vec::default();
    if let Some(request_type) = self.request_schemas.get(self.request_schemas_index) {
      let yaml_schema = serde_yaml::to_string(&request_type.schema)?;
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
        self.request_schemas_styles.push(line_styles);
      }
    }
    Ok(())
  }

  fn init_request_schema(&mut self) -> Result<()> {
    {
      self.request_schemas_styles = vec![];
      self.request_schema_line_offset = 0;
      let state = self.state.read().unwrap();
      if let Some((_path, _method, operation)) = state.active_operation() {
        let mut request_schemas: Vec<RequestType> = vec![];
        if let Some(request_body) = &operation.request_body {
          request_schemas = request_body
            .resolve(&state.openapi_spec)
            .unwrap_or(RequestBody::default())
            .content
            .iter()
            .filter_map(|(media_type, media)| {
              media.schema(&state.openapi_spec).map_or(None, |schema| {
                Some(RequestType {
                  location: String::from("body"),
                  media_type: media_type.to_string(),
                  title: schema.title.clone().unwrap_or("Body".to_string()),
                  schema,
                })
              })
            })
            .collect();
        }
        request_schemas.extend(
          operation.parameters.iter().filter_map(|parameter| parameter.resolve(&state.openapi_spec).ok()).map(
            |parameter| {
              RequestType {
                location: parameter.location,
                media_type: parameter.param_type.unwrap_or_default(),
                schema: parameter.schema.unwrap_or_default(),
                title: parameter.name,
              }
            },
          ),
        );
        self.request_schemas = request_schemas;
      }
    }
    self.set_request_schema_styles()?;
    Ok(())
  }

  fn legend_line(&mut self) -> Line<'_> {
    Line::from(vec![
      Span::raw("[ "),
      Span::styled("Body".to_string(), self.location_color("body")),
      Span::raw(format!(" {} ", symbols::DOT)),
      Span::styled("Path".to_string(), self.location_color("path")),
      Span::raw(format!(" {} ", symbols::DOT)),
      Span::styled("Query".to_string(), self.location_color("query")),
      Span::raw(format!(" {} ", symbols::DOT)),
      Span::styled("Header".to_string(), self.location_color("header")),
      Span::raw(" ]"),
    ])
  }
}
impl Pane for RequestPane {
  fn init(&mut self) -> Result<()> {
    self.init_request_schema()?;
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
        self.request_schemas_index = 0;
        self.init_request_schema()?;
      },
      Action::Down => {
        self.request_schema_line_offset =
          self.request_schema_line_offset.saturating_add(1).min(self.request_schemas_styles.len() - 1);
      },
      Action::Up => {
        self.request_schema_line_offset = self.request_schema_line_offset.saturating_sub(1);
      },
      Action::Tab(index) if index < self.request_schemas.len().try_into()? => {
        self.request_schemas_index = index.try_into()?;
        self.init_request_schema()?;
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
      Tabs::new(self.request_schemas.iter().map(|item| {
        let mut title = item.title.clone();
        if !item.media_type.is_empty() {
          title.push_str(format!(" [{}]", item.media_type).as_str());
        }
        Span::styled(title, Style::default().fg(self.location_color(item.location.as_str()))).dim()
      }))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).not_dim())
      .select(self.request_schemas_index),
      inner,
    );

    let inner_margin: Margin = Margin { horizontal: 1, vertical: 1 };
    let mut inner = inner.inner(&inner_margin);
    inner.height = inner.height.saturating_add(1);
    let lines = self.request_schemas_styles.iter().map(|items| {
      return Line::from(
        items
          .iter()
          .map(|item| {
            return Span::styled(&item.1, item.0.bg(Color::Reset));
          })
          .collect::<Vec<_>>(),
      );
    });
    let mut list_state = ListState::default().with_selected(Some(self.request_schema_line_offset));

    frame.render_stateful_widget(
      List::new(lines).highlight_symbol(symbols::scrollbar::HORIZONTAL.end).highlight_spacing(HighlightSpacing::Always),
      inner,
      &mut list_state,
    );

    frame.render_widget(
      Block::default()
        .title("Request")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type())
        .title_bottom(self.legend_line().right_aligned()),
      area,
    );

    Ok(())
  }
}
