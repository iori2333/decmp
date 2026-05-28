use std::collections::HashMap;
use std::path::{Path, PathBuf};

use decmp_core::{DecmpError, auto_detect_encoding};

use crate::action::Action;
use crate::action::PopupType;
use crate::context::{
  AppContext, ArchiveState, MAX_PREVIEW_BYTES, MAX_PREVIEW_CHARS, Mode, PendingAction, SidePreview,
};
use crate::layout::LayoutNode;
use crate::popup::Popup;
use crate::popups;
use crate::tile::{InputEvent, Tile, TileId};
use crate::tiles;
use crate::tree::DirTree;

pub struct App {
  pub layout: LayoutNode,
  pub tiles: HashMap<TileId, Box<dyn Tile>>,
  pub ctx: AppContext,
  pub active_popup: Option<Box<dyn Popup>>,
  pub last_frame_area: ratatui::layout::Rect,
  extract_dest_for_retry: Option<PathBuf>,
}

impl App {
  pub fn new(
    archive_path: PathBuf,
    handler: Box<dyn decmp_core::ArchiveHandler>,
    mut entries: Vec<decmp_core::ArchiveEntry>,
  ) -> Self {
    normalize_entry_names(&mut entries);
    let tree = DirTree::from_entries(&entries);

    let ctx = AppContext {
      archive: ArchiveState {
        path: archive_path,
        handler,
        entries: entries.clone(),
        tree,
      },
      focus: TileId::FileList,
      password: None,
      encoding: None,
      mode: Mode::Browse,
      status_msg: None,
      should_quit: false,
      pending_extract_entries: None,
      pending_action: None,
    };

    let mut file_list = tiles::file_list::FileListTile::new();
    file_list.init_entries(&entries);

    let mut tiles: HashMap<TileId, Box<dyn Tile>> = HashMap::new();
    tiles.insert(TileId::FileList, Box::new(file_list));
    tiles.insert(
      TileId::Preview,
      Box::new(tiles::preview::PreviewTile::new()),
    );
    tiles.insert(
      TileId::Properties,
      Box::new(tiles::properties::PropertiesTile::new()),
    );
    tiles.insert(
      TileId::StatusBar,
      Box::new(tiles::status_bar::StatusBarTile::new()),
    );

    App {
      layout: LayoutNode::default_layout(),
      tiles,
      ctx,
      active_popup: None,
      last_frame_area: ratatui::layout::Rect::default(),
      extract_dest_for_retry: None,
    }
  }

  pub fn new_password_required(
    archive_path: PathBuf,
    handler: Box<dyn decmp_core::ArchiveHandler>,
  ) -> Self {
    let mut app = Self::new(archive_path, handler, Vec::new());
    app.ctx.mode = Mode::Password;
    app.ctx.pending_action = Some(PendingAction::InitialLoad);
    app.ctx.status_msg = Some("Password required to open archive".to_string());
    app.active_popup = Some(Box::new(popups::password::PasswordPopup::new()));
    app
  }

  pub fn dispatch_all_input(&mut self, event: &InputEvent) {
    if self.ctx.should_quit {
      return;
    }

    if self.active_popup.is_some() {
      self.dispatch_popup_input(event);
    } else {
      self.dispatch_tiles_input(event);
    }
  }

  fn dispatch_popup_input(&mut self, event: &InputEvent) {
    let Self {
      active_popup,
      ctx,
      tiles,
      extract_dest_for_retry,
      ..
    } = self;

    let actions = active_popup
      .as_mut()
      .map(|p| p.handle_input(event, ctx))
      .unwrap_or_default();

    Self::process_actions_inner(ctx, tiles, extract_dest_for_retry, active_popup, actions);
  }

  fn dispatch_tiles_input(&mut self, event: &InputEvent) {
    match event {
      InputEvent::Key(key) => self.dispatch_key(key),
      InputEvent::Mouse(mouse) => self.dispatch_mouse(mouse),
    }
  }

  fn dispatch_key(&mut self, key: &crossterm::event::KeyEvent) {
    use crossterm::event::KeyCode;

    if key.modifiers != crossterm::event::KeyModifiers::NONE {
      return;
    }

    match key.code {
      KeyCode::Char('q') => {
        self.ctx.should_quit = true;
      }
      KeyCode::Tab => {
        self.ctx.focus = TileId::next_focus(self.ctx.focus, &self.tiles);
      }
      KeyCode::Char('?') => {
        let action = Action::ShowPopup(PopupType::Help);
        let Self {
          ctx,
          tiles,
          extract_dest_for_retry,
          active_popup,
          ..
        } = self;
        Self::process_actions_inner(
          ctx,
          tiles,
          extract_dest_for_retry,
          active_popup,
          vec![action],
        );
      }
      KeyCode::Esc | KeyCode::Backspace => {
        self.dispatch_to(&TileId::FileList, &InputEvent::Key(*key));
      }
      KeyCode::PageUp => {
        let focus = self.ctx.focus;
        self.dispatch_to(&focus, &InputEvent::Key(*key));
      }
      KeyCode::PageDown => {
        let focus = self.ctx.focus;
        self.dispatch_to(&focus, &InputEvent::Key(*key));
      }
      _ => {
        let focus = self.ctx.focus;
        self.dispatch_to(&focus, &InputEvent::Key(*key));
      }
    }
  }

  fn dispatch_to(&mut self, tile_id: &TileId, event: &InputEvent) {
    let Self {
      tiles,
      ctx,
      extract_dest_for_retry,
      active_popup,
      ..
    } = self;

    let actions = tiles
      .get_mut(tile_id)
      .map(|t| t.handle_input(event, ctx))
      .unwrap_or_default();

    Self::process_actions_inner(ctx, tiles, extract_dest_for_retry, active_popup, actions);
  }

  fn dispatch_mouse(&mut self, mouse: &crossterm::event::MouseEvent) {
    use crossterm::event::{MouseButton, MouseEventKind};
    let pos = ratatui::layout::Position::new(mouse.column, mouse.row);
    let tile_areas = self.layout.resolve(self.last_frame_area);

    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
      for (id, area) in &tile_areas {
        if in_area(pos, *area)
          && let Some(tile) = self.tiles.get(id)
          && tile.focusable()
        {
          self.ctx.focus = *id;
          break;
        }
      }
    }

    let event = InputEvent::Mouse(*mouse);

    let Self {
      tiles,
      ctx,
      extract_dest_for_retry,
      active_popup,
      ..
    } = self;

    let mut actions = Vec::new();
    for (id, tile) in tiles.iter_mut() {
      if let Some(area) = tile_areas.get(id)
        && in_area(pos, *area)
        && tile.visible()
      {
        actions.extend(tile.handle_input(&event, ctx));
      }
    }

    Self::process_actions_inner(ctx, tiles, extract_dest_for_retry, active_popup, actions);
  }

  fn process_actions_inner(
    ctx: &mut AppContext,
    tiles: &mut HashMap<TileId, Box<dyn Tile>>,
    extract_dest_for_retry: &mut Option<PathBuf>,
    active_popup: &mut Option<Box<dyn Popup>>,
    actions: Vec<Action>,
  ) {
    for action in actions {
      match action {
        Action::Quit => {
          ctx.should_quit = true;
        }
        Action::SelectionChanged { .. } => {
          for tile in tiles.values_mut() {
            tile.handle_action(&action, ctx);
          }
        }
        Action::RequestPreviewLoad { ref full_name } => {
          let preview = Self::load_preview_from_archive(
            &ctx.archive.path,
            ctx.archive.handler.as_ref(),
            full_name,
            ctx.password.as_deref(),
            ctx.encoding.as_deref(),
          );
          match preview {
            Ok(preview) => {
              let preview_action = Action::PreviewLoaded {
                full_name: full_name.clone(),
                preview,
              };
              for tile in tiles.values_mut() {
                tile.handle_action(&preview_action, ctx);
              }
            }
            Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
              ctx.pending_action = Some(PendingAction::Extract);
              ctx.mode = Mode::Password;
              *active_popup = Some(Box::new(popups::password::PasswordPopup::new()));
            }
            Err(e) => {
              ctx.status_msg = Some(format!("Error: {e}"));
            }
          }
        }
        Action::PreviewLoaded { .. } => {
          for tile in tiles.values_mut() {
            tile.handle_action(&action, ctx);
          }
        }
        Action::ShowPopup(popup_type) => match popup_type {
          PopupType::Password => {
            ctx.mode = Mode::Password;
            *active_popup = Some(Box::new(popups::password::PasswordPopup::new()));
          }
          PopupType::ExtractDest => {
            ctx.mode = Mode::ExtractDest;
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            *active_popup = Some(Box::new(popups::extract_dest::ExtractDestPopup::new(cwd)));
          }
          PopupType::Encoding => {
            ctx.mode = Mode::Encoding;
            let current = ctx.encoding.clone();
            *active_popup = Some(Box::new(popups::encoding::EncodingPopup::new(
              current.as_deref(),
            )));
          }
          PopupType::Help => {
            ctx.mode = Mode::Help;
            *active_popup = Some(Box::new(popups::help::HelpPopup::new()));
          }
        },
        Action::ClosePopup => {
          ctx.mode = Mode::Browse;
          *active_popup = None;
        }
        Action::StartExtract { full_name } => {
          ctx.pending_extract_entries = Some(vec![full_name.clone()]);
          ctx.mode = Mode::ExtractDest;
          let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
          let default_dest = cwd.join(&full_name);
          *active_popup = Some(Box::new(popups::extract_dest::ExtractDestPopup::new(
            default_dest,
          )));
        }
        Action::StartExtractAll => {
          ctx.pending_extract_entries = None;
          ctx.mode = Mode::ExtractDest;
          let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
          *active_popup = Some(Box::new(popups::extract_dest::ExtractDestPopup::new(cwd)));
        }
        Action::ConfirmExtract { dest } => {
          *extract_dest_for_retry = Some(dest.clone());
          Self::perform_extraction(ctx, &dest);
          if ctx.mode == Mode::Password {
            *active_popup = Some(Box::new(popups::password::PasswordPopup::new()));
          } else {
            *active_popup = None;
          }
        }
        Action::RequestEncodingInput => {
          ctx.mode = Mode::Encoding;
          let current = ctx.encoding.clone();
          *active_popup = Some(Box::new(popups::encoding::EncodingPopup::new(
            current.as_deref(),
          )));
        }
        Action::RequestEncodingReload(enc) => {
          Self::reload_with_encoding(ctx, tiles, &enc);
        }
        Action::PasswordSubmitted(pw) => {
          ctx.password = Some(pw);
          match ctx.pending_action.clone() {
            Some(PendingAction::InitialLoad) => {
              match ctx.archive.handler.list(
                &ctx.archive.path,
                ctx.password.as_deref(),
                ctx.encoding.as_deref(),
              ) {
                Ok(entries) => {
                  ctx.pending_action = None;
                  ctx.mode = Mode::Browse;
                  ctx.status_msg = None;
                  *active_popup = None;

                  let mut normal_entries = entries;
                  normalize_entry_names(&mut normal_entries);
                  ctx.archive.entries = normal_entries.clone();
                  ctx.archive.tree = DirTree::from_entries(&normal_entries);

                  if let Some(tile) = tiles.get_mut(&TileId::FileList) {
                    tile.reset_with_entries(&normal_entries);
                  }
                  if let Some(tile) = tiles.get_mut(&TileId::Preview) {
                    tile.clear_cache();
                  }
                }
                Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
                  ctx.password = None;
                  ctx.status_msg = Some("Wrong password, try again".to_string());
                }
                Err(e) => {
                  ctx.pending_action = None;
                  ctx.mode = Mode::Browse;
                  ctx.status_msg = Some(format!("Error: {e}"));
                  *active_popup = None;
                }
              }
            }
            Some(PendingAction::Extract) => {
              ctx.pending_action = None;
              ctx.mode = Mode::Browse;
              *active_popup = None;
              if let Some(dest) = extract_dest_for_retry.take() {
                Self::perform_extraction(ctx, &dest);
              }
            }
            None => {
              ctx.mode = Mode::Browse;
              ctx.status_msg = Some("Password set".to_string());
              *active_popup = None;
            }
          }
        }
      }
    }
  }

  fn load_preview_from_archive(
    archive_path: &Path,
    handler: &dyn decmp_core::ArchiveHandler,
    full_name: &str,
    password: Option<&str>,
    encoding: Option<&str>,
  ) -> Result<SidePreview, DecmpError> {
    let bytes = handler.read_entry(
      archive_path,
      full_name,
      password,
      encoding,
      Some(MAX_PREVIEW_BYTES),
    )?;

    let is_byte_truncated = bytes.len() >= MAX_PREVIEW_BYTES;

    if crate::tree::is_binary_content(&bytes) {
      return Ok(SidePreview::binary(full_name));
    }

    let (text, enc_detected) = decode_preview_bytes(&bytes);

    let text = if text.chars().count() > MAX_PREVIEW_CHARS {
      (text.chars().take(MAX_PREVIEW_CHARS).collect(), true)
    } else {
      (text, is_byte_truncated)
    };

    let lines: Vec<String> = text.0.lines().map(String::from).collect();
    let name = std::path::Path::new(full_name)
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or(full_name);
    let highlighted = crate::highlight::highlight_text(&text.0, name);

    Ok(SidePreview::file(
      name,
      lines,
      highlighted,
      text.1,
      enc_detected,
    ))
  }

  fn perform_extraction(ctx: &mut AppContext, dest: &Path) {
    if let Some(ref entries) = ctx.pending_extract_entries {
      let refs: Vec<&str> = entries.iter().map(|s| s.as_str()).collect();
      let result = decmp_core::extract_by_paths(
        ctx.archive.handler.as_ref(),
        &ctx.archive.path,
        &ctx.archive.entries,
        &refs,
        dest,
        ctx.password.as_deref(),
        ctx.encoding.as_deref(),
      );
      match result {
        Ok(()) => {
          ctx.status_msg = Some(format!("Extracted to {}", dest.display()));
          ctx.mode = Mode::Browse;
          ctx.pending_extract_entries = None;
        }
        Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
          ctx.pending_action = Some(PendingAction::Extract);
          ctx.mode = Mode::Password;
        }
        Err(e) => {
          ctx.status_msg = Some(format!("Error: {e}"));
          ctx.mode = Mode::Browse;
          ctx.pending_extract_entries = None;
        }
      }
      return;
    }

    if !dest.exists()
      && let Err(e) = std::fs::create_dir_all(dest)
    {
      ctx.status_msg = Some(format!("Error creating dir: {e}"));
      ctx.mode = Mode::Browse;
      return;
    }

    let result = ctx.archive.handler.extract(
      &ctx.archive.path,
      dest,
      ctx.password.as_deref(),
      ctx.encoding.as_deref(),
    );

    match result {
      Ok(()) => {
        ctx.status_msg = Some(format!("Extracted to {}", dest.display()));
        ctx.mode = Mode::Browse;
        ctx.pending_extract_entries = None;
      }
      Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
        ctx.pending_action = Some(PendingAction::Extract);
        ctx.mode = Mode::Password;
      }
      Err(e) => {
        ctx.status_msg = Some(format!("Error: {e}"));
        ctx.mode = Mode::Browse;
        ctx.pending_extract_entries = None;
      }
    }
  }

  fn reload_with_encoding(
    ctx: &mut AppContext,
    tiles: &mut HashMap<TileId, Box<dyn Tile>>,
    enc: &str,
  ) {
    let actual_enc = if enc.is_empty() {
      None
    } else {
      Some(enc.to_string())
    };
    ctx.encoding = actual_enc.clone();

    match ctx.archive.handler.list(
      &ctx.archive.path,
      ctx.password.as_deref(),
      ctx.encoding.as_deref(),
    ) {
      Ok(entries) => {
        let mut normal_entries = entries;
        normalize_entry_names(&mut normal_entries);
        ctx.archive.entries = normal_entries.clone();
        ctx.archive.tree = DirTree::from_entries(&normal_entries);

        if let Some(tile) = tiles.get_mut(&TileId::FileList) {
          tile.reset_with_entries(&normal_entries);
        }
        if let Some(tile) = tiles.get_mut(&TileId::Preview) {
          tile.clear_cache();
        }

        ctx.mode = Mode::Browse;
        let detected = if actual_enc.is_none() { " (auto)" } else { "" };
        ctx.status_msg = Some(format!(
          "Encoding: {}{detected}",
          ctx.encoding.as_deref().unwrap_or("auto")
        ));
      }
      Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
        ctx.password = None;
        ctx.pending_action = Some(PendingAction::InitialLoad);
        ctx.mode = Mode::Password;
        ctx.status_msg = Some("Password required to reload".to_string());
      }
      Err(e) => {
        ctx.status_msg = Some(format!("Error: {e}"));
      }
    }
  }
}

fn in_area(pos: ratatui::layout::Position, area: ratatui::layout::Rect) -> bool {
  pos.x >= area.x && pos.x < area.x + area.width && pos.y >= area.y && pos.y < area.y + area.height
}

fn decode_preview_bytes(bytes: &[u8]) -> (String, Option<String>) {
  if bytes.is_empty() {
    return (String::new(), None);
  }

  if std::str::from_utf8(bytes).is_ok() {
    return (String::from_utf8_lossy(bytes).into_owned(), None);
  }

  if let Some(enc) = auto_detect_encoding(&[bytes])
    && let Ok(decoded) = decmp_core::encoding::decode_filename(bytes, enc)
  {
    return (decoded, Some(enc.to_string()));
  }

  (String::from_utf8_lossy(bytes).into_owned(), None)
}

fn normalize_entry_names(entries: &mut [decmp_core::ArchiveEntry]) {
  for entry in entries {
    if let Some(stripped) = entry.name.strip_prefix("./") {
      entry.name = stripped.to_string();
    }
  }
}
