use std::fs::File;
use std::io::Seek;
use std::path::Path;

use binrw::{BinRead, BinReaderExt, BinResult};

use crate::utils::clean_path;

#[derive(BinRead, Debug)]
pub(crate) struct Bounding {
  pub(crate) min_x: f32,
  pub(crate) max_x: f32,

  pub(crate) min_y: f32,
  pub(crate) max_y: f32,

  pub(crate) min_z: f32,
  pub(crate) max_z: f32,
}

#[derive(BinRead, Debug)]
pub(crate) struct MemoryEntry {
  pub(crate) offset: i32,
  pub(crate) size: i32,
}

#[derive(BinRead, PartialEq, Copy, Clone, Debug)]
#[br(repr = i32)]
pub enum ComponentKind {
  RenderableModel,
  Texture,
  CollisionModel,
  UserData,
  MotionPack,
  CollisionGrid,
}

#[derive(BinRead, Debug)]
pub(crate) struct ZlibHeader {
  pub(crate) uncached_total_size: i32,
  pub(crate) cached_total_size: i32,

  pub(crate) uncached_amount: i32,
  pub(crate) cached_amount: i32,

  #[br(count = uncached_amount)]
  pub(crate) uncached_sizes: Vec<i32>,

  #[br(count = cached_amount)]
  pub(crate) cached_sizes: Vec<i32>,
}

#[derive(BinRead, Debug)]
pub struct SectionHeader {
  pub name: [u8; 260],

  pub total_component_count: i32,
  pub uncached_component_count: i32,
  pub cached_component_count: i32,

  pub(crate) shared_section_offset: i32,
  pub(crate) uncached_page_offset: i32,
  pub(crate) cached_page_offset: i32,

  pub(crate) link_table: [i32; 8],

  pub(crate) bounding: Bounding,

  pub(crate) memory_entry: MemoryEntry,
  pub(crate) uncached_data_size: i32,
  pub(crate) cached_data_size: i32,

  pub(crate) zlib_header: ZlibHeader,
}

#[derive(BinRead, Debug)]
pub struct ComponentHeader {
  raw_path: [u8; 260],

  pub instance_id: i32,
  pub id: i32,
  pub(crate) memory_entry: MemoryEntry,
  pub kind: ComponentKind,
}

#[derive(BinRead, Debug)]
pub struct Section {
  pub header: SectionHeader,

  #[br(count = header.uncached_component_count)]
  pub uncached_components: Vec<ComponentHeader>,

  #[br(count = header.cached_component_count)]
  pub cached_components: Vec<ComponentHeader>,
}

#[derive(Debug)]
pub struct Toc {
  pub sections: Vec<Section>,
}

impl Toc {
  pub fn read(path: &Path) -> BinResult<Self> {
    let mut file = File::open(path)?;
    Self::read_file(&mut file)
  }

  pub fn read_file(file: &mut File) -> BinResult<Self> {
    let mut sections = Vec::new();
    let file_size = file.metadata()?.len();

    // read sections until the end of the file is reached
    while file.stream_position()? < file_size {
      let section = file.read_be()?;
      sections.push(section);
    }

    Ok(Self { sections })
  }

  pub fn find_section(&self, id: u32) -> Option<&Section> {
    self.sections.get(id as usize)
  }

  pub fn find_ids(&self, instance_id: u32) -> Option<(u32, u32)> {
    for (index, section) in self.sections.iter().enumerate() {
      for component in &section.uncached_components {
        if component.instance_id == instance_id as i32 {
          return Some((index as u32, component.id as u32));
        }
      }
      for component in &section.cached_components {
        if component.instance_id == instance_id as i32 {
          return Some((index as u32, component.id as u32));
        }
      }
    }

    None
  }
}

impl ComponentHeader {
  pub fn path(&self) -> String {
    clean_path(&self.raw_path)
  }
}
