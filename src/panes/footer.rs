use std::sync::{Arc, RwLock};

use color_eyre::eyre::Result;
use ratatui::prelude::*;

use crate::{pages::home::State, panes::Pane, tui::Frame};

#[derive(Default)]
pub struct FooterPane {
  #[allow(dead_code)]
  state: Arc<RwLock<State>>,
}

impl FooterPane {
  pub fn new(state: Arc<RwLock<State>>) -> Self {
    Self { state }
  }
}

impl Pane for FooterPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Max(1)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    const ARROW: &str = symbols::scrollbar::HORIZONTAL.end;
    frame.render_widget(
      Line::from(vec![
        Span::styled(format!("[l/h {ARROW} next/prev pane] [j/k {ARROW} next/prev item] [1-9 {ARROW} select tab] [g/b {ARROW} go/back definitions] [q {ARROW} quit]"), Style::default()),
      ])
      .style(Style::default().fg(Color::DarkGray)),
      area,
    );
    Ok(())
  }
}
