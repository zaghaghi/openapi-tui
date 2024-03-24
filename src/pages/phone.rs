use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use openapi_31::v31::parameter::In;
use ratatui::{
  prelude::*,
  widgets::{Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Table, TableState},
};
use strum::Display;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  pages::Page,
  panes::Pane,
  state::{InputMode, OperationItem, State},
  tui::EventResponse,
};

#[derive(Display)]
pub enum RequestItemLocation {
  Query,
  Header,
  Path,
  Cookie,
  Body,
}

pub struct RequestItem {
  pub name: String,
  pub location: RequestItemLocation,
  pub description: String,
  pub value: Option<String>,
  pub required: bool,
  pub schema: Option<serde_json::Value>,
}

#[derive(Default)]
pub struct Phone {
  operation_item: OperationItem,
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  panes: Vec<Box<dyn Pane>>,
  inputs: Vec<RequestItem>,
  table_state: TableState,
  popup: bool,
}

impl Phone {
  pub fn new(operation_item: OperationItem) -> Result<Self> {
    Ok(Self {
      operation_item,
      command_tx: None,
      config: Config::default(),
      panes: vec![],
      inputs: vec![],
      table_state: TableState::default().with_selected(0),
      popup: false,
    })
  }

  fn method_color(method: &str) -> Color {
    match method {
      "GET" => Color::LightCyan,
      "POST" => Color::LightBlue,
      "PUT" => Color::LightYellow,
      "DELETE" => Color::LightRed,
      _ => Color::Gray,
    }
  }

  fn location_color(&self, location: &RequestItemLocation) -> Color {
    match location {
      RequestItemLocation::Header => Color::LightCyan,
      RequestItemLocation::Path => Color::LightBlue,
      RequestItemLocation::Query => Color::LightMagenta,
      RequestItemLocation::Cookie => Color::LightRed,
      RequestItemLocation::Body => Color::LightYellow,
    }
  }

  fn base_url(&self, state: &State) -> String {
    if let Some(server) = state.openapi_spec.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
      String::from(server.url.trim_end_matches('/'))
    } else if let Some(server) = &self.operation_item.operation.servers.as_ref().map(|v| v.first()).unwrap_or(None) {
      String::from(server.url.trim_end_matches('/'))
    } else {
      String::from("http://localhost")
    }
  }

  fn init_inputs(&mut self, state: &State) -> Result<()> {
    {
      self.inputs = vec![];

      self.operation_item.operation.parameters.iter().flatten().for_each(|parameter_or_ref| {
        let parameter = parameter_or_ref.resolve(&state.openapi_spec).unwrap();
        let value =
          parameter.schema.clone().and_then(|schema| schema.get("default").map(|default| default.to_string()));
        let location = match parameter.r#in {
          In::Query => RequestItemLocation::Query,
          In::Header => RequestItemLocation::Header,
          In::Path => RequestItemLocation::Path,
          In::Cookie => RequestItemLocation::Cookie,
        };
        self.inputs.push(RequestItem {
          name: parameter.name.clone(),
          description: parameter.description.unwrap_or_default(),
          value,
          required: parameter.required.unwrap_or(false),
          location,
          schema: parameter.schema.clone(),
        });
      });

      if let Some(request_body) = &self.operation_item.operation.request_body {
        request_body.resolve(&state.openapi_spec).unwrap().content.iter().for_each(|(media_type, media)| {
          self.inputs.push(RequestItem {
            name: media_type.clone(),
            location: RequestItemLocation::Body,
            description: String::default(),
            value: None,
            required: true,
            schema: media.schema.clone(),
          });
        });
      }
    }

    Ok(())
  }
}

impl Page for Phone {
  fn init(&mut self, state: &State) -> Result<()> {
    for pane in self.panes.iter_mut() {
      pane.init(state)?;
    }
    self.init_inputs(state)?;
    Ok(())
  }

  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.command_tx = Some(tx);
    Ok(())
  }

  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config;
    Ok(())
  }

  fn handle_key_events(&mut self, key: KeyEvent, state: &mut State) -> Result<Option<EventResponse<Action>>> {
    match state.input_mode {
      InputMode::Normal => {
        let response = match key.code {
          KeyCode::Esc => EventResponse::Stop(Action::HangUp),
          KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::FocusNext),
          KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::FocusPrev),
          KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
          KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
          KeyCode::Enter => EventResponse::Stop(Action::Submit),
          _ => {
            return Ok(None);
          },
        };
        Ok(Some(response))
      },
      InputMode::Insert => {
        let response = match key.code {
          KeyCode::Enter => EventResponse::Stop(Action::Submit),
          _ => return Ok(None),
        };
        Ok(Some(response))
      },
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    match action {
      Action::Update => {},
      Action::Down => {
        let i = match self.table_state.selected() {
          Some(i) => {
            if i >= self.inputs.len() - 1 {
              0
            } else {
              i + 1
            }
          },
          None => 0,
        };
        self.table_state.select(Some(i));
      },
      Action::Up => {
        let i = match self.table_state.selected() {
          Some(i) => {
            if i == 0 {
              self.inputs.len() - 1
            } else {
              i - 1
            }
          },
          None => 0,
        };
        self.table_state.select(Some(i));
      },
      Action::Submit => {
        self.popup = !self.popup;
        state.input_mode = match state.input_mode {
          InputMode::Insert => InputMode::Normal,
          InputMode::Normal => InputMode::Insert,
        };
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let outer_layout = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Max(3), Constraint::Fill(3)])
      .split(area);

    frame.render_widget(
      Paragraph::new(Line::from(vec![
        Span::styled(
          format!(" {} ", self.operation_item.method.as_str()),
          Style::default().fg(Self::method_color(self.operation_item.method.as_str())),
        ),
        Span::styled(self.base_url(state), Style::default().fg(Color::DarkGray)),
        Span::styled(&self.operation_item.path, Style::default().fg(Color::White)),
      ]))
      .block(
        Block::new().title(self.operation_item.operation.summary.clone().unwrap_or_default()).borders(Borders::ALL),
      ),
      outer_layout[0],
    );

    let header = [
      String::from(" Location"),
      format!("{} Name", symbols::line::VERTICAL),
      format!("{} Value", symbols::line::VERTICAL),
      format!("{} Description", symbols::line::VERTICAL),
    ]
    .into_iter()
    .map(Cell::from)
    .collect::<Row>()
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows = self.inputs.iter().map(|input| {
      let required = match input.required {
        true => " *",
        false => "  ",
      };
      Row::new(vec![
        Cell::from(Text::from(format!(" {}", input.location)).style(self.location_color(&input.location))),
        Cell::from(Line::from(vec![Span::from(required).style(Color::Red), Span::from(input.name.clone())])),
        Cell::from(Text::from(input.value.clone().unwrap_or_default())),
        Cell::from(Text::from(input.description.clone())),
      ])
    });

    let table =
      Table::new(rows, [Constraint::Max(9 + 1), Constraint::Fill(1), Constraint::Fill(1), Constraint::Fill(3)])
        .header(header)
        .block(
          Block::default()
            .borders(Borders::ALL)
            .title("Parameters")
            .border_style(Style::default().fg(Color::LightGreen)),
        )
        .highlight_symbol(symbols::scrollbar::HORIZONTAL.end)
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(table, outer_layout[1], &mut self.table_state);

    if self.popup {
      let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Fill(1), Constraint::Max(3)])
        .split(outer_layout[1]);

      let margin = Margin { horizontal: 4, vertical: 0 };
      let popup_area = popup_layout[1].inner(&margin);
      frame.render_widget(
        Paragraph::new(self.inputs[self.table_state.selected().unwrap()].value.clone().unwrap_or_default()).block(
          Block::default()
            .borders(Borders::ALL)
            .title(self.inputs[self.table_state.selected().unwrap()].name.clone())
            .border_style(Style::default().fg(Color::LightGreen)),
        ),
        popup_area,
      );
    }
    Ok(())
  }
}
