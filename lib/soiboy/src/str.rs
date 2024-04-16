use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use binrw::io;
use flate2::read::ZlibDecoder;

use crate::toc::ComponentKind;
use crate::{ComponentHeader, Section};

#[derive(Debug)]
pub struct SectionData {
  pub uncached: Vec<ComponentData>,
  pub cached: Vec<ComponentData>,
}

#[derive(Debug)]
pub struct ComponentData {
  pub id: u32,
  pub path: String,
  pub instance_id: u32,
  pub kind: ComponentKind,
  pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct Str {
  file: File,
}

impl Str {
  pub fn read(path: &Path) -> io::Result<Self> {
    let file = File::open(path)?;
    Ok(Self::read_file(file))
  }

  pub fn read_file(file: File) -> Self {
    Self { file }
  }

  pub fn read_section_data(&mut self, section: &Section) -> io::Result<SectionData> {
    let header = &section.header;
    let zlib = &header.zlib_header;

    let section_offset = header.memory_entry.offset as u64;
    self.file.seek(SeekFrom::Start(section_offset))?;

    let uncached = {
      let data = self.decode_zlib_data(header.uncached_data_size as usize, &zlib.uncached_sizes)?;
      extract_components(&section.uncached_components, data)
    };

    let cached = {
      let data = self.decode_zlib_data(header.cached_data_size as usize, &zlib.cached_sizes)?;
      extract_components(&section.cached_components, data)
    };

    Ok(SectionData { uncached, cached })
  }

  pub fn decode_zlib_data(
    &mut self,
    output_size: usize,
    input_sizes: &[i32],
  ) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(output_size);

    for size in input_sizes {
      // reading compressed chunk
      let mut buf = vec![0; *size as usize];
      self.file.read_exact(&mut buf)?;

      // decompressing chunk and appending to merged vector
      let mut decoder = ZlibDecoder::new(&buf[..]);
      decoder.read_to_end(&mut output)?;
    }

    Ok(output)
  }
}

fn extract_components(headers: &[ComponentHeader], data: Vec<u8>) -> Vec<ComponentData> {
  let mut components = Vec::with_capacity(headers.len());

  for header in headers {
    let start = header.memory_entry.offset as usize;
    let end = (header.memory_entry.offset + header.memory_entry.size) as usize;

    let component = ComponentData {
      id: header.id as u32,
      path: header.path(),
      instance_id: header.instance_id as u32,
      kind: header.kind,
      // copy data for each component
      data: data[start..end].to_vec(),
    };

    components.push(component);
  }

  components
}
