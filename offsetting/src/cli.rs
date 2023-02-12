use clap::{Parser, Subcommand};

use offsetting_hash::HashModule;
use offsetting_x360::X360Module;

#[derive(Parser, Debug)]
#[clap(version)]
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
      Module::Hash(module) => {
        module.execute();
        Ok(())
      }
      Module::X360(module) => module.execute(),
    }
  }
}
