use std::fmt::{Display, Formatter};
use std::fs::{create_dir, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

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
    file_input_path: Option<PathBuf>,
  },
  Encode {
    in_file: PathBuf,
    out_file: PathBuf,
    #[clap(short, long, default_value_t = Endian::Little)]
    endian: Endian,
    #[clap(short, long, default_value_t = Format::Json)]
    format: Format,
    #[clap(short = 't', long)]
    file_output_path: Option<PathBuf>,
  },
}

fn find_and_extract_textures(
  data: &mut IndexMap<String, ContainerData>,
  output_path: &PathBuf,
) -> anyhow::Result<()> {
  for (key, data) in data {
    match data {
      ContainerData::Single(data) => {
        if let Data::Container(container) = data {
          if key.contains("Texture#") {
            match (container.get("Name"), container.get("Data")) {
              (
                Some(ContainerData::Single(Data::String(texture_name))),
                Some(ContainerData::Single(texture_data_obj)),
              ) => {
                if let Data::Binary(texture_data) = texture_data_obj {
                  let file_name = format!("{}.dds", texture_name);
                  let mut texture_file = File::create(output_path.join(PathBuf::from(&file_name)))?;
                  texture_file.write(texture_data)?;

                  *container.get_mut("Data").unwrap() =
                    ContainerData::Single(Data::String(format!("file:{}", file_name)));
                  continue;
                }
              }
              _ => (),
            }
          }

          find_and_extract_textures(container, output_path)?;
        }
      }
      _ => unimplemented!("During texture replacing: Multiple container data is not a thing."),
    }
  }

  Ok(())
}

fn find_and_set_textures(
  data: &mut IndexMap<String, ContainerData>,
  input_path: &PathBuf,
) -> anyhow::Result<()> {
  for data in data.values_mut() {
    match data {
      ContainerData::Multiple(_) => {
        unimplemented!("During texture replacing: Multiple container data is not a thing.")
      }
      ContainerData::Single(Data::Container(container)) => {
        find_and_set_textures(container, input_path)?;
      }
      ContainerData::Single(Data::String(string_content)) => {
        if let Some(file_name) = string_content.strip_prefix("file:") {
          let mut texture_file = File::open(input_path.join(file_name))?;
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

impl OctModule {
  pub(super) fn execute(self) -> anyhow::Result<()> {
    match self.command {
      Command::Decode {
        in_file,
        out_file,
        format,
        file_input_path: file_input_path,
      } => {
        let mut file = BufReader::new(File::open(in_file)?);

        let (mut data, endian) = matryoshka::decode(&mut file)?;
        println!("Read file with endian: {}", endian);

        if let Some(output_path) = file_input_path {
          if !output_path.exists() {
            create_dir(&output_path)?;
          }

          println!("{:?}", output_path);
          find_and_extract_textures(&mut data, &output_path)?;
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
        file_output_path,
      } => {
        let file = BufReader::new(File::open(in_file)?);
        let mut data = match format {
          Format::Json => serde_json::from_reader(file)?,
          Format::Yaml => serde_yaml::from_reader(file)?,
        };

        if let Some(input_path) = file_output_path {
          find_and_set_textures(&mut data, &input_path)?;
        }

        let mut file = BufWriter::new(File::create(out_file)?);
        matryoshka::encode(&mut file, data, endian.into())?;
      }
    }

    Ok(())
  }
}
