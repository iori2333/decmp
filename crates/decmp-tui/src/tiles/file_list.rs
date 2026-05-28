use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
  Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
  ScrollbarState,
};

use decmp_core::ArchiveEntry;

use crate::action::Action;
use crate::context::AppContext;
use crate::tile::{InputEvent, Tile, TileId};
use crate::tree::{DirNode, DirTree};

pub struct FileListTile {
  current_path: Vec<String>,
  list_state: ListState,
  scrollbar_state: ScrollbarState,
  area: Rect,
  horizontal_scroll: usize,
}

impl FileListTile {
  pub fn new() -> Self {
    Self {
      current_path: Vec::new(),
      list_state: ListState::default(),
      scrollbar_state: ScrollbarState::default(),
      area: Rect::default(),
      horizontal_scroll: 0,
    }
  }

  pub fn init_entries(&mut self, _entries: &[ArchiveEntry]) {
    self.current_path.clear();
    self.list_state = ListState::default();
  }

  fn current_tree<'a>(&self, ctx: &'a AppContext) -> &'a DirTree {
    let mut tree = &ctx.archive.tree;
    for part in &self.current_path {
      if let Some(DirNode::Dir(subtree)) = tree.children.get(part) {
        tree = subtree;
      }
    }
    tree
  }

  fn current_display_path(&self) -> String {
    if self.current_path.is_empty() {
      String::new()
    } else {
      self.current_path.join("/") + "/"
    }
  }

  pub fn display_entries<'a>(&self, ctx: &'a AppContext) -> Vec<(String, Option<&'a DirNode>)> {
    let mut result = Vec::new();
    if !self.current_path.is_empty() {
      result.push((String::from(".."), None));
    }
    for (name, node) in self.current_tree(ctx).sorted_entries() {
      result.push((name.clone(), Some(node)));
    }
    result
  }

  pub fn selected_info(&self, ctx: &AppContext) -> Option<(String, bool)> {
    let idx = self.list_state.selected()?;
    let entries = self.display_entries(ctx);
    let (name, node) = entries.get(idx)?;
    let is_dir = name == ".." || matches!(node, Some(DirNode::Dir(_)));
    Some((name.clone(), is_dir))
  }

  fn deselection_action() -> Vec<Action> {
    vec![Action::SelectionChanged {
      name: String::new(),
      is_dir: false,
      full_name: String::new(),
      dir_entries: None,
    }]
  }

  fn selection_action(&self, ctx: &AppContext) -> Vec<Action> {
    let Some((name, is_dir)) = self.selected_info(ctx) else {
      return Self::deselection_action();
    };
    let full_name = if name == ".." {
      String::new()
    } else {
      self.build_full_path(&name)
    };
    let dir_entries = if name == ".." || !is_dir {
      None
    } else {
      let tree = self.current_tree(ctx);
      if let Some(DirNode::Dir(subtree)) = tree.children.get(&name) {
        Some(
          subtree
            .sorted_entries()
            .iter()
            .map(|(n, node)| match node {
              DirNode::Dir(_) => format!("{n}/"),
              DirNode::File(f) => format!("{n:<40} {}", format_size(f.size)),
            })
            .collect(),
        )
      } else {
        None
      }
    };
    vec![Action::SelectionChanged {
      name,
      is_dir,
      full_name,
      dir_entries,
    }]
  }

  pub fn build_full_path(&self, name: &str) -> String {
    if self.current_path.is_empty() {
      name.to_string()
    } else {
      format!("{}/{name}", self.current_path.join("/"))
    }
  }

  fn update_scrollbar(&mut self, total: usize) {
    let pos = self.list_state.selected().unwrap_or(0);
    self.scrollbar_state = self
      .scrollbar_state
      .content_length(total)
      .position(pos)
      .viewport_content_length(1);
  }
}

impl Tile for FileListTile {
  fn tile_id(&self) -> TileId {
    TileId::FileList
  }

  fn render(&mut self, area: Rect, frame: &mut Frame, ctx: &AppContext) {
    self.area = area;
    let is_focused = ctx.focus == TileId::FileList;
    let border_style = if is_focused {
      Style::default().fg(Color::Cyan)
    } else {
      Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
      .title(format!(
        " {} {}",
        ctx
          .archive
          .path
          .file_name()
          .unwrap_or_default()
          .to_string_lossy(),
        self.current_display_path()
      ))
      .borders(Borders::ALL)
      .border_style(border_style);

    let entries = self.display_entries(ctx);
    let entry_count = entries.len();
    if entry_count == 0 {
      let dim = Style::default().fg(Color::DarkGray);
      frame.render_widget(Paragraph::new("  (empty)").block(block).style(dim), area);
      return;
    }

    let scroll = self.horizontal_scroll;

    let items: Vec<ListItem> = entries
      .iter()
      .map(|(name, node)| {
        if name == ".." {
          let s = "  ../".to_string();
          ListItem::new(Line::from(clip_str(&s, scroll)))
        } else {
          match node {
            Some(DirNode::Dir(_)) => {
              let s = format!("  {name}/");
              ListItem::new(Line::from(clip_str(&s, scroll)))
            }
            Some(DirNode::File(entry)) => {
              let size = format_size(entry.size);
              let date = entry.modified.as_deref().unwrap_or("");
              let line_str = format!("  {name:<35}{size:>10}  {date:<17} {}   ", entry.method);
              ListItem::new(Line::from(clip_str(&line_str, scroll)))
            }
            None => ListItem::new(""),
          }
        }
      })
      .collect();

    let list = List::new(items)
      .block(block.clone())
      .highlight_style(
        Style::default()
          .bg(Color::DarkGray)
          .fg(Color::White)
          .add_modifier(Modifier::BOLD),
      )
      .highlight_symbol("\u{25b6} ");

    frame.render_stateful_widget(list, area, &mut self.list_state);

    self.update_scrollbar(entry_count);
    let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
      .begin_symbol(None)
      .end_symbol(None);
    frame.render_stateful_widget(sb, area, &mut self.scrollbar_state);
  }

  fn handle_input(&mut self, event: &InputEvent, ctx: &AppContext) -> Vec<Action> {
    match event {
      InputEvent::Key(key) => self.handle_key(key, ctx),
      InputEvent::Mouse(mouse) => self.handle_mouse(mouse, ctx),
    }
  }

  fn reset_with_entries(&mut self, _entries: &[ArchiveEntry]) {
    self.current_path.clear();
    self.list_state = ListState::default();
  }

  fn handle_action(&mut self, _action: &Action, _ctx: &AppContext) {}
}

impl FileListTile {
  fn handle_key(&mut self, key: &crossterm::event::KeyEvent, ctx: &AppContext) -> Vec<Action> {
    match key.code {
      KeyCode::Esc => {
        if self.list_state.selected().is_some() {
          self.list_state.select(None);
          return Self::deselection_action();
        }
        if self.current_path.is_empty() {
          vec![Action::Quit]
        } else {
          self.go_up(ctx)
        }
      }
      KeyCode::Up | KeyCode::Char('k') => self.navigate_list(-1, ctx),
      KeyCode::Down | KeyCode::Char('j') => self.navigate_list(1, ctx),
      KeyCode::PageUp => {
        for _ in 0..10 {
          self.navigate_list(-1, ctx);
        }
        self.selection_action(ctx)
      }
      KeyCode::PageDown => {
        for _ in 0..10 {
          self.navigate_list(1, ctx);
        }
        self.selection_action(ctx)
      }
      KeyCode::Enter => self.enter_selected(ctx),
      KeyCode::Backspace => {
        if self.list_state.selected().is_some() {
          self.list_state.select(None);
          return Self::deselection_action();
        }
        if self.current_path.is_empty() {
          vec![Action::Quit]
        } else {
          self.go_up(ctx)
        }
      }
      KeyCode::Left => {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(4);
        vec![]
      }
      KeyCode::Right => {
        self.horizontal_scroll = self.horizontal_scroll.saturating_add(4);
        vec![]
      }
      KeyCode::Char('e') => {
        let Some((name, is_dir)) = self.selected_info(ctx) else {
          return vec![];
        };
        if &name == ".." || is_dir {
          return vec![];
        }
        let full_name = self.build_full_path(&name);
        vec![Action::StartExtract { full_name }]
      }
      KeyCode::Char('E') => {
        vec![Action::StartExtractAll]
      }
      KeyCode::Char('o') => {
        vec![Action::RequestEncodingInput]
      }
      _ => vec![],
    }
  }

  fn navigate_list(&mut self, delta: isize, ctx: &AppContext) -> Vec<Action> {
    let len = self.display_entries(ctx).len();
    if len == 0 {
      return vec![];
    }
    let new = match self.list_state.selected() {
      Some(i) => (i as isize + delta).clamp(0, len as isize - 1) as usize,
      None => {
        if delta < 0 {
          len - 1
        } else {
          0
        }
      }
    };
    self.list_state.select(Some(new));
    self.selection_action(ctx)
  }

  fn enter_selected(&mut self, ctx: &AppContext) -> Vec<Action> {
    let Some((name, is_dir)) = self.selected_info(ctx) else {
      return vec![];
    };
    if name == ".." || is_dir {
      if name == ".." {
        return self.go_up(ctx);
      } else {
        self.current_path.push(name.to_string());
        self.list_state.select(None);
        return Self::deselection_action();
      }
    }
    let full_name = self.build_full_path(&name);
    vec![Action::RequestPreviewLoad { full_name }]
  }

  fn go_up(&mut self, _ctx: &AppContext) -> Vec<Action> {
    if self.current_path.pop().is_some() {
      self.list_state.select(None);
      Self::deselection_action()
    } else {
      vec![]
    }
  }

  fn handle_mouse(
    &mut self,
    mouse: &crossterm::event::MouseEvent,
    ctx: &AppContext,
  ) -> Vec<Action> {
    let pos = ratatui::layout::Position::new(mouse.column, mouse.row);

    match mouse.kind {
      MouseEventKind::Down(MouseButton::Left) => {
        if !in_area(pos, self.area) {
          return vec![];
        }
        let y = pos.y.saturating_sub(self.area.y + 1) as usize;
        let entries = self.display_entries(ctx);
        if y >= entries.len() {
          return vec![];
        }
        if entries[y].0 == ".." {
          return self.go_up(ctx);
        }
        if self.list_state.selected() == Some(y) {
          return self.enter_selected(ctx);
        }
        self.list_state.select(Some(y));
        self.selection_action(ctx)
      }
      MouseEventKind::ScrollUp => self.navigate_list(-1, ctx),
      MouseEventKind::ScrollDown => self.navigate_list(1, ctx),
      _ => vec![],
    }
  }
}

fn clip_str(s: &str, scroll: usize) -> String {
  if scroll == 0 {
    return s.to_string();
  }
  s.chars().skip(scroll).collect()
}

pub fn format_size(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = KB * 1024;
  const GB: u64 = MB * 1024;
  if bytes >= GB {
    format!("{:.1}G", bytes as f64 / GB as f64)
  } else if bytes >= MB {
    format!("{:.1}M", bytes as f64 / MB as f64)
  } else if bytes >= KB {
    format!("{:.1}K", bytes as f64 / KB as f64)
  } else {
    format!("{bytes}B")
  }
}

fn in_area(pos: ratatui::layout::Position, area: Rect) -> bool {
  pos.x >= area.x && pos.x < area.x + area.width && pos.y >= area.y && pos.y < area.y + area.height
}
