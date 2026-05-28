use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::action::Action;
use crate::context::AppContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileId {
  FileList,
  Preview,
  StatusBar,
  Properties,
}

impl TileId {
  pub const FOCUS_ORDER: &'static [TileId] =
    &[TileId::FileList, TileId::Properties, TileId::Preview];

  pub fn next_focus(current: TileId) -> TileId {
    let pos = Self::FOCUS_ORDER
      .iter()
      .position(|id| *id == current)
      .unwrap_or(0);
    Self::FOCUS_ORDER[(pos + 1) % Self::FOCUS_ORDER.len()]
  }
}

#[derive(Debug, Clone)]
pub enum InputEvent {
  Key(KeyEvent),
  Mouse(MouseEvent),
}

pub trait Tile {
  #[allow(dead_code)]
  fn tile_id(&self) -> TileId;
  fn visible(&self) -> bool {
    true
  }
  fn focusable(&self) -> bool {
    true
  }
  fn render(&mut self, area: Rect, frame: &mut Frame, ctx: &AppContext);
  fn handle_input(&mut self, event: &InputEvent, ctx: &AppContext) -> Vec<Action>;
  fn handle_action(&mut self, _action: &Action, _ctx: &AppContext) {}
  fn reset_with_entries(&mut self, _entries: &[decmp_core::ArchiveEntry]) {}
  fn clear_cache(&mut self) {}
}
