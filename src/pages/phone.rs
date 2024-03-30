use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  prelude::*,
  widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  pages::Page,
  panes::{body_editor::BodyEditor, parameter_editor::ParameterEditor, Pane},
  state::{InputMode, OperationItem, State},
  tui::{Event, EventResponse},
};

#[derive(Default)]
pub struct Phone {
  operation_item: Arc<OperationItem>,
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  focused_pane_index: usize,

  panes: Vec<Box<dyn Pane>>,
}

impl Phone {
  pub fn new(operation_item: OperationItem) -> Result<Self> {
    let focused_border_style = Style::default().fg(Color::LightGreen);
    let operation_item = Arc::new(operation_item);
    let parameter_editor = ParameterEditor::new(operation_item.clone(), true, focused_border_style);
    let body_editor = BodyEditor::new(operation_item.clone(), false, focused_border_style);
    Ok(Self {
      operation_item,
      command_tx: None,
      config: Config::default(),
      panes: vec![Box::new(parameter_editor), Box::new(body_editor)],
      focused_pane_index: 0,
    })
  }

  fn method_color(method: &str) -> Color {
    match method {
      "GET" => Color::LightCyan,
      "POST" => Color::LightBlue,
      "PUT" => Color::LightYellow,
      "DELETE" => Color::LightRed,
      _ => Color::Gray,
    }
  }

  fn base_url(&self, state: &State) -> String {
    if let Some(server) = state.openapi_spec.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
      String::from(server.url.trim_end_matches('/'))
    } else if let Some(server) = &self.operation_item.operation.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
      String::from(server.url.trim_end_matches('/'))
    } else {
      String::from("http://localhost")
    }
  }
}

impl Page for Phone {
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

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Normal => {
        let response = match key.code {
          KeyCode::Esc => EventResponse::Stop(Action::HangUp(self.operation_item.operation.operation_id.clone())),
          KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::FocusNext),
          KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::FocusPrev),
          KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
          KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
          KeyCode::Char(c) if ('1'..='9').contains(&c) => {
            EventResponse::Stop(Action::Tab(c.to_digit(10).unwrap_or(0) - 1))
          },
          KeyCode::Char(']') => EventResponse::Stop(Action::TabNext),
          KeyCode::Char('[') => EventResponse::Stop(Action::TabPrev),
          KeyCode::Enter => EventResponse::Stop(Action::Submit),
          _ => {
            return Ok(None);
          },
        };
        Ok(Some(response))
      },
      InputMode::Insert => {
        // KeyCode::Enter => EventResponse::Stop(Action::Submit),
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          let response = pane.handle_events(Event::Key(key), state)?;
          return Ok(response);
        }
        Ok(None)
      },
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
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

      _ => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          return pane.update(action, state);
        }
      },
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let outer_layout = Layout::vertical(vec![Constraint::Max(3), Constraint::Fill(3)]).split(area);
    let input_layout = Layout::horizontal(vec![Constraint::Fill(1), Constraint::Fill(1)]).split(outer_layout[1]);
    frame.render_widget(
      Paragraph::new(Line::from(vec![
        Span::styled(
          format!(" {} ", self.operation_item.method.as_str()),
          Style::default().fg(Self::method_color(self.operation_item.method.as_str())),
        ),
        Span::styled(self.base_url(state), Style::default().fg(Color::DarkGray)),
        Span::styled(&self.operation_item.path, Style::default().fg(Color::White)),
      ]))
      .block(
        Block::new().title(self.operation_item.operation.summary.clone().unwrap_or_default()).borders(Borders::ALL),
      ),
      outer_layout[0],
    );

    self.panes[0].draw(frame, input_layout[0], state)?;
    self.panes[1].draw(frame, input_layout[1], state)?;
    Ok(())
  }
}
