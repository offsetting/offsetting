use clap::{Parser, Subcommand};
use offsetting_x360::X360Module;

use offsetting_hash::HashModule;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub(crate) struct Offsetting {
  #[clap(subcommand)]
  module: Module,
}

#[derive(Subcommand, Debug)]
enum Module {
  Hash(HashModule),
  X360(X360Module),
}

impl Offsetting {
  pub(crate) fn execute(&self) -> anyhow::Result<()> {
    match &self.module {
      Module::Hash(module) => module.execute(),
      Module::X360(module) => module.execute(),
    }

    Ok(())
  }
}
