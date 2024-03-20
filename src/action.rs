use serde::{Deserialize, Serialize};
use strum::Display;

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
  FocusFooter,
  Filter(String),
  Noop,
}
