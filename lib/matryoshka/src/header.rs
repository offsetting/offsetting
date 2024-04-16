use binrw::{BinRead, BinWrite};

#[derive(BinRead, BinWrite, Debug)]
pub(crate) struct OctHeader {
  #[brw(pad_before = 4)]
  pub(crate) string_table_size: u32,
  pub(crate) data_tree_size: u32,
}
