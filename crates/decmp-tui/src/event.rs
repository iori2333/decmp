use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;
use crate::tile::InputEvent;

pub fn handle_events(app: &mut App) -> std::io::Result<()> {
  if !event::poll(Duration::from_millis(100))? {
    return Ok(());
  }

  match event::read()? {
    Event::Key(key) if key.kind == KeyEventKind::Press => {
      app.dispatch_all_input(&InputEvent::Key(key));
    }
    Event::Mouse(mouse) => {
      app.dispatch_all_input(&InputEvent::Mouse(mouse));
    }
    _ => {}
  }
  Ok(())
}
