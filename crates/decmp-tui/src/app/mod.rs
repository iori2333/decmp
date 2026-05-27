mod extract;
mod navigation;
mod password;
mod preview;
mod scroll;

use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::{ListState, ScrollbarState};

use decmp_core::{ArchiveEntry, ArchiveHandler};

use crate::tree::DirTree;

pub const MAX_PREVIEW_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
  Browse,
  Password,
  ExtractDest,
  Properties,
  Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
  Left,
  Right,
}

#[derive(Clone)]
pub(crate) enum PendingAction {
  Extract,
  InitialLoad,
}

#[derive(Default, Clone)]
pub struct SidePreview {
  pub lines: Vec<String>,
  pub highlighted: Vec<Vec<Span<'static>>>,
  pub name: String,
  pub is_binary: bool,
  pub is_dir: bool,
  pub dir_entries: Vec<String>,
}

pub struct ArchiveState {
  pub path: PathBuf,
  pub handler: Box<dyn ArchiveHandler>,
  pub entries: Vec<ArchiveEntry>,
  pub tree: DirTree,
}

pub struct NavState {
  pub current_path: Vec<String>,
  pub list_state: ListState,
  pub focus: Focus,
}

pub struct PreviewState {
  pub content: SidePreview,
  pub scroll: usize,
  pub horizontal_scroll: usize,
  pub cache: HashMap<String, SidePreview>,
  pub scrollbar: ScrollbarState,
  pub area: Rect,
}

pub struct App {
  pub archive: ArchiveState,
  pub nav: NavState,
  pub preview: PreviewState,
  pub mode: Mode,
  pub password: Option<String>,
  pub password_input: String,
  pub extract_dest_input: String,
  pub status_msg: Option<String>,
  pub should_quit: bool,
  pub properties_entry: Option<ArchiveEntry>,
  pub help_scroll: usize,
  pub file_list_area: Rect,
  pub file_list_scroll: ScrollbarState,
  pending_extract_entries: Option<Vec<String>>,
  pending_action: Option<PendingAction>,
}

impl App {
  pub fn new(
    archive_path: PathBuf,
    handler: Box<dyn ArchiveHandler>,
    mut entries: Vec<ArchiveEntry>,
  ) -> Self {
    normalize_entry_names(&mut entries);
    let tree = DirTree::from_entries(&entries);
    let mut list_state = ListState::default();
    if !tree.sorted_entries().is_empty() {
      list_state.select(Some(0));
    }
    Self {
      archive: ArchiveState {
        path: archive_path,
        handler,
        entries,
        tree,
      },
      nav: NavState {
        current_path: Vec::new(),
        list_state,
        focus: Focus::Left,
      },
      preview: PreviewState {
        content: SidePreview::default(),
        scroll: 0,
        horizontal_scroll: 0,
        cache: HashMap::new(),
        scrollbar: ScrollbarState::default(),
        area: Rect::default(),
      },
      mode: Mode::Browse,
      password: None,
      password_input: String::new(),
      extract_dest_input: String::from("."),
      status_msg: None,
      should_quit: false,
      properties_entry: None,
      help_scroll: 0,
      file_list_area: Rect::default(),
      file_list_scroll: ScrollbarState::default(),
      pending_extract_entries: None,
      pending_action: None,
    }
  }

  pub fn new_password_required(archive_path: PathBuf, handler: Box<dyn ArchiveHandler>) -> Self {
    let mut app = Self::new(archive_path, handler, Vec::new());
    app.mode = Mode::Password;
    app.pending_action = Some(PendingAction::InitialLoad);
    app.status_msg = Some("Password required to open archive".to_string());
    app
  }

  pub fn reload_entries(&mut self, mut entries: Vec<ArchiveEntry>) {
    normalize_entry_names(&mut entries);
    self.archive.entries = entries;
    self.archive.tree = DirTree::from_entries(&self.archive.entries);
    self.nav.current_path.clear();
    self.nav.list_state = ListState::default();
    if !self.archive.tree.sorted_entries().is_empty() {
      self.nav.list_state.select(Some(0));
    }
    self.update_side_preview();
  }

  pub fn toggle_focus(&mut self) {
    self.nav.focus = match self.nav.focus {
      Focus::Left => Focus::Right,
      Focus::Right => Focus::Left,
    };
  }

  fn build_full_path(&self, name: &str) -> String {
    if self.nav.current_path.is_empty() {
      name.to_string()
    } else {
      format!("{}/{name}", self.nav.current_path.join("/"))
    }
  }

  pub fn update_file_list_scrollbar(&mut self, visible_height: usize) {
    let total = self.display_entries().len();
    let pos = self.nav.list_state.selected().unwrap_or(0);
    self.file_list_scroll = self
      .file_list_scroll
      .content_length(total)
      .position(pos)
      .viewport_content_length(visible_height);
  }

  pub fn update_preview_scrollbar(&mut self, visible_height: usize) {
    let total = self.preview.content.lines.len();
    if total == 0 {
      self.preview.scrollbar = ScrollbarState::default();
      return;
    }
    self.preview.scrollbar = self
      .preview
      .scrollbar
      .content_length(total)
      .position(self.preview.scroll)
      .viewport_content_length(visible_height);
  }
}

fn current_dir_str() -> String {
  std::env::current_dir()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|_| ".".to_string())
}

fn normalize_entry_names(entries: &mut [ArchiveEntry]) {
  for entry in entries {
    if let Some(stripped) = entry.name.strip_prefix("./") {
      entry.name = stripped.to_string();
    }
  }
}
