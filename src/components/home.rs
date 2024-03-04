use std::rc::Rc;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use oas3::Spec;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, component::Component, config::Config, tui::EventResponse};

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
  openapi_path: String,
  openapi_spec: Option<Spec>,
  openapi_spec_operations_len: usize,
  openapi_spec_operations_index: usize,
}

impl Home {
  pub fn new(openapi_path: String) -> Self {
    Self {
      command_tx: None,
      config: Config::default(),
      focus: Pane::default(),
      openapi_path,
      openapi_spec: None,
      openapi_spec_operations_len: 0,
      openapi_spec_operations_index: 0,
    }
  }

  fn render_left_panes(&mut self, frame: &mut Frame<'_>, area: Rc<[Rect]>) {
    let focued_border_style = Style::default().fg(Color::LightGreen);

    frame.render_widget(
      Block::default().title("Profiles").borders(Borders::ALL).border_style(match self.focus {
        Pane::Profiles => focued_border_style,
        _ => Style::default(),
      }),
      area[0],
    );
    frame.render_widget(
      Block::default()
        .title("APIs")
        .borders(Borders::ALL)
        .border_style(match self.focus {
          Pane::Apis => focued_border_style,
          _ => Style::default(),
        })
        .title_bottom(
          Line::from(format!(
            "{} of {}",
            self.openapi_spec_operations_index.saturating_add(1),
            self.openapi_spec_operations_len
          ))
          .right_aligned(),
        ),
      area[1],
    );
    frame.render_widget(
      Block::default().title("Tags").borders(Borders::ALL).border_style(match self.focus {
        Pane::Tags => focued_border_style,
        _ => Style::default(),
      }),
      area[2],
    );
  }

  fn render_right_panes(&mut self, frame: &mut Frame<'_>, area: Rc<[Rect]>) {
    let focued_border_style = Style::default().fg(Color::LightGreen);

    frame.render_widget(
      Block::default().borders(Borders::ALL).border_style(match self.focus {
        Pane::Call => focued_border_style,
        _ => Style::default(),
      }),
      area[0],
    );
    frame.render_widget(
      Block::default().title("Request").borders(Borders::ALL).border_style(match self.focus {
        Pane::Request => focued_border_style,
        _ => Style::default(),
      }),
      area[1],
    );
    frame.render_widget(
      Block::default().title("Response").borders(Borders::ALL).border_style(match self.focus {
        Pane::Response => focued_border_style,
        _ => Style::default(),
      }),
      area[2],
    );
  }

  fn method_color(method: &str) -> Color {
    // method.0 is private, so matching
    if method == "GET" {
      return Color::LightCyan;
    }
    if method == "POST" {
      return Color::LightBlue;
    }
    if method == "PUT" {
      return Color::LightYellow;
    }
    if method == "DELETE" {
      return Color::LightRed;
    }

    Color::Gray
  }
}

impl Component for Home {
  fn init(&mut self) -> Result<()> {
    self.openapi_spec = Some(oas3::from_path(self.openapi_path.clone())?);
    self.openapi_spec_operations_len = match &self.openapi_spec {
      Some(spec) => spec.operations().count(),
      None => 0,
    };
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
      Action::Focus(pane) => self.focus = pane,
      Action::Down => {
        match self.focus {
          Pane::Apis => {
            if self.openapi_spec_operations_len > 0 {
              self.openapi_spec_operations_index =
                self.openapi_spec_operations_index.saturating_add(1) % self.openapi_spec_operations_len;
            }
          },
          Pane::Profiles => {},
          _ => {},
        }
      },
      Action::Up => {
        match self.focus {
          Pane::Apis => {
            if self.openapi_spec_operations_len > 0 {
              self.openapi_spec_operations_index =
                self.openapi_spec_operations_index.saturating_add(self.openapi_spec_operations_len - 1)
                  % self.openapi_spec_operations_len;
            }
          },
          Pane::Profiles => {},
          _ => {},
        }
      },
      _ => {},
    }
    Ok(None)
  }

  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    let response = match key.code {
      KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::Focus(self.focus.next())),
      KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::Focus(self.focus.prev())),
      KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
      KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
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
      .split(frame.size());

    let left_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(5), Constraint::Fill(3), Constraint::Fill(1)])
      .split(outer_layout[0]);

    let right_panes = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(5), Constraint::Fill(1), Constraint::Fill(1)])
      .split(outer_layout[1]);

    let unknown = String::from("Unknown");
    if let Some(spec) = &self.openapi_spec {
      let items = spec.operations().map(|operation| {
        Line::from(vec![
          Span::styled(
            format!("{:7}", operation.1.as_str()),
            Style::default().fg(Home::method_color(operation.1.as_str())),
          ),
          Span::styled(
            operation.2.summary.as_ref().unwrap_or(operation.2.operation_id.as_ref().unwrap_or(&unknown)),
            Style::default().fg(Color::White),
          ),
        ])
      });

      let list = List::new(items)
        .block(Block::default().title("List").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
        .direction(ListDirection::TopToBottom);
      let mut state = ListState::default().with_selected(Some(self.openapi_spec_operations_index));

      frame.render_stateful_widget(list, left_panes[1], &mut state);
    }
    self.render_left_panes(frame, left_panes);
    self.render_right_panes(frame, right_panes);
    Ok(())
  }
}
