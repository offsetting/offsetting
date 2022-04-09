use clap::Parser;

use crate::cli::Offsetting;

mod cli;

fn main() -> anyhow::Result<()> {
  Offsetting::parse().execute()
}
