use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  panes::Pane,
  tui::{EventResponse, Frame},
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

  fn update(&mut self, _action: Action) -> Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    frame
      .render_widget(Block::default().title("Address").borders(Borders::ALL).border_style(self.border_style()), area);
    Ok(())
  }
}
