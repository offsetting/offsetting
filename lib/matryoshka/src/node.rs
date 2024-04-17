use std::io::{Read, Seek, Write};

use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, Endian, Error};
use modular_bitfield::prelude::*;
use uuid::{Bytes, Uuid};

#[derive(Debug)]
pub(crate) struct Node {
  pub(crate) id: String,
  pub(crate) data: NodeData,
}

#[derive(Debug)]
pub(crate) enum NodeData {
  Container(Vec<Node>),

  String(String),
  StringVec(Vec<String>),

  Float(f32),
  FloatVec(Vec<f32>),

  Int(i32),
  IntVec(Vec<i32>),

  Uuid(Uuid),
  Binary(Vec<u8>),
}

#[bitfield]
#[repr(u16)]
struct NodeHeader {
  r#type: Type,
  name: bool,
  data_type: DataType,
  // always +1
  len_size: B2,
  // always +1
  int_size: B2,
  level: B6,
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 2]
enum Type {
  None,
  Container,
  Vec,
  Scalar,
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 3]
enum DataType {
  None,
  String,
  Float,
  Int,
  Binary,
}

pub(crate) struct RawNode {
  pub(crate) level: u8,
  pub(crate) node: Node,
}

impl BinRead for RawNode {
  type Args<'a> = &'a [String];

  fn read_options<R: Read + Seek>(
    reader: &mut R,
    endian: Endian,
    args: Self::Args<'_>,
  ) -> BinResult<Self> {
    let header_data: u16 = reader.read_type(endian)?;
    let header = NodeHeader::from(header_data);

    let key_idx: u16 = reader.read_type(endian)?;
    let key = args[key_idx as usize].clone();

    let name = if header.name() {
      let name_idx: u16 = reader.read_type(endian)?;
      Some(args[name_idx as usize].clone())
    } else {
      None
    };

    let level = header.level();

    let len_size = header.len_size() as usize + 1;
    let int_site = header.int_size() as usize + 1;

    let node = Node {
      id: match name {
        Some(name) => format!("{}#{}", key.clone(), name),
        None => key.clone(),
      },
      data: match (header.data_type(), header.r#type()) {
        (DataType::None, Type::Container) => NodeData::Container(vec![]),

        (DataType::String, Type::Scalar) => NodeData::String({
          let idx: u16 = reader.read_type(endian)?;
          args[idx as usize].clone()
        }),
        (DataType::String, Type::Vec) => NodeData::StringVec({
          let len = read_u32(reader, endian, len_size)? as usize;
          let mut vec = Vec::with_capacity(len);
          for _ in 0..len {
            let idx: u16 = reader.read_type(endian)?;
            vec.push(args[idx as usize].clone());
          }
          vec
        }),

        (DataType::Float, Type::Scalar) => NodeData::Float(reader.read_type(endian)?),
        (DataType::Float, Type::Vec) => NodeData::FloatVec({
          let len = read_u32(reader, endian, len_size)? as usize;
          let mut vec = Vec::with_capacity(len);
          for _ in 0..len {
            vec.push(reader.read_type(endian)?);
          }
          vec
        }),
        (DataType::Int, Type::Scalar) => NodeData::Int(read_i32(reader, endian, int_site)?),
        (DataType::Int, Type::Vec) => NodeData::IntVec({
          let len = read_u32(reader, endian, len_size)? as usize;
          let mut vec = Vec::with_capacity(len);
          for _ in 0..len {
            vec.push(read_i32(reader, endian, int_site)?);
          }
          vec
        }),

        (DataType::Binary, Type::Scalar) => {
          assert_ne!(
            int_site, 0,
            "Binary file name hint index is set, todo: impl me"
          );

          let len = read_u32(reader, endian, len_size)? as usize;
          let mut vec = Vec::with_capacity(len);
          for _ in 0..len {
            vec.push(reader.read_type(endian)?);
          }

          // special case, uuids are encoded as binary
          if len == 16 && &key == "Uuid" {
            let mut bytes: Bytes = [0; 16];
            bytes.copy_from_slice(vec.as_slice());

            let uuid = match endian {
              Endian::Big => Uuid::from_bytes(bytes),
              Endian::Little => Uuid::from_bytes_le(bytes),
            };

            NodeData::Uuid(uuid)
          } else {
            NodeData::Binary(vec)
          }
        }

        x => unimplemented!("{:?}", x),
      },
    };

    Ok(RawNode { level, node })
  }
}

fn find(strings: &[String], string: &str) -> u16 {
  for (i, str) in strings.iter().enumerate() {
    if str == string {
      return i as u16;
    }
  }

  panic!("Can't find string");
}

const fn get_u32_size(i: u32) -> u8 {
  let actual_bits = 32 - i.leading_zeros();

  let bytes_used = actual_bits / 8;
  let bits_remaining = actual_bits % 8;

  (if bits_remaining > 0 {
    bytes_used + 1
  } else if i == 0 {
    1
  } else {
    bytes_used
  }) as u8
}

const fn get_i32_size(i: i32) -> u8 {
  let actual_bits = 32 - i.abs().leading_zeros() + 1;
  // +1 because of signing bit

  let bytes_used = actual_bits / 8;
  let bits_remaining = actual_bits % 8;

  (if bits_remaining > 0 {
    bytes_used + 1
  } else {
    bytes_used
  }) as u8
}

#[cfg(test)]
mod tests {
  use crate::node::{get_i32_size, get_u32_size};

  #[test]
  fn test() {
    assert_eq!(get_u32_size(0), 1);
    assert_eq!(get_u32_size(10), 1);
    assert_eq!(get_u32_size(u8::MAX as u32), 1);
    assert_eq!(get_u32_size(u8::MAX as u32 + 1), 2);
    assert_eq!(get_u32_size(u16::MAX as u32), 2);
    assert_eq!(get_u32_size(u16::MAX as u32 + 1), 3);

    assert_eq!(get_i32_size(291), 2);
  }
}

impl BinWrite for RawNode {
  type Args<'a> = &'a [String];

  fn write_options<W: Write + Seek>(
    &self,
    writer: &mut W,
    endian: Endian,
    args: Self::Args<'_>,
  ) -> BinResult<()> {
    let mut len_size = 1;
    let mut int_size = 1;

    let (data_type, r#type) = match &self.node.data {
      NodeData::Container(_) => (DataType::None, Type::Container),
      NodeData::String(_) => (DataType::String, Type::Scalar),
      NodeData::StringVec(data) => {
        let len = data.len();
        len_size = get_u32_size(len as u32);
        (DataType::String, Type::Vec)
      }
      NodeData::Float(_) => (DataType::Float, Type::Scalar),
      NodeData::FloatVec(data) => {
        let len = data.len();
        len_size = get_u32_size(len as u32);
        (DataType::Float, Type::Vec)
      }
      NodeData::Int(data) => {
        int_size = get_i32_size(*data);
        (DataType::Int, Type::Scalar)
      }
      NodeData::IntVec(data) => {
        let len = data.len();
        len_size = get_u32_size(len as u32);
        int_size = data.iter().map(|x| get_i32_size(*x)).max().unwrap_or(1);
        (DataType::Int, Type::Vec)
      }
      NodeData::Binary(data) => {
        let len = data.len();
        len_size = get_u32_size(len as u32);
        (DataType::Binary, Type::Scalar)
      }
      NodeData::Uuid(_) => (DataType::Binary, Type::Scalar),
    };

    let key;
    let name;

    if let Some((k, n)) = self.node.id.split_once('#') {
      key = find(args, k);
      name = Some(find(args, n));
    } else {
      key = find(args, &self.node.id);
      name = None;
    }

    let mut header = NodeHeader::new();
    header.set_type(r#type);
    header.set_name(name.is_some());
    header.set_data_type(data_type);
    header.set_len_size(len_size - 1);
    header.set_int_size(int_size - 1);
    header.set_level(self.level);

    let header: u16 = header.into();

    writer.write_type(&header, endian)?;
    writer.write_type(&key, endian)?;
    if let Some(name) = name {
      writer.write_type(&name, endian)?;
    }

    match &self.node.data {
      NodeData::Container(_) => {}
      NodeData::String(data) => writer.write_type(&find(args, data), endian)?,
      NodeData::StringVec(data) => {
        write_u32(writer, data.len() as u32, endian, len_size as usize)?;
        for x in data {
          writer.write_type(&find(args, x), endian)?;
        }
      }
      NodeData::Float(data) => writer.write_type(data, endian)?,
      NodeData::FloatVec(data) => {
        write_u32(writer, data.len() as u32, endian, len_size as usize)?;
        for x in data {
          writer.write_type(x, endian)?;
        }
      }
      NodeData::Int(data) => {
        write_i32(writer, *data, endian, int_size as usize)?;
      }
      NodeData::IntVec(data) => {
        write_u32(writer, data.len() as u32, endian, len_size as usize)?;
        for x in data {
          write_i32(writer, *x, endian, int_size as usize)?;
        }
      }
      NodeData::Binary(data) => {
        write_u32(writer, data.len() as u32, endian, len_size as usize)?;
        // TODO: binary file name hint
        for x in data {
          writer.write_type(x, endian)?;
        }
      }
      NodeData::Uuid(uuid) => {
        writer.write_type(&16u8, endian)?;
        let bytes = match endian {
          Endian::Big => *uuid.as_bytes(),
          Endian::Little => uuid.to_bytes_le(),
        };
        writer.write_all(&bytes)?;
      }
    };

    Ok(())
  }
}

fn read_u32<R: Read + Seek>(reader: &mut R, endian: Endian, len: usize) -> BinResult<u32> {
  if len > 4 {
    return Err(Error::AssertFail {
      pos: reader.stream_position()?,
      message: "Len may not be greater than 4.".to_string(),
    });
  }

  let mut buf = [0u8; 4];
  Ok(match endian {
    Endian::Big => {
      reader.read_exact(&mut buf[4 - len..])?;
      u32::from_be_bytes(buf)
    }
    Endian::Little => {
      reader.read_exact(&mut buf[..len])?;
      u32::from_le_bytes(buf)
    }
  })
}

fn write_i32<W: Write + Seek>(
  write: &mut W,
  data: i32,
  endian: Endian,
  len: usize,
) -> BinResult<()> {
  match endian {
    Endian::Big => {
      let buf = data.to_be_bytes();
      for byte in buf.iter().skip(4 - len) {
        write.write_be(byte)?;
      }
    }
    Endian::Little => {
      let buf = data.to_le_bytes();
      for byte in buf.iter().take(len) {
        write.write_le(byte)?
      }
    }
  }

  Ok(())
}

// TODO: fix endian

fn write_u32<W: Write + Seek>(
  write: &mut W,
  data: u32,
  endian: Endian,
  len: usize,
) -> BinResult<()> {
  match endian {
    Endian::Big => {
      let buf = data.to_be_bytes();
      for item in buf.iter().skip(4 - len) {
        write.write_be(item)?;
      }
    }
    Endian::Little => {
      let buf = data.to_le_bytes();
      for item in buf.iter().take(len) {
        write.write_le(item)?
      }
    }
  }

  Ok(())
}

fn read_i32<R: Read + Seek>(reader: &mut R, endian: Endian, len: usize) -> BinResult<i32> {
  let data = read_u32(reader, endian, len)?;

  //    1:                         00000001
  //    1: 00000000000000000000000000000001

  //  127:                         01111111
  //  127: 00000000000000000000000001111111

  //   -1:                         11111111
  //   -1: 11111111111111111111111111111111

  // -128:                         10000000
  // -128: 11111111111111111111111110000000

  let bit_size = len as u32 * 8;
  let neg_mask = 1 << (bit_size - 1);

  if data & neg_mask == neg_mask {
    let mask = u32::MAX ^ (neg_mask - 1);
    Ok((data | mask) as i32)
  } else {
    Ok(data as i32)
  }
}
