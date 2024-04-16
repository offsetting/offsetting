use clap::{Parser, Subcommand};

use crate::x360::X360Module;

mod x360;

#[derive(Parser, Debug)]
#[clap(version)]
pub struct Offsetting {
  #[clap(subcommand)]
  module: Module,
}

#[derive(Subcommand, Debug)]
enum Module {
  X360(X360Module),
}

impl Offsetting {
  pub fn execute(&self) -> anyhow::Result<()> {
    match &self.module {
      Module::X360(module) => module.execute(),
    }
  }
}

fn main() -> anyhow::Result<()> {
  Offsetting::parse().execute()
}
