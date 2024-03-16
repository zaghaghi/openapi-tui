use clap::Parser;

use crate::utils::version;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
  #[arg(short, long, value_name = "PATH", help = "Input file, in json or yaml format with openapi specification")]
  pub input: String,
}
