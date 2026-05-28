use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::archive::{ArchiveEntry, ArchiveHandler};
use crate::error::{DecmpError, Result};
use crate::utils::{ensure_dir, strip_archive_extension};

pub struct GzipHandler;

fn output_name(archive_path: &Path) -> String {
  strip_archive_extension(archive_path)
}

impl ArchiveHandler for GzipHandler {
  fn list(
    &self,
    path: &Path,
    _password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>> {
    let meta = std::fs::metadata(path)?;
    let name = output_name(path);
    Ok(vec![ArchiveEntry {
      name,
      size: 0,
      compressed_size: meta.len(),
      is_dir: false,
      method: "gzip".to_string(),
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
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);

    let out_path = dest.join(output_name(path));
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let mut out_file = File::create(&out_path)?;
    std::io::copy(&mut decoder, &mut out_file)?;
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
        "gzip can only compress a single file".to_string(),
      ));
    }

    let src = &sources[0];
    if src.is_dir() {
      return Err(DecmpError::InvalidArchive(
        "gzip cannot compress directories; use tar.gz instead".to_string(),
      ));
    }

    let compression = match level {
      Some(l) => Compression::new(l),
      None => Compression::default(),
    };

    if let Some(parent) = dest.parent() {
      std::fs::create_dir_all(parent)?;
    }

    let in_file = File::open(src)?;
    let out_file = File::create(dest)?;
    let mut encoder = GzEncoder::new(out_file, compression);
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
    let file = File::open(archive_path)?;
    let mut decoder = GzDecoder::new(file);
    let out_path = dest.join(&out_name);
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let mut out_file = File::create(&out_path)?;
    std::io::copy(&mut decoder, &mut out_file)?;
    Ok(())
  }

  fn read_entry(
    &self,
    archive_path: &Path,
    entry_name: &str,
    _password: Option<&str>,
    _encoding: Option<&str>,
    max_bytes: Option<usize>,
  ) -> Result<Vec<u8>> {
    let out_name = output_name(archive_path);
    if entry_name != out_name {
      return Err(DecmpError::InvalidArchive(format!(
        "entry not found: {entry_name}"
      )));
    }
    let file = File::open(archive_path)?;
    let mut decoder = GzDecoder::new(file);
    let mut buf = Vec::new();
    if let Some(limit) = max_bytes {
      let mut limited = (&mut decoder).take(limit as u64);
      std::io::copy(&mut limited, &mut buf)?;
    } else {
      std::io::copy(&mut decoder, &mut buf)?;
    }
    Ok(buf)
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
  fn test_gzip_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let data = b"Hello, gzip world! This is a test content.";
    let src = write_temp_file(tmp.path(), "test.txt", data);
    let archive = tmp.path().join("test.txt.gz");

    let handler = GzipHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    assert!(archive.exists());
    assert!(std::fs::metadata(&archive).unwrap().len() > 0);
  }

  #[test]
  fn test_gzip_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let data = b"Hello, gzip world! Extract test.";
    let src = write_temp_file(tmp.path(), "test.txt", data);
    let archive = tmp.path().join("test.txt.gz");

    let handler = GzipHandler;
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
  fn test_gzip_list() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "data.txt", b"some data");
    let archive = tmp.path().join("data.txt.gz");

    let handler = GzipHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "data.txt");
    assert!(!entries[0].is_dir);
  }

  #[test]
  fn test_gzip_compression_levels() {
    let tmp = tempfile::tempdir().unwrap();
    let data = vec![b'A'; 10000];
    let src = write_temp_file(tmp.path(), "big.txt", &data);

    let handler = GzipHandler;

    let fast = tmp.path().join("fast.gz");
    handler
      .create(&[src.clone()], &fast, None, Some(1))
      .unwrap();

    let best = tmp.path().join("best.gz");
    handler.create(&[src], &best, None, Some(9)).unwrap();

    let fast_size = std::fs::metadata(&fast).unwrap().len();
    let best_size = std::fs::metadata(&best).unwrap().len();
    assert!(best_size <= fast_size);
  }

  #[test]
  fn test_gzip_reject_multiple_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let src1 = write_temp_file(tmp.path(), "a.txt", b"a");
    let src2 = write_temp_file(tmp.path(), "b.txt", b"b");
    let archive = tmp.path().join("out.gz");

    let handler = GzipHandler;
    let result = handler.create(&[src1, src2], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_gzip_reject_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("mydir");
    std::fs::create_dir(&dir).unwrap();
    let archive = tmp.path().join("out.gz");

    let handler = GzipHandler;
    let result = handler.create(&[dir], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_gzip_empty_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "empty.txt", b"");
    let archive = tmp.path().join("empty.txt.gz");

    let handler = GzipHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    handler.extract(&archive, &out_dir, None, None).unwrap();

    let extracted = out_dir.join("empty.txt");
    assert!(extracted.exists());
    assert_eq!(std::fs::metadata(&extracted).unwrap().len(), 0);
  }
}
