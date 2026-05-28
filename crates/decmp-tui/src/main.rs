mod action;
mod app;
mod context;
mod event;
mod highlight;
mod layout;
mod popup;
mod popups;
mod scroll;
mod tile;
mod tiles;
mod tree;
mod ui;

use std::io;
use std::path::PathBuf;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
  EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use decmp_core::{DecmpError, detect_format, get_handler};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = std::env::args().collect();
  if args.len() < 2 {
    eprintln!("Usage: decmp-tui <archive>");
    std::process::exit(1);
  }

  let archive_path = PathBuf::from(&args[1]);
  if !archive_path.exists() {
    eprintln!("Error: file not found: {}", archive_path.display());
    std::process::exit(1);
  }

  let format = detect_format(&archive_path)?;
  let handler = get_handler(&format);

  let mut app = match handler.list(&archive_path, None, None) {
    Ok(entries) => app::App::new(archive_path, handler, entries),
    Err(DecmpError::PasswordRequired) | Err(DecmpError::WrongPassword) => {
      app::App::new_password_required(archive_path, handler)
    }
    Err(e) => {
      eprintln!("Error: {e}");
      std::process::exit(1);
    }
  };

  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let result = run_app(&mut terminal, &mut app);

  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    DisableMouseCapture
  )?;
  terminal.show_cursor()?;

  if let Err(err) = result {
    eprintln!("Error: {err}");
    std::process::exit(1);
  }

  Ok(())
}

fn run_app(
  terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
  app: &mut app::App,
) -> io::Result<()> {
  loop {
    terminal.draw(|f| ui::draw(f, app))?;

    event::handle_events(app)?;

    if app.ctx.should_quit {
      return Ok(());
    }
  }
}
