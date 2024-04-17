use crate::bin_structure::{
  BinDctFooterEntry, BinDctFooterSubEntry, BinDctHeader, BinDctLineEntry, FOOTER_ENTRY_SIZE,
  FOOTER_SUB_ENTRY_SIZE, HEADER_SIZE, LINE_ENTRY_SIZE,
};
use crate::dct_map::DctLineError::{CapacityExceeded, KeyAlreadyExists, KeyDoesNotExist};
use binrw::{BinRead, BinResult, BinWrite, NullString};
use jenkins_hash::lookup2;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DctLineError {
  #[error("the key `{0}` does not exist in this dct map")]
  KeyDoesNotExist(String),
  #[error("the key `{0}` does already exist in the this map")]
  KeyAlreadyExists(String),
  #[error("the capacity of this dct map is already at its maximum")]
  CapacityExceeded,
}

#[derive(Clone, Debug)]
struct LineEntry {
  pub line_id: u32,
  pub text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FooterEntry {
  pub text: String,
  pub sub_entries: Vec<FooterSubEntry>,
}

#[derive(Clone, Debug)]
pub struct FooterSubEntry {
  pub text: String,
  pub to_map_to: u32,
}

#[derive(Clone)]
pub struct DctMap {
  initial_hash_value: u32,
  line_entries: Vec<LineEntry>,
  pub footer_entries: Vec<FooterEntry>,
}

impl DctMap {
  pub fn new(initial_hash_value: u32, capacity: u32, footer_entries: Vec<FooterEntry>) -> Self {
    Self {
      initial_hash_value,
      line_entries: vec![
        LineEntry {
          line_id: 0,
          text: None,
        };
        capacity as usize
      ],
      footer_entries,
    }
  }

  pub fn from_reader<R: Read + Seek>(reader: &mut R) -> BinResult<Self> {
    let header = BinDctHeader::read_le(reader)?;

    let mut line_entries = Vec::with_capacity(header.line_count as usize);
    for _ in 0..header.line_count as usize {
      let bin_line_entry = BinDctLineEntry::read_le(reader)?;

      let text = match bin_line_entry.text_offset {
        None => None,
        Some(text_offset) => {
          let cur_pos = reader.stream_position()?;

          reader.seek(SeekFrom::Start(text_offset))?;
          let text = NullString::read_le(reader)?.to_string();

          reader.seek(SeekFrom::Start(cur_pos))?;
          Some(text)
        }
      };

      let line_entry = LineEntry {
        line_id: bin_line_entry.line_id,
        text,
      };
      line_entries.push(line_entry);
    }

    let mut footer_entries = Vec::<FooterEntry>::with_capacity(header.footer_count as usize);
    for _ in 0..header.footer_count {
      let bin_footer_entry = BinDctFooterEntry::read_le(reader)?;

      let cur_pos = reader.stream_position()?;
      reader.seek(SeekFrom::Start(bin_footer_entry.text_offset))?;
      let text = NullString::read_le(reader)?.to_string();

      let sub_entries: BinResult<Vec<FooterSubEntry>> = bin_footer_entry
        .sub_entries
        .iter()
        .map(|sub_entry| {
          reader.seek(SeekFrom::Start(sub_entry.text_offset))?;
          Ok(FooterSubEntry {
            text: NullString::read_le(reader)?.to_string(),
            to_map_to: sub_entry.to_map_to,
          })
        })
        .collect();

      let footer_entry = FooterEntry {
        text,
        sub_entries: sub_entries?,
      };
      reader.seek(SeekFrom::Start(cur_pos))?;

      footer_entries.push(footer_entry);
    }

    Ok(Self {
      initial_hash_value: header.initial_hash_value,
      line_entries,
      footer_entries,
    })
  }

  pub fn to_writer<W: Write + Seek>(&self, writer: &mut W) -> BinResult<()> {
    let line_count = self.line_entries.len();
    let footer_count = self.footer_entries.len();
    let footer_sub_entry_count = self
      .footer_entries
      .iter()
      .map(|entry| entry.sub_entries.len())
      .sum::<usize>();

    let line_chunk_size = LINE_ENTRY_SIZE * line_count;
    let footer_chunk_size =
      FOOTER_ENTRY_SIZE * footer_count + FOOTER_SUB_ENTRY_SIZE * footer_sub_entry_count;

    let header = BinDctHeader {
      version: (),
      initial_hash_value: self.initial_hash_value,
      line_offset: 19,
      line_count: line_count as u32,
      unknown: (),
      footer_offset: (HEADER_SIZE + line_chunk_size - 1) as u32,
      footer_count: footer_count as u32,
    };
    header.write_le(writer)?;

    let mut text_offset_map = HashMap::<String, u64>::new();
    let mut cur_eof = (HEADER_SIZE + line_chunk_size + footer_chunk_size) as u64;

    let mut get_footer_offset_from_string = |text: &str| -> u64 {
      return match text_offset_map.get(text) {
        None => {
          let text_offset = cur_eof;

          text_offset_map.insert(text.to_string(), text_offset);
          cur_eof += (text.len() + 1) as u64; // include null-byte

          text_offset
        }
        Some(text_offset) => *text_offset,
      };
    };

    for line_entry in &self.line_entries {
      if line_entry.line_id == 0 {
        BinDctLineEntry {
          line_id: 0,
          text_offset: None,
          unknown: (),
        }
        .write_le(writer)?;

        continue;
      }

      let text = line_entry
        .text
        .as_ref()
        .expect("line_id is not 0 but text is none");
      let text_offset = get_footer_offset_from_string(&text);

      let bin_line_entry = BinDctLineEntry {
        line_id: line_entry.line_id,
        text_offset: Some(text_offset),
        unknown: (),
      };

      bin_line_entry.write_le(writer)?;
    }

    for footer_entry in &self.footer_entries {
      let text_offset = get_footer_offset_from_string(&footer_entry.text);

      let bin_sub_entries: Vec<_> = footer_entry
        .sub_entries
        .iter()
        .map(|sub_entry| {
          let text_offset = get_footer_offset_from_string(&sub_entry.text);
          BinDctFooterSubEntry {
            text_offset,
            to_map_to: sub_entry.to_map_to,
          }
        })
        .collect();

      let bin_footer_entry = BinDctFooterEntry {
        text_offset,
        sub_entries: bin_sub_entries,
        unknown0: (),
        unknown1: (),
        unknown2: (),
        unknown3: (),
      };

      bin_footer_entry.write_le(writer)?;
    }

    // sort strings by offset to write them in the correct order
    let mut string_to_write: Vec<_> = text_offset_map.iter().collect();
    string_to_write.sort_by(|(_, a), (_, b)| a.cmp(b));

    for (text, _) in string_to_write {
      NullString::from(text.as_str()).write_le(writer)?;
    }

    Ok(())
  }

  fn mod_entry_lookup(&self, hashed_key: u32) -> Option<usize> {
    let dct_capacity = self.line_entries.len();

    let mut position_guess = hashed_key as usize % dct_capacity;
    for _ in 0..dct_capacity {
      let line_entry_guess = &self.line_entries[position_guess];

      if line_entry_guess.line_id == hashed_key || line_entry_guess.line_id == 0 {
        return Some(position_guess);
      }

      position_guess = (position_guess + 1) % dct_capacity;
    }

    None
  }

  pub fn get_line_entry(&self, key: &str) -> Result<&str, DctLineError> {
    let hashed_key = lookup2(key.as_bytes(), self.initial_hash_value);

    return match self.mod_entry_lookup(hashed_key) {
      None => Err(KeyDoesNotExist(key.to_string())),
      Some(entry_index) => {
        let entry = &self.line_entries[entry_index];

        if entry.line_id == 0 {
          return Err(KeyDoesNotExist(key.to_string()));
        }

        Ok(entry.text.as_ref().unwrap())
      }
    };
  }

  pub fn add_line_entry(&mut self, key: &str, text: &str) -> Result<(), DctLineError> {
    let hashed_key = lookup2(key.as_bytes(), self.initial_hash_value);

    return match self.mod_entry_lookup(hashed_key) {
      None => Err(CapacityExceeded),
      Some(entry_index) => {
        let entry = &mut self.line_entries[entry_index];

        if entry.line_id == hashed_key {
          return Err(KeyAlreadyExists(key.to_string()));
        }

        entry.line_id = hashed_key;
        entry.text = Some(text.to_string());

        Ok(())
      }
    };
  }

  pub fn get_initial_hash_value(&self) -> u32 {
    self.initial_hash_value
  }

  pub fn get_max_capacity(&self) -> u32 {
    self.line_entries.len() as u32
  }

  pub fn get_current_capacity(&self) -> u32 {
    self
      .line_entries
      .iter()
      .filter(|entry| entry.line_id != 0)
      .count() as u32
  }

  pub fn iter_line_entries(&self) -> DctLineEntryIterator {
    DctLineEntryIterator {
      dct_map: self,
      index: 0,
    }
  }
}

pub struct DctLineEntryIterator<'a> {
  dct_map: &'a DctMap,
  index: usize,
}

impl<'a> Iterator for DctLineEntryIterator<'a> {
  type Item = (u32, &'a str);

  fn next(&mut self) -> Option<Self::Item> {
    let capacity_map = self.dct_map.line_entries.len();
    if capacity_map == 0 {
      return None;
    }

    loop {
      if self.index == capacity_map - 1 {
        return None;
      }

      let entry = &self.dct_map.line_entries[self.index];
      self.index += 1;
      if entry.line_id != 0 {
        return Some((entry.line_id.clone(), &entry.text.as_ref().unwrap()));
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::dct_map::DctMap;
  use jenkins_hash::lookup2;

  #[test]
  fn test_add_and_get() {
    let mut dct_map = DctMap::new(0xDEADBEEF, 30, Vec::new());

    dct_map
      .add_line_entry("wow", "cool")
      .expect("Could not add line entry.");

    assert_eq!(
      dct_map
        .get_line_entry("wow")
        .expect("Could not get line entry."),
      "cool"
    );

    assert_eq!(dct_map.get_max_capacity(), 30);
    assert_eq!(dct_map.get_current_capacity(), 1);
  }

  #[test]
  fn test_iterator() {
    const INITIAL_HASH_VALUE: u32 = 0x1FEDBEEF;
    let mut dct_map = DctMap::new(INITIAL_HASH_VALUE, 30, vec![]);

    dct_map.add_line_entry("key1", "test1").unwrap();
    dct_map.add_line_entry("key2", "test2").unwrap();

    let iter_res: Vec<_> = dct_map.iter_line_entries().collect();
    assert!(iter_res.contains(&(lookup2("key1".as_bytes(), INITIAL_HASH_VALUE), "test1")));
    assert!(iter_res.contains(&(lookup2("key2".as_bytes(), INITIAL_HASH_VALUE), "test2")));
  }
}
