use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  pages::home::State,
  panes::Pane,
  tui::{EventResponse, Frame},
};

#[derive(Default)]
pub struct AddressPane {
  focused: bool,
  focused_border_style: Style,
  state: Arc<RwLock<State>>,
}

impl AddressPane {
  pub fn new(state: Arc<RwLock<State>>, focused: bool, focused_border_style: Style) -> Self {
    Self { state, focused, focused_border_style }
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

  fn method_color(method: &str) -> Color {
    match method {
      "GET" => Color::LightCyan,
      "POST" => Color::LightBlue,
      "PUT" => Color::LightYellow,
      "DELETE" => Color::LightRed,
      _ => Color::Gray,
    }
  }
}
impl Pane for AddressPane {
  fn init(&mut self) -> Result<()> {
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
    Constraint::Max(3)
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
      Action::Update => {},
      Action::Submit => {},
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let state = self.state.read().unwrap();
    if let Some(operation_item) = state.active_operation() {
      let base_url = if let Some(server) = state.openapi_spec.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
        server.url.clone()
      } else if let Some(server) = operation_item.operation.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
        server.url.clone()
      } else {
        String::from("http://localhost")
      };
      let title = operation_item.operation.summary.clone().unwrap_or_default();
      const INNER_MARGIN: Margin = Margin { horizontal: 1, vertical: 1 };

      let inner = area.inner(&INNER_MARGIN);
      frame.render_widget(
        Paragraph::new(Line::from(vec![
          Span::styled(
            format!("{:7}", operation_item.method.as_str()),
            Style::default().fg(Self::method_color(operation_item.method.as_str())),
          ),
          Span::styled(base_url, Style::default().fg(Color::DarkGray)),
          Span::styled(&operation_item.path, Style::default().fg(Color::White)),
        ])),
        inner,
      );

      frame.render_widget(
        Block::default()
          .title(title)
          .borders(Borders::ALL)
          .border_style(self.border_style())
          .border_type(self.border_type()),
        area,
      );
    }

    Ok(())
  }
}
