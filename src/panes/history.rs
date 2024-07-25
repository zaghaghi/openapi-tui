use std::ops::Not;

use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  panes::Pane,
  state::{InputMode, OperationItem, State},
  tui::{EventResponse, Frame},
};

#[derive(Default)]
struct OperationHistoryItem {
  operation_id: String,
  method: String,
  path: String,
}

#[derive(Default)]
pub struct HistoryPane {
  history: Vec<OperationHistoryItem>,
  history_item_index: Option<usize>,
}

impl HistoryPane {
  pub fn new(operation_ids: Vec<&OperationItem>) -> Self {
    let history = operation_ids
      .iter()
      .filter_map(|opertation_item| {
        opertation_item.operation.operation_id.as_ref().map(|operation_id| OperationHistoryItem {
          operation_id: operation_id.clone(),
          method: opertation_item.method.clone(),
          path: opertation_item.path.clone(),
        })
      })
      .collect::<Vec<OperationHistoryItem>>();
    let history_item_index = history.is_empty().not().then_some(0);
    Self { history, history_item_index }
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
}

impl Pane for HistoryPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Fill(3)
  }

  fn handle_key_events(
    &mut self,
    key: crossterm::event::KeyEvent,
    state: &mut State,
  ) -> Result<Option<crate::tui::EventResponse<crate::action::Action>>> {
    match state.input_mode {
      InputMode::Normal => {
        let response = match key.code {
          KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
          KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
          KeyCode::Esc => EventResponse::Stop(Action::CloseHistory),
          KeyCode::Enter => {
            if let Some(item_index) = self.history_item_index {
              EventResponse::Stop(Action::NewCall(self.history.get(item_index).map(|item| item.operation_id.clone())))
            } else {
              return Ok(Some(EventResponse::Stop(Action::Noop)));
            }
          },
          _ => {
            return Ok(Some(EventResponse::Stop(Action::Noop)));
          },
        };
        Ok(Some(response))
      },
      InputMode::Insert => Ok(Some(EventResponse::Stop(Action::Noop))),
      InputMode::Command => Ok(Some(EventResponse::Stop(Action::Noop))),
    }
  }

  fn update(&mut self, action: Action, _state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Down => {
        let history_len = self.history.len();
        if history_len > 0 {
          self.history_item_index = self.history_item_index.map(|item_idx| item_idx.saturating_add(1) % history_len);
        } else {
          self.history_item_index = None;
        }
        return Ok(Some(Action::Update));
      },
      Action::Up => {
        let history_len = self.history.len();
        if history_len > 0 {
          self.history_item_index = self
            .history_item_index
            .map(|item_idx| item_idx.saturating_add(history_len.saturating_sub(1)) % history_len);
        } else {
          self.history_item_index = None;
        }
        return Ok(Some(Action::Update));
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, _state: &State) -> Result<()> {
    frame.render_widget(Clear, area);
    let items = self.history.iter().map(|item| {
      Line::from(vec![
        Span::styled(format!(" {:7}", item.method), Self::method_color(item.method.as_str())),
        Span::from(item.path.clone()),
      ])
    });
    let list = List::new(items)
      .block(Block::default().borders(Borders::ALL))
      .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
      .highlight_spacing(HighlightSpacing::Always)
      .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    let mut list_state = ListState::default().with_selected(self.history_item_index);

    frame.render_stateful_widget(list, area, &mut list_state);
    frame.render_widget(Block::default().borders(Borders::ALL).title("Request History").style(Style::default()), area);
    Ok(())
  }
}
