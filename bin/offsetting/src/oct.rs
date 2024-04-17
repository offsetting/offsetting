use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use indexmap::IndexMap;

use matryoshka::{ContainerData, Data};

#[derive(Parser)]
pub(super) struct OctModule {
  #[clap(subcommand)]
  command: Command,
}

#[derive(Clone, ValueEnum)]
pub enum Endian {
  Big,
  Little,
}

#[derive(Clone, ValueEnum)]
pub enum Format {
  Json,
  Yaml,
}

impl Display for Endian {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Endian::Big => f.write_str("big"),
      Endian::Little => f.write_str("little"),
    }
  }
}

impl Display for Format {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Format::Json => f.write_str("json"),
      Format::Yaml => f.write_str("yaml"),
    }
  }
}

impl From<Endian> for matryoshka::Endian {
  fn from(value: Endian) -> Self {
    match value {
      Endian::Big => Self::Big,
      Endian::Little => Self::Little,
    }
  }
}

#[derive(Subcommand)]
enum Command {
  Decode {
    in_file: PathBuf,
    out_file: PathBuf,
    #[clap(short, long, default_value_t = Format::Json)]
    format: Format,
    #[clap(short = 't', long)]
    unpack_textures: bool,
  },
  Encode {
    in_file: PathBuf,
    out_file: PathBuf,
    #[clap(short, long, default_value_t = Endian::Little)]
    endian: Endian,
    #[clap(short, long, default_value_t = Format::Json)]
    format: Format,
    #[clap(short = 't', long)]
    repack_textures: bool,
  },
}

impl OctModule {
  pub(super) fn execute(self) -> anyhow::Result<()> {
    match self.command {
      Command::Decode {
        in_file,
        out_file,
        format,
        unpack_textures,
      } => {
        let mut file = BufReader::new(File::open(in_file)?);

        let (mut data, endian) = matryoshka::decode(&mut file)?;
        println!("Read file with endian: {}", endian);

        if unpack_textures {
          let texture_output = out_file.with_extension("textures");

          println!(
            "Extracting textures to: {}",
            texture_output.to_string_lossy()
          );
          find_and_extract_textures(&mut data, &texture_output)?;
        }

        let mut file = BufWriter::new(File::create(out_file)?);

        match format {
          Format::Json => serde_json::to_writer_pretty(&mut file, &data)?,
          Format::Yaml => serde_yaml::to_writer(&mut file, &data)?,
        }
      }
      Command::Encode {
        in_file,
        out_file,
        endian,
        format,
        repack_textures,
      } => {
        let file = BufReader::new(File::open(&in_file)?);
        let mut data = match format {
          Format::Json => serde_json::from_reader(file)?,
          Format::Yaml => serde_yaml::from_reader(file)?,
        };

        if repack_textures {
          let texture_input = in_file.with_extension("textures");

          println!("Loading textures from: {}", texture_input.to_string_lossy());
          find_and_set_textures(&mut data, &texture_input)?;
        }

        let mut file = BufWriter::new(File::create(out_file)?);
        matryoshka::encode(&mut file, data, endian.into())?;
      }
    }

    Ok(())
  }
}

const TEXTURE_PREFIX: &str = "Texture#";
const PATH_KEY: &str = "SourceFilePath";
const DATA_KEY: &str = "Data";
const DDS: &str = "dds";

fn find_and_extract_textures(
  data: &mut IndexMap<String, ContainerData>,
  output_path: &Path,
) -> anyhow::Result<()> {
  for (key, data) in data {
    match data {
      ContainerData::Single(Data::Container(container)) => {
        if key.starts_with(TEXTURE_PREFIX) {
          if let (
            Some(ContainerData::Single(Data::String(path))),
            Some(ContainerData::Single(Data::Binary(data))),
          ) = (container.get(PATH_KEY), container.get(DATA_KEY))
          {
            let out = output_path
              .join(path.replace('\\', std::path::MAIN_SEPARATOR_STR))
              .with_extension(DDS);
            println!("Extracting: {}", out.to_string_lossy());

            if let Some(parent) = out.parent() {
              if !parent.exists() {
                fs::create_dir_all(parent)?;
              }
            }

            let mut texture_file = File::create(&out)?;
            texture_file.write_all(data)?;

            *container.get_mut(DATA_KEY).unwrap() = ContainerData::Single(Data::String(format!(
              "file:{}",
              out.strip_prefix(output_path)?.to_string_lossy()
            )));
            continue;
          }
        }

        find_and_extract_textures(container, output_path)?;
      }
      ContainerData::Single(_) => {}
      _ => unimplemented!("find_and_extract_textures: Multiple container data is not a thing."),
    }
  }

  Ok(())
}

fn find_and_set_textures(
  data: &mut IndexMap<String, ContainerData>,
  input_path: &Path,
) -> anyhow::Result<()> {
  for data in data.values_mut() {
    match data {
      ContainerData::Multiple(_) => {
        unimplemented!("find_and_set_textures: Multiple container data is not a thing.")
      }
      ContainerData::Single(Data::Container(container)) => {
        find_and_set_textures(container, input_path)?;
      }
      ContainerData::Single(Data::String(string_content)) => {
        if let Some(file_name) = string_content.strip_prefix("file:") {
          let path = input_path.join(file_name);
          println!("Embedding: {}", path.to_string_lossy());
          let mut texture_file = File::open(&path)?;
          let mut texture_buf = Vec::new();
          texture_file.read_to_end(&mut texture_buf)?;

          *data = ContainerData::Single(Data::Binary(texture_buf));
        }
      }
      _ => {}
    }
  }

  Ok(())
}
