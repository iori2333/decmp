use decmp_core::ArchiveEntry;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
  Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::action::Action;
use crate::context::AppContext;
use crate::tile::{InputEvent, Tile, TileId};
use crate::tiles::file_list::format_size;

pub struct PropertiesTile {
  selected_entry: Option<ArchiveEntry>,
  scroll: usize,
  scrollbar_state: ScrollbarState,
}

impl PropertiesTile {
  pub fn new() -> Self {
    Self {
      selected_entry: None,
      scroll: 0,
      scrollbar_state: ScrollbarState::default(),
    }
  }

  fn line_count(&self) -> usize {
    6
  }

  fn scroll_up(&mut self) {
    self.scroll = self.scroll.saturating_sub(1);
  }

  fn scroll_down(&mut self) {
    let max = self.line_count().saturating_sub(1);
    if self.scroll < max {
      self.scroll += 1;
    }
  }
}

impl Tile for PropertiesTile {
  fn tile_id(&self) -> TileId {
    TileId::Properties
  }

  fn render(&mut self, area: Rect, frame: &mut Frame, ctx: &AppContext) {
    let dim = Style::default().fg(Color::DarkGray);
    let is_focused = ctx.focus == TileId::Properties;
    let border_color = if is_focused {
      Color::Green
    } else {
      Color::DarkGray
    };
    let block = Block::default()
      .title(" Properties ")
      .borders(Borders::ALL)
      .fg(border_color);

    let inner = block.inner(area);
    let visible = inner.height as usize;
    let start = self.scroll;
    let total: usize = 6;
    let end = (start + visible).min(total);

    if let Some(ref entry) = self.selected_entry {
      let type_str = if entry.is_dir { "Dir" } else { "File" };
      let date = entry.modified.as_deref().unwrap_or("-").to_string();
      let size_str = format_size(entry.size);
      let comp_str = format_size(entry.compressed_size);
      let all_lines = [
        prop_line("Name:   ", &entry.name, dim),
        prop_line("Size:   ", &size_str, dim),
        prop_line("Comp:   ", &comp_str, dim),
        prop_line("Method: ", &entry.method, dim),
        prop_line("Type:   ", type_str, dim),
        prop_line("Date:   ", &date, dim),
      ];
      let visible_lines: Vec<Line> = all_lines[start..end].to_vec();
      frame.render_widget(Paragraph::new(visible_lines).block(block.clone()), area);
    } else {
      let total_entries = ctx.archive.entries.len();
      let files = ctx.archive.entries.iter().filter(|e| !e.is_dir).count();
      let dirs = total_entries - files;
      let total_size: u64 = ctx.archive.entries.iter().map(|e| e.size).sum();
      let total_comp: u64 = ctx.archive.entries.iter().map(|e| e.compressed_size).sum();
      let archive_name = ctx
        .archive
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| ctx.archive.path.display().to_string());
      let entries_str = total_entries.to_string();
      let files_str = files.to_string();
      let dirs_str = dirs.to_string();
      let total_size_str = format_size(total_size);
      let total_comp_str = format_size(total_comp);

      let all_lines = [
        prop_line("Archive:", &archive_name, dim),
        prop_line("Entries:", &entries_str, dim),
        prop_line("Files:  ", &files_str, dim),
        prop_line("Dirs:   ", &dirs_str, dim),
        prop_line("Size:   ", &total_size_str, dim),
        prop_line("Comp:   ", &total_comp_str, dim),
      ];
      let visible_lines: Vec<Line> = all_lines[start..end].to_vec();
      if visible_lines.is_empty() {
        return;
      }
      frame.render_widget(Paragraph::new(visible_lines).block(block.clone()), area);
    }

    if total > 0 {
      self.scrollbar_state = self
        .scrollbar_state
        .content_length(total)
        .position(self.scroll)
        .viewport_content_length(visible);
      let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
      frame.render_stateful_widget(sb, area, &mut self.scrollbar_state);
    }
  }

  fn handle_input(&mut self, event: &InputEvent, _ctx: &AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
          self.scroll_up();
          vec![]
        }
        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
          self.scroll_down();
          vec![]
        }
        crossterm::event::KeyCode::PageUp => {
          for _ in 0..10 {
            self.scroll_up();
          }
          vec![]
        }
        crossterm::event::KeyCode::PageDown => {
          for _ in 0..10 {
            self.scroll_down();
          }
          vec![]
        }
        _ => vec![],
      },
      InputEvent::Mouse(mouse) => match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
          self.scroll_up();
          vec![]
        }
        crossterm::event::MouseEventKind::ScrollDown => {
          self.scroll_down();
          vec![]
        }
        _ => vec![],
      },
    }
  }

  fn handle_action(&mut self, action: &Action, ctx: &AppContext) {
    if let Action::SelectionChanged {
      name,
      is_dir,
      full_name,
      ..
    } = action
    {
      self.scroll = 0;
      if name == ".." || *is_dir {
        self.selected_entry = None;
      } else if !full_name.is_empty() {
        self.selected_entry = ctx
          .archive
          .entries
          .iter()
          .find(|e| e.name == *full_name)
          .cloned();
      }
    }
  }

  fn reset_with_entries(&mut self, _entries: &[ArchiveEntry]) {
    self.selected_entry = None;
    self.scroll = 0;
  }
}

fn prop_line<'a>(label: &'a str, val: &'a str, dim: Style) -> Line<'a> {
  Line::from(vec![Span::styled(label, dim), Span::raw(val)])
}
