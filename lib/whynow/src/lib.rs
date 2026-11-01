use aes::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use binrw::{binrw, BinResult, BinWrite};
use flate2::{Compress, Compression, Crc, FlushCompress};
use rayon::prelude::*;
use md5::Digest;
use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn map_bytes_to_string(data: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
  String::from_utf8(data)
}

fn map_string_to_bytes(string: &String) -> &[u8] {
  string.as_bytes()
}

#[binrw]
#[brw(repr(u16))]
enum ZipCompressionType {
  CompStored = 0,
  CompShrunk = 1,
  CompReduced1 = 2,
  CompReduced2 = 3,
  CompReduced3 = 4,
  CompReduced4 = 5,
  CompImploded = 6,
  CompToken = 7,
  CompDeflate = 8,
  CompDeflate64 = 9,
}

#[binrw]
#[brw(little, magic = b"PK\x03\x04")]
struct ZipFileRecordHeader {
  pub version: u16,
  pub flags: u16,
  pub compression_type: ZipCompressionType,
  pub file_time: u16,
  pub file_date: u16,
  pub file_crc: u32,
  pub compressed_size: u32,
  pub uncompressed_size: u32,
  #[br(temp)]
  #[bw(calc = file_name.as_bytes().len() as u16)]
  file_name_length: u16,
  #[br(temp)]
  #[bw(calc = file_extra_field.len() as u16)]
  file_extra_field_length: u16,
  #[br(count = file_name_length, try_map = map_bytes_to_string)]
  #[bw(map = map_string_to_bytes)]
  pub file_name: String,
  #[br(count = file_extra_field_length)]
  pub file_extra_field: Vec<u8>,
}

#[binrw]
#[brw(little, magic = b"PK\x01\x02")]
struct ZipDirEntry {
  pub version_made_by: u16,
  pub version_to_extract: u16,
  pub flags: u16,
  pub compression_type: ZipCompressionType,
  pub file_time: u16,
  pub file_date: u16,
  pub file_crc: u32,
  pub compressed_size: u32,
  pub uncompressed_size: u32,
  #[br(temp)]
  #[bw(calc = file_name.as_bytes().len() as u16)]
  file_name_length: u16,
  #[br(temp)]
  #[bw(calc = file_extra_field.len() as u16)]
  file_extra_field_length: u16,
  #[br(temp)]
  #[bw(calc = file_comment.as_bytes().len() as u16)]
  file_comment_length: u16,
  pub disk_number_start: u16,
  pub internal_attributes: u16,
  pub external_attributes: u32,
  pub header_offset: u32,
  #[br(count = file_name_length, try_map = map_bytes_to_string)]
  #[bw(map = map_string_to_bytes)]
  pub file_name: String,
  #[br(count = file_extra_field_length)]
  pub file_extra_field: Vec<u8>,
  #[br(count = file_comment_length, try_map = map_bytes_to_string)]
  #[bw(map = map_string_to_bytes)]
  pub file_comment: String,
}

const ZIP_END_LOCATOR_SIZE: usize = 22;
const MD5_HEADER: [u8; 7] = [0x4B, 0x46, 0x13, 0x00, 0x4D, 0x44, 0x35];
const MD5_EXTRA_FIELD_SIZE: usize = MD5_HEADER.len() + 16;

#[binrw]
#[brw(little, magic = b"PK\x05\x06")]
struct ZipDirEndLocator {
  pub disk_number: u16,
  pub disk_start_number: u16,
  pub entries_on_disk: u16,
  pub entries_in_directory: u16,
  pub directory_size: u32,
  pub directory_offset: u32,
  #[br(temp)]
  #[bw(calc = comment.as_bytes().len() as u16)]
  comment_length: u16,
  #[br(count = comment_length, try_map = map_bytes_to_string)]
  #[bw(map = map_string_to_bytes)]
  pub comment: String,
}

#[binrw]
#[brw(little)]
struct OctaneZipEntry {
  pub name_mmh3: u32,
  pub header_offset: u32,
}

#[binrw]
#[brw(little, magic = b"PK\xFF\xFF")]
struct OctaneZipHeader {
  #[br(temp)]
  #[bw(calc = octane_zip_entries.len() as u32)]
  amount_octane_zip_entries: u32,
  #[br(count = amount_octane_zip_entries)]
  pub octane_zip_entries: Vec<OctaneZipEntry>,
}

fn get_all_file_paths(file_path: &Path) -> Vec<PathBuf> {
  WalkDir::new(file_path)
    .into_iter()
    .filter_map(|file_path| {
      file_path
        .map(|dir_entry| dir_entry.path().to_path_buf())
        .ok()
    })
    .filter(|file_path| !file_path.is_symlink() && !file_path.is_dir())
    .collect()
}

fn calculate_octane_zip_header_length(amount_files: usize) -> usize {
  4 + 4 + (4 + 4) * amount_files
}

fn calculate_zip_dir_entries_header_size(file_paths: &[String]) -> usize {
  (4 + 6 * 2 + 3 * 4 + 5 * 2 + 2 * 4 + MD5_EXTRA_FIELD_SIZE) * file_paths.len()
    + file_paths
      .iter()
      .map(|entry| entry.as_bytes().len())
      .sum::<usize>()
}

fn calculate_file_record_header_size(file_path: &str) -> usize {
  4 + 5 * 2 + 3 * 4 + 2 + 2 + file_path.len()
}

#[derive(Debug, Clone)]
pub struct FileInfo {
  pub header_offset: u32,
  pub uncompressed_size: u32,
  pub compressed_size: u32,
  pub file_crc: u32,
  pub md5_hash: [u8; 16],
  pub file_name: String,
}

// reads the entire contents of a reader into a buffer, returning the number of bytes read.
fn read_full<R: Read>(reader: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
  let mut total = 0;
  while total < buf.len() {
    let n = reader.read(&mut buf[total..])?;
    if n == 0 {
      break;
    }
    total += n;
  }
  Ok(total)
}

// returns the number of bytes left to read, leaving the reader's position unchanged.
fn remaining_len<R: Seek>(reader: &mut R) -> std::io::Result<u64> {
  let position = reader.stream_position()?;
  let end = reader.seek(SeekFrom::End(0))?;
  reader.seek(SeekFrom::Start(position))?;
  Ok(end - position)
}

// compresses a block of data using the DEFLATE algorithm and returns the compressed data along with its CRC32 checksum.
fn deflate_block(block: &[u8]) -> std::io::Result<(Vec<u8>, Crc)> {
  let mut compress = Compress::new(Compression::default(), false);
  let mut out = Vec::with_capacity(block.len() / 2 + 1024);
  let mut consumed = 0;

  loop {
    let before = compress.total_in();
    compress
      .compress_vec(&block[consumed..], &mut out, FlushCompress::Sync)
      .map_err(std::io::Error::other)?;
    consumed += (compress.total_in() - before) as usize;

    if consumed == block.len() && out.len() < out.capacity() {
      break;
    }
    out.reserve(4096);
  }

  let mut crc = Crc::new();
  crc.update(block);
  Ok((out, crc))
}

/// A whole file compressed in memory, ready to be written as a zip entry.
pub struct CompressedFileData {
  zip_file_name: String,
  compressed_bytes: Vec<u8>,
  compressed_size: u32,
  uncompressed_size: u32,
  file_crc: u32,
  md5_hash: [u8; 16],
}

// reads and compresses a whole file in memory. Doesn't touch the zip writer, so it can run on any thread.
fn compress_file_in_memory(path: &Path, zip_file_name: &str) -> anyhow::Result<CompressedFileData> {
  let data = std::fs::read(path)?;
  let (mut compressed_bytes, crc) = deflate_block(&data)?;
  // empty final block, same as the streaming path
  compressed_bytes.extend_from_slice(&[0x03, 0x00]);

  let mut md5_hash = [0u8; 16];
  md5_hash.copy_from_slice(&md5::Md5::digest(&data));

  Ok(CompressedFileData {
    zip_file_name: zip_file_name.to_string(),
    compressed_size: compressed_bytes.len() as u32,
    uncompressed_size: data.len() as u32,
    file_crc: crc.sum(),
    md5_hash,
    compressed_bytes,
  })
}

fn write_compressed_file_to_octane_zip<W: Write + Seek>(
  writer: &mut W,
  file_data: &CompressedFileData,
) -> BinResult<FileInfo> {
  let header_offset = writer.stream_position()?;

  ZipFileRecordHeader {
    version: 20,
    flags: 0,
    compression_type: ZipCompressionType::CompDeflate,
    file_time: 0xA1C3,
    file_date: 0x742F,
    file_crc: file_data.file_crc,
    compressed_size: file_data.compressed_size,
    uncompressed_size: file_data.uncompressed_size,
    file_name: file_data.zip_file_name.clone(),
    file_extra_field: Vec::new(),
  }
  .write(writer)?;
  writer.write_all(&file_data.compressed_bytes)?;

  Ok(FileInfo {
    header_offset: header_offset as u32,
    uncompressed_size: file_data.uncompressed_size,
    compressed_size: file_data.compressed_size,
    file_crc: file_data.file_crc,
    md5_hash: file_data.md5_hash,
    file_name: file_data.zip_file_name.clone(),
  })
}

fn write_file_to_octane_zip<W: Write + Seek, R: Read + Seek>(
  writer: &mut W,
  file_reader: &mut R,
  zip_file_path: &str,
) -> BinResult<FileInfo> {
  const DEFLATE_BLOCK_SIZE: usize = 4 * 1024 * 1024; // use 4MB blocks for deflate compression

  let header_offset = writer.stream_position()?;
  let file_record_size = calculate_file_record_header_size(zip_file_path);
  writer.seek(SeekFrom::Current(file_record_size as i64))?;

  let mut md5 = md5::Md5::new();
  let mut crc = Crc::new();
  let block_per_batch = rayon::current_num_threads();
  let mut remaining = remaining_len(file_reader)?;

  loop {
    // read per batch of blocks
    let mut blocks: Vec<Vec<u8>> = Vec::with_capacity(block_per_batch);
    for _ in 0..block_per_batch {
      // don't allocate a full 4MB block for small files
      let mut block = vec![0u8; remaining.min(DEFLATE_BLOCK_SIZE as u64) as usize];
      let bytes_read = read_full(file_reader, &mut block)?;
      remaining = remaining.saturating_sub(bytes_read as u64);
      block.truncate(bytes_read);
      if bytes_read > 0 {
        blocks.push(block);
      }
      if bytes_read < DEFLATE_BLOCK_SIZE {
        break;
      }
    }
    if blocks.is_empty() {
      break;
    }

    // MD5 update and deflate in parallel
    let (_, result) = rayon::join(
      || {
        for block in &blocks {
          md5.update(block);
        }
      },
      || {
        blocks
          .par_iter()
          .map(|block| deflate_block(block))
          .collect::<std::io::Result<Vec<_>>>()
      },
    );

    // write compressed blocks and update CRC
    for (compressed_block, block_crc) in result? {
      writer.write_all(&compressed_block)?;
      crc.combine(&block_crc);
    }
  }

  writer.write_all(&[0x03, 0x00])?;

  let file_end = writer.stream_position()?;
  writer.seek(SeekFrom::Start(header_offset))?;

  let uncompressed_size = file_reader.stream_position()? as u32;
  let compressed_size = (file_end - header_offset - file_record_size as u64) as u32;
  let mut md5_hash = [0u8; 16];
  md5_hash.copy_from_slice(&md5.finalize());
  let crc32 = crc.sum();

  ZipFileRecordHeader {
    version: 20,
    flags: 0,
    compression_type: ZipCompressionType::CompDeflate,
    file_time: 0xA1C3,
    file_date: 0x742F,
    file_crc: crc32,
    compressed_size,
    uncompressed_size,
    file_name: zip_file_path.to_string(),
    file_extra_field: Vec::new(),
  }
  .write(writer)?;

  writer.seek(SeekFrom::Start(file_end))?;

  Ok(FileInfo {
    header_offset: header_offset as u32,
    uncompressed_size,
    compressed_size,
    file_crc: crc32,
    md5_hash,
    file_name: zip_file_path.to_string(),
  })
}

fn write_zip_dir_entries<W: Write + Seek>(
  writer: &mut W,
  file_infos: &[FileInfo],
) -> BinResult<()> {
  for file_info in file_infos {
    let mut extra_field = Vec::from(MD5_HEADER);
    extra_field.extend_from_slice(&file_info.md5_hash);

    ZipDirEntry {
      version_made_by: 20,
      version_to_extract: 20,
      flags: 0,
      compression_type: ZipCompressionType::CompDeflate,
      file_time: 0xA1C3,
      file_date: 0x742F,
      file_crc: file_info.file_crc,
      compressed_size: file_info.compressed_size,
      uncompressed_size: file_info.uncompressed_size,
      disk_number_start: 0,
      internal_attributes: 0,
      external_attributes: 0,
      header_offset: file_info.header_offset,
      file_name: file_info.file_name.clone(),
      file_extra_field: extra_field,
      file_comment: "".to_string(),
    }
    .write(writer)?;
  }

  Ok(())
}

fn create_zip_end_locator(
  directory_start_offset: u32,
  directory_size: u32,
  amount_file_infos: u16,
) -> ZipDirEndLocator {
  ZipDirEndLocator {
    disk_number: 0,
    disk_start_number: 0,
    directory_offset: directory_start_offset,
    directory_size,
    entries_on_disk: amount_file_infos,
    entries_in_directory: amount_file_infos,
    comment: "".to_string(),
  }
}

pub trait ZipWriter {
  fn get_header_space(&mut self, file_paths: &[String]) -> usize;
  fn write_header<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_infos: &[FileInfo],
  ) -> BinResult<()>;
  fn write_file<W: Write + Seek, R: Read + Seek>(
    &mut self,
    writer: &mut W,
    reader: &mut R,
    zip_file_name: &str,
  ) -> BinResult<FileInfo> {
    write_file_to_octane_zip(writer, reader, &zip_file_name)
  }

  fn write_compressed_file<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_data: &CompressedFileData,
  ) -> BinResult<FileInfo> {
    write_compressed_file_to_octane_zip(writer, file_data)
  }

  fn write_footer<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_infos: &[FileInfo],
  ) -> BinResult<()> {
    let directory_start_offset = writer.stream_position()?;
    write_zip_dir_entries(writer, &file_infos)?;

    let directory_end_offset = writer.stream_position()?;
    create_zip_end_locator(
      directory_start_offset as u32,
      (directory_end_offset - directory_start_offset) as u32,
      file_infos.len() as u16,
    )
    .write(writer)?;

    Ok(())
  }
}

pub struct NewOctaneZipWriter;
impl ZipWriter for NewOctaneZipWriter {
  fn get_header_space(&mut self, file_paths: &[String]) -> usize {
    calculate_octane_zip_header_length(file_paths.len())
  }

  fn write_header<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_infos: &[FileInfo],
  ) -> BinResult<()> {
    let mut octane_zip_entries: Vec<_> = file_infos
      .iter()
      .map(|file_info| OctaneZipEntry {
        name_mmh3: murmur3::murmur3_32(&mut Cursor::new(&file_info.file_name), 0).unwrap(),
        header_offset: file_info.header_offset,
      })
      .collect();
    octane_zip_entries.sort_by(|a, b| a.name_mmh3.cmp(&b.name_mmh3));

    OctaneZipHeader { octane_zip_entries }.write(writer)?;

    Ok(())
  }
}

type Aes128CtrCipher = ctr::Ctr128BE<aes::Aes128>;

struct EncryptedWriter<'a, W: Write + Seek> {
  pub cipher: Aes128CtrCipher,
  pub writer: &'a mut W,
  /// If the cipher reaches this position (ctr value) it won't encrypt anymore.
  /// Required because only the first 0x200 bytes of each file get encrypted.
  pub cipher_disable_position: Option<u64>,
}

impl<W: Write + Seek> EncryptedWriter<'_, W> {
  pub fn reset_cipher_counter(&mut self) {
    self.cipher.seek(0);
  }
}

impl<W: Write + Seek> Write for EncryptedWriter<'_, W> {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    let mut encryption_buffer = Vec::from(buf);

    if let Some(cipher_disable_position) = self.cipher_disable_position {
      let current_cipher_pos = self.cipher.current_pos::<u64>();
      let bytes_left_to_encrypt =
        cipher_disable_position - current_cipher_pos.min(cipher_disable_position);
      let amount_bytes_to_encrypt = (bytes_left_to_encrypt as usize).min(encryption_buffer.len());

      self
        .cipher
        .apply_keystream(&mut encryption_buffer[..amount_bytes_to_encrypt]);

      // synchronize cipher with file even if we won't use it anymore
      // otherwise seeking won't work
      self.cipher.seek(current_cipher_pos + buf.len() as u64);
    } else {
      self.cipher.apply_keystream(&mut encryption_buffer);
    }

    self.writer.write(&encryption_buffer)
  }

  fn flush(&mut self) -> std::io::Result<()> {
    self.writer.flush()
  }
}

impl<S: Write + Seek> Seek for EncryptedWriter<'_, S> {
  fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
    let old_file_pos = self.writer.stream_position()? as i64;
    let new_file_pos = self.writer.seek(pos)? as i64;

    let new_cipher_pos = self.cipher.current_pos::<u64>() as i64 + (new_file_pos - old_file_pos);

    self.cipher.try_seek(new_cipher_pos as u64).map_err(|_| {
      // restoring old file stream position on failure
      self
        .writer
        .seek(SeekFrom::Start(old_file_pos as u64))
        .err()
        .map(|error| error.kind())
        .unwrap_or(std::io::ErrorKind::NotSeekable)
    })?;

    Ok(new_file_pos as u64)
  }
}

pub struct EncryptedNewOctaneZipWriter<'a> {
  pub key: &'a [u8],
}

impl EncryptedNewOctaneZipWriter<'_> {
  fn create_cipher(&self) -> Aes128CtrCipher {
    Aes128CtrCipher::new_from_slices(self.key, &[0x00; 16]).unwrap()
  }

  fn create_encrypted_writer_with_disable_position<'a, W: Write + Seek>(
    &self,
    writer: &'a mut W,
    cipher_disable_position: Option<u64>,
  ) -> EncryptedWriter<'a, W> {
    let cipher = self.create_cipher();

    EncryptedWriter {
      cipher,
      writer,
      cipher_disable_position,
    }
  }

  fn create_encrypted_writer<'a, W: Write + Seek>(
    &self,
    writer: &'a mut W,
  ) -> EncryptedWriter<'a, W> {
    self.create_encrypted_writer_with_disable_position(writer, None)
  }

  fn create_file_writer<'a, W: Write + Seek>(
    &self,
    writer: &'a mut W,
    zip_file_name: &str,
  ) -> EncryptedWriter<'a, W> {
    if !zip_file_name.to_lowercase().ends_with(".dct") {
      self.create_encrypted_writer_with_disable_position(
        writer,
        Some(0x200 + calculate_file_record_header_size(zip_file_name) as u64),
      )
    } else {
      self.create_encrypted_writer(writer)
    }
  }
}

impl ZipWriter for EncryptedNewOctaneZipWriter<'_> {
  fn get_header_space(&mut self, file_paths: &[String]) -> usize {
    NewOctaneZipWriter.get_header_space(file_paths)
  }

  fn write_header<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_infos: &[FileInfo],
  ) -> BinResult<()> {
    NewOctaneZipWriter.write_header(&mut self.create_encrypted_writer(writer), file_infos)
  }

  fn write_file<W: Write + Seek, R: Read + Seek>(
    &mut self,
    writer: &mut W,
    reader: &mut R,
    zip_file_name: &str,
  ) -> BinResult<FileInfo> {
    let mut encrypted_writer = self.create_file_writer(writer, zip_file_name);

    NewOctaneZipWriter.write_file(&mut encrypted_writer, reader, zip_file_name)
  }

  fn write_compressed_file<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_data: &CompressedFileData,
  ) -> BinResult<FileInfo> {
    let mut encrypted_writer = self.create_file_writer(writer, &file_data.zip_file_name);

    NewOctaneZipWriter.write_compressed_file(&mut encrypted_writer, file_data)
  }

  fn write_footer<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_infos: &[FileInfo],
  ) -> BinResult<()> {
    let mut encrypted_writer = self.create_encrypted_writer(writer);

    let directory_start_offset = encrypted_writer.stream_position()?;
    write_zip_dir_entries(&mut encrypted_writer, &file_infos)?;

    encrypted_writer.reset_cipher_counter();

    let directory_end_offset = encrypted_writer.stream_position()?;
    create_zip_end_locator(
      directory_start_offset as u32,
      (directory_end_offset - directory_start_offset) as u32,
      file_infos.len() as u16,
    )
    .write(&mut encrypted_writer)?;

    Ok(())
  }
}

pub struct OldOctaneZipWriter;
impl ZipWriter for OldOctaneZipWriter {
  fn get_header_space(&mut self, file_paths: &[String]) -> usize {
    calculate_zip_dir_entries_header_size(file_paths) + ZIP_END_LOCATOR_SIZE
  }

  fn write_header<W: Write + Seek>(
    &mut self,
    writer: &mut W,
    file_infos: &[FileInfo],
  ) -> BinResult<()> {
    let file_paths: Vec<_> = file_infos
      .iter()
      .map(|file_info| file_info.file_name.clone())
      .collect();

    create_zip_end_locator(
      ZIP_END_LOCATOR_SIZE as u32,
      calculate_zip_dir_entries_header_size(&file_paths) as u32,
      file_infos.len() as u16,
    )
    .write(writer)?;
    write_zip_dir_entries(writer, &file_infos)?;

    Ok(())
  }
}

// compresses a batch of small files in parallel, then writes them in order.
fn write_compressed_batch<ZW: ZipWriter, W: Write + Seek>(
  zip_writer: &mut ZW,
  writer: &mut W,
  batch: &[(&PathBuf, &String)],
  file_infos: &mut Vec<FileInfo>,
) -> anyhow::Result<()> {
  let compressed_files = batch
    .par_iter()
    .map(|(path, zip_file_name)| compress_file_in_memory(path, zip_file_name))
    .collect::<anyhow::Result<Vec<_>>>()?;

  for file_data in &compressed_files {
    file_infos.push(zip_writer.write_compressed_file(writer, file_data)?);
  }

  Ok(())
}

pub fn write_octane_zip<ZW: ZipWriter>(
  source_folder: &Path,
  output_file_path: &Path,
  zip_writer: &mut ZW,
) -> anyhow::Result<()> {
  // files from this size are streamed block by block instead of being compressed in memory
  const LARGE_FILE_THRESHOLD: u64 = 64 * 1024 * 1024;
  // max amount of uncompressed small file data held in memory per batch
  const BATCH_BYTE_BUDGET: u64 = 256 * 1024 * 1024;

  let all_file_paths = get_all_file_paths(source_folder);
  if all_file_paths.is_empty() {
    return Err(anyhow::Error::msg(
      "The folder doesn't exist or the folder and its subfolders contain no files.",
    ));
  }

  let zip_file_names: Vec<_> = all_file_paths
    .iter()
    .map(|file_path| {
      file_path
        .strip_prefix(source_folder)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string()
        .replace("\\", "/")
    })
    .collect();

  let mut octane_zip_writer = BufWriter::new(File::create(output_file_path)?);
  octane_zip_writer.seek(SeekFrom::Current(
    zip_writer.get_header_space(&zip_file_names) as i64,
  ))?;

  let mut file_infos = Vec::with_capacity(all_file_paths.len());
  let mut batch: Vec<(&PathBuf, &String)> = Vec::new();
  let mut batch_bytes = 0u64;

  for (file_path, zip_file_name) in all_file_paths.iter().zip(zip_file_names.iter()) {
    let file_size = std::fs::metadata(file_path)?.len();

    if file_size >= LARGE_FILE_THRESHOLD {
      // write the queued small files first so the files stay in order
      write_compressed_batch(zip_writer, &mut octane_zip_writer, &batch, &mut file_infos)?;
      batch.clear();
      batch_bytes = 0;

      let mut file_reader = File::open(file_path)?;
      let file_info =
        zip_writer.write_file(&mut octane_zip_writer, &mut file_reader, zip_file_name)?;
      file_infos.push(file_info);
      continue;
    }

    batch.push((file_path, zip_file_name));
    batch_bytes += file_size;

    if batch_bytes >= BATCH_BYTE_BUDGET {
      write_compressed_batch(zip_writer, &mut octane_zip_writer, &batch, &mut file_infos)?;
      batch.clear();
      batch_bytes = 0;
    }
  }
  write_compressed_batch(zip_writer, &mut octane_zip_writer, &batch, &mut file_infos)?;

  zip_writer.write_footer(&mut octane_zip_writer, &file_infos)?;

  octane_zip_writer.seek(SeekFrom::Start(0))?;
  zip_writer.write_header(&mut octane_zip_writer, &file_infos)?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::{
    write_octane_zip, EncryptedNewOctaneZipWriter, NewOctaneZipWriter, OldOctaneZipWriter,
  };
  use std::path::PathBuf;

  #[test]
  pub fn test() {
    write_octane_zip(
      &PathBuf::from(r"."),
      &PathBuf::from("test.zip"),
      &mut EncryptedNewOctaneZipWriter { key: &[0x00; 16] },
    )
    .unwrap();
  }

  #[test]
  pub fn test_write_file_to_octane_zip_new() {
    use std::io::Cursor;
    use flate2::read::DeflateDecoder;
    use flate2::Crc;
    use std::io::Read;
    use crate::write_file_to_octane_zip;

    for multiplier in [0, 100, 70000, 200000] {
      let sample_data = b"Hello, World! Testing parallel deflate compression with Rayon and flate2.".repeat(multiplier);
      let mut reader2 = Cursor::new(sample_data.clone());
      let mut writer2 = Cursor::new(Vec::new());
      let info2 = write_file_to_octane_zip(&mut writer2, &mut reader2, "test.txt").unwrap();

      let mut expected_crc = Crc::new();
      expected_crc.update(&sample_data);
      assert_eq!(expected_crc.sum(), info2.file_crc);
      assert_eq!(sample_data.len() as u32, info2.uncompressed_size);

      let header_size = crate::calculate_file_record_header_size("test.txt");
      let compressed_data = &writer2.into_inner()[header_size..header_size + info2.compressed_size as usize];
      let mut decoder = DeflateDecoder::new(compressed_data);
      let mut decompressed = Vec::new();
      decoder.read_to_end(&mut decompressed).unwrap();
      assert_eq!(decompressed, sample_data);
    }
  }
}
