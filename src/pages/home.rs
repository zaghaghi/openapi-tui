use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use oas3::{spec::Operation, Spec};
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  pages::Page,
  panes::{
    address::AddressPane, apis::ApisPane, profiles::ProfilesPane, request::RequestPane, response::ResponsePane,
    tags::TagsPane, Pane,
  },
  tui::EventResponse,
};

#[derive(Default)]
pub struct State {
  pub openapi_path: String,
  pub openapi_spec: Spec,
  pub active_operation_index: usize,
}

impl State {
  pub fn active_operation(&self) -> Option<&Operation> {
    if let Some(operation) = self.openapi_spec.operations().nth(self.active_operation_index) {
      Some(operation.2)
    } else {
      None
    }
  }

  pub fn operations_len(&self) -> usize {
    self.openapi_spec.operations().count()
  }
}

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  panes: Vec<Box<dyn Pane>>,
  focused_pane_index: usize,
  state: Arc<RwLock<State>>,
}

impl Home {
  pub fn new(openapi_path: String) -> Result<Self> {
    let openapi_spec = oas3::from_path(openapi_path.clone())?;
    let state = Arc::new(RwLock::new(State { openapi_spec, openapi_path, active_operation_index: 0 }));
    let focused_border_style = Style::default().fg(Color::LightGreen);

    Ok(Self {
      command_tx: None,
      config: Config::default(),
      panes: vec![
        Box::new(ProfilesPane::new(false, focused_border_style)),
        Box::new(ApisPane::new(state.clone(), true, focused_border_style)),
        Box::new(TagsPane::new(false, focused_border_style)),
        Box::new(AddressPane::new(state.clone(), false, focused_border_style)),
        Box::new(RequestPane::new(false, focused_border_style)),
        Box::new(ResponsePane::new(false, focused_border_style)),
      ],
      focused_pane_index: 1,
      state,
    })
  }
}

impl Page for Home {
  fn init(&mut self) -> Result<()> {
    for pane in self.panes.iter_mut() {
      pane.init()?;
    }
    Ok(())
  }

  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.command_tx = Some(tx);
    Ok(())
  }

  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config;
    Ok(())
  }

  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::Tick => {},
      Action::FocusNext => {
        let next_index = self.focused_pane_index.saturating_add(1) % self.panes.len();
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.unfocus()?;
        }
        self.focused_pane_index = next_index;
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.focus()?;
        }
      },
      Action::FocusPrev => {
        let prev_index = self.focused_pane_index.saturating_add(self.panes.len() - 1) % self.panes.len();
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.unfocus()?;
        }
        self.focused_pane_index = prev_index;
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.focus()?;
        }
      },
      Action::Update => {
        for pane in self.panes.iter_mut() {
          pane.update(action.clone())?;
        }
      },
      _ => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          return pane.update(action);
        }
      },
    }
    Ok(None)
  }

  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    let response = match key.code {
      KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::FocusNext),
      KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::FocusPrev),
      KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
      KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
      KeyCode::Enter => EventResponse::Stop(Action::Submit),
      _ => {
        return Ok(None);
      },
    };
    Ok(Some(response))
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let outer_layout = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Min(30), Constraint::Percentage(100)])
      .split(area);

    let left_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(5), Constraint::Fill(3), Constraint::Fill(1)])
      .split(outer_layout[0]);

    let right_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(5), Constraint::Fill(1), Constraint::Fill(1)])
      .split(outer_layout[1]);

    self.panes[0].draw(frame, left_panes[0])?;
    self.panes[1].draw(frame, left_panes[1])?;
    self.panes[2].draw(frame, left_panes[2])?;
    self.panes[3].draw(frame, right_panes[0])?;
    self.panes[4].draw(frame, right_panes[1])?;
    self.panes[5].draw(frame, right_panes[2])?;
    Ok(())
  }
}
