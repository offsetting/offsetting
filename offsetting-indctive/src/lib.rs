use clap::{Parser, Subcommand};
use indctive::dct_map::{DctMap, FooterEntry, FooterSubEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SerdeMetadata {
  pub initial_hash_value: u32,
  pub footer_entries: Vec<SerdeFooterEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SerdeFooterEntry {
  pub text: String,
  pub sub_entries: Vec<SerdeFooterSubEntry>,
}

impl From<FooterEntry> for SerdeFooterEntry {
  fn from(value: FooterEntry) -> Self {
    Self {
      text: value.text,
      sub_entries: value
        .sub_entries
        .into_iter()
        .map(|entry| SerdeFooterSubEntry::from(entry))
        .collect(),
    }
  }
}

impl Into<FooterEntry> for SerdeFooterEntry {
  fn into(self) -> FooterEntry {
    FooterEntry {
      text: self.text,
      sub_entries: self
        .sub_entries
        .into_iter()
        .map(|entry| entry.into())
        .collect(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SerdeFooterSubEntry {
  pub text: String,
  pub to_map_to: u32,
}

impl From<FooterSubEntry> for SerdeFooterSubEntry {
  fn from(value: FooterSubEntry) -> Self {
    Self {
      text: value.text,
      to_map_to: value.to_map_to,
    }
  }
}

impl Into<FooterSubEntry> for SerdeFooterSubEntry {
  fn into(self) -> FooterSubEntry {
    FooterSubEntry {
      text: self.text,
      to_map_to: self.to_map_to,
    }
  }
}

fn read_dct_file<P: AsRef<Path>>(path: P) -> anyhow::Result<DctMap> {
  let mut dct_file = File::open(path)?;
  let dct_map = DctMap::from_reader(&mut dct_file)?;

  Ok(dct_map)
}

fn write_dct_file<P: AsRef<Path>>(path: P, dct_map: &DctMap) -> anyhow::Result<()> {
  let mut dct_file = File::create(path)?;
  dct_map.to_writer(&mut dct_file)?;

  Ok(())
}

#[derive(Parser, Debug)]
pub struct IndctiveModule {
  #[clap(subcommand)]
  action: Action,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Action {
  Unpack(UnpackAction),
  Pack(PackAction),
}

impl IndctiveModule {
  pub fn execute(&self) -> anyhow::Result<()> {
    match &self.action {
      Action::Unpack(action) => action.execute(),
      Action::Pack(action) => action.execute(),
    }
  }
}

#[derive(Parser, Debug)]
pub(crate) struct UnpackAction {
  lang_dct: PathBuf,
  stringids_dct: PathBuf,
  output_path: PathBuf,
}

impl UnpackAction {
  pub fn execute(&self) -> anyhow::Result<()> {
    let lang_dct = read_dct_file(&self.lang_dct)?;
    let stringids_dct = read_dct_file(&self.stringids_dct)?;

    let mut translation_map = HashMap::new();
    for (_, translation_key) in stringids_dct.iter_line_entries() {
      let translated_text = match lang_dct.get_line_entry(translation_key) {
        Ok(text) => Value::String(text.to_string()),
        Err(_) => Value::Null,
      };

      translation_map.insert(translation_key.to_string(), translated_text);
    }

    if self.output_path.is_file() {
      return Err(anyhow::Error::msg("Output path is a file."));
    }

    if !self.output_path.exists() {
      fs::create_dir(&self.output_path)?;
    }

    let initial_hash_value = lang_dct.get_initial_hash_value();
    let footer_entries: Vec<SerdeFooterEntry> = lang_dct
      .footer_entries
      .into_iter()
      .map(|entry| SerdeFooterEntry::from(entry))
      .collect();

    let metadata = SerdeMetadata {
      initial_hash_value,
      footer_entries,
    };

    let mut translation_path = self.output_path.clone();
    translation_path.push("translation.json");

    let mut metadata_path = self.output_path.clone();
    metadata_path.push("metadata.json");

    let mut translation_file = File::create(&translation_path)?;
    let mut metadata_file = File::create(&metadata_path)?;

    serde_json::to_writer_pretty(&mut translation_file, &translation_map)?;
    serde_json::to_writer_pretty(&mut metadata_file, &metadata)?;

    Ok(())
  }
}

#[derive(Parser, Debug)]
pub(crate) struct PackAction {
  input_path: PathBuf,
  lang_dct: PathBuf,
  stringids_dct: PathBuf,

  #[arg(default_value_t = 0.4)]
  extra_capacity: f32,
}

impl PackAction {
  pub fn execute(&self) -> anyhow::Result<()> {
    let mut translation_path = self.input_path.clone();
    translation_path.push("translation.json");

    let mut metadata_path = self.input_path.clone();
    metadata_path.push("metadata.json");

    let mut translation_file = File::open(&translation_path)?;
    let mut metadata_file = File::open(&metadata_path)?;

    let translation_map: HashMap<String, Value> = serde_json::from_reader(&mut translation_file)?;
    let metadata: SerdeMetadata = serde_json::from_reader(&mut metadata_file)?;

    let initial_hash_value = metadata.initial_hash_value;
    let footer_entries: Vec<FooterEntry> = metadata
      .footer_entries
      .into_iter()
      .map(|entry| entry.into())
      .collect();

    let lang_map_length = translation_map.len();
    let dct_map_size = (lang_map_length as f32 * (self.extra_capacity + 1.0)).round() as u32;

    let mut lang_dct = DctMap::new(initial_hash_value, dct_map_size, footer_entries.clone());
    let mut stringids_dct = DctMap::new(initial_hash_value, dct_map_size, footer_entries);

    for (translation_key, translation_text) in translation_map.iter() {
      stringids_dct.add_line_entry(translation_key, translation_key)?;

      match translation_text {
        Value::Null => Ok(()),
        Value::String(text) => {
          lang_dct.add_line_entry(translation_key, text)?;
          Ok(())
        }
        _ => Err(anyhow::Error::msg(format!(
          "{translation_key} has wrong value type {translation_text:?}."
        ))),
      }?;
    }

    write_dct_file(&self.stringids_dct, &stringids_dct)?;
    write_dct_file(&self.lang_dct, &lang_dct)?;

    Ok(())
  }
}
