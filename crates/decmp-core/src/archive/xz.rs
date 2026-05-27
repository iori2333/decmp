use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use xz2::read::XzDecoder;
use xz2::write::XzEncoder;

use crate::archive::{ArchiveEntry, ArchiveHandler};
use crate::error::{DecmpError, Result};
use crate::utils::{ensure_dir, strip_archive_extension};

pub struct XzHandler;

fn output_name(archive_path: &Path) -> String {
  strip_archive_extension(archive_path)
}

fn is_lzma_extension(path: &Path) -> bool {
  path
    .extension()
    .map(|e| e.to_string_lossy().to_lowercase() == "lzma")
    .unwrap_or(false)
}

impl ArchiveHandler for XzHandler {
  fn list(
    &self,
    path: &Path,
    _password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>> {
    let meta = std::fs::metadata(path)?;
    let name = output_name(path);
    let method = if is_lzma_extension(path) {
      "lzma"
    } else {
      "xz"
    };
    Ok(vec![ArchiveEntry {
      name,
      size: 0,
      compressed_size: meta.len(),
      is_dir: false,
      method: method.to_string(),
      modified: None,
    }])
  }

  fn extract(
    &self,
    path: &Path,
    dest: &Path,
    _password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<()> {
    let out_path = dest.join(output_name(path));
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    if is_lzma_extension(path) {
      let mut input = BufReader::new(File::open(path)?);
      let mut output = Vec::new();
      lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|e| DecmpError::InvalidArchive(format!("lzma decompress error: {e}")))?;
      std::fs::write(&out_path, &output)?;
    } else {
      let file = File::open(path)?;
      let mut decoder = XzDecoder::new(file);
      let mut out_file = File::create(&out_path)?;
      std::io::copy(&mut decoder, &mut out_file)?;
    }

    Ok(())
  }

  fn create(
    &self,
    sources: &[PathBuf],
    dest: &Path,
    _password: Option<&str>,
    level: Option<u32>,
  ) -> Result<()> {
    if sources.len() != 1 {
      return Err(DecmpError::InvalidArchive(
        "xz/lzma can only compress a single file".to_string(),
      ));
    }

    let src = &sources[0];
    if src.is_dir() {
      return Err(DecmpError::InvalidArchive(
        "xz/lzma cannot compress directories; use tar.xz instead".to_string(),
      ));
    }

    if let Some(parent) = dest.parent() {
      std::fs::create_dir_all(parent)?;
    }

    let in_file = File::open(src)?;
    let out_file = File::create(dest)?;
    let level = level.unwrap_or(6);
    let mut encoder = XzEncoder::new(out_file, level);
    let mut reader = std::io::BufReader::new(in_file);
    std::io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
  }

  fn extract_entries(
    &self,
    archive_path: &Path,
    entry_names: &[&str],
    dest: &Path,
    _password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<()> {
    let out_name = output_name(archive_path);
    if !entry_names.iter().any(|n| *n == out_name) {
      return Ok(());
    }
    ensure_dir(dest)?;
    let out_path = dest.join(&out_name);
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    if is_lzma_extension(archive_path) {
      let mut input = BufReader::new(File::open(archive_path)?);
      let mut output = Vec::new();
      lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|e| DecmpError::InvalidArchive(format!("lzma decompress error: {e}")))?;
      std::fs::write(&out_path, &output)?;
    } else {
      let file = File::open(archive_path)?;
      let mut decoder = XzDecoder::new(file);
      let mut out_file = File::create(&out_path)?;
      std::io::copy(&mut decoder, &mut out_file)?;
    }
    Ok(())
  }

  fn read_entry(
    &self,
    archive_path: &Path,
    entry_name: &str,
    _password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<Vec<u8>> {
    let out_name = output_name(archive_path);
    if entry_name != out_name {
      return Err(DecmpError::InvalidArchive(format!(
        "entry not found: {entry_name}"
      )));
    }
    if is_lzma_extension(archive_path) {
      let mut input = BufReader::new(File::open(archive_path)?);
      let mut output = Vec::new();
      lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|e| DecmpError::InvalidArchive(format!("lzma decompress error: {e}")))?;
      Ok(output)
    } else {
      let file = File::open(archive_path)?;
      let mut decoder = XzDecoder::new(file);
      let mut buf = Vec::new();
      std::io::copy(&mut decoder, &mut buf)?;
      Ok(buf)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn write_temp_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
  }

  #[test]
  fn test_xz_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let data = b"Hello, xz world! This is a test content.";
    let src = write_temp_file(tmp.path(), "test.txt", data);
    let archive = tmp.path().join("test.txt.xz");

    let handler = XzHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    assert!(archive.exists());
    assert!(std::fs::metadata(&archive).unwrap().len() > 0);
  }

  #[test]
  fn test_xz_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let data = b"Hello, xz world! Extract test.";
    let src = write_temp_file(tmp.path(), "test.txt", data);
    let archive = tmp.path().join("test.txt.xz");

    let handler = XzHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    handler.extract(&archive, &out_dir, None, None).unwrap();

    let extracted = out_dir.join("test.txt");
    assert!(extracted.exists());
    let content = std::fs::read(&extracted).unwrap();
    assert_eq!(content, data);
  }

  #[test]
  fn test_xz_list() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "data.txt", b"some data");
    let archive = tmp.path().join("data.txt.xz");

    let handler = XzHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "data.txt");
    assert_eq!(entries[0].method, "xz");
  }

  #[test]
  fn test_xz_list_lzma_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "data.txt", b"some data");
    let archive = tmp.path().join("data.txt.lzma");

    let handler = XzHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries[0].method, "lzma");
  }

  #[test]
  fn test_xz_compression_levels() {
    let tmp = tempfile::tempdir().unwrap();
    let data = vec![b'A'; 10000];
    let src = write_temp_file(tmp.path(), "big.txt", &data);

    let handler = XzHandler;

    let fast = tmp.path().join("fast.xz");
    handler
      .create(&[src.clone()], &fast, None, Some(1))
      .unwrap();

    let best = tmp.path().join("best.xz");
    handler.create(&[src], &best, None, Some(9)).unwrap();

    let fast_size = std::fs::metadata(&fast).unwrap().len();
    let best_size = std::fs::metadata(&best).unwrap().len();
    assert!(best_size <= fast_size);
  }

  #[test]
  fn test_xz_reject_multiple_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let src1 = write_temp_file(tmp.path(), "a.txt", b"a");
    let src2 = write_temp_file(tmp.path(), "b.txt", b"b");
    let archive = tmp.path().join("out.xz");

    let handler = XzHandler;
    let result = handler.create(&[src1, src2], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_xz_reject_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("mydir");
    std::fs::create_dir(&dir).unwrap();
    let archive = tmp.path().join("out.xz");

    let handler = XzHandler;
    let result = handler.create(&[dir], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_xz_empty_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "empty.txt", b"");
    let archive = tmp.path().join("empty.txt.xz");

    let handler = XzHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    handler.extract(&archive, &out_dir, None, None).unwrap();

    let extracted = out_dir.join("empty.txt");
    assert!(extracted.exists());
    assert_eq!(std::fs::metadata(&extracted).unwrap().len(), 0);
  }
}
