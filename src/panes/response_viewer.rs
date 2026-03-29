use std::{io::Write, sync::Arc};

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use jaq_core::{
  load::{Arena, File, Loader},
  Compiler, Ctx, RcIter,
};
use jaq_json::Val;
use ratatui::{prelude::*, widgets::*};
use syntect::{
  easy::HighlightLines,
  highlighting::{FontStyle as SyntectFontStyle, ThemeSet},
  parsing::SyntaxSet,
  util::LinesWithEndings,
};

use crate::{
  action::Action,
  formatters::{json::JsonFormatter, FormatterRegistry},
  pages::phone::{RequestBuilder, RequestPane},
  panes::Pane,
  state::{OperationItem, State},
  tui::{EventResponse, Frame},
};

const SYNTAX_THEME: &str = "Solarized (dark)";
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Default, PartialEq)]
enum ViewerMode {
  #[default]
  Normal,
  Search(Vec<usize>, usize), // (match byte offsets in formatted body, current index)
  Jq(String, bool),          // (result text, is_error)
}

pub struct ResponseViewer {
  focused: bool,
  focused_border_style: Style,
  operation_item: Arc<OperationItem>,
  content_types: Vec<String>,
  content_type_index: usize,
  formatter_registry: FormatterRegistry,
  mode: ViewerMode,
  scroll_offset: usize,
  // syntax highlighting cache: (formatted body text) -> highlighted lines
  highlight_cache: Option<(String, Vec<Line<'static>>)>,
  highlighter_syntax_set: SyntaxSet,
  highlighter_theme_set: ThemeSet,
}

impl ResponseViewer {
  pub fn new(operation_item: Arc<OperationItem>, focused: bool, focused_border_style: Style) -> Self {
    let mut formatter_registry = FormatterRegistry::new();
    formatter_registry.register(Box::new(JsonFormatter));
    Self {
      operation_item,
      focused,
      focused_border_style,
      content_types: vec![],
      content_type_index: 0,
      formatter_registry,
      mode: ViewerMode::Normal,
      scroll_offset: 0,
      highlight_cache: None,
      highlighter_syntax_set: SyntaxSet::load_defaults_newlines(),
      highlighter_theme_set: ThemeSet::load_defaults(),
    }
  }

  fn border_style(&self) -> Style {
    match self.focused {
      true => self.focused_border_style,
      false => Style::default(),
    }
  }

  fn border_type(&self) -> BorderType {
    match self.focused {
      true => BorderType::Thick,
      false => BorderType::Plain,
    }
  }

  fn line_number_span(n: usize, width: usize) -> Span<'static> {
    Span::styled(format!(" {n:>width$} "), Style::default().dim())
  }

  fn build_highlight(
    body: &str,
    syntax_name: &str,
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
  ) -> Vec<Line<'static>> {
    let Some(syntax) = syntax_set.find_syntax_by_extension(syntax_name) else {
      return vec![Line::raw(body.to_string())];
    };
    let total = LinesWithEndings::from(body).count();
    let width = total.to_string().len();
    let mut highlighter = HighlightLines::new(syntax, &theme_set.themes[SYNTAX_THEME]);
    LinesWithEndings::from(body)
      .enumerate()
      .filter_map(|(i, line)| highlighter.highlight_line(line, syntax_set).ok().map(|s| (i, s)))
      .map(|(i, segments)| {
        let mut spans = vec![Self::line_number_span(i + 1, width)];
        spans.extend(segments.into_iter().map(|(style, text)| {
          let fg = match style.foreground {
            syntect::highlighting::Color { r, g, b, a } if a > 0 => Some(Color::Rgb(r, g, b)),
            _ => None,
          };
          let fs = style.font_style;
          let mut modifier = Modifier::empty();
          if fs.contains(SyntectFontStyle::BOLD) {
            modifier |= Modifier::BOLD;
          }
          if fs.contains(SyntectFontStyle::ITALIC) {
            modifier |= Modifier::ITALIC;
          }
          if fs.contains(SyntectFontStyle::UNDERLINE) {
            modifier |= Modifier::UNDERLINED;
          }
          let mut ratatui_style =
            Style::default().add_modifier(modifier).underline_color(Color::Reset).bg(Color::Reset);
          if let Some(fg) = fg {
            ratatui_style = ratatui_style.fg(fg);
          }
          Span::styled(text.to_string(), ratatui_style)
        }));
        Line::from(spans)
      })
      .collect()
  }

  fn plain_with_line_numbers(text: &str) -> Text<'static> {
    let total = text.lines().count();
    let width = total.to_string().len();
    Text::from(
      text
        .lines()
        .enumerate()
        .map(|(i, line)| Line::from(vec![Self::line_number_span(i + 1, width), Span::raw(line.to_string())]))
        .collect::<Vec<_>>(),
    )
  }

  /// Returns highlighted lines, reusing the cache if the body text hasn't changed.
  fn highlighted_lines(&mut self, body: &str, syntax_name: &str) -> &[Line<'static>] {
    let needs_rebuild = self.highlight_cache.as_ref().map(|(k, _)| k.as_str()) != Some(body);
    if needs_rebuild {
      let lines = Self::build_highlight(body, syntax_name, &self.highlighter_syntax_set, &self.highlighter_theme_set);
      self.highlight_cache = Some((body.to_string(), lines));
    }
    &self.highlight_cache.as_ref().unwrap().1
  }

  fn run_search(term: &str, body: &str) -> Vec<usize> {
    if term.is_empty() {
      return vec![];
    }
    let lower_body = body.to_lowercase();
    let lower_term = term.to_lowercase();
    lower_body.match_indices(lower_term.as_str()).map(|(idx, _)| idx).collect()
  }

  fn run_jq(filter_str: &str, body: &str) -> (String, bool) {
    let input_val: Val = match serde_json::from_str::<serde_json::Value>(body) {
      Ok(v) => Val::from(v),
      Err(e) => return (format!("JSON parse error: {e}"), true),
    };

    let program = File { code: filter_str, path: () };
    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();

    let modules = match loader.load(&arena, program) {
      Ok(m) => m,
      Err(e) => return (format!("Parse error: {e:?}"), true),
    };

    let filter = match Compiler::default().with_funs(jaq_std::funs().chain(jaq_json::funs())).compile(modules) {
      Ok(f) => f,
      Err(e) => return (format!("Compile error: {e:?}"), true),
    };

    let inputs = RcIter::new(core::iter::empty());
    let mut parts = Vec::new();
    let mut has_error = false;

    for output in filter.run((Ctx::new([], &inputs), input_val)) {
      match output {
        Ok(val) => match serde_json::to_string_pretty(&serde_json::Value::from(val)) {
          Ok(s) => parts.push(s),
          Err(e) => {
            parts.push(format!("Serialization error: {e}"));
            has_error = true;
          },
        },
        Err(e) => {
          parts.push(format!("Error: {e:?}"));
          has_error = true;
        },
      }
    }

    (parts.join("\n"), has_error)
  }

  fn context_line(body: &str, byte_offset: usize) -> &str {
    let before = body[..byte_offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let after = body[byte_offset..].find('\n').map(|i| byte_offset + i).unwrap_or(body.len());
    &body[before..after]
  }
}

impl RequestPane for ResponseViewer {}

impl RequestBuilder for ResponseViewer {
  fn reqeust(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(content_type) = self.content_types.get(self.content_type_index) {
      request.header("accept", content_type)
    } else {
      request
    }
  }
}

impl Pane for ResponseViewer {
  fn init(&mut self, state: &State) -> Result<()> {
    self.content_types = self
      .operation_item
      .operation
      .responses
      .as_ref()
      .and_then(|responses| responses.get("200"))
      .and_then(|ok_response| ok_response.resolve(&state.openapi_spec).ok())
      .and_then(|response| response.content)
      .map(|content| content.keys().cloned().collect::<Vec<_>>())
      .unwrap_or_default();

    Ok(())
  }

  fn height_constraint(&self) -> Constraint {
    match self.focused {
      true => Constraint::Fill(3),
      false => Constraint::Fill(1),
    }
  }

  fn handle_key_events(&mut self, _key: KeyEvent, _state: &mut State) -> Result<Option<EventResponse<Action>>> {
    Ok(None)
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {},
      Action::Submit => return Ok(Some(Action::Dial)),
      Action::Down if self.focused => match &mut self.mode {
        ViewerMode::Normal | ViewerMode::Jq(_, _) => {
          self.scroll_offset = self.scroll_offset.saturating_add(1);
        },
        ViewerMode::Search(matches, current) if !matches.is_empty() => {
          *current = (*current + 1) % matches.len();
        },
        _ => {},
      },
      Action::Up if self.focused => match &mut self.mode {
        ViewerMode::Normal | ViewerMode::Jq(_, _) => {
          self.scroll_offset = self.scroll_offset.saturating_sub(1);
        },
        ViewerMode::Search(matches, current) if !matches.is_empty() => {
          let len = matches.len();
          *current = if *current == 0 { len - 1 } else { *current - 1 };
        },
        _ => {},
      },
      Action::Tab(index) if !self.content_types.is_empty() && index < self.content_types.len().try_into()? => {
        self.content_type_index = index.try_into()?;
      },
      Action::TabNext if !self.content_types.is_empty() => {
        let next_tab_index = self.content_type_index + 1;
        self.content_type_index =
          if next_tab_index < self.content_types.len() { next_tab_index } else { self.content_type_index };
      },
      Action::TabPrev if !self.content_types.is_empty() => {
        self.content_type_index =
          if self.content_type_index > 0 { self.content_type_index - 1 } else { self.content_type_index };
      },
      Action::Focus => {
        self.focused = true;
      },
      Action::UnFocus => {
        self.focused = false;
        self.mode = ViewerMode::Normal;
        self.scroll_offset = 0;
      },
      Action::ApplySearch(term) => {
        let formatted_body = self
          .operation_item
          .operation
          .operation_id
          .as_ref()
          .and_then(|id| state.responses.get(id))
          .map(|r| {
            let ct = r.headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default();
            self.formatter_registry.format(ct, &r.body)
          })
          .unwrap_or_default();
        self.scroll_offset = 0;
        if term.is_empty() {
          self.mode = ViewerMode::Normal;
        } else {
          let matches = Self::run_search(&term, &formatted_body);
          let count = matches.len();
          self.mode = ViewerMode::Search(matches, 0);
          if count == 0 {
            return Ok(Some(Action::TimedStatusLine(format!("no matches for '{term}'"), 3)));
          }
          return Ok(Some(Action::TimedStatusLine(format!("{count} match(es) for '{term}'"), 3)));
        }
      },
      Action::ApplyJqQuery(filter) => {
        self.scroll_offset = 0;
        if filter.is_empty() {
          self.mode = ViewerMode::Normal;
        } else {
          let body = self
            .operation_item
            .operation
            .operation_id
            .as_ref()
            .and_then(|id| state.responses.get(id))
            .map(|r| r.body.clone())
            .unwrap_or_default();
          let (result, is_error) = Self::run_jq(&filter, &body);
          self.mode = ViewerMode::Jq(result, is_error);
        }
      },
      Action::SaveResponsePayload(filepath) => {
        if let Some(response) =
          self.operation_item.operation.operation_id.as_ref().and_then(|operation_id| state.responses.get(operation_id))
        {
          if let Err(error) =
            std::fs::File::create(filepath).and_then(|mut file| file.write_all(response.body.as_bytes()))
          {
            return Ok(Some(Action::TimedStatusLine(format!("can't create or write file content: {error}"), 5)));
          }
        } else {
          return Ok(Some(Action::TimedStatusLine("response is not available".into(), 5)));
        }
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    let inner_panes = Layout::horizontal([Constraint::Fill(3), Constraint::Fill(1)]).split(inner);

    let is_pending = self
      .operation_item
      .operation
      .operation_id
      .as_ref()
      .map(|id| state.pending_operations.contains(id))
      .unwrap_or(false);
    let spinner_char = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];

    let mut status_line = String::default();

    if let Some(response) =
      self.operation_item.operation.operation_id.as_ref().and_then(|operation_id| state.responses.get(operation_id))
    {
      let loading_prefix = if is_pending { format!("{spinner_char} ") } else { String::new() };
      status_line = format!(
        "{loading_prefix}[{:?} {} {} {}]",
        response.version,
        response.status.as_str(),
        symbols::DOT,
        humansize::format_size(response.content_length.unwrap_or(response.body.len() as u64), humansize::DECIMAL)
      );

      let content_type =
        response.headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();

      let body_block =
        Block::default().borders(Borders::RIGHT).border_style(self.border_style()).border_type(self.border_type());

      match &self.mode {
        ViewerMode::Normal => {
          let formatted_body = self.formatter_registry.format(&content_type, &response.body);
          let syntax = self.formatter_registry.syntax_name(&content_type);

          if let Some(syntax_name) = syntax {
            let lines = self.highlighted_lines(&formatted_body, syntax_name);
            let text = Text::from(lines.to_vec());
            frame.render_widget(
              Paragraph::new(text).scroll((self.scroll_offset as u16, 0)).block(body_block),
              inner_panes[0],
            );
          } else {
            frame.render_widget(
              Paragraph::new(Self::plain_with_line_numbers(&formatted_body))
                .scroll((self.scroll_offset as u16, 0))
                .block(body_block),
              inner_panes[0],
            );
          }
        },
        ViewerMode::Search(matches, current) => {
          let current = *current;
          let formatted_body = self.formatter_registry.format(&content_type, &response.body);
          let match_lines: Vec<Line<'_>> = matches
            .iter()
            .enumerate()
            .map(|(i, &offset)| {
              let ctx = Self::context_line(&formatted_body, offset);
              let style = if i == current {
                Style::default().add_modifier(Modifier::BOLD).fg(Color::LightYellow)
              } else {
                Style::default()
              };
              Line::styled(format!(" {ctx}"), style)
            })
            .collect();

          let count_str = format!(" {}/{}", current + 1, matches.len());
          let mut list_state =
            ListState::default().with_selected(if matches.is_empty() { None } else { Some(current) });
          frame.render_stateful_widget(
            List::new(match_lines)
              .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
              .highlight_spacing(HighlightSpacing::Always)
              .block(body_block.title_bottom(Line::from(count_str).right_aligned())),
            inner_panes[0],
            &mut list_state,
          );
        },
        ViewerMode::Jq(result, is_error) => {
          let style = if *is_error { Style::default().fg(Color::LightRed) } else { Style::default() };
          frame.render_widget(
            Paragraph::new(Self::plain_with_line_numbers(result))
              .style(style)
              .scroll((self.scroll_offset as u16, 0))
              .block(body_block),
            inner_panes[0],
          );
        },
      }

      frame.render_widget(
        List::new(
          response
            .headers
            .iter()
            .map(|(hk, hv)| {
              Line::from(vec![
                Span::styled(format!("{}: ", hk), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(hv.to_str().unwrap_or("ERROR")),
              ])
            })
            .collect::<Vec<_>>(),
        ),
        inner_panes[1],
      );
    } else if is_pending {
      frame.render_widget(
        Paragraph::new(format!(" {spinner_char} Waiting for response…")).style(Style::default().fg(Color::LightCyan)),
        inner,
      );
    } else {
      frame.render_widget(
        Paragraph::new(" No response is available. Press enter or try [send] command.").style(Style::default().dim()),
        inner,
      );
    }

    let content_types = if !self.content_types.is_empty() {
      let ctype = self.content_types[self.content_type_index].clone();
      let ctype_progress = if self.content_types.len() > 1 {
        format!("[{}/{}]", self.content_type_index + 1, self.content_types.len())
      } else {
        String::default()
      };
      format!(": {ctype} {ctype_progress}")
    } else {
      String::default()
    };

    let mode_hint = match &self.mode {
      ViewerMode::Normal => String::new(),
      ViewerMode::Search(_, _) => " [j/k navigate · :search to clear]".to_string(),
      ViewerMode::Jq(_, _) => " [j/k scroll · :jq to clear]".to_string(),
    };

    frame.render_widget(
      Block::default()
        .title(format!("Response{content_types}"))
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type())
        .title_bottom(Line::from(format!("{status_line}{mode_hint}")).right_aligned()),
      area,
    );

    Ok(())
  }
}
