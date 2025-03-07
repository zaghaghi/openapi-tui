use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  config::Config,
  pages::Page,
  panes::{
    address::AddressPane, body_editor::BodyEditor, parameter_editor::ParameterEditor, response_viewer::ResponseViewer,
    Pane,
  },
  request::Request,
  state::{InputMode, OperationItem, State},
  tui::{Event, EventResponse},
};

#[derive(Default)]
pub struct Phone {
  operation_item: Arc<OperationItem>,
  command_tx: Option<UnboundedSender<Action>>,
  request_tx: Option<UnboundedSender<Request>>,
  config: Config,
  focused_pane_index: usize,
  panes: Vec<Box<dyn RequestPane>>,
  fullscreen_pane_index: Option<usize>,
}

pub trait RequestBuilder {
  fn path(&self, url: String) -> String {
    url
  }

  fn reqeust(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
  }
}

pub trait RequestPane: Pane + RequestBuilder {}

impl Phone {
  pub fn new(operation_item: OperationItem, request_tx: UnboundedSender<Request>, _state: &State) -> Result<Self> {
    let focused_border_style = Style::default().fg(Color::LightGreen);
    let operation_item = Arc::new(operation_item);

    Ok(Self {
      operation_item: operation_item.clone(),
      command_tx: None,
      request_tx: Some(request_tx),
      config: Config::default(),
      panes: vec![
        Box::new(AddressPane::new(false, focused_border_style)),
        Box::new(ParameterEditor::new(operation_item.clone(), true, focused_border_style)),
        Box::new(BodyEditor::new(operation_item.clone(), false, focused_border_style)),
        Box::new(ResponseViewer::new(operation_item.clone(), false, focused_border_style)),
      ],
      focused_pane_index: 1,
      fullscreen_pane_index: None,
    })
  }

  fn build_request(&self) -> Result<reqwest::Request> {
    let url = self.panes.iter().fold(self.operation_item.path.clone(), |url, pane| pane.path(url));
    let method = reqwest::Method::from_bytes(self.operation_item.method.as_bytes())?;
    let request_builder = self
      .panes
      .iter()
      .fold(reqwest::Client::new().request(method, url), |request_builder, pane| pane.reqeust(request_builder));

    Ok(request_builder.build()?)
  }

  fn handle_commands(&self, command_args: String) -> Option<Action> {
    if command_args.eq("q") {
      return Some(Action::Quit);
    }
    if command_args.eq("send") || command_args.eq("s") {
      return Some(Action::Dial);
    }
    if command_args.starts_with("query ") || command_args.starts_with("q ") {
      let command_parts = command_args.split(' ').filter(|item| !item.is_empty()).collect::<Vec<_>>();
      if command_parts.len() == 3 {
        if command_parts[1].eq("add") {
          return Some(Action::AddQuery(command_parts[2].into()));
        }
        if command_parts[1].eq("rm") {
          return Some(Action::RemoveQuery(command_parts[2].into()));
        }
      }
      return Some(Action::TimedStatusLine("invalid query args. query add/rm <query-name>".into(), 3));
    }
    if command_args.starts_with("header ") || command_args.starts_with("h ") {
      let command_parts = command_args.split(' ').filter(|item| !item.is_empty()).collect::<Vec<_>>();
      if command_parts.len() == 3 {
        if command_parts[1].eq("add") {
          return Some(Action::AddHeader(command_parts[2].into()));
        }
        if command_parts[1].eq("rm") {
          return Some(Action::RemoveHeader(command_parts[2].into()));
        }
      }
      return Some(Action::TimedStatusLine("invalid header args. header add/rm <query-name>".into(), 3));
    }
    if command_args.starts_with("request ") || command_args.starts_with("r ") {
      let command_parts = command_args.split(' ').filter(|item| !item.is_empty()).collect::<Vec<_>>();
      if command_parts.len() == 3 && command_parts[1].eq("open") {
        return Some(Action::OpenRequestPayload(command_parts[2].into()));
      }
      return Some(Action::TimedStatusLine("invalid request args. request open <payload-file-name>".into(), 3));
    }
    if command_args.starts_with("response ") || command_args.starts_with("s ") {
      let command_parts = command_args.split(' ').filter(|item| !item.is_empty()).collect::<Vec<_>>();
      if command_parts.len() == 3 && command_parts[1].eq("save") {
        return Some(Action::SaveResponsePayload(command_parts[2].into()));
      }
      return Some(Action::TimedStatusLine("invalid response args. response save <payload-file-name>".into(), 3));
    }
    Some(Action::TimedStatusLine(
      "unknown command. available commands are: send, query, header, request, response".into(),
      3,
    ))
  }
}

impl Page for Phone {
  fn init(&mut self, state: &State) -> Result<()> {
    for pane in self.panes.iter_mut() {
      pane.init(state)?;
    }
    Ok(())
  }

  fn focus(&mut self) -> Result<()> {
    if let Some(command_tx) = &self.command_tx {
      const ARROW: &str = symbols::scrollbar::HORIZONTAL.end;
      let status_line = format!(
        "[⏎ {ARROW} edit mode/execute request] [1-9 {ARROW} select items] [ESC {ARROW} close] [q {ARROW} quit]"
      );
      command_tx.send(Action::StatusLine(status_line))?;
    }
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
          KeyCode::Esc => EventResponse::Stop(Action::HangUp(self.operation_item.operation.operation_id.clone())),
          KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => EventResponse::Stop(Action::FocusNext),
          KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => EventResponse::Stop(Action::FocusPrev),
          KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => EventResponse::Stop(Action::Down),
          KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => EventResponse::Stop(Action::Up),
          KeyCode::Char('f') | KeyCode::Char('F') => EventResponse::Stop(Action::ToggleFullScreen),
          KeyCode::Char(c) if ('1'..='9').contains(&c) => {
            EventResponse::Stop(Action::Tab(c.to_digit(10).unwrap_or(0) - 1))
          },
          KeyCode::Char(']') => EventResponse::Stop(Action::TabNext),
          KeyCode::Char('[') => EventResponse::Stop(Action::TabPrev),
          KeyCode::Enter => EventResponse::Stop(Action::Submit),
          KeyCode::Char(':') => EventResponse::Stop(Action::FocusFooter(":".into(), None)),
          _ => {
            return Ok(None);
          },
        };
        Ok(Some(response))
      },
      InputMode::Insert => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          let response = pane.handle_events(Event::Key(key), state)?;
          return Ok(response);
        }
        Ok(None)
      },
      InputMode::Command => Ok(None),
    }
  }

  fn update(&mut self, action: Action, state: &mut State) -> Result<Option<Action>> {
    let mut actions: Vec<Option<Action>> = vec![];

    match action {
      Action::FocusNext => {
        let next_index = self.focused_pane_index.saturating_add(1) % self.panes.len();
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(Action::UnFocus, state)?);
        }
        self.focused_pane_index = next_index;
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(Action::Focus, state)?);
        }
      },
      Action::FocusPrev => {
        let prev_index = self.focused_pane_index.saturating_add(self.panes.len() - 1) % self.panes.len();
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(Action::UnFocus, state)?);
        }
        self.focused_pane_index = prev_index;
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(Action::Focus, state)?);
        }
      },
      Action::ToggleFullScreen => {
        self.fullscreen_pane_index = self.fullscreen_pane_index.map_or(Some(self.focused_pane_index), |_| None);
      },
      Action::Update => {
        for pane in self.panes.iter_mut() {
          actions.push(pane.update(action.clone(), state)?);
        }
      },
      Action::Dial => {
        if let Some(request_tx) = &self.request_tx {
          request_tx.send(Request {
            request: self.build_request()?,
            operation_id: self.operation_item.operation.operation_id.clone().unwrap_or_default(),
          })?;
        }
      },
      Action::FocusFooter(..) => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(Action::UnFocus, state)?);
        }
      },
      Action::FooterResult(cmd, Some(args)) if cmd.eq(":") => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          pane.update(Action::Focus, state)?;
        }
        if let Some(action) = self.handle_commands(args) {
          for pane in self.panes.iter_mut() {
            actions.push(pane.update(action.clone(), state)?);
          }
          if let Action::TimedStatusLine(_, _) = action {
            actions.push(Some(action))
          }
        }
      },
      Action::FooterResult(_cmd, None) => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(Action::Focus, state)?);
        }
      },
      _ => {
        if let Some(pane) = self.panes.get_mut(self.focused_pane_index) {
          actions.push(pane.update(action, state)?);
        }
      },
    }
    if let Some(tx) = &mut self.command_tx {
      actions.into_iter().flatten().for_each(|action| {
        tx.send(action).ok();
      });
    }
    Ok(None)
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, state: &State) -> Result<()> {
    let outer_layout =
      Layout::vertical(vec![Constraint::Max(3), self.panes[1].height_constraint(), self.panes[2].height_constraint()])
        .split(area);
    if let Some(fullscreen_pane_index) = self.fullscreen_pane_index {
      self.panes[fullscreen_pane_index].draw(frame, area, state)?;
    } else {
      let input_layout = Layout::horizontal(vec![Constraint::Fill(1), Constraint::Fill(1)]).split(outer_layout[1]);
      self.panes[0].draw(frame, outer_layout[0], state)?;
      self.panes[1].draw(frame, input_layout[0], state)?;
      self.panes[2].draw(frame, input_layout[1], state)?;
      self.panes[3].draw(frame, outer_layout[2], state)?;
    }
    Ok(())
  }
}
