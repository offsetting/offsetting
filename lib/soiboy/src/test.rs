use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};

use x_flipper_360::{Config, Format, TextureFormat, TextureHeader, TextureSize2D};

use crate::ComponentKind::Texture;
use crate::{ComponentData, SoiSoup, Str};

#[test]
fn extract() {
  let toc_path = Path::new("data/CR_03.x360.toc");
  let soi_path = Path::new("data/CR_03.x360.soi");
  let str_path = Path::new("data/CR_03.x360.str");

  let soup = SoiSoup::cook(toc_path, soi_path).unwrap();
  let mut str = Str::read(str_path).unwrap();

  for (id, section) in soup.find_sections().iter().enumerate() {
    let section_data = str.read_section_data(section).unwrap();

    for component in section_data.uncached {
      process_component(&soup, id as u32, component);
    }

    for component in section_data.cached {
      process_component(&soup, id as u32, component);
    }
  }
}

fn process_component(soup: &SoiSoup<TextureHeader>, section_id: u32, component: ComponentData) {
  if component.kind != Texture {
    return;
  }

  let header = match soup.find_texture_header(section_id, component.id, component.instance_id) {
    Some(header) => header,
    None => panic!("can not find texture header by section and component id nor instance id"),
  };

  let metadata = header.metadata();
  println!("{} {:?}", component.path, metadata.format());

  let texture_size: TextureSize2D =
    TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

  let config = Config {
    width: texture_size.width() as u32 + 1,
    height: texture_size.height() as u32 + 1,
    depth: None,
    pitch: metadata.pitch() as u32,
    tiled: metadata.tiled(),
    packed_mips: metadata.packed_mips(),
    format: match metadata.format() {
      TextureFormat::Dxt1 => Format::Dxt1,
      TextureFormat::Dxt4_5 => Format::Dxt5,
      _ => Format::RGBA8,
    },
    mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
    base_address: metadata.base_address(),
    mip_address: metadata.mip_address(),
  };

  let path = PathBuf::from(format!(
    "./data/out/{}.dds",
    component.path.replace("\\", "/")
  ));
  create_dir_all(path.parent().unwrap()).unwrap();
  let mut out = File::create(path).unwrap();

  x_flipper_360::convert_to_dds(&config, &component.data, &mut out).unwrap();
}
