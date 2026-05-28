use std::path::PathBuf;

use ratatui::text::Span;

use decmp_core::{ArchiveEntry, ArchiveHandler, Format};

use crate::tile::TileId;
use crate::tree::DirTree;

pub const MAX_PREVIEW_BYTES: usize = 128 * 1024;
pub const MAX_PREVIEW_CHARS: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
  Browse,
  Password,
  ExtractDest,
  Help,
  Encoding,
}

#[derive(Clone)]
pub enum PendingAction {
  Extract,
  InitialLoad,
}

#[derive(Debug, Default, Clone)]
pub struct SidePreview {
  pub lines: Vec<String>,
  pub highlighted: Vec<Vec<Span<'static>>>,
  pub name: String,
  pub is_binary: bool,
  pub is_dir: bool,
  pub is_truncated: bool,
  pub dir_entries: Vec<String>,
  pub encoding_detected: Option<String>,
}

impl SidePreview {
  pub fn binary(name: &str) -> Self {
    Self {
      name: name.to_string(),
      is_binary: true,
      ..Default::default()
    }
  }

  pub fn file(
    name: &str,
    lines: Vec<String>,
    highlighted: Vec<Vec<Span<'static>>>,
    is_truncated: bool,
    encoding_detected: Option<String>,
  ) -> Self {
    Self {
      name: name.to_string(),
      lines,
      highlighted,
      is_truncated,
      encoding_detected,
      ..Default::default()
    }
  }

  pub fn dir(name: &str, dir_entries: Vec<String>) -> Self {
    Self {
      name: name.to_string(),
      is_dir: true,
      dir_entries,
      ..Default::default()
    }
  }

  pub fn empty_with_name(name: &str) -> Self {
    Self {
      name: name.to_string(),
      ..Default::default()
    }
  }
}

pub struct ArchiveState {
  pub path: PathBuf,
  pub handler: Box<dyn ArchiveHandler>,
  pub entries: Vec<ArchiveEntry>,
  pub tree: DirTree,
  pub format: Format,
}

pub struct AppContext {
  pub archive: ArchiveState,
  pub focus: TileId,
  pub password: Option<String>,
  pub encoding: Option<String>,
  pub mode: Mode,
  pub status_msg: Option<String>,
  pub should_quit: bool,
  pub pending_extract_entries: Option<Vec<String>>,
  pub pending_action: Option<PendingAction>,
}
