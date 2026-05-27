use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::layout::Position;

use crate::app::{App, Focus, Mode};

pub fn handle_events(app: &mut App) -> std::io::Result<()> {
  if !event::poll(Duration::from_millis(100))? {
    return Ok(());
  }

  match event::read()? {
    Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key.code),
    Event::Mouse(mouse) => handle_mouse(app, mouse),
    _ => {}
  }
  Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) {
  match app.mode {
    Mode::Browse => handle_browse(app, code),
    Mode::Password => handle_input(app, code, |a| &mut a.password_input, App::submit_password),
    Mode::ExtractDest => handle_input(
      app,
      code,
      |a| &mut a.extract_dest_input,
      App::confirm_extract,
    ),
    Mode::Properties => {
      if dismiss_key(code) {
        app.mode = Mode::Browse;
        app.properties_entry = None;
      }
    }
    Mode::Help => match code {
      KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.mode = Mode::Browse,
      KeyCode::Up | KeyCode::Char('k') => app.help_scroll = app.help_scroll.saturating_sub(1),
      KeyCode::Down | KeyCode::Char('j') => app.help_scroll = app.help_scroll.saturating_add(1),
      _ => {}
    },
  }
}

fn handle_input(
  app: &mut App,
  code: KeyCode,
  field: fn(&mut App) -> &mut String,
  submit: fn(&mut App),
) {
  match code {
    KeyCode::Esc | KeyCode::Char('q') => app.cancel_mode(),
    KeyCode::Enter => submit(app),
    KeyCode::Backspace => {
      field(app).pop();
    }
    KeyCode::Char(c) => {
      field(app).push(c);
    }
    _ => {}
  }
}

fn dismiss_key(code: KeyCode) -> bool {
  matches!(
    code,
    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('p') | KeyCode::Char('?')
  )
}

fn handle_browse(app: &mut App, code: KeyCode) {
  match code {
    KeyCode::Char('q') => app.should_quit = true,
    KeyCode::Esc => {
      if app.nav.current_path.is_empty() {
        app.should_quit = true;
      } else {
        app.go_up();
      }
    }
    KeyCode::Tab => app.toggle_focus(),
    KeyCode::Up | KeyCode::Char('k') => app.move_up(),
    KeyCode::Down | KeyCode::Char('j') => app.move_down(),
    KeyCode::PageUp => app.scroll_preview_page_up(),
    KeyCode::PageDown => app.scroll_preview_page_down(),
    KeyCode::Left => {
      app.preview.horizontal_scroll = app.preview.horizontal_scroll.saturating_sub(4)
    }
    KeyCode::Right => {
      app.preview.horizontal_scroll = app.preview.horizontal_scroll.saturating_add(4)
    }
    KeyCode::Enter => app.enter_selected(),
    KeyCode::Backspace => {
      if app.nav.current_path.is_empty() {
        app.should_quit = true;
      } else {
        app.go_up();
      }
    }
    KeyCode::Char('e') => app.extract_selected(),
    KeyCode::Char('E') => app.extract_all(),
    KeyCode::Char('p') => app.show_properties(),
    KeyCode::Char('?') => {
      app.help_scroll = 0;
      app.mode = Mode::Help;
    }
    _ => {}
  }
}

// ── Mouse ──────────────────────────────────────────────────────

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
  let pos = Position::new(mouse.column, mouse.row);

  if app.mode == Mode::Help {
    match mouse.kind {
      MouseEventKind::ScrollUp => app.help_scroll = app.help_scroll.saturating_sub(3),
      MouseEventKind::ScrollDown => app.help_scroll = app.help_scroll.saturating_add(3),
      _ => {}
    }
    return;
  }

  if app.mode != Mode::Browse {
    return;
  }

  match mouse.kind {
    MouseEventKind::Down(MouseButton::Left) => handle_left_click(app, pos),
    MouseEventKind::ScrollUp => handle_scroll(app, pos, -3),
    MouseEventKind::ScrollDown => handle_scroll(app, pos, 3),
    _ => {}
  }
}

fn handle_left_click(app: &mut App, pos: Position) {
  if in_area(pos, app.preview.area) {
    if app.nav.focus != Focus::Right {
      app.nav.focus = Focus::Right;
    } else {
      app.scroll_preview_down();
    }
    return;
  }
  if !in_area(pos, app.file_list_area) {
    return;
  }

  app.nav.focus = Focus::Left;
  let y = pos.y.saturating_sub(app.file_list_area.y + 1) as usize;
  let entries = app.display_entries();
  if y >= entries.len() {
    return;
  }

  if entries[y].0 == ".." {
    app.go_up();
  } else if app.nav.list_state.selected() == Some(y) {
    app.enter_selected();
  } else {
    app.nav.list_state.select(Some(y));
    app.update_side_preview();
  }
}

fn handle_scroll(app: &mut App, pos: Position, delta: isize) {
  let on_preview = app.nav.focus == Focus::Right || in_area(pos, app.preview.area);
  let on_list = in_area(pos, app.file_list_area);

  for _ in 0..delta.abs() {
    if delta < 0 && on_preview {
      app.scroll_preview_up();
    } else if delta < 0 && on_list {
      app.move_up();
    } else if delta > 0 && on_preview {
      app.scroll_preview_down();
    } else if delta > 0 && on_list {
      app.move_down();
    }
  }
}

fn in_area(pos: Position, area: ratatui::layout::Rect) -> bool {
  pos.x >= area.x && pos.x < area.x + area.width && pos.y >= area.y && pos.y < area.y + area.height
}
