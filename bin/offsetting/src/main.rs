use clap::{Parser, Subcommand};

use crate::matryoshka::MatryoshkaModule;
use crate::x360::X360Module;

mod x360;
mod matryoshka;

#[derive(Parser)]
#[clap(version)]
pub struct Offsetting {
  #[clap(subcommand)]
  module: Module,
}

#[derive(Subcommand)]
enum Module {
  X360(X360Module),
  Matryoshka(MatryoshkaModule),
}

impl Offsetting {
  pub fn execute(self) -> anyhow::Result<()> {
    match self.module {
      Module::X360(module) => module.execute(),
      Module::Matryoshka(module) => module.execute(),
    }
  }
}

fn main() -> anyhow::Result<()> {
  Offsetting::parse().execute()
}
