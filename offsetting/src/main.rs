use color_eyre::eyre::Result;

use crate::cli::execute;

mod cli;

fn main() -> Result<()> {
  color_eyre::install()?;

  execute();

  Ok(())
}
