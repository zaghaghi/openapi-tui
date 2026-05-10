use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
  action::Action,
  panes::Pane,
  state::{InputMode, State},
  tui::{EventResponse, Frame},
};

#[derive(Default)]
pub struct AuthPane {
  selected: usize,
  input: Input,
  scheme_count: usize,
}

impl AuthPane {
  pub fn new(state: &State) -> Self {
    Self { selected: 0, input: Input::default(), scheme_count: state.auth_schemes.len() }
  }

  fn mask(value: &str) -> String {
    if value.is_empty() {
      String::new()
    } else if value.len() <= 4 {
      "•".repeat(value.len())
    } else {
      let visible = &value[value.len().saturating_sub(4)..];
      format!("{}{visible}", "•".repeat(value.len() - 4))
    }
  }
}

impl Pane for AuthPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Fill(3)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Normal => {
        let response = match key.code {
          KeyCode::Down | KeyCode::Char('j') => EventResponse::Stop(Action::Down),
          KeyCode::Up | KeyCode::Char('k') => EventResponse::Stop(Action::Up),
          KeyCode::Esc => EventResponse::Stop(Action::CloseAuth),
          KeyCode::Enter => EventResponse::Stop(Action::Submit),
          KeyCode::Char('d') => {
            if let Some(scheme) = state.auth_schemes.get(self.selected) {
              state.auth_values.remove(&scheme.name);
            }
            EventResponse::Stop(Action::Update)
          },
          _ => return Ok(Some(EventResponse::Stop(Action::Noop))),
        };
        Ok(Some(response))
      },
      InputMode::Insert => match key.code {
        KeyCode::Enter => Ok(Some(EventResponse::Stop(Action::Submit))),
        KeyCode::Esc => Ok(Some(EventResponse::Stop(Action::CloseAuth))),
        _ => {
          self.input.handle_event(&Event::Key(key));
          Ok(Some(EventResponse::Stop(Action::Noop)))
        },
      },
      InputMode::Command => Ok(Some(EventResponse::Stop(Action::Noop))),
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Down if self.scheme_count > 0 => {
        self.selected = (self.selected + 1) % self.scheme_count;
      },
      Action::Up if self.scheme_count > 0 => {
        self.selected = (self.selected + self.scheme_count - 1) % self.scheme_count;
      },
      Action::Submit if state.input_mode == InputMode::Normal => {
        if let Some(scheme) = state.auth_schemes.get(self.selected) {
          if !scheme.kind.is_supported() {
            return Ok(Some(Action::TimedStatusLine("scheme type not supported".into(), 2)));
          }
          let current = state.auth_values.get(&scheme.name).cloned().unwrap_or_default();
          self.input = self.input.clone().with_value(current);
          state.input_mode = InputMode::Insert;
        }
      },
      Action::Submit if state.input_mode == InputMode::Insert => {
        if let Some(scheme) = state.auth_schemes.get(self.selected) {
          let value = self.input.value().to_string();
          if value.is_empty() {
            state.auth_values.remove(&scheme.name);
          } else {
            state.auth_values.insert(scheme.name.clone(), value);
          }
        }
        self.input.reset();
        state.input_mode = InputMode::Normal;
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    frame.render_widget(Clear, area);
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    let row_widths = [Constraint::Fill(2), Constraint::Fill(3), Constraint::Fill(3)];
    let column_widths = Layout::horizontal(row_widths).split(inner);

    let rows = state.auth_schemes.iter().enumerate().map(|(i, scheme)| {
      let stored = state.auth_values.get(&scheme.name);
      let value_cell = match (state.input_mode == InputMode::Insert && i == self.selected, stored) {
        (true, _) => Span::default(),
        (false, Some(v)) => Span::styled(Self::mask(v), Style::default().fg(Color::LightGreen)),
        (false, None) => Span::styled("(unset)", Style::default().dim()),
      };
      let name_style = if scheme.kind.is_supported() { Style::default() } else { Style::default().dim() };
      Row::new(vec![
        Cell::from(Span::styled(scheme.name.clone(), name_style)),
        Cell::from(Span::styled(scheme.kind.label(), Style::default().fg(Color::LightCyan))),
        Cell::from(value_cell),
      ])
    });

    let table = Table::new(rows, vec![column_widths[0].width, column_widths[1].width, column_widths[2].width])
      .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
      .highlight_spacing(HighlightSpacing::Always)
      .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));
    let mut table_state = TableState::default().with_selected(Some(self.selected));
    frame.render_stateful_widget(table, inner, &mut table_state);

    if state.input_mode == InputMode::Insert {
      let input_area = Rect {
        x: inner.x + column_widths[0].width + column_widths[1].width,
        y: inner.y + self.selected as u16,
        width: column_widths[2].width,
        height: 1,
      };
      let scroll = self.input.visual_scroll(input_area.width as usize);
      let input =
        Paragraph::new(Line::from(vec![Span::styled(self.input.value(), Style::default().fg(Color::LightBlue))]))
          .scroll((0, scroll as u16));
      frame.set_cursor_position(Position::new(
        input_area.x + self.input.visual_cursor().saturating_sub(scroll) as u16,
        input_area.y,
      ));
      frame.render_widget(input, input_area);
    }

    let title = if state.auth_schemes.is_empty() {
      " Authentication — no security schemes in spec "
    } else {
      " Authentication — [⏎ edit] [d clear] [Esc close] "
    };
    frame.render_widget(Block::default().borders(Borders::ALL).title(title), area);

    if state.auth_schemes.is_empty() {
      frame.render_widget(
        Paragraph::new("This API does not declare any securitySchemes.").style(Style::default().dim()),
        inner,
      );
    }
    Ok(())
  }
}
