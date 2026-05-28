use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use crate::action::{Action, PopupType};
use crate::context::AppContext;
use crate::popup::Popup;
use crate::scroll::{ScrollState, Scrollable};
use crate::tile::InputEvent;

pub struct HelpPopup {
  scroll: ScrollState,
}

impl HelpPopup {
  pub fn new() -> Self {
    Self {
      scroll: ScrollState::new(),
    }
  }
}

impl Scrollable for HelpPopup {
  fn supports_vertical_scroll(&self) -> bool {
    true
  }

  fn scroll_state(&self) -> &ScrollState {
    &self.scroll
  }

  fn scroll_state_mut(&mut self) -> &mut ScrollState {
    &mut self.scroll
  }

  fn content_line_count(&self, _ctx: &AppContext) -> usize {
    HelpPopup::help_lines().len()
  }

  fn scroll_up(&mut self, _ctx: &AppContext) {
    self.scroll.v_scroll = self.scroll.v_scroll.saturating_sub(1);
  }

  fn scroll_down(&mut self, _ctx: &AppContext) {
    let max = Self::help_lines()
      .len()
      .saturating_sub(self.scroll.last_viewport.get().max(1));
    if self.scroll.v_scroll < max {
      self.scroll.v_scroll += 1;
    }
  }
}

impl HelpPopup {
  fn help_lines() -> Vec<Line<'static>> {
    vec![
      Line::from(""),
      heading_line(" Navigation", Color::Yellow),
      Line::from("  \u{2191}/k/j        Move cursor"),
      Line::from("  Enter         Open dir / Preview file"),
      Line::from("  Backspace     Go to parent directory"),
      Line::from("  Esc           Back / Quit at root"),
      Line::from("  Tab           Switch focus (Left/Right)"),
      Line::from(""),
      heading_line(" Actions", Color::Yellow),
      Line::from("  e             Extract selected file"),
      Line::from("  E             Extract all files"),
      Line::from("  o             Change filename encoding"),
      Line::from(""),
      heading_line(" Properties", Color::Yellow),
      Line::from("  Top-right panel shows file info"),
      Line::from("  Select a file to see its properties"),
      Line::from("  Archive summary shown otherwise"),
      Line::from(""),
      heading_line(" Preview", Color::Yellow),
      Line::from("  Directories: shown automatically"),
      Line::from("  Files: Enter or Click (if selected) to load"),
      Line::from("  Focus right (Tab), then \u{2191}/\u{2193}/PgUp/PgDn scroll"),
      Line::from("  Syntax highlighting for source code"),
      Line::from(""),
      heading_line(" Mouse", Color::Yellow),
      Line::from("  Click         Select (files & dirs)"),
      Line::from("  Click \"..\"    Go to parent directory"),
      Line::from("  Scroll wheel  Scroll list / preview"),
      Line::from(""),
      heading_line(" General", Color::Yellow),
      Line::from("  ?             Toggle help"),
      Line::from("  q             Quit"),
    ]
  }
}

impl Popup for HelpPopup {
  fn popup_type(&self) -> PopupType {
    PopupType::Help
  }

  fn render(&self, area: Rect, frame: &mut Frame, _ctx: &AppContext) {
    let popup_area = centered_rect(55, 55, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
      .title(" Help ")
      .borders(Borders::ALL)
      .fg(Color::Cyan)
      .padding(Padding::horizontal(1));

    let lines = Self::help_lines();
    let inner = block.inner(popup_area);
    let visible = inner.height as usize;
    self.scroll.last_viewport.set(visible);
    let scroll = self.scroll.v_scroll.min(lines.len().saturating_sub(1));
    let end = (scroll + visible).min(lines.len());
    let clipped: Vec<Line> = lines[scroll..end].to_vec();

    frame.render_widget(Paragraph::new(clipped).block(block), popup_area);
  }

  fn handle_input(&mut self, event: &InputEvent, ctx: &mut AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
          vec![Action::ClosePopup]
        }
        KeyCode::Up | KeyCode::Char('k') => {
          self.scroll_up(ctx);
          vec![]
        }
        KeyCode::Down | KeyCode::Char('j') => {
          self.scroll_down(ctx);
          vec![]
        }
        _ => vec![],
      },
      InputEvent::Mouse(mouse) => match mouse.kind {
        MouseEventKind::ScrollUp => {
          for _ in 0..3 {
            self.scroll_up(ctx);
          }
          vec![]
        }
        MouseEventKind::ScrollDown => {
          for _ in 0..3 {
            self.scroll_down(ctx);
          }
          vec![]
        }
        _ => vec![],
      },
    }
  }
}

fn heading_line(s: &str, color: Color) -> Line<'_> {
  Line::from(Span::styled(s, Style::default().fg(color)))
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
  let popup = Layout::vertical([
    Constraint::Percentage((100 - percent_y) / 2),
    Constraint::Percentage(percent_y),
    Constraint::Percentage((100 - percent_y) / 2),
  ])
  .split(r);
  let h = Layout::horizontal([
    Constraint::Percentage((100 - percent_x) / 2),
    Constraint::Percentage(percent_x),
    Constraint::Percentage((100 - percent_x) / 2),
  ])
  .split(popup[1]);
  h[1]
}
