use clap::{Parser, Subcommand};
use indctive::dct_map::DctMap;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct IndctiveModule {
  #[clap(subcommand)]
  action: Action,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Action {
  Unpack(UnpackAction),
}

#[derive(Parser, Debug)]
pub(crate) struct UnpackAction {
  lang_dct: PathBuf,
  stringids_dct: PathBuf,
  json: PathBuf,
}

impl IndctiveModule {
  pub fn execute(&self) -> anyhow::Result<()> {
    match &self.action {
      Action::Unpack(action) => action.execute(),
    }
  }
}

fn read_dct_file<P: AsRef<Path>>(path: P) -> anyhow::Result<DctMap> {
  let mut dct_file = File::open(path)?;
  let dct_map = DctMap::from_reader(&mut dct_file)?;

  Ok(dct_map)
}

impl UnpackAction {
  pub fn execute(&self) -> anyhow::Result<()> {
    let lang_map = read_dct_file(&self.lang_dct)?;
    let stringids_map = read_dct_file(&self.stringids_dct)?;

    let mut result_map = HashMap::new();
    for (_, translation_key) in stringids_map.iter_line_entries() {
      let translated_text = match lang_map.get_line_entry(translation_key) {
        Ok(text) => Some(text.clone()),
        Err(_) => None,
      };

      result_map.insert(translation_key.clone(), translated_text);
    }

    let mut json_file = File::create(&self.json)?;
    serde_json::to_writer_pretty(&mut json_file, &result_map)?;

    Ok(())
  }
}
