use anyhow::anyhow;
pub use binrw::Endian;
use binrw::{BinReaderExt, BinWriterExt, NullString};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use uuid::Uuid;

use crate::header::OctHeader;
use crate::node::{Node, NodeData, RawNode};

mod header;
mod node;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContainerData {
  Single(Data),
  Multiple(Vec<Data>),
}

fn deserialize_f64_null_as_nan<'de, D: Deserializer<'de>>(des: D) -> Result<f32, D::Error> {
  let optional = Option::<f32>::deserialize(des)?;
  Ok(optional.unwrap_or(f32::NAN))
}

fn deserialize_vec_f64_null_as_nan<'de, D: Deserializer<'de>>(
  des: D,
) -> Result<Vec<f32>, D::Error> {
  Ok(
    Vec::<Option<f32>>::deserialize(des)?
      .iter()
      .map(|val| val.unwrap_or(f32::NAN))
      .collect(),
  )
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Data {
  Container(IndexMap<String, ContainerData>),

  Binary(#[serde(with = "base64")] Vec<u8>),
  Uuid(Uuid),

  Int(i32),
  IntVec(Vec<i32>),

  Float(#[serde(deserialize_with = "deserialize_f64_null_as_nan")] f32),
  FloatVec(#[serde(deserialize_with = "deserialize_vec_f64_null_as_nan")] Vec<f32>),

  String(String),
  StringVec(Vec<String>),
}

mod base64 {
  use base64::{engine::general_purpose, Engine as _};
  use serde::{Deserialize, Serialize};
  use serde::{Deserializer, Serializer};

  pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
    let base64 = general_purpose::STANDARD_NO_PAD.encode(v);
    String::serialize(&format!("base64:{base64}"), s)
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let base64 = String::deserialize(d)?;
    match base64.strip_prefix("base64:") {
      None => Err(serde::de::Error::custom("missing \"base64:\" prefix")),
      Some(base64) => general_purpose::STANDARD_NO_PAD
        .decode(base64.as_bytes())
        .map_err(serde::de::Error::custom),
    }
  }
}

pub fn decode<R: Read + Seek>(
  read: &mut R,
) -> anyhow::Result<(IndexMap<String, ContainerData>, Endian)> {
  let mut magic: [u8; 8] = [0u8; 8];
  read.read_exact(&mut magic)?;

  let endian = match magic {
    [0x29, 0x76, 0x01, 0x45, 0xcd, 0xcc, 0x8c, 0x3f] => Endian::Little,
    [0x45, 0x01, 0x76, 0x29, 0x3f, 0x8c, 0xcc, 0xcd] => Endian::Big,
    _ => return Err(anyhow!("Invalid magic: {magic:x?}")),
  };

  let header: OctHeader = read.read_type(endian)?;

  // 40 byte padding
  read.seek(SeekFrom::Current(40))?;

  let start = read.stream_position()?;
  let mut string_table = Vec::new();
  while (read.stream_position()? - start) < header.string_table_size as u64 {
    let null_string: NullString = read.read_type(endian)?;
    string_table.push(null_string.to_string());
  }

  let start = read.stream_position()?;

  let RawNode { level, node } = read.read_type_args(endian, string_table.as_slice())?;

  let root_level = level;
  let mut root_node = node;

  while (read.stream_position()? - start) < header.data_tree_size as u64 {
    let RawNode { level, node } = read.read_type_args(endian, string_table.as_slice())?;

    let mut curr_level = root_level;
    let mut curr_node = &mut root_node;

    while curr_level < level {
      curr_level += 1;
      let nodes = if let NodeData::Container(children) = &mut curr_node.data {
        children
      } else {
        return Err(anyhow!("Expected container"));
      };

      if curr_level == level {
        nodes.push(node);
        break;
      } else {
        curr_node = nodes.last_mut().unwrap();
      }
    }
  }

  if let Data::Container(children) = root_node.data.try_into()? {
    Ok((children, endian))
  } else {
    Err(anyhow!("Expected root node to be an container"))
  }
}

pub fn encode<R: Write + Seek>(
  write: &mut R,
  data: IndexMap<String, ContainerData>,
  endian: Endian,
) -> anyhow::Result<()> {
  let mut nodes = Vec::new();
  nodes.push(RawNode {
    level: 0,
    node: Node {
      id: "".to_string(),
      data: NodeData::Container(vec![]),
    },
  });
  extract_nodes(&mut nodes, data, 1);

  let mut strings = IndexSet::new();
  for RawNode { node, .. } in &nodes {
    if let Some((key, name)) = node.id.split_once('#') {
      strings.insert(key.to_string());
      strings.insert(name.to_string());
    } else {
      strings.insert(node.id.to_string());
    }

    match &node.data {
      NodeData::String(data) => {
        strings.insert(data.to_string());
      }
      NodeData::StringVec(data) => {
        for x in data {
          strings.insert(x.to_string());
        }
      }

      // TODO: binary file name hint
      NodeData::Binary(_) => {}
      _ => {}
    }
  }

  let strings = strings.into_iter().collect::<Vec<_>>();

  write.write_type(
    match endian {
      Endian::Big => b"\x45\x01\x76\x29\x3f\x8c\xcc\xcd",
      Endian::Little => b"\x29\x76\x01\x45\xCD\xCC\x8C\x3F",
    },
    endian,
  )?;

  // 12 byte header + 40 byte padding
  write.seek(SeekFrom::Current(12 + 40))?;

  let start = write.stream_position()?;
  for x in &strings {
    write.write_type(&NullString::from(x.as_str()), endian)?;
  }
  let string_size = write.stream_position()? - start;

  let start = write.stream_position()?;
  for x in &nodes {
    write.write_type_args(x, endian, strings.as_slice())?;
  }
  let node_size = write.stream_position()? - start;

  write.seek(SeekFrom::Start(8))?;
  write.write_type(
    &OctHeader {
      string_table_size: string_size as u32,
      data_tree_size: node_size as u32,
    },
    endian,
  )?;

  Ok(())
}

fn extract_nodes(nodes: &mut Vec<RawNode>, data: IndexMap<String, ContainerData>, level: u8) {
  for (id, node) in data {
    let n = match node {
      ContainerData::Single(x) => vec![x],
      ContainerData::Multiple(x) => x,
    };

    for node in n {
      if let Data::Container(childs) = node {
        nodes.push(RawNode {
          level,
          node: Node {
            id: id.clone(),
            data: NodeData::Container(vec![]),
          },
        });
        extract_nodes(nodes, childs, level + 1);
      } else {
        nodes.push(RawNode {
          level,
          node: Node {
            id: id.clone(),
            data: match node {
              Data::Container(..) => unreachable!(),
              Data::String(data) => NodeData::String(data),
              Data::StringVec(data) => NodeData::StringVec(data),
              Data::Float(data) => NodeData::Float(data),
              Data::FloatVec(data) => NodeData::FloatVec(data),
              Data::Int(data) => NodeData::Int(data),
              Data::IntVec(data) => NodeData::IntVec(data),
              Data::Binary(data) => NodeData::Binary(data),
              Data::Uuid(data) => NodeData::Uuid(data),
            },
          },
        });
      }
    }
  }
}

impl TryFrom<NodeData> for Data {
  type Error = anyhow::Error;

  fn try_from(node_data: NodeData) -> Result<Self, Self::Error> {
    Ok(match node_data {
      NodeData::Container(child) => {
        let mut childs = IndexMap::new();
        for node in child {
          if childs.contains_key(&node.id) {
            let data = childs.swap_remove(&node.id).unwrap(); // TODO maybe shift remove
            match data {
              ContainerData::Single(first) => childs.insert(
                node.id,
                ContainerData::Multiple(vec![first, node.data.try_into()?]),
              ),
              ContainerData::Multiple(mut list) => {
                list.push(node.data.try_into()?);
                childs.insert(node.id, ContainerData::Multiple(list))
              }
            };
          } else {
            childs.insert(node.id, ContainerData::Single(node.data.try_into()?));
          }
        }
        Data::Container(childs)
      }
      NodeData::String(str) => Data::String(str),
      NodeData::StringVec(str_vec) => Data::StringVec(str_vec),
      NodeData::Float(str_vec) => Data::Float(str_vec),
      NodeData::FloatVec(str_vec) => Data::FloatVec(str_vec),
      NodeData::Int(str_vec) => Data::Int(str_vec),
      NodeData::IntVec(str_vec) => Data::IntVec(str_vec),
      NodeData::Binary(str_vec) => Data::Binary(str_vec),
      NodeData::Uuid(uuid) => Data::Uuid(uuid),
    })
  }
}

impl From<Data> for NodeData {
  fn from(value: Data) -> Self {
    match value {
      Data::Container(child) => {
        let mut childs = Vec::with_capacity(child.len());
        for (id, data) in child {
          let n = match data {
            ContainerData::Single(x) => vec![x],
            ContainerData::Multiple(x) => x,
          };
          for data in n {
            childs.push(Node {
              id: id.clone(),
              data: data.into(),
            });
          }
        }
        NodeData::Container(childs)
      }
      Data::String(data) => NodeData::String(data),
      Data::StringVec(data) => NodeData::StringVec(data),
      Data::Float(data) => NodeData::Float(data),
      Data::FloatVec(data) => NodeData::FloatVec(data),
      Data::Int(data) => NodeData::Int(data),
      Data::IntVec(data) => NodeData::IntVec(data),
      Data::Binary(data) => NodeData::Binary(data),
      Data::Uuid(data) => NodeData::Uuid(data),
    }
  }
}
