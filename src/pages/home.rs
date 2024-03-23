use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  pages::Page,
  panes::{address::AddressPane, apis::ApisPane, request::RequestPane, response::ResponsePane, tags::TagsPane, Pane},
  state::{InputMode, State},
  tui::EventResponse,
};

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  panes: Vec<Box<dyn Pane>>,
  focused_pane_index: usize,
  fullscreen_pane_index: Option<usize>,
}

impl Home {
  pub fn new() -> Result<Self> {
    let focused_border_style = Style::default().fg(Color::LightGreen);

    Ok(Self {
      command_tx: None,
      config: Config::default(),
      panes: vec![
        Box::new(ApisPane::new(true, focused_border_style)),
        Box::new(TagsPane::new(false, focused_border_style)),
        Box::new(AddressPane::new(false, focused_border_style)),
        Box::new(RequestPane::new(false, focused_border_style)),
        Box::new(ResponsePane::new(false, focused_border_style)),
      ],

      focused_pane_index: 0,
      fullscreen_pane_index: None,
    })
  }
}

impl Page for Home {
  fn init(&mut self, state: &State) -> Result<()> {
    for pane in self.panes.iter_mut() {
      pane.init(state)?;
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

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
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
          pane.update(action.clone(), state)?;
        }
      },
      Action::ToggleFullScreen => {
        self.fullscreen_pane_index = self.fullscreen_pane_index.map_or(Some(self.focused_pane_index), |_| None);
      },
      Action::FocusFooter => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.unfocus()?;
        }
      },
      Action::Filter(filter) => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.focus()?;
        }
        state.active_operation_index = 0;
        state.active_filter = filter;

        return Ok(Some(Action::Update));
      },
      _ => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          return pane.update(action, state);
        }
      },
    }
    Ok(None)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
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
      InputMode::Insert => Ok(None),
    }
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    if let Some(fullscreen_pane_index) = self.fullscreen_pane_index {
      self.panes[fullscreen_pane_index].draw(frame, area, state)?;
    } else {
      let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Fill(1), Constraint::Fill(3)])
        .split(area);

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

      self.panes[0].draw(frame, left_panes[0], state)?;
      self.panes[1].draw(frame, left_panes[1], state)?;
      self.panes[2].draw(frame, right_panes[0], state)?;
      self.panes[3].draw(frame, right_panes[1], state)?;
      self.panes[4].draw(frame, right_panes[2], state)?;
    }
    Ok(())
  }
}
