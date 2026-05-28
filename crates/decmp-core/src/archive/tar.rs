use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use tar::{Archive, Builder, Header};
use walkdir::WalkDir;

use crate::archive::{ArchiveEntry, ArchiveHandler, Format};
use crate::encoding;
use crate::error::{DecmpError, Result};
use crate::utils::ensure_dir;

pub struct TarHandler {
  format: Format,
}

impl TarHandler {
  pub fn new(format: Format) -> Self {
    Self { format }
  }
}

fn open_tar_reader(path: &Path, format: &Format) -> Result<Box<dyn Read>> {
  let file = File::open(path)?;
  match format {
    Format::Tar => Ok(Box::new(BufReader::new(file))),
    Format::TarGz => Ok(Box::new(flate2::read::GzDecoder::new(BufReader::new(file)))),
    Format::TarXz => Ok(Box::new(xz2::read::XzDecoder::new(BufReader::new(file)))),
    Format::TarZst => Ok(Box::new(zstd::Decoder::new(BufReader::new(file))?)),
    Format::TarBz2 => Ok(Box::new(bzip2::read::BzDecoder::new(BufReader::new(file)))),
    Format::TarLzma => {
      let mut input = BufReader::new(file);
      let mut buf = Vec::new();
      input.read_to_end(&mut buf)?;
      let mut cursor = std::io::Cursor::new(buf);
      let mut output = Vec::new();
      lzma_rs::lzma_decompress(&mut cursor, &mut output)
        .map_err(|e| DecmpError::InvalidArchive(format!("lzma error: {e}")))?;
      Ok(Box::new(std::io::Cursor::new(output)))
    }
    _ => Err(DecmpError::UnsupportedFormat(format.to_string())),
  }
}

fn append_sources(builder: &mut Builder<impl Write>, sources: &[PathBuf]) -> Result<()> {
  for src in sources {
    if src.is_dir() {
      let base_name = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
      for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let rel_path = entry.path().strip_prefix(src).unwrap();
        let tar_path = Path::new(&base_name).join(rel_path);

        if entry.file_type().is_dir() {
          let mut header = Header::new_gnu();
          header.set_entry_type(tar::EntryType::Directory);
          header.set_size(0);
          header.set_path(&tar_path)?;
          header.set_mode(0o755);
          header.set_cksum();
          builder.append(&header, std::io::empty())?;
        } else if entry.file_type().is_file() {
          let mut file = File::open(entry.path())?;
          let meta = file.metadata()?;
          let mut header = Header::new_gnu();
          header.set_entry_type(tar::EntryType::Regular);
          header.set_size(meta.len());
          header.set_path(&tar_path)?;
          header.set_mode(0o644);
          header.set_cksum();
          builder.append(&header, &mut file)?;
        }
      }
    } else {
      let file_name = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
      let mut file = File::open(src)?;
      builder.append_file(&file_name, &mut file)?;
    }
  }
  Ok(())
}

impl ArchiveHandler for TarHandler {
  fn list(
    &self,
    path: &Path,
    _password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>> {
    let reader = open_tar_reader(path, &self.format)?;
    let mut archive = Archive::new(reader);
    let mut entries = Vec::new();

    for entry in archive.entries()? {
      let entry = entry?;
      let name_bytes = entry.path_bytes().into_owned();
      let name = if let Some(enc) = encoding {
        encoding::decode_filename(&name_bytes, enc)?
      } else {
        String::from_utf8_lossy(&name_bytes).into_owned()
      };

      let modified = entry.header().mtime().ok().and_then(|secs| {
        std::time::UNIX_EPOCH
          .checked_add(std::time::Duration::from_secs(secs))
          .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.format("%Y-%m-%d %H:%M").to_string()
          })
      });

      entries.push(ArchiveEntry {
        name,
        size: entry.size(),
        compressed_size: 0,
        is_dir: entry.header().entry_type().is_dir(),
        method: self.format.to_string(),
        modified,
      });
    }

    Ok(entries)
  }

  fn extract(
    &self,
    path: &Path,
    dest: &Path,
    _password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;

    let reader = open_tar_reader(path, &self.format)?;
    let mut archive = Archive::new(reader);

    if let Some(enc) = encoding {
      let mut entries = archive.entries()?;
      while let Some(Ok(mut entry)) = entries.next() {
        let raw_name = entry.path_bytes().into_owned();
        let decoded_name = encoding::decode_filename(&raw_name, enc)?;
        let out_path = dest.join(&decoded_name);

        if let Some(parent) = out_path.parent() {
          ensure_dir(parent)?;
        }

        entry.unpack(&out_path)?;
      }
    } else {
      archive.unpack(dest)?;
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
    if sources.is_empty() {
      return Err(DecmpError::NoSources);
    }

    if let Some(parent) = dest.parent() {
      ensure_dir(parent)?;
    }

    match self.format {
      Format::Tar => {
        let file = File::create(dest)?;
        let mut builder = Builder::new(file);
        append_sources(&mut builder, sources)?;
        builder.into_inner()?;
      }
      Format::TarGz => {
        let file = File::create(dest)?;
        let level = level.unwrap_or(6);
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::new(level));
        let mut builder = Builder::new(encoder);
        append_sources(&mut builder, sources)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
      }
      Format::TarXz => {
        let file = File::create(dest)?;
        let level = level.unwrap_or(6);
        let encoder = xz2::write::XzEncoder::new(file, level);
        let mut builder = Builder::new(encoder);
        append_sources(&mut builder, sources)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
      }
      Format::TarZst => {
        let file = File::create(dest)?;
        let level = level.map(|l| l as i32).unwrap_or(0);
        let encoder = zstd::Encoder::new(file, level)?;
        let mut builder = Builder::new(encoder);
        append_sources(&mut builder, sources)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
      }
      Format::TarBz2 => {
        let file = File::create(dest)?;
        let level = level.unwrap_or(6);
        let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::new(level));
        let mut builder = Builder::new(encoder);
        append_sources(&mut builder, sources)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
      }
      Format::TarLzma => {
        let file = File::create(dest)?;
        let level = level.unwrap_or(6);
        let encoder = xz2::write::XzEncoder::new(file, level);
        let mut builder = Builder::new(encoder);
        append_sources(&mut builder, sources)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
      }
      _ => return Err(DecmpError::UnsupportedFormat(self.format.to_string())),
    }

    Ok(())
  }

  fn extract_entries(
    &self,
    archive_path: &Path,
    entry_names: &[&str],
    dest: &Path,
    _password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;
    let wanted: std::collections::HashSet<&str> = entry_names.iter().copied().collect();

    let reader = open_tar_reader(archive_path, &self.format)?;
    let mut archive = Archive::new(reader);

    let mut entries_iter = archive.entries()?;
    while let Some(Ok(mut entry)) = entries_iter.next() {
      let name_bytes = entry.path_bytes().into_owned();
      let mut name = if let Some(enc) = encoding {
        encoding::decode_filename(&name_bytes, enc)?
      } else {
        String::from_utf8_lossy(&name_bytes).into_owned()
      };
      if let Some(stripped) = name.strip_prefix("./") {
        name = stripped.to_string();
      }

      if !wanted.contains(name.as_str()) {
        continue;
      }

      let out_path = dest.join(&name);
      if let Some(parent) = out_path.parent() {
        ensure_dir(parent)?;
      }
      entry.unpack(&out_path)?;
    }
    Ok(())
  }

  fn read_entry(
    &self,
    archive_path: &Path,
    entry_name: &str,
    _password: Option<&str>,
    encoding: Option<&str>,
    max_bytes: Option<usize>,
  ) -> Result<Vec<u8>> {
    let reader = open_tar_reader(archive_path, &self.format)?;
    let mut archive = Archive::new(reader);

    let mut entries_iter = archive.entries()?;
    while let Some(Ok(mut entry)) = entries_iter.next() {
      let name_bytes = entry.path_bytes().into_owned();
      let mut name = if let Some(enc) = encoding {
        encoding::decode_filename(&name_bytes, enc)?
      } else {
        String::from_utf8_lossy(&name_bytes).into_owned()
      };
      if let Some(stripped) = name.strip_prefix("./") {
        name = stripped.to_string();
      }

      if name == entry_name {
        let mut buf = Vec::new();
        if let Some(limit) = max_bytes {
          let mut limited = (&mut entry).take(limit as u64);
          std::io::copy(&mut limited, &mut buf)?;
        } else {
          std::io::copy(&mut entry, &mut buf)?;
        }
        return Ok(buf);
      }
    }
    Err(DecmpError::InvalidArchive(format!(
      "entry not found: {entry_name}"
    )))
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

  fn create_test_dir(dir: &Path) {
    std::fs::create_dir_all(dir.join("subdir")).unwrap();
    std::fs::write(dir.join("file1.txt"), b"content1").unwrap();
    std::fs::write(dir.join("subdir/file2.txt"), b"content2").unwrap();
    std::fs::write(dir.join("subdir/file3.txt"), b"content3").unwrap();
  }

  #[test]
  fn test_tar_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar");

    let handler = TarHandler::new(Format::Tar);
    handler.create(&[src_dir], &archive, None, None).unwrap();
    assert!(archive.exists());
    assert!(std::fs::metadata(&archive).unwrap().len() > 0);
  }

  #[test]
  fn test_tar_list() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar");

    let handler = TarHandler::new(Format::Tar);
    handler.create(&[src_dir], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 4);
    let names: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
    assert!(names.contains(&"src/file1.txt".to_string()));
    assert!(names.contains(&"src/subdir/file2.txt".to_string()));
  }

  #[test]
  fn test_tar_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar");

    let handler = TarHandler::new(Format::Tar);
    handler.create(&[src_dir], &archive, None, None).unwrap();

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();

    assert!(out_dir.join("src/file1.txt").exists());
    assert!(out_dir.join("src/subdir/file2.txt").exists());
    assert_eq!(
      std::fs::read(out_dir.join("src/file1.txt")).unwrap(),
      b"content1"
    );
    assert_eq!(
      std::fs::read(out_dir.join("src/subdir/file2.txt")).unwrap(),
      b"content2"
    );
  }

  #[test]
  fn test_tar_single_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "hello.txt", b"Hello!");
    let archive = tmp.path().join("test.tar");

    let handler = TarHandler::new(Format::Tar);
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(std::fs::read(out_dir.join("hello.txt")).unwrap(), b"Hello!");
  }

  #[test]
  fn test_tar_gz_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar.gz");

    let handler = TarHandler::new(Format::TarGz);
    handler.create(&[src_dir], &archive, None, Some(6)).unwrap();
    assert!(archive.exists());

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(
      std::fs::read(out_dir.join("src/file1.txt")).unwrap(),
      b"content1"
    );
  }

  #[test]
  fn test_tar_xz_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar.xz");

    let handler = TarHandler::new(Format::TarXz);
    handler.create(&[src_dir], &archive, None, Some(3)).unwrap();
    assert!(archive.exists());

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(
      std::fs::read(out_dir.join("src/file1.txt")).unwrap(),
      b"content1"
    );
  }

  #[test]
  fn test_tar_zst_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar.zst");

    let handler = TarHandler::new(Format::TarZst);
    handler.create(&[src_dir], &archive, None, None).unwrap();
    assert!(archive.exists());

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(
      std::fs::read(out_dir.join("src/file1.txt")).unwrap(),
      b"content1"
    );
  }

  #[test]
  fn test_tar_bz2_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.tar.bz2");

    let handler = TarHandler::new(Format::TarBz2);
    handler.create(&[src_dir], &archive, None, None).unwrap();
    assert!(archive.exists());

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(
      std::fs::read(out_dir.join("src/file1.txt")).unwrap(),
      b"content1"
    );
  }

  #[test]
  fn test_tar_empty_archive() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = tmp.path().join("empty.tar");
    let writer = File::create(&archive).unwrap();
    let builder = Builder::new(writer);
    builder.into_inner().unwrap();

    let handler = TarHandler::new(Format::Tar);
    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 0);
  }

  #[test]
  fn test_tar_no_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = tmp.path().join("test.tar");
    let handler = TarHandler::new(Format::Tar);
    let result = handler.create(&[], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_tar_compression_size_ordering() {
    let tmp = tempfile::tempdir().unwrap();
    let data = vec![b'A'; 10000];
    let src = write_temp_file(tmp.path(), "big.txt", &data);

    let tar = tmp.path().join("plain.tar");
    let tgz = tmp.path().join("fast.tar.gz");
    let tgz_best = tmp.path().join("best.tar.gz");

    let handler_tar = TarHandler::new(Format::Tar);
    handler_tar
      .create(&[src.clone()], &tar, None, None)
      .unwrap();

    let handler_tgz = TarHandler::new(Format::TarGz);
    handler_tgz
      .create(&[src.clone()], &tgz, None, Some(1))
      .unwrap();
    handler_tgz
      .create(&[src], &tgz_best, None, Some(9))
      .unwrap();

    let tar_size = std::fs::metadata(&tar).unwrap().len();
    let tgz_size = std::fs::metadata(&tgz).unwrap().len();
    let tgz_best_size = std::fs::metadata(&tgz_best).unwrap().len();

    assert!(tgz_size <= tar_size);
    assert!(tgz_best_size <= tgz_size);
  }

  #[test]
  fn test_tar_list_with_encoding() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "test.txt", b"hello");
    let archive = tmp.path().join("test.tar");

    let handler = TarHandler::new(Format::Tar);
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, Some("utf-8")).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "test.txt");
  }
}
