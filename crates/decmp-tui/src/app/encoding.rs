use decmp_core::DecmpError;

use crate::app::App;

impl App {
  pub fn open_encoding(&mut self) {
    self.encoding_input = self.encoding.clone().unwrap_or_default();
    self.mode = super::Mode::Encoding;
  }

  pub fn submit_encoding(&mut self) {
    let enc = if self.encoding_input.trim().is_empty() {
      None
    } else {
      Some(self.encoding_input.trim().to_string())
    };
    self.encoding = enc;
    self.encoding_input.clear();
    self.mode = super::Mode::Browse;
    self.reload_with_encoding();
  }

  fn reload_with_encoding(&mut self) {
    match self.archive.handler.list(
      &self.archive.path,
      self.password.as_deref(),
      self.encoding.as_deref(),
    ) {
      Ok(entries) => {
        self.preview.content = crate::app::SidePreview::default();
        self.preview.cache.clear();
        self.reload_entries(entries);
        let detected = if self.encoding.is_none() {
          " (auto)"
        } else {
          ""
        };
        self.status_msg = Some(format!(
          "Encoding: {}{detected}",
          self.encoding.as_deref().unwrap_or("auto")
        ));
      }
      Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
        self.password = None;
        self.password_input.clear();
        self.pending_action = Some(super::PendingAction::InitialLoad);
        self.mode = super::Mode::Password;
        self.status_msg = Some("Password required to reload".to_string());
      }
      Err(e) => {
        self.status_msg = Some(format!("Error: {e}"));
      }
    }
  }
}
