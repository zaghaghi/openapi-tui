use ratatui::prelude::*;
use tui_prompts::TextPrompt;

use super::state::SchemaEditorPageState;

pub fn render_page(area: Rect, buf: &mut Buffer, state: &SchemaEditorPageState<'_>) {
  let area = area.inner(&Margin::new(0, 1));
  let areas = split_layout(area, state.fields.len());
  for (idx, (key, area)) in state.fields.iter().zip(areas).enumerate() {
    if area.area() == 0 {
      continue;
    }

    if state.selected == idx {
      if let Some(left) = area.columns().next() {
        let symbol = if state.inside { symbols::line::VERTICAL } else { symbols::scrollbar::HORIZONTAL.end };
        Text::styled(symbol, Style::default().dim()).render(left, buf);
      }
    }

    let area = area.inner(&Margin::new(2, 0));

    if let Some(value) = state.prompt_states.get(key) {
      let mut state = value.1.write().unwrap();
      TextPrompt::from(key.clone()).render(area, buf, &mut state);
    } else if let Some(_) = state.children.get(key) {
      Text::from(format!("🗀 {key} ›")).style(Style::default().white()).render(area, buf);
    }
  }
}

pub fn split_layout(area: Rect, properties: usize) -> Vec<Rect> {
  Layout::default()
    .direction(Direction::Vertical)
    .constraints(vec![Constraint::Length(1); properties])
    .split(area)
    .to_vec()
}
