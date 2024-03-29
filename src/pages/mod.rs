use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  state::State,
  tui::{Event, EventResponse, Frame},
};

pub mod home;
pub mod phone;

pub trait Page {
  #[allow(unused_variables)]
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    Ok(())
  }

  #[allow(unused_variables)]
  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    Ok(())
  }

  fn init(&mut self, _state: &State) -> Result<()> {
    Ok(())
  }

  fn handle_events(&mut self, event: Event, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    let r = match event {
      Event::Key(key_event) => self.handle_key_events(key_event, state)?,
      Event::Mouse(mouse_event) => self.handle_mouse_events(mouse_event, state)?,
      _ => None,
    };
    Ok(r)
  }

  #[allow(unused_variables)]
  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  #[allow(unused_variables)]
  fn handle_mouse_events(&mut self, mouse: MouseEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  #[allow(unused_variables)]
  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &State) -> Result<()>;
}
