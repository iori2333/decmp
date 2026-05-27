use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DecmpError {
  #[error("unsupported archive format: {0}")]
  UnsupportedFormat(String),

  #[error("password required for encrypted archive")]
  PasswordRequired,

  #[error("wrong password or decryption failed")]
  WrongPassword,

  #[error("encoding error: {0}")]
  EncodingError(String),

  #[error("archive not found: {0}")]
  ArchiveNotFound(PathBuf),

  #[error("invalid archive: {0}")]
  InvalidArchive(String),

  #[error("no source files specified")]
  NoSources,

  #[error(transparent)]
  Io(#[from] std::io::Error),

  #[error(transparent)]
  Zip(#[from] zip::result::ZipError),

  #[error(transparent)]
  SevenZ(#[from] sevenz_rust::Error),

  #[error(transparent)]
  WalkDir(#[from] walkdir::Error),
}

pub type Result<T> = std::result::Result<T, DecmpError>;
