use clap::Parser;
use std::path::PathBuf;
use std::u128;

fn decode_encryption_key(hex_string: &str) -> Result<[u8; 16], String> {
  Ok(
    u128::from_str_radix(hex_string, 16)
      .map_err(|_| "Encryption key is not a valid 16 byte hex string.".to_string())?
      .to_be_bytes(),
  )
}

/// Creates unencrypted new octane zips. e.g. Cars 3: DTW
#[derive(Parser)]
#[command(about)]
pub struct WhyNowModule {
  in_folder: PathBuf,
  out_file: PathBuf,
}

/// Creates old octane zips. e.g. Cars 2, Toy Story, DI 1.0 and 2.0
#[derive(Parser)]
#[command(about)]
pub struct WhyModule {
  in_folder: PathBuf,
  out_file: PathBuf,
}

/// Creates encrypted new octane zips. e.g. DI 3.0
#[derive(Parser)]
#[command(about)]
pub struct WhyJustWhyModule {
  in_folder: PathBuf,
  out_file: PathBuf,
  /// A 16 byte hex string of the encryption key. Ask your friends... 😂
  #[clap(about, short = 'e', long = "enc-key", value_parser = decode_encryption_key)]
  encryption_key: [u8; 16],
}

impl WhyNowModule {
  pub fn execute(self) -> anyhow::Result<()> {
    whynow::write_octane_zip(
      &self.in_folder,
      &self.out_file,
      &mut whynow::NewOctaneZipWriter,
    )
  }
}

impl WhyModule {
  pub fn execute(self) -> anyhow::Result<()> {
    whynow::write_octane_zip(
      &self.in_folder,
      &self.out_file,
      &mut whynow::OldOctaneZipWriter,
    )
  }
}

impl WhyJustWhyModule {
  pub fn execute(self) -> anyhow::Result<()> {
    whynow::write_octane_zip(
      &self.in_folder,
      &self.out_file,
      &mut whynow::EncryptedNewOctaneZipWriter {
        key: &self.encryption_key,
      },
    )
  }
}
