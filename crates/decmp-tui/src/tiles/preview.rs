use std::collections::HashMap;

use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
  Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::action::Action;
use crate::context::{AppContext, SidePreview};
use crate::tile::{InputEvent, Tile, TileId};

pub struct PreviewTile {
  content: SidePreview,
  scroll: usize,
  cache: HashMap<String, SidePreview>,
  scrollbar_state: ScrollbarState,
  area: Rect,
}

impl PreviewTile {
  pub fn new() -> Self {
    Self {
      content: SidePreview::default(),
      scroll: 0,
      cache: HashMap::new(),
      scrollbar_state: ScrollbarState::default(),
      area: Rect::default(),
    }
  }

  fn handle_selection_changed(
    &mut self,
    name: &str,
    is_dir: bool,
    full_name: &str,
    dir_entries: Option<&[String]>,
  ) {
    if name.is_empty() {
      self.content = SidePreview::default();
      return;
    }
    if name == ".." {
      self.content = SidePreview::default();
      return;
    }
    if is_dir {
      if let Some(entries) = dir_entries {
        self.content = SidePreview::dir(name, entries.to_vec());
      }
      return;
    }
    self.content = self
      .cache
      .get(full_name)
      .cloned()
      .unwrap_or_else(|| SidePreview::empty_with_name(name));
  }

  fn handle_preview_loaded(&mut self, full_name: String, preview: SidePreview) {
    self.cache.insert(full_name, preview.clone());
    self.content = preview;
    self.scroll = 0;
  }

  fn update_scrollbar(&mut self, visible_height: usize) {
    let total = self.content.lines.len();
    if total == 0 {
      self.scrollbar_state = ScrollbarState::default();
      return;
    }
    self.scrollbar_state = self
      .scrollbar_state
      .content_length(total)
      .position(self.scroll)
      .viewport_content_length(visible_height);
  }

  fn scroll_up(&mut self) {
    self.scroll = self.scroll.saturating_sub(1);
  }

  fn scroll_down(&mut self) {
    if !self.content.lines.is_empty() && self.scroll < self.content.lines.len().saturating_sub(1) {
      self.scroll += 1;
    }
  }

  fn scroll_page_up(&mut self) {
    for _ in 0..20 {
      self.scroll_up();
    }
  }

  fn scroll_page_down(&mut self) {
    for _ in 0..20 {
      self.scroll_down();
    }
  }

  fn render_dir_preview(&self, area: Rect, frame: &mut Frame, block: Block<'_>) {
    let items: Vec<Line> = self
      .content
      .dir_entries
      .iter()
      .map(|e| {
        if e.ends_with('/') {
          Line::from(Span::styled(e.as_str(), Style::default().fg(Color::Blue)))
        } else {
          Line::from(e.as_str())
        }
      })
      .collect();
    frame.render_widget(Paragraph::new(items).block(block), area);
  }
}

impl Tile for PreviewTile {
  fn tile_id(&self) -> TileId {
    TileId::Preview
  }

  fn focusable(&self) -> bool {
    self.content.is_dir || !self.content.lines.is_empty()
  }

  fn render(&mut self, area: Rect, frame: &mut Frame, ctx: &AppContext) {
    self.area = area;
    let is_focused = ctx.focus == TileId::Preview;
    let border_style = if is_focused {
      Style::default().fg(Color::Cyan)
    } else {
      Style::default().fg(Color::DarkGray)
    };

    let title = if self.content.name.is_empty() {
      " Preview ".to_string()
    } else if let Some(ref enc) = self.content.encoding_detected {
      format!(" {} [{enc}] ", self.content.name)
    } else {
      format!(" {} ", self.content.name)
    };

    let block = Block::default()
      .title(title)
      .borders(Borders::ALL)
      .border_style(border_style);

    let dim = Style::default().fg(Color::DarkGray);

    if self.content.name.is_empty() {
      frame.render_widget(
        Paragraph::new("  Select a file to preview")
          .block(block)
          .centered()
          .style(dim),
        area,
      );
      return;
    }
    if self.content.is_dir {
      self.render_dir_preview(area, frame, block);
      return;
    }
    if self.content.is_binary {
      frame.render_widget(
        Paragraph::new("  Binary file - cannot preview")
          .block(block)
          .centered()
          .style(dim),
        area,
      );
      return;
    }
    if self.content.lines.is_empty() {
      frame.render_widget(
        Paragraph::new("  Press Enter to preview")
          .block(block)
          .centered()
          .style(dim),
        area,
      );
      return;
    }

    let inner = block.inner(area);
    let visible = inner.height as usize;
    let start = self.scroll;
    let end = (start + visible).min(self.content.lines.len());
    let mut lines: Vec<Line> = if !self.content.highlighted.is_empty() {
      self.content.highlighted[start..end]
        .iter()
        .map(|spans| Line::from(spans.clone()))
        .collect()
    } else {
      self.content.lines[start..end]
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect()
    };

    if self.content.is_truncated {
      lines.push(Line::from(Span::styled("\n--- Preview truncated ---", dim)));
    }

    frame.render_widget(Paragraph::new(lines).block(block.clone()), area);

    self.update_scrollbar(visible);
    let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
      .begin_symbol(None)
      .end_symbol(None);
    frame.render_stateful_widget(sb, area, &mut self.scrollbar_state);
  }

  fn handle_input(&mut self, event: &InputEvent, _ctx: &AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
          self.scroll_up();
          vec![]
        }
        KeyCode::Down | KeyCode::Char('j') => {
          self.scroll_down();
          vec![]
        }
        KeyCode::PageUp => {
          self.scroll_page_up();
          vec![]
        }
        KeyCode::PageDown => {
          self.scroll_page_down();
          vec![]
        }
        _ => vec![],
      },
      InputEvent::Mouse(mouse) => {
        let pos = Position::new(mouse.column, mouse.row);
        match mouse.kind {
          MouseEventKind::ScrollUp => {
            if in_area(pos, self.area) {
              self.scroll_up();
            }
            vec![]
          }
          MouseEventKind::ScrollDown => {
            if in_area(pos, self.area) {
              self.scroll_down();
            }
            vec![]
          }
          _ => vec![],
        }
      }
    }
  }

  fn handle_action(&mut self, action: &Action, _ctx: &AppContext) {
    match action {
      Action::SelectionChanged {
        name,
        is_dir,
        full_name,
        dir_entries,
      } => {
        self.handle_selection_changed(name, *is_dir, full_name, dir_entries.as_deref());
      }
      Action::PreviewLoaded { full_name, preview } => {
        self.handle_preview_loaded(full_name.clone(), preview.clone());
      }
      _ => {}
    }
  }

  fn clear_cache(&mut self) {
    self.cache.clear();
    self.content = SidePreview::default();
  }
}

fn in_area(pos: Position, area: Rect) -> bool {
  pos.x >= area.x && pos.x < area.x + area.width && pos.y >= area.y && pos.y < area.y + area.height
}
