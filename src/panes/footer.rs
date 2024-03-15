use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::Paragraph};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
  action::Action,
  pages::home::State,
  panes::Pane,
  tui::{EventResponse, Frame},
};

#[derive(Default)]
pub struct FooterPane {
  focused: bool,
  #[allow(dead_code)]
  state: Arc<RwLock<State>>,
  input: Input,
}

impl FooterPane {
  pub fn new(state: Arc<RwLock<State>>) -> Self {
    Self { focused: false, state, input: Input::default() }
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

  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<EventResponse<Action>>> {
    self.input.handle_event(&Event::Key(key));
    let response = match key.code {
      KeyCode::Enter => Some(EventResponse::Stop(Action::Filter(self.input.to_string()))),
      KeyCode::Esc => {
        let filter: String;
        {
          let state = self.state.read().unwrap();
          filter = state.active_filter.clone();
        }
        Some(EventResponse::Stop(Action::Filter(filter)))
      },
      _ => Some(EventResponse::Stop(Action::Noop)),
    };
    Ok(response)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    const ARROW: &str = symbols::scrollbar::HORIZONTAL.end;
    if self.focused {
      let search_label = "Filter: ";
      let width = area.width.max(3);
      let scroll = self.input.visual_scroll(width as usize);
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
          Span::styled(format!("[l,h,j,k {ARROW} movement] [/ {ARROW} search] [1-9 {ARROW} select tab] [g,b {ARROW} go/back definitions] [q {ARROW} quit]"), Style::default()),
        ])
        .style(Style::default().fg(Color::DarkGray)),
        area,
      );
    }
    Ok(())
  }
}
