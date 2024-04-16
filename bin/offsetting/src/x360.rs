use std::fs::{create_dir_all, File};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::ProgressBar;
use soiboy::ComponentKind::Texture;
use soiboy::{ComponentData, SoiSoup, Str};
use x_flipper_360::{convert_to_dds, TextureHeader, TextureSize2D};

#[derive(Parser)]
pub(super) struct X360Module {
  #[clap(subcommand)]
  action: Action,
}

#[derive(Subcommand)]
pub(crate) enum Action {
  Unpack(UnpackAction),
  Repack,
  Ls(LsAction),
}

#[derive(Parser)]
pub(crate) struct UnpackAction {
  soi: PathBuf,
  toc: PathBuf,
  str: PathBuf,
}

#[derive(Parser)]
pub(crate) struct LsAction {
  soi: PathBuf,
  toc: PathBuf,
}

impl X360Module {
  pub(super) fn execute(&self) -> anyhow::Result<()> {
    match &self.action {
      Action::Unpack(action) => action.execute(),
      Action::Repack => Ok(()),
      Action::Ls(action) => action.execute(),
    }
  }
}

impl UnpackAction {
  fn execute(&self) -> anyhow::Result<()> {
    let soup = SoiSoup::cook(self.toc.as_path(), self.soi.as_path())?;
    let mut str = Str::read(self.str.as_path())?;

    let bar = ProgressBar::new(soup.component_count() as u64);

    for (section_id, section) in soup.find_sections().iter().enumerate() {
      let section_data = str.read_section_data(section)?;

      for component in section_data.uncached {
        bar.inc(1);
        extract_component(&soup, section_id as u32, &component)?;
      }

      for component in section_data.cached {
        bar.inc(1);
        extract_component(&soup, section_id as u32, &component)?;
      }
    }

    bar.finish_and_clear();

    Ok(())
  }
}

impl LsAction {
  fn execute(&self) -> anyhow::Result<()> {
    let soup = SoiSoup::<TextureHeader>::cook(self.toc.as_path(), self.soi.as_path())?;

    println!();
    println!(
      "{:<10} | {:<75} | {}",
      "Section ID".bold(),
      "Path".bold(),
      "Format".bold()
    );
    println!("-----------+-----------------------------------------------------------------------------|-------");

    for (section_id, _, component) in soup.find_components() {
      if component.kind != Texture {
        // TODO:
        continue;
      }

      let metadata = soup
        .find_texture_header(
          section_id,
          component.id as u32,
          component.instance_id as u32,
        )
        .unwrap()
        .metadata();

      println!(
        "{:<10} | {:<75} | {:?}",
        section_id,
        component.path(),
        metadata.format()
      );
    }

    println!();

    Ok(())
  }
}

fn extract_component(
  soup: &SoiSoup<TextureHeader>,
  section_id: u32,
  component: &ComponentData,
) -> anyhow::Result<()> {
  if component.kind != Texture {
    // TODO:
    return Ok(());
  }

  let metadata = soup
    .find_texture_header(section_id, component.id, component.instance_id)
    .unwrap()
    .metadata();

  let texture_size = TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

  let config = x_flipper_360::Config {
    width: (texture_size.width() + 1) as u32,
    height: (texture_size.height() + 1) as u32,
    depth: Some(1),
    pitch: metadata.pitch() as u32,
    tiled: metadata.tiled(),
    packed_mips: metadata.packed_mips(),
    format: match metadata.format() {
      x_flipper_360::TextureFormat::Dxt1 => x_flipper_360::Format::Dxt1,
      x_flipper_360::TextureFormat::Dxt4_5 => x_flipper_360::Format::Dxt5,
      x_flipper_360::TextureFormat::_8_8_8_8 => return Ok(()), // TODO:
      _ => panic!("{:?}", metadata.format()),
    },
    mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
    base_address: metadata.base_address(),
    mip_address: metadata.mip_address(),
  };

  let path = PathBuf::from(format!("data/out/{}.dds", component.path));
  create_dir_all(path.parent().unwrap())?;

  let mut file = File::create(path)?;
  Ok(convert_to_dds(&config, &component.data, &mut file)?)
}
