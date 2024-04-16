use std::path::Path;

use binrw::{BinRead, BinResult};

use crate::{ComponentHeader, Section, Soi, Toc};

pub struct SoiSoup<TH: BinRead<Args<'static> = ()> + 'static> {
  toc: Toc,
  soi: Soi<TH>,
}

impl<TH: BinRead<Args<'static> = ()>> SoiSoup<TH> {
  pub fn cook(toc_path: &Path, soi_path: &Path) -> BinResult<Self> {
    let toc = Toc::read(toc_path)?;
    let soi = Soi::read(soi_path)?;

    Ok(Self { toc, soi })
  }

  pub fn find_sections(&self) -> &Vec<Section> {
    &self.toc.sections
  }

  pub fn find_components(&self) -> Vec<(u32, &Section, &ComponentHeader)> {
    let mut components = Vec::new();

    for (id, section) in self.toc.sections.iter().enumerate() {
      let id = id as u32;

      for component in &section.uncached_components {
        components.push((id, section, component));
      }

      for component in &section.cached_components {
        components.push((id, section, component));
      }
    }

    components
  }

  pub fn component_count(&self) -> u32 {
    let mut sum = 0;

    for section in &self.toc.sections {
      sum += section.header.total_component_count;
    }

    sum as u32
  }

  pub fn find_texture_header(
    &self,
    section_id: u32,
    component_id: u32,
    instance_id: u32,
  ) -> Option<&TH> {
    if let Some(header) = self.soi.find_texture_header(section_id, component_id) {
      return Some(header);
    }

    let (section_id, component_id) = self.toc.find_ids(instance_id)?;
    self.soi.find_texture_header(section_id, component_id)
  }
}
