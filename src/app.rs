use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{action::Action, component::Component, components::home::Home, config::Config, tui};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
  #[default]
  Home,
}

pub struct App {
  pub config: Config,
  pub openapi_path: String,
  pub components: Vec<Box<dyn Component>>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub last_tick_key_events: Vec<KeyEvent>,
}

impl App {
  pub fn new(openapi_path: String) -> Result<Self> {
    let home = Home::new();
    let config = Config::new()?;
    let mode = Mode::Home;
    Ok(Self {
      components: vec![Box::new(home)],
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      openapi_path,
      last_tick_key_events: Vec::new(),
    })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    let mut tui = tui::Tui::new()?;
    tui.enter()?;

    for component in self.components.iter_mut() {
      component.register_action_handler(action_tx.clone())?;
    }

    for component in self.components.iter_mut() {
      component.register_config_handler(self.config.clone())?;
    }

    for component in self.components.iter_mut() {
      component.init()?;
    }

    loop {
      if let Some(e) = tui.next().await {
        let mut stop_event_propagation = false;
        for component in self.components.iter_mut() {
          if let Some(response) = component.handle_events(Some(e.clone()))? {
            match response {
              tui::EventResponse::Continue(action) => {
                action_tx.send(action)?;
              },
              tui::EventResponse::Stop(action) => {
                action_tx.send(action)?;
                stop_event_propagation = true;
                break;
              },
            }
          }
        }
        if !stop_event_propagation {
          match e {
            tui::Event::Quit => action_tx.send(Action::Quit)?,
            tui::Event::Tick => action_tx.send(Action::Tick)?,
            tui::Event::Render => action_tx.send(Action::Render)?,
            tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            tui::Event::Key(key) => {
              if let Some(keymap) = self.config.keybindings.get(&self.mode) {
                if let Some(action) = keymap.get(&vec![key.clone()]) {
                  log::info!("Got action: {action:?}");
                  action_tx.send(action.clone())?;
                } else {
                  // If the key was not handled as a single key action,
                  // then consider it for multi-key combinations.
                  self.last_tick_key_events.push(key);

                  // Check for multi-key combinations
                  if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    log::info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                  }
                }
              };
            },
            _ => {},
          }
        }
      }

      while let Ok(action) = action_rx.try_recv() {
        if action != Action::Tick && action != Action::Render {
          log::debug!("{action:?}");
        }
        match action {
          Action::Tick => {
            self.last_tick_key_events.drain(..);
          },
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.size());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
                }
              }
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.size());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
                }
              }
            })?;
          },
          _ => {},
        }
        for component in self.components.iter_mut() {
          if let Some(action) = component.update(action.clone())? {
            action_tx.send(action)?
          };
        }
      }
      if self.should_suspend {
        tui.suspend()?;
        action_tx.send(Action::Resume)?;
        tui = tui::Tui::new()?;
        tui.enter()?;
      } else if self.should_quit {
        tui.stop()?;
        break;
      }
    }
    tui.exit()?;
    Ok(())
  }
}
