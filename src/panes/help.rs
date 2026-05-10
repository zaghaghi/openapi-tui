use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};

use crate::{
  action::Action,
  panes::Pane,
  state::{InputMode, State},
  tui::{EventResponse, Frame},
};

struct Section {
  title: &'static str,
  rows: &'static [(&'static str, &'static str)],
}

const SECTIONS: &[Section] = &[
  Section {
    title: "Navigation",
    rows: &[
      ("→, l", "Move to next pane"),
      ("←, h", "Move to previous pane"),
      ("↓, j", "Move down in lists"),
      ("↑, k", "Move up in lists"),
      ("1…9", "Select tab by number"),
      ("]", "Next tab"),
      ("[", "Previous tab"),
      ("g", "Go into nested item / definition"),
      ("Backspace, b", "Back out of nested item"),
      (",  /  .", "Previous / next schema variant"),
      ("a", "Toggle annotated/YAML schema view"),
      ("f", "Toggle fullscreen pane"),
      ("Enter", "Submit / edit / execute"),
      ("Esc", "Cancel / close popup / exit edit mode"),
    ],
  },
  Section {
    title: "Global",
    rows: &[
      ("?", "Show this help"),
      ("/", "Filter APIs"),
      (":", "Run a command"),
      ("q", "Quit"),
      ("Ctrl+C, Ctrl+D", "Quit"),
      ("Ctrl+Z", "Suspend"),
    ],
  },
  Section {
    title: "Commands — Main page",
    rows: &[
      (":q", "Quit"),
      (":request, :r", "Open request page for selected operation"),
      (":history", "Open request history popup"),
      (":auth", "Open authentication popup"),
      (":help", "Show this help"),
    ],
  },
  Section {
    title: "Commands — Request page",
    rows: &[
      (":q", "Quit"),
      (":send, :s", "Send the request"),
      (":auth", "Open authentication popup"),
      (":copy [curl|httpie]", "Copy request to clipboard (default: curl)"),
      (":query add|rm <name>", "Add or remove a query parameter"),
      (":header add|rm <name>", "Add or remove a header"),
      (":request open <path>", "Load request payload from file"),
      (":response save <path>", "Save response payload to file"),
      (":jq <expr>", "Filter JSON response with jq (empty clears)"),
      (":search <term>", "Search response body (empty clears)"),
      (":help", "Show this help"),
    ],
  },
];

#[derive(Default)]
pub struct HelpPane {
  scroll: u16,
}

impl HelpPane {
  pub fn new() -> Self {
    Self::default()
  }

  fn lines(&self) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let key_width = SECTIONS.iter().flat_map(|s| s.rows.iter()).map(|(k, _)| k.chars().count()).max().unwrap_or(20);

    for (idx, section) in SECTIONS.iter().enumerate() {
      if idx > 0 {
        lines.push(Line::from(""));
      }
      lines.push(Line::from(Span::styled(
        section.title,
        Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
      )));
      for (key, desc) in section.rows {
        lines.push(Line::from(vec![
          Span::styled(format!("  {key:<key_width$}  "), Style::default().fg(Color::LightYellow)),
          Span::raw(*desc),
        ]));
      }
    }
    lines
  }

  fn total_lines(&self) -> u16 {
    let count: usize = SECTIONS.iter().map(|s| s.rows.len() + 1).sum::<usize>() + SECTIONS.len().saturating_sub(1);
    count as u16
  }
}

impl Pane for HelpPane {
  fn height_constraint(&self) -> Constraint {
    Constraint::Fill(5)
  }

  fn width_constraint(&self) -> Constraint {
    Constraint::Fill(3)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    if state.input_mode != InputMode::Normal {
      return Ok(Some(EventResponse::Stop(Action::Noop)));
    }
    let response = match key.code {
      KeyCode::Down | KeyCode::Char('j') => EventResponse::Stop(Action::Down),
      KeyCode::Up | KeyCode::Char('k') => EventResponse::Stop(Action::Up),
      KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => EventResponse::Stop(Action::CloseHelp),
      _ => return Ok(Some(EventResponse::Stop(Action::Noop))),
    };
    Ok(Some(response))
  }

  fn update(&mut self, action: Action, _state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Down => {
        self.scroll = self.scroll.saturating_add(1);
      },
      Action::Up => {
        self.scroll = self.scroll.saturating_sub(1);
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, _state: &State) -> Result<()> {
    frame.render_widget(Clear, area);
    let inner = area.inner(Margin { horizontal: 2, vertical: 1 });

    let total = self.total_lines();
    let max_scroll = total.saturating_sub(inner.height);
    if self.scroll > max_scroll {
      self.scroll = max_scroll;
    }

    let paragraph = Paragraph::new(self.lines()).scroll((self.scroll, 0));
    frame.render_widget(paragraph, inner);

    frame.render_widget(
      Block::default()
        .borders(Borders::ALL)
        .title(" Help — keys & commands ")
        .title_bottom(Line::from(" [j,k scroll] [Esc / q / ? close] ").right_aligned()),
      area,
    );
    Ok(())
  }
}
