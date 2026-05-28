use std::path::PathBuf;

use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::action::{Action, PopupType};
use crate::context::AppContext;
use crate::popup::Popup;
use crate::scroll::{ScrollState, Scrollable};
use crate::tile::InputEvent;

pub struct ExtractDestPopup {
  input: String,
  scroll: ScrollState,
}

impl ExtractDestPopup {
  pub fn new(default_dest: PathBuf) -> Self {
    Self {
      input: default_dest.display().to_string(),
      scroll: ScrollState::new(),
    }
  }
}

impl Scrollable for ExtractDestPopup {
  fn scroll_state(&self) -> &ScrollState {
    &self.scroll
  }

  fn scroll_state_mut(&mut self) -> &mut ScrollState {
    &mut self.scroll
  }
}

impl Popup for ExtractDestPopup {
  fn popup_type(&self) -> PopupType {
    PopupType::ExtractDest
  }

  fn render(&self, area: Rect, frame: &mut Frame, _ctx: &AppContext) {
    let popup_area = centered_rect(60, 15, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
      .title(" Extract to ")
      .borders(Borders::ALL)
      .fg(Color::Cyan);
    frame.render_widget(
      Paragraph::new(self.input.as_str())
        .block(block)
        .fg(Color::White),
      popup_area,
    );
  }

  fn handle_input(&mut self, event: &InputEvent, _ctx: &mut AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        KeyCode::Esc | KeyCode::Char('q') => vec![Action::ClosePopup],
        KeyCode::Enter => {
          let dest = PathBuf::from(&self.input);
          self.input.clear();
          vec![Action::ConfirmExtract { dest }]
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
