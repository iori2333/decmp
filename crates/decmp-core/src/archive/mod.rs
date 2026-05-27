pub mod bzip2;
pub mod gzip;
pub mod sevenz;
pub mod tar;
pub mod xz;
pub mod zip;
pub mod zstd;

use std::path::{Path, PathBuf};

use crate::error::{DecmpError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
  Zip,
  SevenZ,
  Tar,
  TarGz,
  TarXz,
  TarZst,
  TarBz2,
  TarLzma,
  Gz,
  Zst,
  Xz,
  Lzma,
  Bz2,
}

impl Format {
  pub fn extension(&self) -> &'static str {
    match self {
      Self::Zip => "zip",
      Self::SevenZ => "7z",
      Self::Tar => "tar",
      Self::TarGz => "tar.gz",
      Self::TarXz => "tar.xz",
      Self::TarZst => "tar.zst",
      Self::TarBz2 => "tar.bz2",
      Self::TarLzma => "tar.lzma",
      Self::Gz => "gz",
      Self::Zst => "zst",
      Self::Xz => "xz",
      Self::Lzma => "lzma",
      Self::Bz2 => "bz2",
    }
  }
}

impl std::fmt::Display for Format {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.extension())
  }
}

impl std::str::FromStr for Format {
  type Err = DecmpError;

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "zip" => Ok(Self::Zip),
      "7z" => Ok(Self::SevenZ),
      "tar" => Ok(Self::Tar),
      "tar.gz" | "tgz" => Ok(Self::TarGz),
      "tar.xz" | "txz" => Ok(Self::TarXz),
      "tar.zst" | "tar.zstd" => Ok(Self::TarZst),
      "tar.bz2" | "tbz2" | "tbz" => Ok(Self::TarBz2),
      "tar.lzma" => Ok(Self::TarLzma),
      "gz" | "gzip" => Ok(Self::Gz),
      "zst" | "zstd" => Ok(Self::Zst),
      "xz" => Ok(Self::Xz),
      "lzma" => Ok(Self::Lzma),
      "bz2" | "bzip2" => Ok(Self::Bz2),
      _ => Err(DecmpError::UnsupportedFormat(s.to_string())),
    }
  }
}

pub fn detect_format(path: &Path) -> Result<Format> {
  let name = path
    .file_name()
    .ok_or_else(|| DecmpError::UnsupportedFormat("empty filename".into()))?
    .to_string_lossy()
    .to_lowercase();

  if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
    return Ok(Format::TarGz);
  }
  if name.ends_with(".tar.xz") || name.ends_with(".txz") {
    return Ok(Format::TarXz);
  }
  if name.ends_with(".tar.zst") || name.ends_with(".tar.zstd") {
    return Ok(Format::TarZst);
  }
  if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz") {
    return Ok(Format::TarBz2);
  }
  if name.ends_with(".tar.lzma") {
    return Ok(Format::TarLzma);
  }

  let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());

  match ext.as_deref() {
    Some("zip") => Ok(Format::Zip),
    Some("7z") => Ok(Format::SevenZ),
    Some("tar") => Ok(Format::Tar),
    Some("gz") | Some("gzip") => Ok(Format::Gz),
    Some("zst") | Some("zstd") => Ok(Format::Zst),
    Some("xz") => Ok(Format::Xz),
    Some("lzma") => Ok(Format::Lzma),
    Some("bz2") => Ok(Format::Bz2),
    Some(other) => Err(DecmpError::UnsupportedFormat(other.to_string())),
    None => Err(DecmpError::UnsupportedFormat(
      "no file extension".to_string(),
    )),
  }
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
  pub name: String,
  pub size: u64,
  pub compressed_size: u64,
  pub is_dir: bool,
  pub method: String,
  pub modified: Option<String>,
}

impl std::fmt::Display for ArchiveEntry {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let dir_mark = if self.is_dir { "d" } else { "-" };
    write!(
      f,
      "{} {:>10} {:>10} {}",
      dir_mark, self.compressed_size, self.size, self.name
    )
  }
}

pub trait ArchiveHandler {
  fn list(
    &self,
    path: &Path,
    password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<Vec<ArchiveEntry>>;

  fn extract(
    &self,
    path: &Path,
    dest: &Path,
    password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<()>;

  fn create(
    &self,
    sources: &[PathBuf],
    dest: &Path,
    password: Option<&str>,
    level: Option<u32>,
  ) -> Result<()>;

  fn extract_entries(
    &self,
    archive_path: &Path,
    entry_names: &[&str],
    dest: &Path,
    password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<()> {
    let _ = entry_names;
    self.extract(archive_path, dest, password, encoding)
  }

  fn read_entry(
    &self,
    archive_path: &Path,
    entry_name: &str,
    password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<Vec<u8>> {
    let _ = (archive_path, entry_name, password, encoding);
    Err(DecmpError::InvalidArchive(
      "read_entry not supported for this format".to_string(),
    ))
  }
}

pub fn get_handler(format: &Format) -> Box<dyn ArchiveHandler> {
  match format {
    Format::Zip => Box::new(zip::ZipHandler),
    Format::SevenZ => Box::new(sevenz::SevenZHandler),
    Format::Tar
    | Format::TarGz
    | Format::TarXz
    | Format::TarZst
    | Format::TarBz2
    | Format::TarLzma => Box::new(tar::TarHandler::new(format.clone())),
    Format::Gz => Box::new(gzip::GzipHandler),
    Format::Zst => Box::new(zstd::ZstdHandler),
    Format::Xz | Format::Lzma => Box::new(xz::XzHandler),
    Format::Bz2 => Box::new(bzip2::Bzip2Handler),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_detect_format_zip() {
    assert_eq!(detect_format(Path::new("test.zip")).unwrap(), Format::Zip);
  }

  #[test]
  fn test_detect_format_tar_gz() {
    assert_eq!(
      detect_format(Path::new("archive.tar.gz")).unwrap(),
      Format::TarGz
    );
  }

  #[test]
  fn test_detect_format_tgz() {
    assert_eq!(
      detect_format(Path::new("archive.tgz")).unwrap(),
      Format::TarGz
    );
  }

  #[test]
  fn test_detect_format_tar_xz() {
    assert_eq!(
      detect_format(Path::new("archive.tar.xz")).unwrap(),
      Format::TarXz
    );
  }

  #[test]
  fn test_detect_format_tar_zst() {
    assert_eq!(
      detect_format(Path::new("archive.tar.zst")).unwrap(),
      Format::TarZst
    );
  }

  #[test]
  fn test_detect_format_tar_bz2() {
    assert_eq!(
      detect_format(Path::new("archive.tar.bz2")).unwrap(),
      Format::TarBz2
    );
  }

  #[test]
  fn test_detect_format_tar_lzma() {
    assert_eq!(
      detect_format(Path::new("archive.tar.lzma")).unwrap(),
      Format::TarLzma
    );
  }

  #[test]
  fn test_detect_format_7z() {
    assert_eq!(detect_format(Path::new("test.7z")).unwrap(), Format::SevenZ);
  }

  #[test]
  fn test_detect_format_single_gz() {
    assert_eq!(detect_format(Path::new("file.txt.gz")).unwrap(), Format::Gz);
  }

  #[test]
  fn test_detect_format_single_zst() {
    assert_eq!(detect_format(Path::new("file.zst")).unwrap(), Format::Zst);
  }

  #[test]
  fn test_detect_format_unsupported() {
    assert!(detect_format(Path::new("file.xyz")).is_err());
  }

  #[test]
  fn test_detect_format_no_extension() {
    assert!(detect_format(Path::new("noext")).is_err());
  }

  #[test]
  fn test_format_from_str() {
    assert_eq!("zip".parse::<Format>().unwrap(), Format::Zip);
    assert_eq!("tgz".parse::<Format>().unwrap(), Format::TarGz);
    assert_eq!("tar.gz".parse::<Format>().unwrap(), Format::TarGz);
    assert_eq!("7z".parse::<Format>().unwrap(), Format::SevenZ);
    assert!("rar".parse::<Format>().is_err());
  }

  #[test]
  fn test_format_display() {
    assert_eq!(Format::Zip.to_string(), "zip");
    assert_eq!(Format::TarGz.to_string(), "tar.gz");
  }

  #[test]
  fn test_format_extension() {
    assert_eq!(Format::Zip.extension(), "zip");
    assert_eq!(Format::SevenZ.extension(), "7z");
    assert_eq!(Format::TarGz.extension(), "tar.gz");
  }

  #[test]
  fn test_archive_entry_display() {
    let entry = ArchiveEntry {
      name: "test.txt".to_string(),
      size: 1024,
      compressed_size: 512,
      is_dir: false,
      method: "deflate".to_string(),
      modified: None,
    };
    let s = format!("{entry}");
    assert!(s.contains("test.txt"));
    assert!(s.contains("512"));
    assert!(s.contains("1024"));
  }

  #[test]
  fn test_archive_entry_display_dir() {
    let entry = ArchiveEntry {
      name: "mydir/".to_string(),
      size: 0,
      compressed_size: 0,
      is_dir: true,
      method: "stored".to_string(),
      modified: None,
    };
    let s = format!("{entry}");
    assert!(s.starts_with('d'));
  }
}
