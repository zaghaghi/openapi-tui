use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  pages::phone::{RequestBuilder, RequestPane},
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

impl RequestPane for ResponseViewer {
}

impl RequestBuilder for ResponseViewer {
  fn reqeust(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(content_type) = self.content_types.get(self.content_type_index) {
      request.header("accept", content_type)
    } else {
      request
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
    match self.focused {
      true => Constraint::Fill(3),
      false => Constraint::Fill(1),
    }
  }

  fn handle_key_events(&mut self, _key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Normal => Ok(None),
      InputMode::Insert => Ok(None),
    }
  }

  fn update(&mut self, action: Action, _state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {},
      Action::Submit => return Ok(Some(Action::Dial)),
      Action::Tab(index) if !self.content_types.is_empty() && index < self.content_types.len().try_into()? => {
        self.content_type_index = index.try_into()?;
      },
      Action::TabNext if !self.content_types.is_empty() => {
        let next_tab_index = self.content_type_index + 1;
        self.content_type_index =
          if next_tab_index < self.content_types.len() { next_tab_index } else { self.content_type_index };
      },
      Action::TabPrev if !self.content_types.is_empty() => {
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
    let inner_panes = Layout::horizontal([Constraint::Fill(3), Constraint::Fill(1)]).split(inner);

    let mut status_line = String::default();

    if let Some(response) =
      self.operation_item.operation.operation_id.as_ref().and_then(|operation_id| state.responses.get(operation_id))
    {
      status_line = format!(
        "[{:?} {} {} {}]",
        response.version,
        response.status.as_str(),
        symbols::DOT,
        humansize::format_size(response.content_length.unwrap_or(response.body.len() as u64), humansize::DECIMAL)
      );
      frame.render_widget(
        Paragraph::new(response.body.clone()).wrap(Wrap { trim: false }).block(
          Block::default().borders(Borders::RIGHT).border_style(self.border_style()).border_type(self.border_type()),
        ),
        inner_panes[0],
      );
      frame.render_widget(
        List::new(
          response
            .headers
            .iter()
            .map(|(hk, hv)| {
              Line::from(vec![
                Span::styled(format!("{}: ", hk), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(hv.to_str().unwrap_or("ERROR")),
              ])
            })
            .collect::<Vec<_>>(),
        ),
        inner_panes[1],
      );
    }

    let content_types = if !self.content_types.is_empty() {
      let ctype = self.content_types[self.content_type_index].clone();
      let ctype_progress = if self.content_types.len() > 1 {
        format!("[{}/{}]", self.content_type_index + 1, self.content_types.len())
      } else {
        String::default()
      };
      format!(": {ctype} {ctype_progress}")
    } else {
      String::default()
    };

    frame.render_widget(
      Block::default()
        .title(format!("Response{content_types}"))
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type())
        .title_bottom(Line::from(status_line).right_aligned()),
      area,
    );

    Ok(())
  }
}
