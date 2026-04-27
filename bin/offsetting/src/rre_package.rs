use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use binrw::BinWrite;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use indicatif::ProgressBar;
use soiboy::*;
use x_flipper_360::{convert_to_dds, Config, Format, TextureFormat, TextureHeader, TextureSize2D};

#[derive(ValueEnum, PartialEq, Clone, Copy, Debug)]
pub enum Platform {
  Xbox360 = 0,
  Wii = 1,
  Xbox,
}

#[derive(ValueEnum, PartialEq, Clone, Copy, Debug)]
pub enum Game {
  TheVideoGame = 0,
  MaterNational = 1,
  RaceORama = 2,
}

#[derive(ValueEnum, PartialEq, Clone, Copy, Debug)]
pub enum Developer {
  Rainbow = 0,
  Incinerator = 1,
}

impl Developer {
  pub fn from(platform: Platform, game: Game) -> Developer {
    if game == Game::RaceORama || platform == Platform::Wii {
      return Developer::Incinerator;
    } else {
      return Developer::Rainbow;
    }
  }
}

/// Extracts/dumps scene information from Rhythm Racing Engine streaming packages. (.str + .soi + .toc)
#[derive(Parser)]
pub(super) struct RREPackageModule {
  #[clap(subcommand)]
  action: Action,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Action {
  PrintScn(PrintScnCmd),
  Extract(ExtractCmd),
}

#[derive(Parser, Debug, Clone)]
pub struct PrintScnCmd {
  soi_path: PathBuf,
  toc_path: PathBuf,
  str_path: PathBuf,
  platform: Platform,
  out_scn: PathBuf,
}

impl PrintScnCmd {
  pub fn execute(&self) -> Option<()> {
    match self.platform {
      Platform::Xbox360 => {
        let soup = SoiSoup::<TextureHeader, X360StaticTextureHeader, XNGHeader>::cook(
          &self.toc_path,
          &self.soi_path,
          binrw::Endian::Big,
        )
        .unwrap();
        // let mut str = Str::read(&self.str_path).unwrap();

        let mut num_anim_models = 1;
        let mut num_static_models = 1;
        let mut num_objects = 1;

        for renderable_model in soup.renderable_models().iter() {
          if renderable_model.model_info.is_animated == 0 {
            println!("[Model{}]\n{}", num_static_models, renderable_model);
            num_static_models += 1;
          }
        }
        for renderable_model in soup.renderable_models().iter() {
          if renderable_model.model_info.is_animated == 1 {
            println!("[AnimatedModel{}]\n{}", num_anim_models, renderable_model);
            num_anim_models += 1;
          }
        }
        for collision_model in soup.collision_models().iter() {
          println!("[Object{}]\n{}", num_objects, collision_model);
          num_objects += 1;
        }
      }
      Platform::Wii => {
        let soup = SoiSoup::<GCNTextureHeader, GCNStaticTextureHeader, GCGHeader>::cook(
          &self.toc_path,
          &self.soi_path,
          binrw::Endian::Big,
        )
        .unwrap();
        // let mut str = Str::read(&self.str_path).unwrap();

        let mut num_anim_models = 1;
        let mut num_static_models = 1;
        let mut num_objects = 1;

        for renderable_model in soup.renderable_models().iter() {
          if renderable_model.model_info.is_animated == 0 {
            println!("[Model{}]\n{}", num_static_models, renderable_model);
            num_static_models += 1;
          }
        }
        for renderable_model in soup.renderable_models().iter() {
          if renderable_model.model_info.is_animated == 1 {
            println!("[AnimatedModel{}]\n{}", num_anim_models, renderable_model);
            num_anim_models += 1;
          }
        }
        for collision_model in soup.collision_models().iter() {
          println!("[Object{}]\n{}", num_objects, collision_model);
          num_objects += 1;
        }
      }
      Platform::Xbox => {
        let soup = SoiSoup::<DXTTextureHeader, DXTStaticTextureHeader, DXGHeader>::cook(
          &self.toc_path,
          &self.soi_path,
          binrw::Endian::Little,
        )
        .unwrap();
        // let mut str = Str::read(&self.str_path).unwrap();

        let mut num_anim_models = 1;
        let mut num_static_models = 1;
        let mut num_objects = 1;

        for renderable_model in soup.renderable_models().iter() {
          if renderable_model.model_info.is_animated == 0 {
            println!("[Model{}]\n{}", num_static_models, renderable_model);
            num_static_models += 1;
          }
        }
        for renderable_model in soup.renderable_models().iter() {
          if renderable_model.model_info.is_animated == 1 {
            println!("[AnimatedModel{}]\n{}", num_anim_models, renderable_model);
            num_anim_models += 1;
          }
        }
        for collision_model in soup.collision_models().iter() {
          println!("[Object{}]\n{}", num_objects, collision_model);
          num_objects += 1;
        }
      }
    };
    Some(())
  }
}

#[derive(Parser, Debug, Clone)]
pub struct ExtractCmd {
  soi_path: PathBuf,
  toc_path: PathBuf,
  str_path: PathBuf,
  game: Game,
  platform: Platform,
  out_folder: PathBuf,
}

impl ExtractCmd {
  pub fn execute(&self) -> Option<()> {
    match self.platform {
      Platform::Xbox360 => {
        let soup = SoiSoup::<TextureHeader, X360StaticTextureHeader, XNGHeader>::cook(
          &self.toc_path,
          &self.soi_path,
          binrw::Endian::Big,
        )
        .unwrap();
        let mut str = Str::read(&self.str_path).unwrap();

        for (id, section) in soup.find_sections().iter().enumerate() {
          let section_data = str.read_section_data(section).unwrap();

          for component in section_data.uncached {
            process_component_xbox360(
              &soup,
              id as u32,
              &component,
              &self.out_folder,
              self.game == Game::RaceORama,
            );
          }

          for component in section_data.cached {
            process_component_xbox360(
              &soup,
              id as u32,
              &component,
              &self.out_folder,
              self.game == Game::RaceORama,
            );
          }
        }
        for static_texture in soup.static_textures().iter() {
          println!(
            "Extracting file: {}.dds...",
            soiboy::clean_path(&static_texture.model_info.name)
          );
          let path = self.out_folder.join(Path::new(&format!(
            "{}.dds",
            soiboy::clean_path(&static_texture.model_info.name)
          )));
          std::fs::create_dir_all(path.parent().unwrap()).unwrap();
          let mut out = std::fs::File::create(path).unwrap();
          out
            .write_all(&static_texture.static_texture_header.header_file)
            .unwrap();
        }
      }
      Platform::Wii => {
        let soup = SoiSoup::<GCNTextureHeader, GCNStaticTextureHeader, GCGHeader>::cook(
          &self.toc_path,
          &self.soi_path,
          binrw::Endian::Big,
        )
        .unwrap();
        let mut str = Str::read(&self.str_path).unwrap();

        for (id, section) in soup.find_sections().iter().enumerate() {
          let section_data = str.read_section_data(section).unwrap();

          for component in section_data.uncached {
            process_component_wii(
              &soup,
              id as u32,
              &component,
              &self.out_folder,
              self.game == Game::RaceORama,
            );
          }

          for component in section_data.cached {
            process_component_wii(
              &soup,
              id as u32,
              &component,
              &self.out_folder,
              self.game == Game::RaceORama,
            );
          }
        }
        for static_texture in soup.static_textures().iter() {
          println!(
            "Extracting file: {}.gct...",
            soiboy::clean_path(&static_texture.model_info.name)
          );
          let path = self.out_folder.join(Path::new(&format!(
            "{}.gct",
            soiboy::clean_path(&static_texture.model_info.name)
          )));
          std::fs::create_dir_all(path.parent().unwrap()).unwrap();
          let mut out = std::fs::File::create(path).unwrap();
          static_texture
            .static_texture_header
            .write_be(&mut out)
            .unwrap();
        }
      }
      Platform::Xbox => {
        let soup = SoiSoup::<DXTTextureHeader, DXTStaticTextureHeader, DXGHeader>::cook(
          &self.toc_path,
          &self.soi_path,
          binrw::Endian::Little,
        )
        .unwrap();
        let mut str = Str::read(&self.str_path).unwrap();

        for (id, section) in soup.find_sections().iter().enumerate() {
          let section_data = str.read_section_data(section).unwrap();

          for component in section_data.uncached {
            // process_component_xbox(&soup, id as u32, &component, &self.out_folder, self.game == Game::RaceORama);
          }

          for component in section_data.cached {
            // process_component_xbox(&soup, id as u32, &component, &self.out_folder, self.game == Game::RaceORama);
          }
        }
        for static_texture in soup.static_textures().iter() {
          println!(
            "Extracting file: {}.dxt...",
            soiboy::clean_path(&static_texture.model_info.name)
          );
          let path = self.out_folder.join(Path::new(&format!(
            "{}.dxt",
            soiboy::clean_path(&static_texture.model_info.name)
          )));
          std::fs::create_dir_all(path.parent().unwrap()).unwrap();
          let mut out = std::fs::File::create(path).unwrap();
          static_texture
            .static_texture_header
            .write_be(&mut out)
            .unwrap();
        }
      }
    };
    Some(())
  }
}

impl RREPackageModule {
  pub(super) fn execute(&self) -> anyhow::Result<()> {
    match &self.action {
      Action::Extract(action) => action
        .execute()
        .ok_or_else(|| anyhow::anyhow!("Failed to extract scene.")),
      Action::PrintScn(action) => action
        .execute()
        .ok_or_else(|| anyhow::anyhow!("Failed to print scene metadata.")),
    }
  }
}

fn process_component_wii(
  soup: &SoiSoup<GCNTextureHeader, GCNStaticTextureHeader, GCGHeader>,
  section_id: u32,
  component: &ComponentData,
  out_dir: &Path,
  is_ror: bool,
) {
  let extension = match component.kind {
    ComponentKind::MotionPack => "wot",
    ComponentKind::RenderableModel => "gcg",
    ComponentKind::CollisionModel => "gol",
    ComponentKind::Texture => "gct",
    _ => "bin",
  };
  let path = out_dir.join(Path::new(&format!("{}.{}", component.path, extension)));
  println!("Extracting file: {}.{}...", component.path, extension);

  if component.kind == ComponentKind::MotionPack {
    let header = soup
      .find_motion_pack(section_id, component.id, component.instance_id)
      .unwrap();

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = std::fs::File::create(path).unwrap();
    header.header.write_be(&mut out).unwrap();
    out.write_all(&component.data).unwrap();
  } else if component.kind == ComponentKind::RenderableModel {
    let header = soup
      .find_model(section_id, component.id, component.instance_id)
      .unwrap();

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = std::fs::File::create(path).unwrap();
    header
      .streaming_model_header
      .write_options(
        &mut out,
        binrw::Endian::Big,
        &GCGHeaderArgs {
          streaming_data: component.data.clone(),
        },
      )
      .unwrap();
  } else if component.kind == ComponentKind::CollisionModel {
    let header = soup
      .find_collision_model(section_id, component.id, component.instance_id)
      .unwrap();

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = std::fs::File::create(path).unwrap();
    header
      .collision_model
      .write_options(
        &mut out,
        binrw::Endian::Big,
        &CollisionModelArgs {
          ror: is_ror,
          streaming_data: component.data.clone(),
        },
      )
      .unwrap();
  } else if component.kind == ComponentKind::Texture {
    match soup.find_streaming_texture(section_id, component.id, component.instance_id) {
      Some(streaming_texture) => {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut out = std::fs::File::create(path).unwrap();

        // Write the version out (streamed GCTs have a flag OR'd on the version, so we set it manually to 2 [otherwise we would use streaming_texture.version])
        u32::write_options(&2, &mut out, binrw::Endian::Big, ()).unwrap();
        streaming_texture.header.write_be(&mut out).unwrap();

        // Keeps track of our position in the streaming data.
        let mut offset = 0;
        let mip_count = streaming_texture.header.mip_count;

        // Holds the offsets and sizes (in a Range<usize>) for each mip so we can iterate backwards over this later
        let mut mips = Vec::with_capacity(mip_count as usize);

        // Since mips are stored in forwards order (biggest to smallest) in the streaming data we need to first collect them in the vec,
        for i in 0..mip_count {
          let mip_width = 1.max(streaming_texture.header.width as usize / (2 as usize).pow(i));
          let mip_height = 1.max(streaming_texture.header.height as usize / (2 as usize).pow(i));

          let mip_size = streaming_texture
            .header
            .format
            .calculate_mip_size(mip_width, mip_height);

          mips.push(offset..offset + mip_size);

          offset += mip_size as usize;
        }

        // (At this point, we should have read through the entire streaming data)
        assert_eq!(offset, streaming_texture.header.calculate_image_size());
        assert_eq!(offset, component.data.len());

        // and we can then terate over the mips in backwards order and write them to the GCT.
        for i in (0..mip_count).rev() {
          let mip_width = 1.max(streaming_texture.header.width as usize / (2 as usize).pow(i));
          let mip_height = 1.max(streaming_texture.header.height as usize / (2 as usize).pow(i));

          let surface_header = GCTSurfaceHeader {
            width: mip_width as u32,
            height: mip_height as u32,
            size: streaming_texture
              .header
              .format
              .calculate_mip_size(mip_width, mip_height) as u32,
          };
          surface_header.write_be(&mut out).unwrap();

          component.data[mips.get(i as usize).unwrap().clone()]
            .write(&mut out)
            .unwrap();
        }
      }
      None => {
        panic!("Failed to find texture header.");
      }
    }
  }
}

fn process_component_xbox360(
  soup: &SoiSoup<TextureHeader, X360StaticTextureHeader, XNGHeader>,
  section_id: u32,
  component: &ComponentData,
  out_dir: &Path,
  is_ror: bool,
) {
  let extension = match component.kind {
    ComponentKind::MotionPack => "got",
    ComponentKind::RenderableModel => "xng",
    ComponentKind::CollisionModel => "gol",
    ComponentKind::Texture => "dds",
    _ => "bin",
  };
  let path = out_dir.join(Path::new(&format!("{}.{}", component.path, extension)));
  println!("Extracting file: {}.{}...", component.path, extension);

  if component.kind == ComponentKind::MotionPack {
    let header = soup
      .find_motion_pack(section_id, component.id, component.instance_id)
      .unwrap();

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = std::fs::File::create(path).unwrap();
    header.header.write_be(&mut out).unwrap();
    out.write_all(&component.data).unwrap();
  } else if component.kind == ComponentKind::RenderableModel {
    let header = soup
      .find_model(section_id, component.id, component.instance_id)
      .unwrap();

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = std::fs::File::create(path).unwrap();
    header
      .streaming_model_header
      .write_options(
        &mut out,
        binrw::Endian::Big,
        &XNGHeaderArgs {
          streaming_data: component.data.clone(),
        },
      )
      .unwrap();
  } else if component.kind == ComponentKind::CollisionModel {
    let header = soup
      .find_collision_model(section_id, component.id, component.instance_id)
      .unwrap();

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut out = std::fs::File::create(path).unwrap();
    header
      .collision_model
      .write_options(
        &mut out,
        binrw::Endian::Big,
        &CollisionModelArgs {
          ror: is_ror,
          streaming_data: component.data.clone(),
        },
      )
      .unwrap();
  } else if component.kind == ComponentKind::Texture {
    match soup.find_streaming_texture(section_id, component.id, component.instance_id) {
      Some(header) => {
        let metadata = header.header.metadata();

        match metadata.format() {
          TextureFormat::Dxt1 => {
            let texture_size: TextureSize2D =
              TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

            let config = Config {
              width: texture_size.width() as u32 + 1,
              height: texture_size.height() as u32 + 1,
              depth: None,
              pitch: metadata.pitch() as u32,
              tiled: metadata.tiled(),
              packed_mips: metadata.packed_mips(),
              format: Format::Dxt1,
              mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
              base_address: metadata.base_address(),
              mip_address: metadata.mip_address(),
            };

            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut out = std::fs::File::create(path).unwrap();
            x_flipper_360::convert_to_dds(&config, &component.data, &mut out).unwrap();
          }
          TextureFormat::Dxt4_5 => {
            let texture_size: TextureSize2D =
              TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

            let config = Config {
              width: texture_size.width() as u32 + 1,
              height: texture_size.height() as u32 + 1,
              depth: None,
              pitch: metadata.pitch() as u32,
              tiled: metadata.tiled(),
              packed_mips: metadata.packed_mips(),
              format: Format::Dxt5,
              mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
              base_address: metadata.base_address(),
              mip_address: metadata.mip_address(),
            };

            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut out = std::fs::File::create(path).unwrap();
            x_flipper_360::convert_to_dds(&config, &component.data, &mut out).unwrap();
          }
          TextureFormat::Dxt2_3 => {
            let texture_size: TextureSize2D =
              TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

            let config = Config {
              width: texture_size.width() as u32 + 1,
              height: texture_size.height() as u32 + 1,
              depth: None,
              pitch: metadata.pitch() as u32,
              tiled: metadata.tiled(),
              packed_mips: metadata.packed_mips(),
              format: Format::Dxt3,
              mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
              base_address: metadata.base_address(),
              mip_address: metadata.mip_address(),
            };

            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut out = std::fs::File::create(path).unwrap();
            x_flipper_360::convert_to_dds(&config, &component.data, &mut out).unwrap();
          }
          _ => {
            let texture_size: TextureSize2D =
              TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

            let config = Config {
              width: texture_size.width() as u32 + 1,
              height: texture_size.height() as u32 + 1,
              depth: None,
              pitch: metadata.pitch() as u32,
              tiled: metadata.tiled(),
              packed_mips: metadata.packed_mips(),
              format: Format::RGBA8,
              mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
              base_address: metadata.base_address(),
              mip_address: metadata.mip_address(),
            };

            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut out = std::fs::File::create(path).unwrap();
            x_flipper_360::convert_to_dds(&config, &component.data, &mut out).unwrap();
          }
        }
      }
      None => match soup.find_static_texture(section_id, component.id, component.instance_id) {
        Some(static_texture) => {
          std::fs::create_dir_all(path.parent().unwrap()).unwrap();
          let mut out = std::fs::File::create(path).unwrap();
          out
            .write_all(&static_texture.static_texture_header.header_file)
            .unwrap();
        }
        None => panic!("Failed to find texture header."),
      },
    }
  }
}
