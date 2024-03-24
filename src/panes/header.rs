use color_eyre::eyre::Result;
use ratatui::prelude::*;

use crate::{panes::Pane, state::State, tui::Frame};

#[derive(Default)]
pub struct HeaderPane {}

impl HeaderPane {
  pub fn new() -> Self {
    Self {}
  }
}

impl Pane for HeaderPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Max(1)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
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
