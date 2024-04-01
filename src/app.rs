use std::collections::HashMap;

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
  layout::{Constraint, Direction, Layout},
  prelude::Rect,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
  action::Action,
  config::Config,
  pages::{home::Home, phone::Phone, Page},
  panes::{footer::FooterPane, header::HeaderPane, Pane},
  request::Request,
  response::Response,
  state::{InputMode, OperationItemType, State},
  tui,
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
  #[default]
  Home,
}

pub struct App {
  pub config: Config,
  pub pages: Vec<Box<dyn Page>>,
  pub history: HashMap<String, Box<dyn Page>>,
  pub active_page: usize,
  pub footer: FooterPane,
  pub header: HeaderPane,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub last_tick_key_events: Vec<KeyEvent>,
  pub state: State,
}

impl App {
  pub async fn new(input: String) -> Result<Self> {
    let state = State::from_input(input).await?;
    let home = Home::new()?;
    let config = Config::new()?;
    let mode = Mode::Home;

    Ok(Self {
      pages: vec![Box::new(home)],
      history: HashMap::default(),
      active_page: 0,
      footer: FooterPane::new(),
      header: HeaderPane::new(),
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      last_tick_key_events: Vec::new(),
      state,
    })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<Request>();

    let mut tui = tui::Tui::new()?;
    tui.enter()?;

    for page in self.pages.iter_mut() {
      page.register_action_handler(action_tx.clone())?;
    }

    for page in self.pages.iter_mut() {
      page.register_config_handler(self.config.clone())?;
    }

    for page in self.pages.iter_mut() {
      page.init(&self.state)?;
    }

    self.header.init(&self.state)?;
    self.footer.init(&self.state)?;

    loop {
      if let Some(e) = tui.next().await {
        let mut stop_event_propagation = self
          .pages
          .get_mut(self.active_page)
          .and_then(|page| page.handle_events(e.clone(), &mut self.state).ok())
          .map(|response| {
            match response {
              Some(tui::EventResponse::Continue(action)) => {
                action_tx.send(action).ok();
                false
              },
              Some(tui::EventResponse::Stop(action)) => {
                action_tx.send(action).ok();
                true
              },
              _ => false,
            }
          })
          .unwrap_or(false);

        stop_event_propagation = stop_event_propagation
          || self
            .footer
            .handle_events(e.clone(), &mut self.state)
            .map(|response| {
              match response {
                Some(tui::EventResponse::Continue(action)) => {
                  action_tx.send(action).ok();
                  false
                },
                Some(tui::EventResponse::Stop(action)) => {
                  action_tx.send(action).ok();
                  true
                },
                _ => false,
              }
            })
            .unwrap_or(false);

        if !stop_event_propagation {
          match e {
            tui::Event::Quit if self.state.input_mode == InputMode::Normal => action_tx.send(Action::Quit)?,
            tui::Event::Tick => action_tx.send(Action::Tick)?,
            tui::Event::Render => action_tx.send(Action::Render)?,
            tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            tui::Event::Key(key) => {
              if let Some(keymap) = self.config.keybindings.get(&self.mode) {
                if let Some(action) = keymap.get(&vec![key]) {
                  action_tx.send(action.clone())?;
                } else {
                  // If the key was not handled as a single key action,
                  // then consider it for multi-key combinations.
                  self.last_tick_key_events.push(key);

                  // Check for multi-key combinations
                  if let Some(action) = keymap.get(&self.last_tick_key_events) {
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
          Action::Quit if self.state.input_mode == InputMode::Normal => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            tui.draw(|f| {
              self.draw(f).unwrap_or_else(|err| {
                action_tx.send(Action::Error(format!("Failed to draw: {:?}", err))).unwrap();
              })
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              self.draw(f).unwrap_or_else(|err| {
                action_tx.send(Action::Error(format!("Failed to draw: {:?}", err))).unwrap();
              })
            })?;
          },
          Action::NewCall => {
            if let Some(operation_item) = self.state.active_operation() {
              if let OperationItemType::Path = operation_item.r#type {
                if let Some(page) = operation_item
                  .operation
                  .operation_id
                  .clone()
                  .and_then(|operation_id| self.history.remove(&operation_id))
                {
                  self.pages.insert(0, page);
                } else if let Ok(mut page) = Phone::new(operation_item.clone(), request_tx.clone()) {
                  page.init(&self.state)?;
                  self.pages.insert(0, Box::new(page));
                }
              }
            }
          },
          Action::HangUp(ref operation_id) => {
            if self.pages.len() > 1 {
              let page = self.pages.remove(0);
              if let Some(operation_id) = operation_id {
                self.history.insert(operation_id.clone(), page);
              }
            }
          },
          _ => {},
        }
        if let Some(page) = self.pages.get_mut(self.active_page) {
          if let Some(action) = page.update(action.clone(), &mut self.state)? {
            action_tx.send(action)?
          };
        }
        if let Some(action) = self.header.update(action.clone(), &mut self.state)? {
          action_tx.send(action)?
        };
        if let Some(action) = self.footer.update(action.clone(), &mut self.state)? {
          action_tx.send(action)?
        };
      }

      while let Ok(request) = request_rx.try_recv() {
        if let Ok(response) = reqwest::Client::new().execute(request.request).await {
          self.state.responses.insert(request.operation_id, vec![Response {
            status: response.status(),
            version: response.version(),
            headers: response.headers().clone(),
            content_length: response.content_length(),
            body: response.text().await?.clone(),
          }]);
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

  fn draw(&mut self, frame: &mut tui::Frame<'_>) -> Result<()> {
    let vertical_layout = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(1), Constraint::Fill(1), Constraint::Max(1)])
      .split(frame.size());

    self.header.draw(frame, vertical_layout[0], &self.state)?;

    if let Some(page) = self.pages.get_mut(self.active_page) {
      page.draw(frame, vertical_layout[1], &self.state)?;
    };

    self.footer.draw(frame, vertical_layout[2], &self.state)?;
    Ok(())
  }
}
