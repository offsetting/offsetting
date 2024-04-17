use binrw::{BinResult, Endian};
use std::io::{Read, Seek, Write};

fn parse_absolute_offset<R: Read + Seek>(reader: &mut R, endian: Endian) -> BinResult<Option<u64>> {
  let stream_pos: u64 = reader.stream_position()?;

  let mut byte_array = [0u8; 4];
  reader.read(&mut byte_array[..])?;

  let relative_offset = match endian {
    Endian::Big => u32::from_be_bytes(byte_array),
    Endian::Little => u32::from_le_bytes(byte_array),
  };

  // empty line entries have relative offset 0
  if relative_offset == 0 {
    return Ok(None);
  }

  let absolute_offset = relative_offset as u64 + stream_pos + 1;
  Ok(Some(absolute_offset))
}

#[binrw::parser(reader, endian)]
pub(crate) fn absolute_offset_parser() -> BinResult<u64> {
  Ok(parse_absolute_offset(reader, endian)?.unwrap_or(0))
}

#[binrw::parser(reader, endian)]
pub(crate) fn absolute_offset_option_parser() -> BinResult<Option<u64>> {
  parse_absolute_offset(reader, endian)
}

fn write_relative_offset<R: Write + Seek>(
  writer: &mut R,
  endian: Endian,
  absolute_offset: &Option<u64>,
) -> BinResult<()> {
  let stream_pos: u64 = writer.stream_position()?;

  let relative_offset = match absolute_offset {
    None => 0u64,
    Some(absolute_offset) => *absolute_offset - stream_pos - 1,
  };

  let byte_array = match endian {
    Endian::Big => (relative_offset as u32).to_be_bytes(),
    Endian::Little => (relative_offset as u32).to_le_bytes(),
  };

  writer.write(&byte_array)?;
  Ok(())
}

#[binrw::writer(writer, endian)]
pub(crate) fn relative_offset_writer(absolute_offset: &u64) -> BinResult<()> {
  write_relative_offset(writer, endian, &Some(*absolute_offset))
}

#[binrw::writer(writer, endian)]
pub(crate) fn relative_offset_option_writer(absolute_offset: &Option<u64>) -> BinResult<()> {
  write_relative_offset(writer, endian, absolute_offset)
}
