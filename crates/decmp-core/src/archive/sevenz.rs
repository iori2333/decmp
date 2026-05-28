use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use sevenz_rust::{Password, SevenZArchiveEntry, SevenZWriter};
use walkdir::WalkDir;

use crate::archive::{ArchiveEntry, ArchiveHandler};
use crate::error::{DecmpError, Result};
use crate::utils::ensure_dir;

pub struct SevenZHandler;

fn make_password(password: Option<&str>) -> Password {
  match password {
    Some(pw) => Password::from(pw),
    None => Password::empty(),
  }
}

impl ArchiveHandler for SevenZHandler {
  fn list(
    &self,
    path: &Path,
    password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>> {
    let pw = make_password(password);
    let reader = sevenz_rust::SevenZReader::open(path, pw)
      .map_err(|e| DecmpError::InvalidArchive(format!("7z open error: {e}")))?;

    let mut entries = Vec::new();
    for entry in &reader.archive().files {
      entries.push(ArchiveEntry {
        name: entry.name().to_string(),
        size: entry.size(),
        compressed_size: 0,
        is_dir: entry.is_directory(),
        method: "7z".to_string(),
        modified: format_7z_time(entry),
      });
    }

    Ok(entries)
  }

  fn extract(
    &self,
    path: &Path,
    dest: &Path,
    password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;

    let pw = make_password(password);
    let mut reader = sevenz_rust::SevenZReader::open(path, pw)
      .map_err(|e| DecmpError::InvalidArchive(format!("7z open error: {e}")))?;

    reader
      .for_each_entries(|entry, reader| {
        let out_path = dest.join(entry.name());

        if entry.is_directory() {
          let _ = std::fs::create_dir_all(&out_path);
        } else {
          if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
          }
          let mut file = match File::create(&out_path) {
            Ok(f) => f,
            Err(_) => return Ok(true),
          };
          let _ = std::io::copy(reader, &mut file);
        }
        Ok(true)
      })
      .map_err(|e| DecmpError::InvalidArchive(format!("7z extract error: {e}")))?;

    Ok(())
  }

  fn create(
    &self,
    sources: &[PathBuf],
    dest: &Path,
    password: Option<&str>,
    _level: Option<u32>,
  ) -> Result<()> {
    if sources.is_empty() {
      return Err(DecmpError::NoSources);
    }

    if let Some(parent) = dest.parent() {
      ensure_dir(parent)?;
    }

    let pw = make_password(password);
    let mut writer = SevenZWriter::create(dest)
      .map_err(|e| DecmpError::InvalidArchive(format!("7z create error: {e}")))?;

    if !pw.is_empty() {
      writer.set_encrypt_header(true);
    }

    for src in sources {
      if src.is_dir() {
        let base_name = src
          .file_name()
          .map(|s| s.to_string_lossy().to_string())
          .unwrap_or_default();

        for entry in WalkDir::new(src).min_depth(1) {
          let entry = entry?;
          let rel_path = entry.path().strip_prefix(src).unwrap();
          let entry_name = Path::new(&base_name)
            .join(rel_path)
            .to_string_lossy()
            .to_string();

          if entry.file_type().is_file() {
            let archive_entry = SevenZArchiveEntry::from_path(entry.path(), entry_name);
            let file = File::open(entry.path())?;
            writer
              .push_archive_entry(archive_entry, Some(file))
              .map_err(|e| DecmpError::InvalidArchive(format!("7z write error: {e}")))?;
          }
        }
      } else {
        let file_name = src
          .file_name()
          .map(|s| s.to_string_lossy().to_string())
          .unwrap_or_default();
        let archive_entry = SevenZArchiveEntry::from_path(src, file_name);
        let file = File::open(src)?;
        writer
          .push_archive_entry(archive_entry, Some(file))
          .map_err(|e| DecmpError::InvalidArchive(format!("7z write error: {e}")))?;
      }
    }

    writer
      .finish()
      .map_err(|e| DecmpError::InvalidArchive(format!("7z finish error: {e}")))?;

    Ok(())
  }

  fn extract_entries(
    &self,
    archive_path: &Path,
    entry_names: &[&str],
    dest: &Path,
    password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;
    let wanted: std::collections::HashSet<&str> = entry_names.iter().copied().collect();

    let pw = make_password(password);
    let mut reader = sevenz_rust::SevenZReader::open(archive_path, pw)
      .map_err(|e| DecmpError::InvalidArchive(format!("7z open error: {e}")))?;

    reader
      .for_each_entries(|entry, reader| {
        if !wanted.contains(entry.name()) {
          // Drain non-matching files to advance the solid stream position
          let _ = std::io::copy(reader, &mut std::io::sink());
          return Ok(true);
        }

        let out_path = dest.join(entry.name());
        if entry.is_directory() {
          let _ = std::fs::create_dir_all(&out_path);
        } else {
          if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
          }
          let mut file = match File::create(&out_path) {
            Ok(f) => f,
            Err(_) => return Ok(true),
          };
          let _ = std::io::copy(reader, &mut file);
        }
        Ok(true)
      })
      .map_err(|e| DecmpError::InvalidArchive(format!("7z extract error: {e}")))?;

    Ok(())
  }

  fn read_entry(
    &self,
    archive_path: &Path,
    entry_name: &str,
    password: Option<&str>,
    _encoding: Option<&str>,
    max_bytes: Option<usize>,
  ) -> Result<Vec<u8>> {
    let pw = make_password(password);
    let mut reader = sevenz_rust::SevenZReader::open(archive_path, pw)
      .map_err(|e| DecmpError::InvalidArchive(format!("7z open error: {e}")))?;

    let mut result = None;
    let target = entry_name.to_string();

    reader
      .for_each_entries(|entry, reader| {
        if entry.name() == target {
          let mut buf = Vec::new();
          if let Some(limit) = max_bytes {
            let mut limited = reader.take(limit as u64);
            let _ = std::io::copy(&mut limited, &mut buf);
          } else {
            let _ = std::io::copy(reader, &mut buf);
          }
          result = Some(buf);
          return Ok(false);
        }
        // Drain non-matching files to advance the solid stream position
        let _ = std::io::copy(reader, &mut std::io::sink());
        Ok(true)
      })
      .map_err(|e| DecmpError::InvalidArchive(format!("7z read error: {e}")))?;

    result.ok_or_else(|| DecmpError::InvalidArchive(format!("entry not found: {entry_name}")))
  }
}

fn format_7z_time(entry: &SevenZArchiveEntry) -> Option<String> {
  if !entry.has_last_modified_date {
    return None;
  }
  let raw = entry.last_modified_date.to_raw();
  const NT_EPOCH_OFFSET: u64 = 116_444_736_000_000_000;
  if raw <= NT_EPOCH_OFFSET {
    return None;
  }
  let unix_nanos = (raw - NT_EPOCH_OFFSET) * 100;
  let secs = (unix_nanos / 1_000_000_000) as i64;
  let nanos = (unix_nanos % 1_000_000_000) as u32;
  chrono::DateTime::from_timestamp(secs, nanos).map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
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
  }

  #[test]
  fn test_7z_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.7z");

    let handler = SevenZHandler;
    handler.create(&[src_dir], &archive, None, None).unwrap();
    assert!(archive.exists());
    assert!(std::fs::metadata(&archive).unwrap().len() > 0);
  }

  #[test]
  fn test_7z_list() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.7z");

    let handler = SevenZHandler;
    handler.create(&[src_dir], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    let names: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
    assert!(names.iter().any(|n| n.contains("file1.txt")));
    assert!(names.iter().any(|n| n.contains("file2.txt")));
  }

  #[test]
  fn test_7z_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.7z");

    let handler = SevenZHandler;
    handler.create(&[src_dir], &archive, None, None).unwrap();

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();

    let found = std::fs::read_dir(&out_dir)
      .unwrap()
      .filter_map(|e| e.ok())
      .collect::<Vec<_>>();
    assert!(!found.is_empty());
  }

  #[test]
  fn test_7z_single_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "hello.txt", b"Hello!");
    let archive = tmp.path().join("test.7z");

    let handler = SevenZHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(std::fs::read(out_dir.join("hello.txt")).unwrap(), b"Hello!");

    // read_entry must return file content
    let data = handler
      .read_entry(&archive, "hello.txt", None, None, None)
      .unwrap();
    assert_eq!(data, b"Hello!");
  }

  #[test]
  fn test_7z_read_entry_multiple() {
    let tmp = tempfile::tempdir().unwrap();
    let f1 = write_temp_file(tmp.path(), "a.txt", b"AAA");
    let f2 = write_temp_file(tmp.path(), "b.txt", b"BBBBBB");
    let f3 = write_temp_file(tmp.path(), "c.txt", b"CCCCCCCCC");
    let archive = tmp.path().join("multi.7z");

    let handler = SevenZHandler;
    handler.create(&[f1, f2, f3], &archive, None, None).unwrap();

    // Test WITH max_bytes (as the TUI always calls it)
    assert_eq!(
      handler
        .read_entry(&archive, "c.txt", None, None, Some(64 * 1024))
        .unwrap(),
      b"CCCCCCCCC"
    );
    assert_eq!(
      handler
        .read_entry(&archive, "a.txt", None, None, Some(64 * 1024))
        .unwrap(),
      b"AAA"
    );
    assert_eq!(
      handler
        .read_entry(&archive, "b.txt", None, None, Some(64 * 1024))
        .unwrap(),
      b"BBBBBB"
    );
  }

  #[test]
  fn test_7z_read_entry_via_listed_names() {
    // Simulate the exact TUI flow: list, then read_entry with listed names
    let tmp = tempfile::tempdir().unwrap();
    let f1 = write_temp_file(tmp.path(), "first.rs", b"// first file");
    let f2 = write_temp_file(tmp.path(), "second.py", b"# second file");
    let archive = tmp.path().join("test.7z");

    let handler = SevenZHandler;
    handler.create(&[f1, f2], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    for e in &entries {
      if e.is_dir {
        continue;
      }
      let data = handler
        .read_entry(&archive, &e.name, None, None, Some(64 * 1024))
        .unwrap();
      assert!(!data.is_empty(), "empty data for {}", e.name);
    }
  }

  #[test]
  fn test_7z_read_fixture() {
    use std::path::PathBuf;
    let path = PathBuf::from("tests/fixtures/codes.7z");
    if !path.exists() {
      return;
    }
    let handler = SevenZHandler;
    let entries = handler.list(&path, None, None).unwrap();
    assert!(entries.len() >= 2, "need at least 2 files");

    let name_a = &entries[0].name;
    let name_b = &entries[1].name;

    let data_a = handler
      .read_entry(&path, name_a, None, None, Some(64 * 1024))
      .unwrap();
    let data_b = handler
      .read_entry(&path, name_b, None, None, Some(64 * 1024))
      .unwrap();

    assert!(
      data_a != data_b,
      "read_entry returned same content for {name_a} and {name_b}"
    );
  }

  #[test]
  fn test_7z_read_cli_archive() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("input");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), b"AAA").unwrap();
    std::fs::write(dir.join("b.txt"), b"BBB").unwrap();

    let archive = tmp.path().join("cli.7z");
    let status = std::process::Command::new("7z")
      .args([
        "a",
        "-t7z",
        &archive.to_string_lossy().to_string(),
        "a.txt",
        "b.txt",
      ])
      .current_dir(&dir)
      .status()
      .unwrap();
    assert!(status.success(), "7z CLI failed");

    let handler = SevenZHandler;
    let entries = handler.list(&archive, None, None).unwrap();

    // Entry names from CLI-created 7z are just the filenames (no dir prefix)
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    eprintln!("entries: {names:?}");

    // Read second file first, then first
    let data_b = handler
      .read_entry(&archive, "b.txt", None, None, Some(64 * 1024))
      .unwrap();
    let data_a = handler
      .read_entry(&archive, "a.txt", None, None, Some(64 * 1024))
      .unwrap();

    assert_eq!(data_a, b"AAA", "a.txt mismatch");
    assert_eq!(data_b, b"BBB", "b.txt mismatch");
  }

  #[test]
  fn test_7z_read_entry_rapid_switching() {
    let tmp = tempfile::tempdir().unwrap();
    let mut sources = Vec::new();
    let mut expected: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
    for i in 0..10 {
      let name = format!("file_{i:02}.txt");
      let content = format!("content_of_file_{i:02}\n");
      expected.insert(name.clone(), content.as_bytes().to_vec());
      sources.push(write_temp_file(tmp.path(), &name, content.as_bytes()));
    }
    let archive = tmp.path().join("many.7z");

    let handler = SevenZHandler;
    handler.create(&sources, &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    let file_entries: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();
    assert_eq!(file_entries.len(), 10);

    for e in &file_entries {
      let data = handler
        .read_entry(&archive, &e.name, None, None, Some(64 * 1024))
        .unwrap();
      assert_eq!(data, expected[&e.name], "content mismatch for {}", e.name);
    }
  }

  #[test]
  fn test_7z_encrypted_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "secret.txt", b"secret data");
    let archive = tmp.path().join("encrypted.7z");

    let handler = SevenZHandler;
    handler
      .create(&[src], &archive, Some("mypassword"), None)
      .unwrap();

    let out_dir = tmp.path().join("out");
    handler
      .extract(&archive, &out_dir, Some("mypassword"), None)
      .unwrap();

    assert_eq!(
      std::fs::read(out_dir.join("secret.txt")).unwrap(),
      b"secret data"
    );
  }

  #[test]
  fn test_7z_no_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = tmp.path().join("test.7z");
    let handler = SevenZHandler;
    let result = handler.create(&[], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_7z_empty_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "empty.txt", b"");
    let archive = tmp.path().join("test.7z");

    let handler = SevenZHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].size, 0);
  }
}
