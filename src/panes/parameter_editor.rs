use std::{str::FromStr, sync::Arc};

use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use openapi_31::v31::parameter::In;
use ratatui::{prelude::*, widgets::*};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
  action::Action,
  pages::phone::{RequestBuilder, RequestPane},
  panes::Pane,
  state::{InputMode, OperationItem, State},
  tui::{EventResponse, Frame},
};

pub struct ParameterEditor {
  focused: bool,
  focused_border_style: Style,
  operation_item: Arc<OperationItem>,
  parameters: Vec<ParameterTab>,
  selected_parameter: usize,
  input: Input,
}

#[derive(Default)]
pub struct ParameterItem {
  pub name: String,
  pub value: Option<String>,
  pub required: bool,
  pub schema: Option<serde_json::Value>,
}

#[derive(Default)]
pub struct ParameterTab {
  pub location: String,
  pub items: Vec<ParameterItem>,
  pub table_state: TableState,
}

impl ParameterEditor {
  pub fn new(operation_item: Arc<OperationItem>, focused: bool, focused_border_style: Style) -> Self {
    Self {
      operation_item,
      focused,
      focused_border_style,
      parameters: vec![],
      selected_parameter: 0,
      input: Input::default(),
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

  fn location_color(&self, status: &str) -> Color {
    if status.eq_ignore_ascii_case("header") {
      return Color::LightCyan;
    }
    if status.eq_ignore_ascii_case("path") {
      return Color::LightBlue;
    }
    if status.eq_ignore_ascii_case("query") {
      return Color::LightMagenta;
    }
    if status.eq_ignore_ascii_case("cookie") {
      return Color::LightRed;
    }
    Color::default()
  }

  fn init_parameters(&mut self, state: &State) -> Result<()> {
    {
      let mut path_items = vec![];
      let mut query_items = vec![];
      let mut header_items = vec![];
      let mut cookie_items = vec![];

      self.operation_item.operation.parameters.iter().flatten().for_each(|parameter_or_ref| {
        let parameter = parameter_or_ref.resolve(&state.openapi_spec).unwrap();
        let value =
          parameter.schema.clone().and_then(|schema| schema.get("default").map(|default| default.to_string()));
        match parameter.r#in {
          In::Query => &mut query_items,
          In::Header => &mut header_items,
          In::Path => &mut path_items,
          In::Cookie => &mut cookie_items,
        }
        .push(ParameterItem {
          name: parameter.name.clone(),
          value,
          required: parameter.required.unwrap_or(false),
          schema: parameter.schema.clone(),
        });
      });
      if !path_items.is_empty() {
        self.parameters.push(ParameterTab {
          location: "Path".to_string(),
          items: path_items,
          table_state: TableState::default().with_selected(0),
        });
      }
      self.parameters.push(ParameterTab {
        location: "Query".to_string(),
        items: query_items,
        table_state: TableState::default().with_selected(0),
      });
      self.parameters.push(ParameterTab {
        location: "Header".to_string(),
        items: header_items,
        table_state: TableState::default().with_selected(0),
      });
      if !cookie_items.is_empty() {
        self.parameters.push(ParameterTab {
          location: "Cookie".to_string(),
          items: cookie_items,
          table_state: TableState::default().with_selected(0),
        });
      }
    }

    Ok(())
  }

  fn select_parameters<'a>(&'a self, parameter_type: &'a str) -> impl Iterator<Item = &ParameterItem> + 'a {
    self
      .parameters
      .iter()
      .filter_map(
        |parameter| {
          if parameter.location.eq_ignore_ascii_case(parameter_type) {
            Some(&parameter.items)
          } else {
            None
          }
        },
      )
      .flatten()
  }

  fn path_parameters(&self) -> impl Iterator<Item = &ParameterItem> {
    self.select_parameters("path")
  }

  fn query_parameters(&self) -> impl Iterator<Item = &ParameterItem> {
    self.select_parameters("query")
  }

  fn header_parameters(&self) -> impl Iterator<Item = &ParameterItem> {
    self.select_parameters("header")
  }

  #[allow(dead_code)]
  fn cookie_parameters(&self) -> impl Iterator<Item = &ParameterItem> {
    self.select_parameters("cookie")
  }
}

impl RequestPane for ParameterEditor {}

impl RequestBuilder for ParameterEditor {
  fn path(&self, url: String) -> String {
    self.path_parameters().fold(url, |url, path_param| {
      if let Some(value) = &path_param.value {
        url.replace(format!("{{{}}}", path_param.name).as_str(), value.as_str())
      } else {
        url
      }
    })
  }

  fn reqeust(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let query_params = self
      .query_parameters()
      .filter_map(|query_param| {
        let name = query_param.name.clone();
        let value = query_param.value.clone();
        if !query_param.required && value.is_none() {
          return None;
        }
        Some((name, value.unwrap()))
      })
      .collect::<Vec<_>>();

    let header_params = self
      .header_parameters()
      .filter_map(|header_param| {
        let name = header_param.name.as_str();
        let value = header_param.value.as_deref().unwrap_or_default();
        HeaderName::from_str(name)
          .ok()
          .and_then(|header_name| HeaderValue::from_str(value).ok().map(|header_value| (header_name, header_value)))
      })
      .collect::<HeaderMap<_>>();
    request.query(&query_params).headers(header_params)
  }
}

impl Pane for ParameterEditor {
  fn init(&mut self, state: &State) -> Result<()> {
    self.init_parameters(state)?;
    Ok(())
  }

  fn height_constraint(&self) -> Constraint {
    Constraint::Fill(1)
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Insert => match key.code {
        KeyCode::Enter => Ok(Some(EventResponse::Stop(Action::Submit))),
        _ => {
          self.input.handle_event(&Event::Key(key));
          Ok(Some(EventResponse::Stop(Action::Noop)))
        },
      },
      _ => Ok(None),
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {},
      Action::Down => {
        if let Some(parameters) = self.parameters.get_mut(self.selected_parameter).as_mut() {
          let i = match parameters.table_state.selected() {
            Some(i) => {
              if i >= parameters.items.len().saturating_sub(1) {
                0
              } else {
                i + 1
              }
            },
            None => 0,
          };
          parameters.table_state.select(Some(i));
        }
      },
      Action::Up => {
        if let Some(parameters) = self.parameters.get_mut(self.selected_parameter).as_mut() {
          let i = match parameters.table_state.selected() {
            Some(i) => {
              if i == 0 {
                parameters.items.len().saturating_sub(1)
              } else {
                i - 1
              }
            },
            None => 0,
          };
          parameters.table_state.select(Some(i));
        }
      },
      Action::Tab(index) if index < self.parameters.len().try_into()? => {
        self.selected_parameter = index as usize;
      },
      Action::TabNext => {
        let next_tab_index = self.selected_parameter + 1;
        self.selected_parameter = if next_tab_index < self.parameters.len() { next_tab_index } else { 0 };
      },
      Action::TabPrev => {
        self.selected_parameter =
          if self.selected_parameter > 0 { self.selected_parameter - 1 } else { self.parameters.len() - 1 };
      },
      Action::Focus => {
        self.focused = true;
      },
      Action::UnFocus => {
        self.focused = false;
      },
      Action::Submit if state.input_mode == InputMode::Normal && !self.parameters.is_empty() => {
        state.input_mode = InputMode::Insert;
        if let Some(parameter) = self
          .parameters
          .get(self.selected_parameter)
          .and_then(|parameters| parameters.table_state.selected().and_then(|i| parameters.items.get(i)))
        {
          self.input = self.input.clone().with_value(parameter.value.clone().unwrap_or_default());
        }
      },
      Action::Submit if state.input_mode == InputMode::Insert && !self.parameters.is_empty() => {
        state.input_mode = InputMode::Normal;

        if let Some(parameter) = self
          .parameters
          .get_mut(self.selected_parameter)
          .and_then(|parameters| parameters.table_state.selected().and_then(|i| parameters.items.get_mut(i)))
        {
          if !self.input.value().is_empty() {
            parameter.value = Some(self.input.value().to_string());
          } else {
            parameter.value = None;
          }
        }
        self.input.reset();
      },
      Action::AddHeader(header_name) => {
        if let Some(param_tab) = self.parameters.iter_mut().find(|item| item.location.to_lowercase().eq("header")) {
          param_tab.items.push(ParameterItem { name: header_name, ..Default::default() });
        }
      },
      Action::RemoveHeader(header_name) => {
        if let Some(param_tab) = self.parameters.iter_mut().find(|item| item.location.to_lowercase().eq("header")) {
          if let Some(last_header_index) = param_tab
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| if item.name.eq(&header_name) { Some(index) } else { None })
            .last()
          {
            param_tab.items.remove(last_header_index);
          }
        }
      },
      Action::AddQuery(query_name) => {
        if let Some(param_tab) = self.parameters.iter_mut().find(|item| item.location.to_lowercase().eq("query")) {
          param_tab.items.push(ParameterItem { name: query_name, ..Default::default() });
        }
      },
      Action::RemoveQuery(query_name) => {
        if let Some(param_tab) = self.parameters.iter_mut().find(|item| item.location.to_lowercase().eq("query")) {
          if let Some(last_query_index) = param_tab
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| if item.name.eq(&query_name) { Some(index) } else { None })
            .last()
          {
            param_tab.items.remove(last_query_index);
          }
        }
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });

    frame.render_widget(
      Tabs::new(self.parameters.iter().map(|item| {
        Span::styled(item.location.clone(), Style::default().fg(self.location_color(item.location.as_str()))).dim()
      }))
      .divider(symbols::DOT)
      .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).not_dim())
      .select(self.selected_parameter),
      inner,
    );

    let inner = Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: inner.height - 1 };

    if let Some(parameters) = self.parameters.get_mut(self.selected_parameter) {
      let selected = parameters.table_state.selected().unwrap_or(0);
      let rows = parameters.items.iter().enumerate().map(|(index, item)| {
        let required = match item.required {
          true => " * ",
          false => "   ",
        };
        let value = match &item.value {
          Some(value) => Span::from(value),
          None => Span::styled(String::from("No Value"), Style::default().dim()),
        };

        let value = match state.input_mode {
          InputMode::Insert if selected == index && self.focused => Span::default(),
          _ => value,
        };
        Row::new(vec![
          Cell::from(Line::from(vec![Span::from(required).style(Color::Red), Span::from(item.name.clone())])),
          Cell::from(Line::from(vec![Span::from(symbols::line::VERTICAL), value])),
        ])
      });
      let row_widths = [Constraint::Fill(1), Constraint::Fill(2)];
      let column_widths = Layout::horizontal(row_widths).split(inner);
      if !parameters.items.is_empty() {
        let table = Table::new(rows, vec![column_widths[0].width, column_widths[1].width])
          .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
          .highlight_spacing(HighlightSpacing::Always)
          .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(table, inner, &mut parameters.table_state);
      } else {
        let location = parameters.location.to_lowercase();
        let empty_msg = if location.eq("query") || location.eq("header") {
          format!(" No {location} item available. try [{location} add {location}-name] command to add one.")
        } else {
          format!(" No {location} item available.")
        };
        frame.render_widget(Paragraph::new(empty_msg).style(Style::default().dim()), inner);
      }

      if self.focused && InputMode::Insert == state.input_mode {
        let input_area = Rect {
          x: inner.x + column_widths[0].width + 3,
          y: inner.y + selected.saturating_sub(parameters.table_state.offset()) as u16,
          width: column_widths[1].width - 3,
          height: 1,
        };

        let scroll = self.input.visual_scroll(input_area.width as usize);
        let input =
          Paragraph::new(Line::from(vec![
            Span::styled(self.input.value(), Style::default().fg(Color::LightBlue)).not_dim()
          ]))
          .scroll((0, scroll as u16));
        frame.set_cursor_position(Position::new(
          input_area.x + self.input.visual_cursor().saturating_sub(scroll) as u16,
          input_area.y,
        ));
        frame.render_widget(input, input_area);
      }
    }

    frame.render_widget(
      Block::default()
        .title("Parameters")
        .borders(Borders::ALL)
        .border_style(self.border_style())
        .border_type(self.border_type()),
      area,
    );

    Ok(())
  }
}
