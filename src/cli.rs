use clap::Parser;

use crate::utils::version;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
  #[arg(
    short,
    long,
    value_name = "PATH",
    help = "Input file path, URL, or `-` to read from stdin (json or yaml openapi spec)"
  )]
  pub input: String,

  #[arg(
    short = 'H',
    long = "header",
    value_name = "NAME: VALUE",
    help = "Global header to attach to every request, in `Name: Value` form. May be repeated."
  )]
  pub headers: Vec<String>,
}
