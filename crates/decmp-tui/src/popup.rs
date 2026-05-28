use ratatui::Frame;
use ratatui::layout::Rect;

use crate::action::Action;
use crate::action::PopupType;
use crate::context::AppContext;
use crate::tile::InputEvent;

pub trait Popup {
  #[allow(dead_code)]
  fn popup_type(&self) -> PopupType;
  fn render(&self, area: Rect, frame: &mut Frame, ctx: &AppContext);
  fn handle_input(&mut self, event: &InputEvent, ctx: &mut AppContext) -> Vec<Action>;
}
