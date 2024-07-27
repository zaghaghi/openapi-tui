use color_eyre::eyre::Result;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{action::Action, components::schema_viewer::SchemaViewer, panes::Pane, state::State, tui::Frame};

pub struct ResponseType {
  status: String,
  media_type: String,
  schema: serde_json::Value,
}

#[derive(Default)]
pub struct ResponsePane {
  focused: bool,
  focused_border_style: Style,

  schemas: Vec<ResponseType>,
  schemas_index: usize,
  schema_viewer: SchemaViewer,
}

impl ResponsePane {
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

  fn init_schema(&mut self, state: &State) -> Result<()> {
    {
      self.schemas = vec![];

      if let Some(operation_item) = state.active_operation() {
        self.schemas = operation_item
          .operation
          .responses
          .iter()
          .flatten()
          .filter_map(|(status, value)| {
            value.resolve(&state.openapi_spec).map_or(None, |v| Some((status.to_string(), v)))
          })
          .flat_map(|(status, response)| {
            response
              .content
              .iter()
              .flatten()
              .filter_map(|(media_type, media)| {
                media.schema.as_ref().map(|schema| {
                  Some(ResponseType {
                    status: status.clone(),
                    media_type: media_type.to_string(),
                    schema: schema.clone(),
                  })
                })
              })
              .flatten()
              .collect::<Vec<ResponseType>>()
          })
          .collect();
      }
    }
    if let Some(response_type) = self.schemas.get(self.schemas_index) {
      self.schema_viewer.set(response_type.schema.clone())?;
    } else {
      self.schema_viewer.clear();
    }
    Ok(())
  }
}

impl Pane for ResponsePane {
  fn init(&mut self, state: &State) -> Result<()> {
    self.schema_viewer.set_components(state);
    self.init_schema(state)?;
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
      Action::Focus => {
        self.focused = true;
        static STATUS_LINE: &str = "[1-9 → select tab] [g,b → go/back definitions]";
        return Ok(Some(Action::TimedStatusLine(STATUS_LINE.into(), 3)));
      },
      Action::UnFocus => {
        self.focused = false;
      },
      Action::Go => self.schema_viewer.go()?,
      Action::Back => {
        if let Some(response_type) = self.schemas.get(self.schemas_index) {
          self.schema_viewer.back(response_type.schema.clone())?;
        }
      },
      _ => {},
    }

    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, _state: &State) -> Result<()> {
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    frame.render_widget(
      Tabs::new(self.schemas.iter().map(|resp| {
        Span::styled(
          format!("{} [{}]", resp.status, resp.media_type),
          Style::default().fg(self.status_color(resp.status.as_str())).dim(),
        )
      }))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).not_dim())
      .select(self.schemas_index),
      inner,
    );

    let mut inner = inner.inner(Margin { horizontal: 1, vertical: 1 });
    inner.height = inner.height.saturating_add(1);
    self.schema_viewer.render_widget(frame, inner);

    frame.render_widget(
      Block::default()
        .title("Responses")
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
