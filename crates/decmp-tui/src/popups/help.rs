use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
  Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::action::{Action, PopupType};
use crate::context::AppContext;
use crate::popup::Popup;
use crate::tile::InputEvent;

pub struct HelpPopup {
  scroll: usize,
}

impl HelpPopup {
  pub fn new() -> Self {
    Self { scroll: 0 }
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

    let lines = vec![
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
    ];

    let inner = block.inner(popup_area);
    let visible = inner.height as usize;
    let scroll = self.scroll.min(lines.len().saturating_sub(1));
    let end = (scroll + visible).min(lines.len());
    let clipped: Vec<Line> = lines[scroll..end].to_vec();

    frame.render_widget(Paragraph::new(clipped).block(block.clone()), popup_area);

    let mut sb_state = ScrollbarState::default()
      .content_length(lines.len())
      .position(scroll)
      .viewport_content_length(visible);
    let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
      .begin_symbol(None)
      .end_symbol(None);
    frame.render_stateful_widget(sb, popup_area, &mut sb_state);
  }

  fn handle_input(&mut self, event: &InputEvent, _ctx: &mut AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
          vec![Action::ClosePopup]
        }
        KeyCode::Up | KeyCode::Char('k') => {
          self.scroll = self.scroll.saturating_sub(1);
          vec![]
        }
        KeyCode::Down | KeyCode::Char('j') => {
          self.scroll = self.scroll.saturating_add(1);
          vec![]
        }
        _ => vec![],
      },
      InputEvent::Mouse(mouse) => match mouse.kind {
        MouseEventKind::ScrollUp => {
          self.scroll = self.scroll.saturating_sub(3);
          vec![]
        }
        MouseEventKind::ScrollDown => {
          self.scroll = self.scroll.saturating_add(3);
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
