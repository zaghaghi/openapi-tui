mod breadcrumb;
mod page;
mod state;

use std::marker::PhantomData;

use ratatui::prelude::*;
pub use state::{SchemaEditorPageState, SchemaEditorState};

use self::{breadcrumb::render_breadcrumb, page::render_page};

#[derive(Clone, Copy)]
pub struct SchemaEditor<'a> {
  _marker: &'a PhantomData<()>,
}

impl Default for SchemaEditor<'_> {
  fn default() -> Self {
    Self::new()
  }
}

impl SchemaEditor<'_> {
  pub fn new() -> Self {
    Self { _marker: &PhantomData }
  }

  pub fn schema_path(&self) -> Vec<String> {
    vec![]
  }
}

impl<'a> StatefulWidget for SchemaEditor<'a> {
  type State = SchemaEditorState<'a>;

  fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
    let json = format!("{:?}", state.to_json().map(|j| j.to_string()));
    Span::raw(json).render(area, buf);
    let area = area.inner(&Margin::new(0, 1));
    if let Some((path, state)) = state.page() {
      render_breadcrumb(area, buf, path);
      render_page(area, buf, state)
    }
  }
}
