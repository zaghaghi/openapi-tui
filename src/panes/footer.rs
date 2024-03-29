use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::Paragraph};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
  action::Action,
  panes::Pane,
  state::{InputMode, State},
  tui::{EventResponse, Frame},
};

#[derive(Default)]
pub struct FooterPane {
  focused: bool,
  input: Input,
}

impl FooterPane {
  pub fn new() -> Self {
    Self { focused: false, input: Input::default() }
  }
}

impl Pane for FooterPane {
  fn focus(&mut self) -> Result<()> {
    self.focused = true;
    Ok(())
  }

  fn unfocus(&mut self) -> Result<()> {
    self.focused = false;
    Ok(())
  }

  fn height_constraint(&self) -> Constraint {
    Constraint::Max(1)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Normal => Ok(None),
      InputMode::Insert => {
        self.input.handle_event(&Event::Key(key));
        let response = match key.code {
          KeyCode::Enter => Some(EventResponse::Stop(Action::Filter(self.input.to_string()))),
          KeyCode::Esc => Some(EventResponse::Stop(Action::Filter(state.active_filter.clone()))),
          _ => None,
        };
        Ok(response)
      },
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::FocusFooter => {
        self.focus()?;
        state.input_mode = InputMode::Insert;
        Ok(Some(Action::Update))
      },
      Action::Filter(_) => {
        state.input_mode = InputMode::Normal;
        self.unfocus()?;
        Ok(Some(Action::Update))
      },
      _ => Ok(None),
    }
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    const ARROW: &str = symbols::scrollbar::HORIZONTAL.end;
    if self.focused {
      let mut area = area;
      area.width = area.width.saturating_sub(4);

      let search_label = "Filter: ";
      let width = area.width.max(3);
      let scroll = self.input.visual_scroll(width as usize - search_label.len());
      let input = Paragraph::new(Line::from(vec![
        Span::styled(search_label, Style::default().fg(Color::LightBlue)),
        Span::styled(self.input.value(), Style::default()),
      ]))
      .scroll((0, scroll as u16));
      frame.render_widget(input, area);

      frame.set_cursor(
        area.x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + search_label.len() as u16,
        area.y + 1,
      )
    } else {
      frame.render_widget(
       Line::from(vec![
          Span::styled(format!("[l,h,j,k {ARROW} movement] [/ {ARROW} filter] [1-9 {ARROW} select tab] [g,b {ARROW} go/back definitions] [q {ARROW} quit]"), Style::default()),
        ])
        .style(Style::default().fg(Color::DarkGray)),
        area,
      );
    }
    frame.render_widget(
      Line::from(vec![match state.input_mode {
        InputMode::Normal => Span::from("[N]"),
        InputMode::Insert => Span::from("[I]"),
      }])
      .right_aligned(),
      area,
    );

    Ok(())
  }
}
