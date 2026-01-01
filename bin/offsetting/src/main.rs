use crate::dct::DctModule;
use clap::{Parser, Subcommand};

use crate::oct::OctModule;
use crate::whynow::{DI3ZipModule, C2ZipModule, C3ZipModule};
use crate::rre_package::RREPackageModule;

mod dct;
mod oct;
mod whynow;
mod rre_package;

#[derive(Parser)]
#[clap(version)]
pub struct Offsetting {
  #[clap(subcommand)]
  module: Module,
}

#[derive(Subcommand)]
enum Module {
  #[command(name = "rre-package")]
  RREPackage(RREPackageModule),
  Oct(OctModule),
  Dct(DctModule),
  #[command(name = "zip-c2", visible_alias = "zip")]
  C2Zip(C2ZipModule),
  #[command(name = "zip-di3")]
  DI3Zip(DI3ZipModule),
  #[command(name = "zip-c3")]
  C3Zip(C3ZipModule),
}

impl Offsetting {
  pub fn execute(self) -> anyhow::Result<()> {
    match self.module {
      Module::RREPackage(module) => module.execute(),
      Module::Oct(module) => module.execute(),
      Module::Dct(module) => module.execute(),
      Module::C2Zip(module) => module.execute(),
      Module::DI3Zip(module) => module.execute(),
      Module::C3Zip(module) => module.execute(),
    }
  }
}

fn main() -> anyhow::Result<()> {
  Offsetting::parse().execute()
}
