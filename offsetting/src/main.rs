use clap::Parser;

use crate::lib::Offsetting;

mod lib;

fn main() -> anyhow::Result<()> {
  Offsetting::parse().execute()
}
