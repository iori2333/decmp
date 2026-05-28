use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::action::{Action, PopupType};
use crate::context::AppContext;
use crate::popup::Popup;
use crate::tile::InputEvent;

pub struct PasswordPopup {
  input: String,
}

impl PasswordPopup {
  pub fn new() -> Self {
    Self {
      input: String::new(),
    }
  }
}

impl Popup for PasswordPopup {
  fn popup_type(&self) -> PopupType {
    PopupType::Password
  }

  fn render(&self, area: Rect, frame: &mut Frame, _ctx: &AppContext) {
    let popup_area = centered_rect_fixed(50, 3, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
      .title(" Enter Password ")
      .borders(Borders::ALL)
      .fg(Color::Yellow);
    let inner = block.inner(popup_area);
    let w = inner.width.saturating_sub(1) as usize;
    let pw_len = self.input.len();

    let masked = if pw_len <= w {
      "*".repeat(pw_len)
    } else {
      format!("<{}*", &"*".repeat(w.saturating_sub(2)))
    };
    let masked = if masked.is_empty() {
      " ".to_string()
    } else {
      masked
    };

    frame.render_widget(Paragraph::new(masked).block(block), popup_area);
  }

  fn handle_input(&mut self, event: &InputEvent, ctx: &mut AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
          self.input.clear();
          if matches!(
            ctx.pending_action,
            Some(crate::context::PendingAction::InitialLoad)
          ) {
            vec![Action::Quit]
          } else {
            vec![Action::ClosePopup]
          }
        }
        KeyCode::Enter => {
          let pw = self.input.clone();
          self.input.clear();
          vec![Action::PasswordSubmitted(pw)]
        }
        KeyCode::Backspace => {
          self.input.pop();
          vec![]
        }
        KeyCode::Char(c) => {
          self.input.push(c);
          vec![]
        }
        _ => vec![],
      },
      _ => vec![],
    }
  }
}

fn centered_rect_fixed(percent_x: u16, height: u16, r: Rect) -> Rect {
  let popup = Layout::vertical([
    Constraint::Min(0),
    Constraint::Length(height),
    Constraint::Min(0),
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
