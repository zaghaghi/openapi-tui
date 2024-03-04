use clap::Parser;

use crate::utils::version;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
  #[arg(
    short,
    long,
    value_name = "PATH",
    help = "Input file, i.e. json or yaml file with openapi specification",
    default_value_t= String::from("openapi.json")
  )]
  pub openapi_path: String,
}
