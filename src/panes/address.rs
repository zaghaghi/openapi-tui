use std::collections::VecDeque;

use color_eyre::eyre::Result;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  pages::phone::{RequestBuilder, RequestPane},
  panes::Pane,
  state::{OperationItemType, State},
  tui::Frame,
};

#[derive(Default)]
pub struct AddressPane {
  focused: bool,
  focused_border_style: Style,
  base_urls: VecDeque<String>,
}

impl AddressPane {
  pub fn new(focused: bool, focused_border_style: Style) -> Self {
    Self { focused, focused_border_style, base_urls: VecDeque::new() }
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

impl RequestPane for AddressPane {}

impl RequestBuilder for AddressPane {
  fn path(&self, url: String) -> String {
    format!("{}{}", self.base_urls.front().cloned().unwrap_or_default(), url)
  }
}

impl Pane for AddressPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Max(3)
  }

  fn init(&mut self, state: &State) -> Result<()> {
    self.base_urls = state.default_server_urls(&None).into();
    Ok(())
  }

  fn update(&mut self, action: Action, _state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Focus => {
        self.focused = true;
        static STATUS_LINE: &str = "[ENTER → request]";
        return Ok(Some(Action::TimedStatusLine(STATUS_LINE.into(), 3)));
      },
      Action::UnFocus => {
        self.focused = false;
      },
      Action::Up => {
        if let Some(front) = self.base_urls.pop_front() {
          self.base_urls.push_back(front.to_string());
        }
      },
      Action::Down => {
        if let Some(back) = self.base_urls.pop_back() {
          self.base_urls.push_front(back.to_string());
        }
      },
      Action::Update => {},
      Action::Submit => {},

      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    if let Some(operation_item) = state.active_operation() {
      let base_url = self.base_urls.front().cloned().unwrap_or(String::new());
      let title = operation_item.operation.summary.clone().unwrap_or_default();

      let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
      frame.render_widget(
        match operation_item.r#type {
          OperationItemType::Path => Paragraph::new(Line::from(vec![
            Span::styled(
              format!("{:7}", operation_item.method.as_str()),
              Style::default().fg(Self::method_color(operation_item.method.as_str())),
            ),
            Span::styled(base_url, Style::default().fg(Color::DarkGray)),
            Span::styled(&operation_item.path, Style::default().fg(Color::White)),
          ])),
          OperationItemType::Webhook => Paragraph::new(Line::from(vec![
            Span::styled("EVENT ", Style::default().fg(Color::LightMagenta)),
            Span::styled(
              format!("{} ", operation_item.method.as_str()),
              Style::default().fg(Self::method_color(operation_item.method.as_str())),
            ),
            Span::styled(&operation_item.path, Style::default().fg(Color::White)),
          ])),
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
