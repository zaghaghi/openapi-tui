use std::sync::Arc;

use color_eyre::eyre::Result;
use openapi_31::v31::parameter::In;
use ratatui::{
  prelude::*,
  widgets::{block::*, *},
};

use crate::{
  action::Action,
  panes::Pane,
  state::{OperationItem, State},
  tui::Frame,
};

pub struct ParameterEditor {
  focused: bool,
  focused_border_style: Style,
  operation_item: Arc<OperationItem>,
  parameters: Vec<ParameterTab>,
  selected_parameter: usize,
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
    Self { operation_item, focused, focused_border_style, parameters: vec![], selected_parameter: 0 }
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
      if !query_items.is_empty() {
        self.parameters.push(ParameterTab {
          location: "Query".to_string(),
          items: query_items,
          table_state: TableState::default().with_selected(0),
        });
      }
      if !header_items.is_empty() {
        self.parameters.push(ParameterTab {
          location: "Header".to_string(),
          items: header_items,
          table_state: TableState::default().with_selected(0),
        });
      }
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
}

impl Pane for ParameterEditor {
  fn init(&mut self, state: &State) -> Result<()> {
    self.init_parameters(state)?;
    Ok(())
  }

  fn focus(&mut self) -> Result<()> {
    self.focused = true;
    Ok(())
  }

  fn unfocus(&mut self) -> Result<()> {
    self.focused = false;
    Ok(())
  }

  fn height_constraint(&self) -> Constraint {
    Constraint::Fill(1)
  }

  fn update(&mut self, action: Action, _state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {},
      Action::Down => {
        if let Some(parameters) = self.parameters.get_mut(self.selected_parameter).as_mut() {
          let i = match parameters.table_state.selected() {
            Some(i) => {
              if i >= parameters.items.len() - 1 {
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
                parameters.items.len() - 1
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

      Action::Submit => {},
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, _state: &State) -> Result<()> {
    let margin_h1_v1: Margin = Margin { horizontal: 1, vertical: 1 };
    let inner = area.inner(&margin_h1_v1);

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

    if let Some(parameters) = self.parameters.get_mut(self.selected_parameter).as_mut() {
      let rows = parameters.items.iter().map(|item| {
        let required = match item.required {
          true => " * ",
          false => "   ",
        };
        let value = match &item.value {
          Some(value) => Span::from(value),
          None => Span::styled(String::from("No Value"), Style::default().dim()),
        };
        Row::new(vec![
          Cell::from(Line::from(vec![Span::from(required).style(Color::Red), Span::from(item.name.clone())])),
          Cell::from(Line::from(vec![Span::from(symbols::line::VERTICAL), value])),
        ])
      });
      let table = Table::new(rows, [Constraint::Fill(1), Constraint::Fill(2)])
        .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

      frame.render_stateful_widget(table, inner, &mut parameters.table_state);
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
