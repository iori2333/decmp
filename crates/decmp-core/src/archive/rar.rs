use std::path::{Path, PathBuf};

use unrar::Archive;

use crate::archive::{ArchiveEntry, ArchiveHandler};
use crate::error::{DecmpError, Result};
use crate::utils::ensure_dir;

pub struct RarHandler;

impl ArchiveHandler for RarHandler {
  fn list(
    &self,
    path: &Path,
    password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>> {
    let archive = open_archive(path, password);
    let entries: Vec<ArchiveEntry> = archive
      .open_for_listing()
      .map_err(rar_error)?
      .filter_map(|r| r.ok())
      .map(|header| ArchiveEntry {
        name: header.filename.to_string_lossy().into_owned(),
        size: header.unpacked_size,
        compressed_size: 0,
        is_dir: header.is_directory(),
        method: method_name(header.method),
        modified: dos_datetime(header.file_time),
      })
      .collect();
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
    let archive = open_archive(path, password);
    let mut archive = archive.open_for_processing().map_err(rar_error)?;

    loop {
      let next = archive.read_header().map_err(rar_error)?;
      match next {
        Some(entry_archive) => {
          archive = entry_archive.extract_with_base(dest).map_err(rar_error)?;
        }
        None => break,
      }
    }
    Ok(())
  }

  fn extract_entries(
    &self,
    path: &Path,
    entry_names: &[&str],
    dest: &Path,
    password: Option<&str>,
    _encoding: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;
    let wanted: std::collections::HashSet<&str> = entry_names.iter().copied().collect();
    let archive = open_archive(path, password);
    let mut archive = archive.open_for_processing().map_err(rar_error)?;

    loop {
      let next = archive.read_header().map_err(rar_error)?;
      match next {
        Some(entry_archive) => {
          let name = entry_archive.entry().filename.to_string_lossy();
          if wanted.contains(name.as_ref()) {
            archive = entry_archive.extract_with_base(dest).map_err(rar_error)?;
          } else {
            archive = entry_archive.skip().map_err(rar_error)?;
          }
        }
        None => break,
      }
    }
    Ok(())
  }

  fn read_entry(
    &self,
    path: &Path,
    entry_name: &str,
    password: Option<&str>,
    _encoding: Option<&str>,
    max_bytes: Option<usize>,
  ) -> Result<Vec<u8>> {
    let archive = open_archive(path, password);
    let mut archive = archive.open_for_processing().map_err(rar_error)?;

    loop {
      let next = archive.read_header().map_err(rar_error)?;
      match next {
        Some(entry_archive) => {
          let name = entry_archive.entry().filename.to_string_lossy();
          if name == entry_name {
            let (mut data, _) = entry_archive.read().map_err(rar_error)?;
            if let Some(limit) = max_bytes {
              data.truncate(limit);
            }
            return Ok(data);
          }
          archive = entry_archive.skip().map_err(rar_error)?;
        }
        None => break,
      }
    }
    Err(DecmpError::InvalidArchive(format!(
      "entry not found: {entry_name}"
    )))
  }

  fn create(
    &self,
    _sources: &[PathBuf],
    _dest: &Path,
    _password: Option<&str>,
    _level: Option<u32>,
  ) -> Result<()> {
    Err(DecmpError::InvalidArchive(
      "creating RAR archives is not supported".to_string(),
    ))
  }
}

fn open_archive<'a>(path: &'a Path, password: Option<&'a str>) -> Archive<'a> {
  match password {
    Some(pw) => Archive::with_password(path, pw.as_bytes()),
    None => Archive::new(path),
  }
}

fn rar_error(e: unrar::error::UnrarError) -> DecmpError {
  DecmpError::InvalidArchive(format!("rar error: {e}"))
}

fn method_name(method: u32) -> String {
  match method {
    0x30 => "stored".to_string(),
    0x31 => "fastest".to_string(),
    0x32 => "fast".to_string(),
    0x33 => "normal".to_string(),
    0x34 => "good".to_string(),
    0x35 => "best".to_string(),
    _ => format!("0x{method:x}"),
  }
}

fn dos_datetime(file_time: u32) -> Option<String> {
  if file_time == 0 {
    return None;
  }
  let datepart = (file_time >> 16) as u16;
  let timepart = file_time as u16;

  let year = (datepart >> 9) as u32 + 1980;
  let month = ((datepart >> 5) & 0x0F) as u32;
  let day = (datepart & 0x1F) as u32;
  let hour = (timepart >> 11) as u32;
  let minute = ((timepart >> 5) & 0x3F) as u32;
  let second = ((timepart & 0x1F) * 2) as u32;

  if month == 0 || month > 12 || day == 0 || day > 31 {
    return None;
  }

  Some(format!(
    "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn create_test_dir(dir: &Path) {
    std::fs::create_dir_all(dir.join("subdir")).unwrap();
    std::fs::write(dir.join("file1.txt"), b"content1").unwrap();
    std::fs::write(dir.join("subdir/file2.txt"), b"content2").unwrap();
  }

  fn has_rar() -> bool {
    std::process::Command::new("rar")
      .arg("-?")
      .output()
      .map(|o| o.status.success())
      .unwrap_or(false)
  }

  #[test]
  fn test_rar_list() {
    if !has_rar() {
      return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("input");
    create_test_dir(&dir);

    let archive_path = tmp.path().join("test.rar");
    let status = std::process::Command::new("rar")
      .args([
        "a",
        "-r",
        "-idq",
        &archive_path.to_string_lossy().to_string(),
        ".",
      ])
      .current_dir(&dir)
      .status()
      .unwrap();
    assert!(status.success());

    let handler = RarHandler;
    let entries = handler.list(&archive_path, None, None).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"file1.txt"));
    assert!(names.contains(&"subdir"));
    assert!(names.contains(&"subdir/file2.txt"));
  }
}
