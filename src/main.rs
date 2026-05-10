pub mod action;
pub mod app;
pub mod auth;
pub mod cli;
pub mod components;
pub mod config;
pub mod exporters;
pub mod formatters;
pub mod pages;
pub mod panes;
pub mod request;
pub mod response;
pub mod state;
pub mod tui;
pub mod utils;

use clap::Parser;
use cli::Cli;
use color_eyre::eyre::{eyre, Result};

use crate::{
  app::App,
  utils::{initialize_logging, initialize_panic_handler},
};

fn parse_header(raw: &str) -> Result<(String, String)> {
  let (name, value) = raw.split_once(':').ok_or_else(|| eyre!("invalid header `{raw}`: expected `Name: Value`"))?;
  let name = name.trim();
  if name.is_empty() {
    return Err(eyre!("invalid header `{raw}`: empty name"));
  }
  Ok((name.to_string(), value.trim().to_string()))
}

async fn tokio_main() -> Result<()> {
  initialize_logging()?;

  initialize_panic_handler()?;

  let args = Cli::parse();
  let global_headers = args.headers.iter().map(|h| parse_header(h)).collect::<Result<Vec<_>>>()?;
  let mut app = App::new(args.input, global_headers).await?;
  app.run().await?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  if let Err(e) = tokio_main().await {
    eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
    Err(e)
  } else {
    Ok(())
  }
}
