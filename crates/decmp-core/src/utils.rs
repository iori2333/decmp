use std::path::Path;

pub fn format_size(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = KB * 1024;
  const GB: u64 = MB * 1024;

  if bytes >= GB {
    format!("{:.2} GB", bytes as f64 / GB as f64)
  } else if bytes >= MB {
    format!("{:.2} MB", bytes as f64 / MB as f64)
  } else if bytes >= KB {
    format!("{:.2} KB", bytes as f64 / KB as f64)
  } else {
    format!("{bytes} B")
  }
}

pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
  if !path.exists() {
    std::fs::create_dir_all(path)?;
  }
  Ok(())
}

pub fn file_stem(path: &Path) -> Option<String> {
  path.file_stem().map(|s| s.to_string_lossy().to_string())
}

pub fn strip_archive_extension(path: &Path) -> String {
  let name = path
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_default();

  let name = name
    .strip_suffix(".tar.gz")
    .or_else(|| name.strip_suffix(".tar.xz"))
    .or_else(|| name.strip_suffix(".tar.zst"))
    .or_else(|| name.strip_suffix(".tar.bz2"))
    .or_else(|| name.strip_suffix(".tar.lzma"))
    .or_else(|| name.strip_suffix(".tgz"))
    .or_else(|| name.strip_suffix(".txz"))
    .or_else(|| name.strip_suffix(".tbz2"))
    .or_else(|| name.strip_suffix(".tbz"))
    .or_else(|| name.strip_suffix(".gz"))
    .or_else(|| name.strip_suffix(".zst"))
    .or_else(|| name.strip_suffix(".xz"))
    .or_else(|| name.strip_suffix(".bz2"))
    .or_else(|| name.strip_suffix(".lzma"))
    .unwrap_or(&name);

  name.to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_format_size_bytes() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(1023), "1023 B");
  }

  #[test]
  fn test_format_size_kb() {
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(1536), "1.50 KB");
  }

  #[test]
  fn test_format_size_mb() {
    assert_eq!(format_size(1048576), "1.00 MB");
    assert_eq!(format_size(1048576 * 5), "5.00 MB");
  }

  #[test]
  fn test_format_size_gb() {
    assert_eq!(format_size(1073741824), "1.00 GB");
  }

  #[test]
  fn test_ensure_dir_creates_nested() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("a").join("b").join("c");
    ensure_dir(&nested).unwrap();
    assert!(nested.exists());
  }

  #[test]
  fn test_ensure_dir_existing() {
    let tmp = tempfile::tempdir().unwrap();
    ensure_dir(tmp.path()).unwrap();
  }

  #[test]
  fn test_strip_archive_extension_tar_gz() {
    assert_eq!(strip_archive_extension(Path::new("data.tar.gz")), "data");
  }

  #[test]
  fn test_strip_archive_extension_tgz() {
    assert_eq!(strip_archive_extension(Path::new("data.tgz")), "data");
  }

  #[test]
  fn test_strip_archive_extension_tar_xz() {
    assert_eq!(strip_archive_extension(Path::new("data.tar.xz")), "data");
  }

  #[test]
  fn test_strip_archive_extension_tar_zst() {
    assert_eq!(strip_archive_extension(Path::new("data.tar.zst")), "data");
  }

  #[test]
  fn test_strip_archive_extension_tar_bz2() {
    assert_eq!(strip_archive_extension(Path::new("data.tar.bz2")), "data");
  }

  #[test]
  fn test_strip_archive_extension_plain() {
    assert_eq!(strip_archive_extension(Path::new("data.zip")), "data.zip");
    assert_eq!(
      strip_archive_extension(Path::new("data.txt.gz")),
      "data.txt"
    );
    assert_eq!(
      strip_archive_extension(Path::new("data.txt.zst")),
      "data.txt"
    );
    assert_eq!(
      strip_archive_extension(Path::new("data.txt.xz")),
      "data.txt"
    );
    assert_eq!(
      strip_archive_extension(Path::new("data.txt.bz2")),
      "data.txt"
    );
    assert_eq!(
      strip_archive_extension(Path::new("data.txt.lzma")),
      "data.txt"
    );
  }

  #[test]
  fn test_file_stem() {
    assert_eq!(
      file_stem(Path::new("/path/to/file.txt")),
      Some("file".to_string())
    );
    assert_eq!(file_stem(Path::new("/")), None);
  }
}
