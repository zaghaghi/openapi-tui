use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::{Constraint, Rect};

use crate::{
  action::Action,
  tui::{EventResponse, Frame},
};

pub mod address;
pub mod apis;
pub mod request;
pub mod response;
pub mod tags;

pub trait Pane {
  fn init(&mut self) -> Result<()> {
    Ok(())
  }

  fn focus(&mut self) -> Result<()> {
    Ok(())
  }

  fn unfocus(&mut self) -> Result<()> {
    Ok(())
  }

  fn height_constraint(&self) -> Constraint;

  #[allow(unused_variables)]
  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  #[allow(unused_variables)]
  fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  #[allow(unused_variables)]
  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()>;
}
