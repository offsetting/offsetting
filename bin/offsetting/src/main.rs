use crate::dct::DctModule;
use clap::{Parser, Subcommand};

use crate::oct::OctModule;
use crate::x360::X360Module;

mod dct;
mod oct;
mod x360;

#[derive(Parser)]
#[clap(version)]
pub struct Offsetting {
  #[clap(subcommand)]
  module: Module,
}

#[derive(Subcommand)]
enum Module {
  X360(X360Module),
  Oct(OctModule),
  Dct(DctModule),
}

impl Offsetting {
  pub fn execute(self) -> anyhow::Result<()> {
    match self.module {
      Module::X360(module) => module.execute(),
      Module::Oct(module) => module.execute(),
      Module::Dct(module) => module.execute(),
    }
  }
}

fn main() -> anyhow::Result<()> {
  Offsetting::parse().execute()
}
