use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::action::Action;
use crate::context::AppContext;
use crate::tile::{InputEvent, Tile, TileId};

pub struct StatusBarTile;

impl StatusBarTile {
  pub fn new() -> Self {
    Self
  }
}

impl Tile for StatusBarTile {
  fn tile_id(&self) -> TileId {
    TileId::StatusBar
  }

  fn focusable(&self) -> bool {
    false
  }

  fn render(&mut self, area: Rect, frame: &mut Frame, ctx: &AppContext) {
    let msg = ctx.status_msg.as_deref().unwrap_or(
      "[Enter]Open/Preview [e]Extract [E]All [o]Encoding [Tab]Focus [P/U]Scroll [?]Help [q]Quit",
    );
    let style = if ctx.status_msg.is_some() {
      Style::default().fg(Color::Yellow)
    } else {
      Style::default().fg(Color::DarkGray)
    };
    let text = Line::from(msg);
    frame.render_widget(Paragraph::new(text).style(style), area);
  }

  fn handle_input(&mut self, _event: &InputEvent, _ctx: &AppContext) -> Vec<Action> {
    vec![]
  }
}
