use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use openapi_31::v31::{Openapi, Operation};
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  pages::Page,
  panes::{
    address::AddressPane, apis::ApisPane, footer::FooterPane, header::HeaderPane, request::RequestPane,
    response::ResponsePane, tags::TagsPane, Pane,
  },
  tui::EventResponse,
};

#[derive(Default)]
pub enum InputMode {
  #[default]
  Normal,
  Insert,
}

pub enum OperationItemType {
  Path,
  Webhook,
}
pub struct OperationItem {
  pub path: String,
  pub method: String,
  pub operation: Operation,
  pub r#type: OperationItemType,
}

impl OperationItem {
  pub fn has_tag(&self, tag: &String) -> bool {
    self.operation.tags.as_ref().map_or(false, |tags| tags.contains(tag))
  }
}

#[derive(Default)]
pub struct State {
  pub openapi_path: String,
  pub openapi_spec: Openapi,
  pub openapi_operations: Vec<OperationItem>,
  pub active_operation_index: usize,
  pub active_tag_name: Option<String>,
  pub active_filter: String,
}

impl State {
  pub fn active_operation(&self) -> Option<&OperationItem> {
    if let Some(active_tag) = &self.active_tag_name {
      self
        .openapi_operations
        .iter()
        .filter(|flat_operation| {
          flat_operation.has_tag(active_tag) && flat_operation.path.contains(self.active_filter.as_str())
        })
        .nth(self.active_operation_index)
    } else {
      self
        .openapi_operations
        .iter()
        .filter(|flat_operation| flat_operation.path.contains(self.active_filter.as_str()))
        .nth(self.active_operation_index)
    }
  }

  pub fn operations_len(&self) -> usize {
    if let Some(active_tag) = &self.active_tag_name {
      self
        .openapi_operations
        .iter()
        .filter(|item| item.has_tag(active_tag) && item.path.contains(self.active_filter.as_str()))
        .count()
    } else {
      self
        .openapi_operations
        .iter()
        .filter(|flat_operation| flat_operation.path.contains(self.active_filter.as_str()))
        .count()
    }
  }
}

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  panes: Vec<Box<dyn Pane>>,
  static_panes: Vec<Box<dyn Pane>>,
  focused_pane_index: usize,
  #[allow(dead_code)]
  state: Arc<RwLock<State>>,
  fullscreen_pane_index: Option<usize>,
  input_mode: InputMode,
}

impl Home {
  pub async fn new(openapi_path: String) -> Result<Self> {
    let openapi_spec = if let Ok(url) = reqwest::Url::parse(openapi_path.as_str()) {
      let resp: String = reqwest::get(url.clone()).await?.text().await?;
      let mut spec = serde_yaml::from_str::<Openapi>(resp.as_str())?;
      if spec.servers.is_none() {
        let origin = url.origin().ascii_serialization();
        spec.servers = Some(vec![openapi_31::v31::Server::new(format!("{}/", origin))]);
      }
      spec
    } else {
      tokio::fs::read_to_string(&openapi_path)
        .await
        .map(|content| serde_yaml::from_str::<Openapi>(content.as_str()))??
    };

    let openapi_operations = openapi_spec
      .into_operations()
      .map(|(path, method, operation)| {
        if path.starts_with('/') {
          OperationItem { path, method, operation, r#type: OperationItemType::Path }
        } else {
          OperationItem { path, method, operation, r#type: OperationItemType::Webhook }
        }
      })
      .collect::<Vec<_>>();
    let state = Arc::new(RwLock::new(State {
      openapi_spec,
      openapi_path,
      openapi_operations,
      active_operation_index: 0,
      active_tag_name: None,
      active_filter: String::default(),
    }));
    let focused_border_style = Style::default().fg(Color::LightGreen);

    Ok(Self {
      command_tx: None,
      config: Config::default(),
      panes: vec![
        Box::new(ApisPane::new(state.clone(), true, focused_border_style)),
        Box::new(TagsPane::new(state.clone(), false, focused_border_style)),
        Box::new(AddressPane::new(state.clone(), false, focused_border_style)),
        Box::new(RequestPane::new(state.clone(), false, focused_border_style)),
        Box::new(ResponsePane::new(state.clone(), false, focused_border_style)),
      ],
      static_panes: vec![Box::new(HeaderPane::new(state.clone())), Box::new(FooterPane::new(state.clone()))],
      focused_pane_index: 0,
      state,
      fullscreen_pane_index: None,
      input_mode: InputMode::Normal,
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
      Action::ToggleFullScreen => {
        self.fullscreen_pane_index = self.fullscreen_pane_index.map_or(Some(self.focused_pane_index), |_| None);
      },
      Action::FocusFooter => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.unfocus()?;
        }
        self.static_panes[1].focus()?;
        self.input_mode = InputMode::Insert;
      },
      Action::Filter(filter) => {
        self.static_panes[1].unfocus()?;
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.focus()?;
        }
        self.input_mode = InputMode::Normal;
        {
          let mut state = self.state.write().unwrap();
          state.active_operation_index = 0;
          state.active_filter = filter;
        }
        return Ok(Some(Action::Update));
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
    match self.input_mode {
      InputMode::Normal => {
        let response = match key.code {
          KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::FocusNext),
          KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::FocusPrev),
          KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
          KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
          KeyCode::Char('g') | KeyCode::Char('G') => EventResponse::Stop(Action::Go),
          KeyCode::Backspace | KeyCode::Char('b') | KeyCode::Char('B') => EventResponse::Stop(Action::Back),
          KeyCode::Enter => EventResponse::Stop(Action::Submit),
          KeyCode::Char('f') | KeyCode::Char('F') => EventResponse::Stop(Action::ToggleFullScreen),
          KeyCode::Char(c) if ('1'..='9').contains(&c) => {
            EventResponse::Stop(Action::Tab(c.to_digit(10).unwrap_or(0) - 1))
          },
          KeyCode::Char(']') => EventResponse::Stop(Action::TabNext),
          KeyCode::Char('[') => EventResponse::Stop(Action::TabPrev),
          KeyCode::Char('/') => EventResponse::Stop(Action::FocusFooter),
          _ => {
            return Ok(None);
          },
        };
        Ok(Some(response))
      },
      InputMode::Insert => self.static_panes[1].handle_key_events(key),
    }
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let verical_layout = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(1), Constraint::Fill(1), Constraint::Max(1)])
      .split(area);

    self.static_panes[0].draw(frame, verical_layout[0])?;
    self.static_panes[1].draw(frame, verical_layout[2])?;

    if let Some(fullscreen_pane_index) = self.fullscreen_pane_index {
      self.panes[fullscreen_pane_index].draw(frame, verical_layout[1])?;
    } else {
      let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Fill(1), Constraint::Fill(3)])
        .split(verical_layout[1]);

      let left_panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![self.panes[0].height_constraint(), self.panes[1].height_constraint()])
        .split(outer_layout[0]);

      let right_panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
          self.panes[2].height_constraint(),
          self.panes[3].height_constraint(),
          self.panes[4].height_constraint(),
        ])
        .split(outer_layout[1]);

      self.panes[0].draw(frame, left_panes[0])?;
      self.panes[1].draw(frame, left_panes[1])?;
      self.panes[2].draw(frame, right_panes[0])?;
      self.panes[3].draw(frame, right_panes[1])?;
      self.panes[4].draw(frame, right_panes[2])?;
    }
    Ok(())
  }
}
