use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use super::response;
use crate::{
  action::Action,
  panes::Pane,
  state::{InputMode, OperationItem, State},
  tui::{EventResponse, Frame},
};

pub struct ResponseViewer {
  focused: bool,
  focused_border_style: Style,
  operation_item: Arc<OperationItem>,
  content_types: Vec<String>,
  content_type_index: usize,
}

impl ResponseViewer {
  pub fn new(operation_item: Arc<OperationItem>, focused: bool, focused_border_style: Style) -> Self {
    Self { operation_item, focused, focused_border_style, content_types: vec![], content_type_index: 0 }
  }

  fn border_style(&self) -> Style {
    match self.focused {
      true => self.focused_border_style,
      false => Style::default(),
    }
  }

  fn border_type(&self) -> BorderType {
    match self.focused {
      true => BorderType::Thick,
      false => BorderType::Plain,
    }
  }
}

impl Pane for ResponseViewer {
  fn init(&mut self, state: &State) -> Result<()> {
    self.content_types = self
      .operation_item
      .operation
      .responses
      .as_ref()
      .and_then(|responses| responses.get("200"))
      .and_then(|ok_response| ok_response.resolve(&state.openapi_spec).ok())
      .and_then(|response| response.content)
      .map(|content| content.keys().cloned().collect::<Vec<_>>())
      .unwrap_or_default();

    Ok(())
  }

  fn focus(&mut self) -> Result<()> {
    self.focused = true;
    Ok(())
  }

  fn unfocus(&mut self) -> Result<()> {
    self.focused = false;
    Ok(())
  }

  fn height_constraint(&self) -> Constraint {
    Constraint::Fill(1)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Normal => Ok(None),
      InputMode::Insert => Ok(None),
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    if self.content_types.is_empty() {
      return Ok(None);
    }
    match action {
      Action::Update => {},
      Action::Tab(index) if index < self.content_types.len().try_into()? => {
        self.content_type_index = index.try_into()?;
      },
      Action::TabNext => {
        let next_tab_index = self.content_type_index + 1;
        self.content_type_index =
          if next_tab_index < self.content_types.len() { next_tab_index } else { self.content_type_index };
      },
      Action::TabPrev => {
        self.content_type_index =
          if self.content_type_index > 0 { self.content_type_index - 1 } else { self.content_type_index };
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let margin_h1_v1: Margin = Margin { horizontal: 1, vertical: 1 };
    let inner = area.inner(&margin_h1_v1);

    if !self.content_types.is_empty() {
      let ctype = self.content_types[self.content_type_index].clone();
      let ctype_progress = if self.content_types.len() > 1 {
        format!("[{}/{}]", self.content_type_index + 1, self.content_types.len())
      } else {
        String::default()
      };
      let line = Line::from(vec![Span::styled(format!(" Accept: {ctype} {ctype_progress}",), Style::default())]);
      frame.render_widget(line, inner);
    }
    frame.render_widget(
      Block::default()
        .title("Response")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type()),
      area,
    );

    Ok(())
  }
}
