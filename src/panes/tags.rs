use color_eyre::eyre::Result;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{action::Action, panes::Pane, state::State, tui::Frame};

#[derive(Default)]
pub struct TagsPane {
  focused: bool,
  focused_border_style: Style,
  current_tag_index: usize,
}

impl TagsPane {
  pub fn new(focused: bool, focused_border_style: Style) -> Self {
    Self { focused, focused_border_style, current_tag_index: 0 }
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

  fn update_active_tag(&mut self, state: &mut State) {
    if self.current_tag_index > 0 {
      if let Some(tag) = state.openapi_spec.tags.as_ref().into_iter().flatten().nth(self.current_tag_index - 1) {
        state.active_tag_name = Some(tag.name.clone());
        state.active_operation_index = 0;
      }
    } else {
      state.active_tag_name = None;
      state.active_operation_index = 0;
    }
  }
}

impl Pane for TagsPane {
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

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Down => {
        {
          let tags_list_len = state.openapi_spec.tags.as_ref().into_iter().flatten().count().saturating_add(1);
          if tags_list_len > 0 {
            self.current_tag_index = self.current_tag_index.saturating_add(1) % tags_list_len;
          }
        }
        self.update_active_tag(state);
        return Ok(Some(Action::Update));
      },
      Action::Up => {
        {
          let tags_list_len = state.openapi_spec.tags.as_ref().into_iter().flatten().count().saturating_add(1);
          if tags_list_len > 0 {
            self.current_tag_index = self.current_tag_index.saturating_add(tags_list_len - 1) % tags_list_len;
          }
        }
        self.update_active_tag(state);
        return Ok(Some(Action::Update));
      },
      Action::Submit => {},
      _ => {},
    }

    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let mut items: Vec<Line<'_>> = state
      .openapi_spec
      .tags
      .iter()
      .flatten()
      .map(|tag| Line::from(vec![Span::styled(format!(" {}", tag.name), Style::default())]))
      .collect();

    items.insert(0, Line::styled(" [ALL]", Style::default()));

    let list = List::new(items)
      .block(Block::default().borders(Borders::ALL))
      .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
      .highlight_spacing(HighlightSpacing::Always)
      .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    let mut list_state = ListState::default().with_selected(Some(self.current_tag_index));

    frame.render_stateful_widget(list, area, &mut list_state);
    let items_len = state.openapi_spec.tags.as_ref().into_iter().flatten().count() + 1;
    frame.render_widget(
      Block::default()
        .title("Tags")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type())
        .title_bottom(
          Line::from(format!("{} of {}", self.current_tag_index.saturating_add(1), items_len)).right_aligned(),
        ),
      area,
    );
    Ok(())
  }
}
