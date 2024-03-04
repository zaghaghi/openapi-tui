use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use oas3::Spec;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  panes::Pane,
  tui::{EventResponse, Frame},
};

pub struct ApisPane {
  focused: bool,
  focused_border_style: Style,
  openapi_spec: Arc<Spec>,
  openapi_spec_operations_index: usize,
  openapi_spec_operations_len: usize,
}

impl ApisPane {
  pub fn new(openapi_spec: Arc<Spec>, focused: bool, focused_border_style: Style) -> Self {
    Self {
      focused,
      focused_border_style,
      openapi_spec: openapi_spec.clone(),
      openapi_spec_operations_index: 0,
      openapi_spec_operations_len: openapi_spec.clone().operations().count(),
    }
  }

  fn border_style(&self) -> Style {
    match self.focused {
      true => self.focused_border_style,
      false => Style::default(),
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
impl Pane for ApisPane {
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

  fn handle_key_events(&mut self, _key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  #[allow(unused_variables)]
  fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::Down => {
        if self.openapi_spec_operations_len > 0 {
          self.openapi_spec_operations_index =
            self.openapi_spec_operations_index.saturating_add(1) % self.openapi_spec_operations_len;
        }
      },
      Action::Up => {
        if self.openapi_spec_operations_len > 0 {
          self.openapi_spec_operations_index =
            self.openapi_spec_operations_index.saturating_add(self.openapi_spec_operations_len - 1)
              % self.openapi_spec_operations_len;
        }
      },
      _ => {},
    }

    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let unknown = String::from("Unknown");
    let items = self.openapi_spec.operations().map(|operation| {
      Line::from(vec![
        Span::styled(
          format!("{:7}", operation.1.as_str()),
          Style::default().fg(ApisPane::method_color(operation.1.as_str())),
        ),
        Span::styled(
          operation.2.summary.as_ref().unwrap_or(operation.2.operation_id.as_ref().unwrap_or(&unknown)),
          Style::default().fg(Color::White),
        ),
      ])
    });

    let list = List::new(items)
      .block(Block::default().title("List").borders(Borders::ALL))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
      .direction(ListDirection::TopToBottom);
    let mut state = ListState::default().with_selected(Some(self.openapi_spec_operations_index));

    frame.render_stateful_widget(list, area, &mut state);

    frame.render_widget(
      Block::default().title("APIs").borders(Borders::ALL).border_style(self.border_style()).title_bottom(
        Line::from(format!(
          "{} of {}",
          self.openapi_spec_operations_index.saturating_add(1),
          self.openapi_spec_operations_len
        ))
        .right_aligned(),
      ),
      area,
    );
    Ok(())
  }
}
