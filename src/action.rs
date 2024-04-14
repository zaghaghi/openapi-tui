use serde::{Deserialize, Serialize};
use strum::Display;

type Command = String;
type Args = Option<String>;

#[derive(Debug, Clone, PartialEq, Serialize, Display, Deserialize)]
pub enum Action {
  Tick,
  Render,
  Resize(u16, u16),
  Suspend,
  Resume,
  Quit,
  Refresh,
  Error(String),
  Help,
  FocusNext,
  FocusPrev,
  Focus,
  UnFocus,
  Up,
  Down,
  Submit,
  Update,
  Tab(u32),
  TabNext,
  TabPrev,
  Go,
  Back,
  ToggleFullScreen,
  StatusLine(String),
  TimedStatusLine(String, u64),
  FocusFooter(Command, Args),
  FooterResult(Command, Args),
  Noop,
  NewCall(Option<String>),
  HangUp(Option<String>),
  Dial,
  History,
  CloseHistory,
}
