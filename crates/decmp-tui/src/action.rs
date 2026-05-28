use std::path::PathBuf;

use crate::context::SidePreview;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PopupType {
  Password,
  ExtractDest,
  Encoding,
  Help,
}

#[derive(Debug, Clone)]
pub enum Action {
  Quit,
  SelectionChanged {
    name: String,
    is_dir: bool,
    full_name: String,
    dir_entries: Option<Vec<String>>,
  },
  RequestPreviewLoad {
    full_name: String,
  },
  PreviewLoaded {
    full_name: String,
    preview: SidePreview,
  },
  ShowPopup(PopupType),
  ClosePopup,
  PasswordSubmitted(String),
  StartExtract {
    full_name: String,
  },
  StartExtractAll,
  ConfirmExtract {
    dest: PathBuf,
  },
  RequestEncodingInput,
  RequestEncodingReload(String),
}
