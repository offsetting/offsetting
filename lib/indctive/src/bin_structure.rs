use crate::bin_parser_writer::*;
use binrw::{binrw, BinRead, BinWrite};

pub const HEADER_SIZE: usize = 8 * 4;
pub const LINE_ENTRY_SIZE: usize = 3 * 4;
pub const FOOTER_ENTRY_SIZE: usize = 6 * 4;
pub const FOOTER_SUB_ENTRY_SIZE: usize = 2 * 4;

#[derive(Debug, BinRead, BinWrite)]
#[brw(magic = b"DICT")]
#[allow(dead_code)]
pub struct BinDctHeader {
  #[brw(magic = 0x2000u32)]
  pub version: (),
  pub initial_hash_value: u32,
  pub line_offset: u32,
  pub line_count: u32,
  #[brw(magic = 1u32)]
  pub unknown: (),
  pub footer_offset: u32,
  pub footer_count: u32,
}

#[binrw]
#[derive(Debug)]
#[allow(dead_code)]
pub struct BinDctLineEntry {
  pub line_id: u32,

  #[br(parse_with = absolute_offset_option_parser)]
  #[bw(write_with = relative_offset_option_writer)]
  pub text_offset: Option<u64>,

  #[brw(magic = 0u32)]
  pub unknown: (),
}

#[binrw]
#[allow(dead_code)]
pub struct BinDctFooterEntry {
  #[br(parse_with = absolute_offset_parser)]
  #[bw(write_with = relative_offset_writer)]
  pub text_offset: u64,

  #[bw(try_calc(sub_entries.len().try_into()))]
  pub amount_sub_entries: u32,
  #[br(args { count: amount_sub_entries as usize })]
  pub sub_entries: Vec<BinDctFooterSubEntry>,

  #[brw(magic = 0xFFFFFFDFu32)]
  pub unknown0: (),
  #[brw(magic = 11u32)]
  pub unknown1: (),
  #[brw(magic = 12u32)]
  pub unknown2: (),
  #[brw(magic = 0u32)]
  pub unknown3: (),
}

#[derive(Debug, BinRead, BinWrite)]
#[allow(dead_code)]
pub struct BinDctFooterSubEntry {
  #[br(parse_with = absolute_offset_parser)]
  #[bw(write_with = relative_offset_writer)]
  pub text_offset: u64,

  pub to_map_to: u32,
}
