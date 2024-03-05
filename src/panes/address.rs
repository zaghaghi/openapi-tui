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
    let action_operation = state.active_operation();

    let inner_margin = Margin { horizontal: 1, vertical: 1 };
    frame
      .render_widget(Block::default().title("Address").borders(Borders::ALL).border_style(self.border_style()), area);
    let inner = area.inner(&inner_margin);
    if let Some(operation) = action_operation {
      if let Some(operation_id) = &operation.operation_id {
        frame.render_widget(Paragraph::new(operation_id.as_str()), inner)
      }
    }
    Ok(())
  }
}
