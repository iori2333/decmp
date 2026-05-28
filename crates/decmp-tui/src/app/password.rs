use decmp_core::DecmpError;

use crate::app::App;

impl App {
  pub fn submit_password(&mut self) {
    self.password = Some(self.password_input.clone());
    match self.pending_action.clone() {
      Some(super::PendingAction::InitialLoad) => {
        match self.archive.handler.list(
          &self.archive.path,
          self.password.as_deref(),
          self.encoding.as_deref(),
        ) {
          Ok(entries) => {
            self.pending_action = None;
            self.reload_entries(entries);
            self.mode = super::Mode::Browse;
            self.status_msg = None;
          }
          Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
            self.password = None;
            self.password_input.clear();
            self.status_msg = Some("Wrong password, try again".to_string());
          }
          Err(e) => {
            self.pending_action = None;
            self.mode = super::Mode::Browse;
            self.status_msg = Some(format!("Error: {e}"));
          }
        }
      }
      Some(super::PendingAction::Extract) => {
        self.pending_action = None;
        self.mode = super::Mode::Browse;
        self.confirm_extract();
      }
      None => {
        self.mode = super::Mode::Browse;
        self.status_msg = Some("Password set".to_string());
      }
    }
  }

  pub fn cancel_mode(&mut self) {
    if matches!(self.pending_action, Some(super::PendingAction::InitialLoad)) {
      self.should_quit = true;
      return;
    }
    self.mode = super::Mode::Browse;
    self.password_input.clear();
    self.pending_extract_entries = None;
    self.pending_action = None;
  }
}
