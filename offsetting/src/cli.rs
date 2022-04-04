use clap::{Parser, Subcommand};

use offsetting_hash::HashModule;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
  #[clap(subcommand)]
  module: Module,
}

#[derive(Subcommand, Debug)]
enum Module {
  Hash(HashModule),
}

fn execute_module(args: Args) {
  match args.module {
    Module::Hash(module) => module.execute(),
  }
}

pub fn execute() {
  let args = Args::parse();

  execute_module(args);
}
