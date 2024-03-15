use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use ratatui::prelude::*;

use crate::{pages::home::State, panes::Pane, tui::Frame};

#[derive(Default)]
pub struct HeaderPane {
  state: Arc<RwLock<State>>,
}

impl HeaderPane {
  pub fn new(state: Arc<RwLock<State>>) -> Self {
    Self { state }
  }
}

impl Pane for HeaderPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Max(1)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    let state = self.state.read().unwrap();
    frame.render_widget(
      Line::from(vec![
        Span::styled(
          format!("[ {} {} ", state.openapi_spec.info.title, symbols::DOT),
          Style::default().fg(Color::Blue),
        ),
        Span::styled(format!("{} ", state.openapi_spec.info.version), Style::default().fg(Color::LightCyan)),
        Span::styled("]", Style::default().fg(Color::Blue)),
      ])
      .right_aligned(),
      area,
    );

    Ok(())
  }
}
