use crate::app::{App, MAX_PREVIEW_BYTES, SidePreview};
use crate::tree::DirNode;
use crate::ui::format_size;

use decmp_core::DecmpError;

impl SidePreview {
  fn binary(name: &str) -> Self {
    Self {
      name: name.to_string(),
      is_binary: true,
      ..Default::default()
    }
  }
  fn file(
    name: &str,
    lines: Vec<String>,
    highlighted: Vec<Vec<ratatui::text::Span<'static>>>,
  ) -> Self {
    Self {
      name: name.to_string(),
      lines,
      highlighted,
      ..Default::default()
    }
  }
  fn dir(name: &str, dir_entries: Vec<String>) -> Self {
    Self {
      name: name.to_string(),
      is_dir: true,
      dir_entries,
      ..Default::default()
    }
  }
  fn empty_with_name(name: &str) -> Self {
    Self {
      name: name.to_string(),
      ..Default::default()
    }
  }
}

impl App {
  pub fn load_preview(&mut self) {
    let Some((name, is_dir)) = self.selected_info() else {
      return;
    };
    if &name == ".." || is_dir {
      return;
    }

    let full_name = self.build_full_path(&name);
    if let Some(cached) = self.preview.cache.get(&full_name) {
      self.preview.content = cached.clone();
      self.preview.scroll = 0;
      return;
    }

    if !crate::tree::is_text_file(&name) {
      self.cache_and_show_preview(full_name, SidePreview::binary(&name));
      return;
    }

    match self.archive.handler.read_entry(
      &self.archive.path,
      &full_name,
      self.password.as_deref(),
      None,
    ) {
      Ok(bytes) => {
        let truncated = bytes.len() > MAX_PREVIEW_BYTES;
        let content = if truncated {
          &bytes[..MAX_PREVIEW_BYTES]
        } else {
          &bytes
        };
        let preview = match std::str::from_utf8(content) {
          Ok(text) => {
            let lines: Vec<String> = text.lines().map(String::from).collect();
            let highlighted = crate::highlight::highlight_text(text, &name);
            SidePreview::file(&name, lines, highlighted)
          }
          Err(_) => SidePreview::binary(&name),
        };
        self.cache_and_show_preview(full_name, preview);
      }
      Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
        self.password_input.clear();
        self.pending_action = Some(super::PendingAction::Extract);
        self.mode = super::Mode::Password;
      }
      Err(e) => {
        self.status_msg = Some(format!("Error: {e}"));
      }
    }
  }

  fn cache_and_show_preview(&mut self, full_name: String, preview: SidePreview) {
    self.preview.cache.insert(full_name, preview.clone());
    self.preview.content = preview;
    self.preview.scroll = 0;
  }

  pub fn update_side_preview(&mut self) {
    let Some((name, is_dir)) = self.selected_info() else {
      self.preview.content = SidePreview::default();
      return;
    };
    if &name == ".." {
      self.preview.content = SidePreview::default();
      return;
    }
    if is_dir {
      self.update_dir_preview();
      return;
    }
    let full_name = self.build_full_path(&name);
    self.preview.content = self
      .preview
      .cache
      .get(&full_name)
      .cloned()
      .unwrap_or_else(|| SidePreview::empty_with_name(&name));
  }

  fn update_dir_preview(&mut self) {
    let Some((name, _)) = self.selected_info() else {
      return;
    };
    if &name == ".." {
      return;
    }
    let tree = self.current_tree();
    if let Some(DirNode::Dir(subtree)) = tree.children.get(&name) {
      let entries: Vec<String> = subtree
        .sorted_entries()
        .iter()
        .map(|(n, node)| match node {
          DirNode::Dir(_) => format!("{n}/"),
          DirNode::File(f) => format!("{n:<40} {}", format_size(f.size)),
        })
        .collect();
      self.preview.content = SidePreview::dir(&name, entries);
    }
  }
}
