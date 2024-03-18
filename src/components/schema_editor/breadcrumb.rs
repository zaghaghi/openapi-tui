use ratatui::prelude::*;

const ARROW: &'static str = "›";

pub fn render_breadcrumb(area: Rect, buf: &mut Buffer, path: Vec<String>) {
  let mut spans = vec![];

  for p in path {
    spans.push(Span::raw(ARROW).light_cyan());
    spans.push(Span::raw(format!(" {p} ")).cyan());
  }

  Line::from(spans).render(area, buf);
}
