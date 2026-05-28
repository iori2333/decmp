use decmp_core::ArchiveEntry;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::action::Action;
use crate::context::AppContext;
use crate::scroll::{ScrollState, Scrollable};
use crate::tile::{InputEvent, Tile, TileId};
use crate::tiles::file_list::format_size;

pub struct PropertiesTile {
  selected_entry: Option<ArchiveEntry>,
  scroll: ScrollState,
}

impl PropertiesTile {
  pub fn new() -> Self {
    Self {
      selected_entry: None,
      scroll: ScrollState::new(),
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
    self.scroll.v_scroll = self.scroll.v_scroll.min(6usize.saturating_sub(visible));
    let start = self.scroll.v_scroll;
    let end = (start + visible).min(6);

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

    self.render_scrollbar(area, visible, frame, ctx);
  }

  fn handle_input(&mut self, event: &InputEvent, ctx: &AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => match key.code {
        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
          self.scroll_up(ctx);
          vec![]
        }
        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
          self.scroll_down(ctx);
          vec![]
        }
        crossterm::event::KeyCode::PageUp => {
          self.scroll_page_up(ctx);
          vec![]
        }
        crossterm::event::KeyCode::PageDown => {
          self.scroll_page_down(ctx);
          vec![]
        }
        _ => vec![],
      },
      InputEvent::Mouse(mouse) => match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
          self.scroll_up(ctx);
          vec![]
        }
        crossterm::event::MouseEventKind::ScrollDown => {
          self.scroll_down(ctx);
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
      self.scroll.reset();
      if name.is_empty() || name == ".." || *is_dir {
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
    self.scroll.reset();
  }
}

impl Scrollable for PropertiesTile {
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
    6
  }
}

fn prop_line<'a>(label: &'a str, val: &'a str, dim: Style) -> Line<'a> {
  Line::from(vec![Span::styled(label, dim), Span::raw(val)])
}
