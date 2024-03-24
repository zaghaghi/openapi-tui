use color_eyre::eyre::Result;
use openapi_31::v31::parameter::In;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{action::Action, components::schema_viewer::SchemaViewer, panes::Pane, state::State, tui::Frame};

pub struct RequestType {
  location: String,
  schema: serde_json::Value,
  title: String,
}

#[derive(Default)]
pub struct RequestPane {
  focused: bool,
  focused_border_style: Style,

  schemas: Vec<RequestType>,
  schemas_index: usize,
  schema_viewer: SchemaViewer,
}

impl RequestPane {
  pub fn new(focused: bool, focused_border_style: Style) -> Self {
    Self {
      focused,
      focused_border_style,
      schemas: Vec::default(),
      schemas_index: 0,
      schema_viewer: SchemaViewer::default(),
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
    if status.starts_with("cookie") {
      return Color::LightRed;
    }
    Color::default()
  }

  fn init_schema(&mut self, state: &State) -> Result<()> {
    {
      self.schemas = vec![];

      macro_rules! push_schema {
        ($map:ident, $title:expr, $location:expr) => {{
          if !$map.is_empty() {
            self.schemas.push(RequestType {
              location: $location.to_string(),
              schema: serde_json::Value::Object($map),
              title: $title.to_string(),
            })
          }
        }};
      }
      if let Some(operation_item) = state.active_operation() {
        if let Some(request_body) = &operation_item.operation.request_body {
          let mut bodies = serde_json::Map::new();

          request_body.resolve(&state.openapi_spec).unwrap().content.iter().for_each(|(media_type, media)| {
            if let Some(schema) = &media.schema {
              bodies.insert(media_type.clone(), schema.clone());
            }
          });

          push_schema!(bodies, "Body", "body");
        }
        let mut query_parameters = serde_json::Map::new();
        let mut header_parameters = serde_json::Map::new();
        let mut path_parameters = serde_json::Map::new();
        let mut cookie_parameters = serde_json::Map::new();
        operation_item.operation.parameters.iter().flatten().for_each(|parameter_or_ref| {
          let parameter = parameter_or_ref.resolve(&state.openapi_spec).unwrap();
          match parameter.r#in {
            In::Query => &mut query_parameters,
            In::Header => &mut header_parameters,
            In::Path => &mut path_parameters,
            In::Cookie => &mut cookie_parameters,
          }
          .insert(parameter.name.clone(), parameter.schema.as_ref().unwrap_or(&serde_json::Value::Null).clone());
        });

        push_schema!(query_parameters, "Query", "query");
        push_schema!(header_parameters, "Header", "header");
        push_schema!(path_parameters, "Path", "path");
        push_schema!(cookie_parameters, "Cookie", "cookie");
      }
    }
    if let Some(request_type) = self.schemas.get(self.schemas_index) {
      self.schema_viewer.set(request_type.schema.clone())?;
    } else {
      self.schema_viewer.clear();
    }
    Ok(())
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
  fn init(&mut self, state: &State) -> Result<()> {
    self.schema_viewer.set_components(state);
    self.init_schema(state)?;
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
      return Constraint::Min(2);
    }
    match self.focused {
      true => Constraint::Fill(30),
      false => Constraint::Fill(10),
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {
        self.schemas_index = 0;
        self.init_schema(state)?;
      },
      Action::Down => {
        self.schema_viewer.down();
      },
      Action::Up => {
        self.schema_viewer.up();
      },
      Action::Tab(index) if index < self.schemas.len().try_into()? => {
        self.schemas_index = index.try_into()?;
        self.init_schema(state)?;
      },
      Action::TabNext => {
        let next_tab_index = self.schemas_index + 1;
        self.schemas_index = if next_tab_index < self.schemas.len() { next_tab_index } else { 0 };
        self.init_schema(state)?;
      },
      Action::TabPrev => {
        self.schemas_index = if self.schemas_index > 0 { self.schemas_index - 1 } else { self.schemas.len() - 1 };
        self.init_schema(state)?;
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

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, _state: &State) -> Result<()> {
    let inner_margin: Margin = Margin { horizontal: 1, vertical: 1 };

    let inner = area.inner(&inner_margin);

    frame.render_widget(
      Tabs::new(self.schemas.iter().map(|item| {
        let title = item.title.clone();
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
