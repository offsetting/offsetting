use clap::{Parser, Subcommand};

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
}

impl Offsetting {
  pub(crate) fn execute(&self) -> anyhow::Result<()> {
    match &self.module {
      Module::Hash(module) => module.execute(),
    }

    Ok(())
  }
}
