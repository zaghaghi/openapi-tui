use std::{collections::HashMap, default, time::Duration};

use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use serde::{
  de::{self, Deserializer, Visitor},
  Deserialize, Serialize,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  component::Component,
  config::{Config, KeyBindings},
  tui::EventResponse,
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Pane {
  Profiles,
  #[default]
  Apis,
  Tags,
  Call,
  Request,
  Response,
}

impl Pane {
  pub fn next(&self) -> Pane {
    match self {
      Pane::Profiles => Pane::Apis,
      Pane::Apis => Pane::Tags,
      Pane::Tags => Pane::Call,
      Pane::Call => Pane::Request,
      Pane::Request => Pane::Response,
      Pane::Response => Pane::Profiles,
    }
  }

  pub fn prev(&self) -> Pane {
    match self {
      Pane::Apis => Pane::Profiles,
      Pane::Tags => Pane::Apis,
      Pane::Call => Pane::Tags,
      Pane::Request => Pane::Call,
      Pane::Response => Pane::Request,
      Pane::Profiles => Pane::Response,
    }
  }
}

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  focus: Pane,
}

impl Home {
  pub fn new() -> Self {
    Self::default()
  }
}

impl Component for Home {
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
      Action::Focus(pane) => self.focus = pane,
      _ => {},
    }
    Ok(None)
  }

  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    let response = match key.code {
      KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::Focus(self.focus.next())),
      KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::Focus(self.focus.prev())),
      _ => return Ok(None),
    };
    Ok(Some(response))
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let focued_border_style = Style::default().fg(Color::LightGreen);
    let outer_layout = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Min(20), Constraint::Percentage(100)])
      .split(frame.size());

    let left_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(5), Constraint::Fill(3), Constraint::Fill(1)])
      .split(outer_layout[0]);

    let right_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(5), Constraint::Fill(1), Constraint::Fill(1)])
      .split(outer_layout[1]);

    frame.render_widget(
      Block::default().title("Profiles").borders(Borders::ALL).border_style(match self.focus {
        Pane::Profiles => focued_border_style,
        _ => Style::default(),
      }),
      left_panes[0],
    );
    frame.render_widget(
      Block::default().title("APIs").borders(Borders::ALL).border_style(match self.focus {
        Pane::Apis => focued_border_style,
        _ => Style::default(),
      }),
      left_panes[1],
    );
    frame.render_widget(
      Block::default().title("Tags").borders(Borders::ALL).border_style(match self.focus {
        Pane::Tags => focued_border_style,
        _ => Style::default(),
      }),
      left_panes[2],
    );

    frame.render_widget(
      Block::default().borders(Borders::ALL).border_style(match self.focus {
        Pane::Call => focued_border_style,
        _ => Style::default(),
      }),
      right_panes[0],
    );
    frame.render_widget(
      Block::default().title("Request").borders(Borders::ALL).border_style(match self.focus {
        Pane::Request => focued_border_style,
        _ => Style::default(),
      }),
      right_panes[1],
    );
    frame.render_widget(
      Block::default().title("Response").borders(Borders::ALL).border_style(match self.focus {
        Pane::Response => focued_border_style,
        _ => Style::default(),
      }),
      right_panes[2],
    );

    Ok(())
  }
}
