use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::archive::{ArchiveEntry, ArchiveHandler};
use crate::encoding;
use crate::error::{DecmpError, Result};
use crate::utils::ensure_dir;

fn convert_zip_error(e: zip::result::ZipError) -> DecmpError {
  match &e {
    zip::result::ZipError::UnsupportedArchive(msg)
      if *msg == zip::result::ZipError::PASSWORD_REQUIRED =>
    {
      DecmpError::PasswordRequired
    }
    zip::result::ZipError::InvalidPassword => DecmpError::WrongPassword,
    _ => DecmpError::Zip(e),
  }
}

pub struct ZipHandler;

fn compression_method(level: Option<u32>) -> CompressionMethod {
  match level {
    Some(0) => CompressionMethod::Stored,
    _ => CompressionMethod::Deflated,
  }
}

impl ArchiveHandler for ZipHandler {
  fn list(
    &self,
    path: &Path,
    password: Option<&str>,
    enc: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(convert_zip_error)?;
    let mut entries = Vec::new();

    for i in 0..archive.len() {
      let entry = if let Some(pw) = password {
        archive
          .by_index_decrypt(i, pw.as_bytes())
          .map_err(convert_zip_error)?
      } else {
        archive.by_index(i).map_err(convert_zip_error)?
      };
      let name = if let Some(encoding_name) = enc {
        let raw = entry.name_raw();
        encoding::decode_filename(raw, encoding_name)?
      } else {
        entry.name().to_string()
      };

      let modified = entry.last_modified().map(|dt| format!("{dt}"));

      entries.push(ArchiveEntry {
        name,
        size: entry.size(),
        compressed_size: entry.compressed_size(),
        is_dir: entry.is_dir(),
        method: format!("{:?}", entry.compression()),
        modified,
      });
    }

    Ok(entries)
  }

  fn extract(
    &self,
    path: &Path,
    dest: &Path,
    password: Option<&str>,
    enc: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;

    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(convert_zip_error)?;

    for i in 0..archive.len() {
      let mut entry = if let Some(pw) = password {
        archive
          .by_index_decrypt(i, pw.as_bytes())
          .map_err(convert_zip_error)?
      } else {
        archive.by_index(i).map_err(convert_zip_error)?
      };

      let name = if let Some(encoding_name) = enc {
        let raw = entry.name_raw();
        encoding::decode_filename(raw, encoding_name)?
      } else {
        entry.name().to_string()
      };

      let out_path = dest.join(&name);

      if entry.is_dir() {
        ensure_dir(&out_path)?;
      } else {
        if let Some(parent) = out_path.parent() {
          ensure_dir(parent)?;
        }
        let mut out_file = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
      }

      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = entry.unix_mode() {
          std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
        }
      }
    }

    Ok(())
  }

  fn create(
    &self,
    sources: &[PathBuf],
    dest: &Path,
    password: Option<&str>,
    level: Option<u32>,
  ) -> Result<()> {
    if sources.is_empty() {
      return Err(DecmpError::NoSources);
    }

    if let Some(parent) = dest.parent() {
      ensure_dir(parent)?;
    }

    let file = File::create(dest)?;
    let mut writer = ZipWriter::new(file);

    let options = SimpleFileOptions::default().compression_method(compression_method(level));

    for src in sources {
      if src.is_dir() {
        let base_name = src
          .file_name()
          .map(|s| s.to_string_lossy().to_string())
          .unwrap_or_default();

        for entry in WalkDir::new(src).min_depth(1) {
          let entry = entry?;
          let rel_path = entry
            .path()
            .strip_prefix(src)
            .map_err(|e| DecmpError::InvalidArchive(format!("path error: {e}")))?;
          let zip_path = Path::new(&base_name).join(rel_path);

          if entry.file_type().is_dir() {
            writer.add_directory(zip_path.to_string_lossy(), options)?;
          } else if entry.file_type().is_file() {
            let mut file = File::open(entry.path())?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            if let Some(pw) = password {
              writer
                .start_file(
                  zip_path.to_string_lossy(),
                  options.with_aes_encryption(zip::AesMode::Aes256, pw),
                )
                .map_err(DecmpError::Zip)?;
            } else {
              writer.start_file(zip_path.to_string_lossy(), options)?;
            }
            writer.write_all(&buf)?;
          }
        }
      } else {
        let file_name = src
          .file_name()
          .map(|s| s.to_string_lossy().to_string())
          .unwrap_or_default();
        let mut file = File::open(src)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        if let Some(pw) = password {
          writer
            .start_file(
              file_name,
              options.with_aes_encryption(zip::AesMode::Aes256, pw),
            )
            .map_err(DecmpError::Zip)?;
        } else {
          writer.start_file(file_name, options)?;
        }
        writer.write_all(&buf)?;
      }
    }

    writer.finish()?;
    Ok(())
  }

  fn extract_entries(
    &self,
    archive_path: &Path,
    entry_names: &[&str],
    dest: &Path,
    password: Option<&str>,
    enc: Option<&str>,
  ) -> Result<()> {
    ensure_dir(dest)?;
    let wanted: std::collections::HashSet<&str> = entry_names.iter().copied().collect();

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(convert_zip_error)?;

    for i in 0..archive.len() {
      let mut entry = if let Some(pw) = password {
        archive
          .by_index_decrypt(i, pw.as_bytes())
          .map_err(convert_zip_error)?
      } else {
        archive.by_index(i).map_err(convert_zip_error)?
      };

      let name = if let Some(encoding_name) = enc {
        let raw = entry.name_raw();
        encoding::decode_filename(raw, encoding_name)?
      } else {
        entry.name().to_string()
      };

      if !wanted.contains(name.as_str()) {
        continue;
      }

      let out_path = dest.join(&name);
      if entry.is_dir() {
        ensure_dir(&out_path)?;
      } else {
        if let Some(parent) = out_path.parent() {
          ensure_dir(parent)?;
        }
        let mut out_file = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
      }
    }
    Ok(())
  }

  fn read_entry(
    &self,
    archive_path: &Path,
    entry_name: &str,
    password: Option<&str>,
    enc: Option<&str>,
    max_bytes: Option<usize>,
  ) -> Result<Vec<u8>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(convert_zip_error)?;

    for i in 0..archive.len() {
      let mut entry = if let Some(pw) = password {
        archive
          .by_index_decrypt(i, pw.as_bytes())
          .map_err(convert_zip_error)?
      } else {
        archive.by_index(i).map_err(convert_zip_error)?
      };

      let name = if let Some(encoding_name) = enc {
        let raw = entry.name_raw();
        encoding::decode_filename(raw, encoding_name)?
      } else {
        entry.name().to_string()
      };

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
  }

  #[test]
  fn test_zip_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.zip");

    let handler = ZipHandler;
    handler.create(&[src_dir], &archive, None, None).unwrap();
    assert!(archive.exists());
    assert!(std::fs::metadata(&archive).unwrap().len() > 0);
  }

  #[test]
  fn test_zip_list() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.zip");

    let handler = ZipHandler;
    handler.create(&[src_dir], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    let names: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
    assert!(names.iter().any(|n| n.contains("file1.txt")));
    assert!(names.iter().any(|n| n.contains("file2.txt")));
  }

  #[test]
  fn test_zip_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    create_test_dir(&src_dir);
    let archive = tmp.path().join("test.zip");

    let handler = ZipHandler;
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
  fn test_zip_single_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "hello.txt", b"Hello!");
    let archive = tmp.path().join("test.zip");

    let handler = ZipHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");

    let out_dir = tmp.path().join("out");
    handler.extract(&archive, &out_dir, None, None).unwrap();
    assert_eq!(std::fs::read(out_dir.join("hello.txt")).unwrap(), b"Hello!");
  }

  #[test]
  fn test_zip_compression_levels() {
    let tmp = tempfile::tempdir().unwrap();
    let data = vec![b'A'; 10000];
    let src = write_temp_file(tmp.path(), "big.txt", &data);

    let handler = ZipHandler;

    let stored = tmp.path().join("stored.zip");
    handler
      .create(&[src.clone()], &stored, None, Some(0))
      .unwrap();

    let deflated = tmp.path().join("deflated.zip");
    handler.create(&[src], &deflated, None, None).unwrap();

    let stored_size = std::fs::metadata(&stored).unwrap().len();
    let deflated_size = std::fs::metadata(&deflated).unwrap().len();
    assert!(deflated_size < stored_size);
  }

  #[test]
  fn test_zip_encrypted_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "secret.txt", b"secret data");
    let archive = tmp.path().join("encrypted.zip");

    let handler = ZipHandler;
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
  fn test_zip_encrypted_wrong_password() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "secret.txt", b"secret data");
    let archive = tmp.path().join("encrypted.zip");

    let handler = ZipHandler;
    handler
      .create(&[src], &archive, Some("correct"), None)
      .unwrap();

    let out_dir = tmp.path().join("out");
    let result = handler.extract(&archive, &out_dir, Some("wrong"), None);
    assert!(result.is_err());
  }

  #[test]
  fn test_zip_empty_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "empty.txt", b"");
    let archive = tmp.path().join("test.zip");

    let handler = ZipHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].size, 0);
  }

  #[test]
  fn test_zip_no_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = tmp.path().join("test.zip");
    let handler = ZipHandler;
    let result = handler.create(&[], &archive, None, None);
    assert!(result.is_err());
  }

  #[test]
  fn test_zip_list_with_encoding() {
    let tmp = tempfile::tempdir().unwrap();
    let src = write_temp_file(tmp.path(), "test.txt", b"hello");
    let archive = tmp.path().join("test.zip");

    let handler = ZipHandler;
    handler.create(&[src], &archive, None, None).unwrap();

    let entries = handler.list(&archive, None, Some("utf-8")).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "test.txt");
  }
}
