use std::time::Instant;

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

struct TimedStatusLine {
  created: Instant,
  show_time: u64,
  status_line: String,
}

#[derive(Default)]
pub struct FooterPane {
  focused: bool,
  input: Input,
  command: String,
  status_line: String,
  timed_status_line: Option<TimedStatusLine>,
}

impl FooterPane {
  pub fn new() -> Self {
    Self {
      focused: false,
      input: Input::default(),
      command: String::default(),
      status_line: String::default(),
      timed_status_line: None,
    }
  }

  fn get_status_line(&mut self) -> &String {
    if self.timed_status_line.as_ref().is_some_and(|tsl| tsl.created.elapsed().as_secs() < tsl.show_time) {
      return &self.timed_status_line.as_ref().unwrap().status_line;
    }
    self.timed_status_line = None;
    &self.status_line
  }
}

impl Pane for FooterPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Max(1)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Command => {
        self.input.handle_event(&Event::Key(key));
        let response = match key.code {
          KeyCode::Enter => {
            Some(EventResponse::Stop(Action::FooterResult(self.command.clone(), Some(self.input.to_string()))))
          },
          KeyCode::Esc => Some(EventResponse::Stop(Action::FooterResult(self.command.clone(), None))),
          _ => None,
        };
        Ok(response)
      },
      _ => Ok(None),
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::FocusFooter(cmd, args) => {
        self.focused = true;
        state.input_mode = InputMode::Command;
        if let Some(args) = args {
          self.input = self.input.clone().with_value(args);
        } else {
          self.input = self.input.clone().with_value("".into());
        }
        self.command = cmd;
        Ok(Some(Action::Update))
      },
      Action::FooterResult(..) => {
        state.input_mode = InputMode::Normal;
        self.focused = false;
        Ok(Some(Action::Update))
      },
      Action::StatusLine(status_line) => {
        self.status_line = status_line;
        Ok(None)
      },
      Action::TimedStatusLine(status_line, show_time) => {
        self.timed_status_line = Some(TimedStatusLine { status_line, show_time, created: Instant::now() });
        Ok(None)
      },
      _ => Ok(None),
    }
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    if self.focused {
      let mut area = area;
      area.width = area.width.saturating_sub(4);

      let width = area.width.max(3);
      let scroll = self.input.visual_scroll(width as usize - self.command.len());
      let input = Paragraph::new(Line::from(vec![
        Span::styled(&self.command, Style::default().fg(Color::LightBlue)),
        Span::styled(self.input.value(), Style::default()),
      ]))
      .scroll((0, scroll as u16));
      frame.render_widget(input, area);

      frame.set_cursor(
        area.x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + self.command.len() as u16,
        area.y + 1,
      )
    } else {
      frame.render_widget(
        Line::from(vec![Span::styled(self.get_status_line(), Style::default())])
          .style(Style::default().fg(Color::DarkGray)),
        area,
      );
    }
    frame.render_widget(
      Line::from(vec![match state.input_mode {
        InputMode::Normal => Span::from("[N]"),
        InputMode::Insert => Span::from("[I]"),
        InputMode::Command => Span::from("[C]"),
      }])
      .right_aligned(),
      area,
    );

    Ok(())
  }
}
