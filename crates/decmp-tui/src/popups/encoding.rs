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

pub struct EncodingPopup {
  input: String,
}

impl EncodingPopup {
  pub fn new(current: Option<&str>) -> Self {
    Self {
      input: current.unwrap_or_default().to_string(),
    }
  }
}

impl Popup for EncodingPopup {
  fn popup_type(&self) -> PopupType {
    PopupType::Encoding
  }

  fn render(&self, area: Rect, frame: &mut Frame, ctx: &AppContext) {
    let popup_area = centered_rect(60, 15, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
      .title(" Change Encoding (blank=auto) ")
      .borders(Borders::ALL)
      .fg(Color::Magenta);
    let current = ctx.encoding.as_deref().unwrap_or("auto");
    let hint = format!(
      "Current: {}\nInput (e.g. gbk, shift_jis, utf-8):\n> {}",
      current, self.input
    );
    frame.render_widget(
      Paragraph::new(hint).block(block).fg(Color::White),
      popup_area,
    );
  }

  fn handle_input(&mut self, event: &InputEvent, _ctx: &mut AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        KeyCode::Esc | KeyCode::Char('q') => vec![Action::ClosePopup],
        KeyCode::Enter => {
          let enc = self.input.trim().to_string();
          let enc = if enc.is_empty() { None } else { Some(enc) };
          vec![Action::RequestEncodingReload(
            enc.unwrap_or_else(|| "".to_string()),
          )]
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
