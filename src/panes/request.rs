use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use oas3::{spec::RequestBody, Schema};
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  components::schema_viewer::SchemaViewer,
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

  schemas: Vec<RequestType>,
  schemas_index: usize,
  schema_viewer: SchemaViewer,
}

impl RequestPane {
  pub fn new(state: Arc<RwLock<State>>, focused: bool, focused_border_style: Style) -> Self {
    Self {
      focused,
      focused_border_style,
      schemas: Vec::default(),
      schemas_index: 0,
      schema_viewer: SchemaViewer::from(state.clone()),
      state,
    }
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

  fn init_schema(&mut self) -> Result<()> {
    {
      let state = self.state.read().unwrap();
      if let Some((_path, _method, operation)) = state.active_operation() {
        let mut schemas: Vec<RequestType> = vec![];
        if let Some(request_body) = &operation.request_body {
          schemas = request_body
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
        schemas.extend(
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
        self.schemas = schemas;
      }
    }
    if let Some(request_type) = self.schemas.get(self.schemas_index) {
      self.schema_viewer.set(request_type.schema.clone())?;
    } else {
      self.schema_viewer.clear();
    }
    Ok(())
  }

  fn legend_line(&self) -> Line {
    if self.schema_viewer.schema_path().is_empty() {
      Line::from(vec![
        Span::raw("["),
        Span::styled("Body".to_string(), self.location_color("body")),
        Span::raw("/"),
        Span::styled("Path".to_string(), self.location_color("path")),
        Span::raw("/"),
        Span::styled("Query".to_string(), self.location_color("query")),
        Span::raw("/"),
        Span::styled("Header".to_string(), self.location_color("header")),
        Span::raw("]"),
      ])
    } else {
      Line::from(vec![
        Span::raw("["),
        Span::styled("B".to_string(), self.location_color("body")),
        Span::raw("/"),
        Span::styled("P".to_string(), self.location_color("path")),
        Span::raw("/"),
        Span::styled("Q".to_string(), self.location_color("query")),
        Span::raw("/"),
        Span::styled("H".to_string(), self.location_color("header")),
        Span::raw("]"),
      ])
    }
  }

  fn nested_schema_path_line(&self) -> Line {
    let schema_path = self.schema_viewer.schema_path();
    if schema_path.is_empty() {
      return Line::default();
    }
    let mut line = String::from("[ ");
    line.push_str(&schema_path.join(" > "));
    line.push_str(" ]");
    Line::from(line)
  }
}

impl Pane for RequestPane {
  fn init(&mut self) -> Result<()> {
    self.init_schema()?;
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

  fn height_constraint(&self) -> Constraint {
    if self.schemas.get(self.schemas_index).is_none() {
      return Constraint::Max(2);
    }
    match self.focused {
      true => Constraint::Fill(3),
      false => Constraint::Fill(1),
    }
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
        self.schemas_index = 0;
        self.init_schema()?;
      },
      Action::Down => {
        self.schema_viewer.down();
      },
      Action::Up => {
        self.schema_viewer.up();
      },
      Action::Tab(index) if index < self.schemas.len().try_into()? => {
        self.schemas_index = index.try_into()?;
        self.init_schema()?;
      },
      Action::Go => self.schema_viewer.go()?,
      Action::Back => {
        if let Some(request_type) = self.schemas.get(self.schemas_index) {
          self.schema_viewer.back(request_type.schema.clone())?;
        }
      },
      _ => {},
    }

    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let inner_margin: Margin = Margin { horizontal: 1, vertical: 1 };

    let inner = area.inner(&inner_margin);

    frame.render_widget(
      Tabs::new(self.schemas.iter().map(|item| {
        let mut title = item.title.clone();
        if !item.media_type.is_empty() {
          title.push_str(format!(" [{}]", item.media_type).as_str());
        }
        Span::styled(title, Style::default().fg(self.location_color(item.location.as_str()))).dim()
      }))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).not_dim())
      .select(self.schemas_index),
      inner,
    );

    let inner_margin: Margin = Margin { horizontal: 1, vertical: 1 };
    let mut inner = inner.inner(&inner_margin);
    inner.height = inner.height.saturating_add(1);
    self.schema_viewer.render_widget(frame, inner);

    frame.render_widget(
      Block::default()
        .title("Request")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type())
        .title_bottom(self.legend_line().style(Style::default().dim()).right_aligned())
        .title_bottom(
          self
            .nested_schema_path_line()
            .style(Style::default().fg(Color::White).dim().add_modifier(Modifier::ITALIC))
            .left_aligned(),
        ),
      area,
    );

    Ok(())
  }
}
