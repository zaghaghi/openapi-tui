use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::{Constraint, Rect};

use crate::{
  action::Action,
  state::State,
  tui::{Event, EventResponse, Frame},
};

pub mod address;
pub mod apis;
pub mod body_editor;
pub mod footer;
pub mod header;
pub mod parameter_editor;
pub mod request;
pub mod response;
pub mod response_viewer;
pub mod tags;

pub trait Pane {
  fn init(&mut self, _state: &State) -> Result<()> {
    Ok(())
  }

  fn height_constraint(&self) -> Constraint;

  fn handle_events(&mut self, event: Event, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    let r = match event {
      Event::Key(key_event) => self.handle_key_events(key_event, state)?,
      Event::Mouse(mouse_event) => self.handle_mouse_events(mouse_event, state)?,
      _ => None,
    };
    Ok(r)
  }

  fn handle_key_events(&mut self, _key: KeyEvent, _state: &mut State) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  fn handle_mouse_events(&mut self, _mouse: MouseEvent, _state: &mut State) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  fn update(&mut self, _action: Action, _state: &mut State) -> Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &State) -> Result<()>;
}
