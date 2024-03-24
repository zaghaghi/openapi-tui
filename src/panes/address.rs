use color_eyre::eyre::Result;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  panes::Pane,
  state::{OperationItemType, State},
  tui::Frame,
};

#[derive(Default)]
pub struct AddressPane {
  focused: bool,
  focused_border_style: Style,
}

impl AddressPane {
  pub fn new(focused: bool, focused_border_style: Style) -> Self {
    Self { focused, focused_border_style }
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

  fn update(&mut self, action: Action, _state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {},
      Action::Submit => {},
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    if let Some(operation_item) = state.active_operation() {
      let base_url = if let Some(server) = state.openapi_spec.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
        String::from(server.url.trim_end_matches('/'))
      } else if let Some(server) = operation_item.operation.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
        String::from(server.url.trim_end_matches('/'))
      } else {
        String::from("http://localhost")
      };
      let title = operation_item.operation.summary.clone().unwrap_or_default();
      const INNER_MARGIN: Margin = Margin { horizontal: 1, vertical: 1 };

      let inner = area.inner(&INNER_MARGIN);
      frame.render_widget(
        match operation_item.r#type {
          OperationItemType::Path => {
            Paragraph::new(Line::from(vec![
              Span::styled(
                format!("{:7}", operation_item.method.as_str()),
                Style::default().fg(Self::method_color(operation_item.method.as_str())),
              ),
              Span::styled(base_url, Style::default().fg(Color::DarkGray)),
              Span::styled(&operation_item.path, Style::default().fg(Color::White)),
            ]))
          },
          OperationItemType::Webhook => {
            Paragraph::new(Line::from(vec![
              Span::styled("EVENT ", Style::default().fg(Color::LightMagenta)),
              Span::styled(
                format!("{} ", operation_item.method.as_str()),
                Style::default().fg(Self::method_color(operation_item.method.as_str())),
              ),
              Span::styled(&operation_item.path, Style::default().fg(Color::White)),
            ]))
          },
        },
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
    } else {
      frame.render_widget(
        Block::default()
          .title("[No Active API]")
          .borders(Borders::ALL)
          .border_style(self.border_style())
          .border_type(self.border_type()),
        area,
      );
    }

    Ok(())
  }
}
