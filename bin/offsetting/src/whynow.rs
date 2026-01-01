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

/// Creates new, unencrypted octane zips, for use in Cars 3: Driven to Win
#[derive(Parser)]
#[command(about)]
pub struct C3ZipModule {
  in_folder: PathBuf,
  out_file: PathBuf,
}

/// Creates old octane zips, for use in Cars 2, Toy Story 3, and Disney Infinity 1.0/2.0
#[derive(Parser)]
#[command(about)]
pub struct C2ZipModule {
  in_folder: PathBuf,
  out_file: PathBuf,
}

/// Creates new, encrypted octane zips, for use in Disney Infinity 3.0
#[derive(Parser)]
#[command(about)]
pub struct DI3ZipModule {
  in_folder: PathBuf,
  out_file: PathBuf,
  /// A 16 byte hex string of the encryption key. Ask your friends... 😂
  #[clap(about, short = 'e', long = "enc-key", value_parser = decode_encryption_key)]
  encryption_key: [u8; 16],
}

impl C3ZipModule {
  pub fn execute(self) -> anyhow::Result<()> {
    whynow::write_octane_zip(
      &self.in_folder,
      &self.out_file,
      &mut whynow::NewOctaneZipWriter,
    )
  }
}

impl C2ZipModule {
  pub fn execute(self) -> anyhow::Result<()> {
    whynow::write_octane_zip(
      &self.in_folder,
      &self.out_file,
      &mut whynow::OldOctaneZipWriter,
    )
  }
}

impl DI3ZipModule {
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
