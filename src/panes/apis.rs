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

pub struct ApisPane {
  focused: bool,
  focused_border_style: Style,
  state: Arc<RwLock<State>>,
  current_operation_index: usize,
}

impl ApisPane {
  pub fn new(state: Arc<RwLock<State>>, focused: bool, focused_border_style: Style) -> Self {
    Self { focused, focused_border_style, state, current_operation_index: 0 }
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
        let mut state = self.state.write().unwrap();
        let operations_len = state.operations_len();
        if operations_len > 0 {
          self.current_operation_index = self.current_operation_index.saturating_add(1) % operations_len;
        }
        state.active_operation_index = self.current_operation_index;
        return Ok(Some(Action::Update));
      },
      Action::Up => {
        let mut state = self.state.write().unwrap();
        let operations_len = state.operations_len();
        if operations_len > 0 {
          self.current_operation_index =
            self.current_operation_index.saturating_add(operations_len - 1) % operations_len;
        }
        state.active_operation_index = self.current_operation_index;
        return Ok(Some(Action::Update));
      },
      Action::Submit => {},
      Action::Update => {
        let state = self.state.read().unwrap();
        self.current_operation_index = state.active_operation_index;
      },
      _ => {},
    }

    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let state = self.state.read().unwrap();
    let unknown = String::from("Unknown");
    let items = state.openapi_spec.operations().filter_map(|operation| {
      if let Some(active_tag) = &state.active_tag_name {
        if !operation.2.tags.contains(active_tag) {
          return None;
        }
      }
      Some(Line::from(vec![
        Span::styled(
          format!(" {:7}", operation.1.as_str()),
          Style::default().fg(Self::method_color(operation.1.as_str())),
        ),
        Span::styled(
          operation.2.summary.as_ref().unwrap_or(operation.2.operation_id.as_ref().unwrap_or(&unknown)),
          Style::default().fg(Color::White),
        ),
      ]))
    });

    let list = List::new(items)
      .block(Block::default().borders(Borders::ALL))
      .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
      .highlight_spacing(HighlightSpacing::Always)
      .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    let mut list_state = ListState::default().with_selected(Some(self.current_operation_index));

    frame.render_stateful_widget(list, area, &mut list_state);
    let active_tag = format!("[{}]", state.active_tag_name.clone().unwrap_or(String::from("ALL")));
    frame.render_widget(
      Block::default()
        .title("APIs")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type())
        .title_bottom(
          Line::from(format!("{} of {}", self.current_operation_index.saturating_add(1), state.operations_len()))
            .right_aligned(),
        )
        .title(Line::styled(active_tag, Style::default().add_modifier(Modifier::ITALIC)).right_aligned()),
      area,
    );
    Ok(())
  }
}
