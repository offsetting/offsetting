use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
  },
  Encode {
    in_file: PathBuf,
    out_file: PathBuf,
    #[clap(short, long, default_value_t = Endian::Little)]
    endian: Endian,
    #[clap(short, long, default_value_t = Format::Json)]
    format: Format,
  },
}

impl OctModule {
  pub(super) fn execute(self) -> anyhow::Result<()> {
    match self.command {
      Command::Decode {
        in_file,
        out_file,
        format,
      } => {
        let mut file = BufReader::new(File::open(in_file)?);

        let (data, endian) = matryoshka::decode(&mut file)?;
        println!("Read file with endian: {}", endian);

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
      } => {
        let file = BufReader::new(File::open(in_file)?);
        let data = match format {
          Format::Json => serde_json::from_reader(file)?,
          Format::Yaml => serde_yaml::from_reader(file)?,
        };

        let mut file = BufWriter::new(File::create(out_file)?);
        matryoshka::encode(&mut file, data, endian.into())?;
      }
    }

    Ok(())
  }
}
