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
  panes::{address::AddressPane, apis::ApisPane, request::RequestPane, response::ResponsePane, tags::TagsPane, Pane},
  tui::EventResponse,
};

#[derive(Default)]
pub struct State {
  pub openapi_path: String,
  pub openapi_spec: Spec,
  pub active_operation_index: usize,
  pub active_tag_name: Option<String>,
}

impl State {
  pub fn active_operation(&self) -> Option<(String, String, &Operation)> {
    if let Some(active_tag) = &self.active_tag_name {
      if let Some((path, method, operation)) =
        self.openapi_spec.operations().filter(|item| item.2.tags.contains(active_tag)).nth(self.active_operation_index)
      {
        return Some((path, method.to_string(), operation));
      }
    } else if let Some((path, method, operation)) = self.openapi_spec.operations().nth(self.active_operation_index) {
      return Some((path, method.to_string(), operation));
    }
    None
  }

  pub fn operations_len(&self) -> usize {
    if let Some(active_tag) = &self.active_tag_name {
      self.openapi_spec.operations().filter(|item| item.2.tags.contains(active_tag)).count()
    } else {
      self.openapi_spec.operations().count()
    }
  }
}

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  panes: Vec<Box<dyn Pane>>,
  focused_pane_index: usize,
  #[allow(dead_code)]
  state: Arc<RwLock<State>>,
}

impl Home {
  pub fn new(openapi_path: String) -> Result<Self> {
    let openapi_spec = oas3::from_path(openapi_path.clone())?;
    let state =
      Arc::new(RwLock::new(State { openapi_spec, openapi_path, active_operation_index: 0, active_tag_name: None }));
    let focused_border_style = Style::default().fg(Color::LightGreen);

    Ok(Self {
      command_tx: None,
      config: Config::default(),
      panes: vec![
        Box::new(ApisPane::new(state.clone(), true, focused_border_style)),
        Box::new(TagsPane::new(state.clone(), false, focused_border_style)),
        Box::new(AddressPane::new(state.clone(), false, focused_border_style)),
        Box::new(RequestPane::new(state.clone(), false, focused_border_style)),
        Box::new(ResponsePane::new(false, focused_border_style)),
      ],
      focused_pane_index: 0,
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
      KeyCode::Char(c) if ('1'..='9').contains(&c) => EventResponse::Stop(Action::Tab(c.to_digit(10).unwrap_or(0) - 1)),
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
      .constraints(vec![Constraint::Fill(3), Constraint::Fill(1)])
      .split(outer_layout[0]);

    let right_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(3), Constraint::Fill(1), Constraint::Fill(1)])
      .split(outer_layout[1]);

    self.panes[0].draw(frame, left_panes[0])?;
    self.panes[1].draw(frame, left_panes[1])?;
    self.panes[2].draw(frame, right_panes[0])?;
    self.panes[3].draw(frame, right_panes[1])?;
    self.panes[4].draw(frame, right_panes[2])?;
    Ok(())
  }
}
