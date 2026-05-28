use std::path::PathBuf;

use decmp_core::DecmpError;

use crate::app::App;

impl App {
  pub fn extract_selected(&mut self) {
    let Some((name, is_dir)) = self.selected_info() else {
      return;
    };
    if &name == ".." || is_dir {
      return;
    }
    self.extract_by_name(&name);
  }

  fn extract_by_name(&mut self, name: &str) {
    let full_name = self.build_full_path(name);
    self.extract_dest_input = super::current_dir_str();
    self.pending_extract_entries = Some(vec![full_name]);
    self.mode = super::Mode::ExtractDest;
  }

  pub fn extract_all(&mut self) {
    self.extract_dest_input = super::current_dir_str();
    self.pending_extract_entries = None;
    self.mode = super::Mode::ExtractDest;
  }

  pub fn confirm_extract(&mut self) {
    let dest = PathBuf::from(&self.extract_dest_input);
    if !dest.exists()
      && let Err(e) = std::fs::create_dir_all(&dest)
    {
      self.status_msg = Some(format!("Error creating dir: {e}"));
      self.mode = super::Mode::Browse;
      return;
    }

    let result = if let Some(ref entries) = self.pending_extract_entries {
      let refs: Vec<&str> = entries.iter().map(|s| s.as_str()).collect();
      self.archive.handler.extract_entries(
        &self.archive.path,
        &refs,
        &dest,
        self.password.as_deref(),
        self.encoding.as_deref(),
      )
    } else {
      self.archive.handler.extract(
        &self.archive.path,
        &dest,
        self.password.as_deref(),
        self.encoding.as_deref(),
      )
    };

    match result {
      Ok(()) => {
        self.status_msg = Some(format!("Extracted to {}", dest.display()));
        self.mode = super::Mode::Browse;
        self.pending_extract_entries = None;
      }
      Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
        self.password_input.clear();
        self.pending_action = Some(super::PendingAction::Extract);
        self.mode = super::Mode::Password;
      }
      Err(e) => {
        self.status_msg = Some(format!("Error: {e}"));
        self.mode = super::Mode::Browse;
        self.pending_extract_entries = None;
      }
    }
  }

  pub fn show_properties(&mut self) {
    let Some((name, is_dir)) = self.selected_info() else {
      return;
    };
    if &name == ".." || is_dir {
      return;
    }
    let full_name = self.build_full_path(&name);
    if let Some(entry) = self.archive.entries.iter().find(|e| e.name == full_name) {
      self.properties_entry = Some(entry.clone());
      self.mode = super::Mode::Properties;
    }
  }
}
