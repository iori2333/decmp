use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
  Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
  ScrollbarState,
};

use crate::app::{App, Focus, Mode};
use crate::tree::DirNode;

// ── Main draw ──────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
  let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(f.area());

  draw_main_panels(f, app, chunks[0]);

  draw_popup(f, app);

  draw_status_bar(f, app, chunks[1]);
}

fn draw_popup(f: &mut Frame, app: &mut App) {
  match app.mode {
    Mode::Password => draw_password_popup(f, app),
    Mode::ExtractDest => draw_extract_dest_popup(f, app),
    Mode::Properties => draw_properties_popup(f, app),
    Mode::Help => draw_help_popup(f, app),
    Mode::Browse => {}
  }
}

fn draw_main_panels(f: &mut Frame, app: &mut App, area: Rect) {
  let chunks =
    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
  app.file_list_area = chunks[0];
  app.preview.area = chunks[1];
  draw_file_list(f, app, chunks[0]);
  draw_side_preview(f, app, chunks[1]);
}

// ── File list ──────────────────────────────────────────────────

fn draw_file_list(f: &mut Frame, app: &mut App, area: Rect) {
  let block = Block::default()
    .title(format!(
      " {} {}",
      app
        .archive
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy(),
      app.current_display_path()
    ))
    .borders(Borders::ALL)
    .border_style(focus_style(app.nav.focus == Focus::Left));

  let entries = app.display_entries();
  let entry_count = entries.len();
  if entry_count == 0 {
    f.render_widget(Paragraph::new("  (empty)").block(block).style(dim()), area);
    return;
  }

  let scroll = app.preview.horizontal_scroll;

  let items: Vec<ListItem> = entries
    .iter()
    .map(|(name, node)| {
      if name == ".." {
        let s = "  ../".to_string();
        let visible = clip_str(&s, scroll);
        return ListItem::new(Line::from(visible));
      }
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
    .highlight_symbol("▶ ");

  f.render_stateful_widget(list, area, &mut app.nav.list_state);

  app.update_file_list_scrollbar(entry_count);
  render_scrollbar(f, area, &mut app.file_list_scroll);
}

fn clip_str(s: &str, scroll: usize) -> String {
  let scroll = scroll.min(s.len().saturating_sub(1));
  s[scroll..].to_string()
}

// ── Side preview ───────────────────────────────────────────────

fn draw_side_preview(f: &mut Frame, app: &mut App, area: Rect) {
  let preview = &app.preview.content;

  let title = if preview.name.is_empty() {
    " Preview "
  } else {
    &preview.name
  };
  let block = Block::default()
    .title(format!(" {title} "))
    .borders(Borders::ALL)
    .border_style(focus_style(app.nav.focus == Focus::Right));

  if preview.name.is_empty() {
    render_empty_preview(f, area, block, "  Select a file to preview");
    return;
  }
  if preview.is_dir {
    render_dir_preview(f, area, block, preview);
    return;
  }
  if preview.is_binary {
    render_empty_preview(f, area, block, "  Binary file - cannot preview");
    return;
  }
  if preview.lines.is_empty() {
    render_empty_preview(f, area, block, "  Press Enter to preview");
    return;
  }

  let inner = block.inner(area);
  let visible = inner.height as usize;
  let start = app.preview.scroll;
  let end = (start + visible).min(preview.lines.len());
  let mut lines: Vec<Line> = if !preview.highlighted.is_empty() {
    preview.highlighted[start..end]
      .iter()
      .map(|spans| Line::from(spans.clone()))
      .collect()
  } else {
    preview.lines[start..end]
      .iter()
      .map(|l| Line::from(l.as_str()))
      .collect()
  };

  if preview.is_truncated {
    lines.push(Line::from(Span::styled(
      "\n--- Preview truncated (file exceeds 64KB) ---",
      dim(),
    )));
  }

  f.render_widget(Paragraph::new(lines).block(block.clone()), area);

  app.update_preview_scrollbar(visible);
  render_scrollbar(f, area, &mut app.preview.scrollbar);
}

fn render_empty_preview(f: &mut Frame, area: Rect, block: Block<'_>, msg: &str) {
  f.render_widget(
    Paragraph::new(msg).block(block).centered().style(dim()),
    area,
  );
}

fn render_dir_preview(
  f: &mut Frame,
  area: Rect,
  block: Block<'_>,
  preview: &crate::app::SidePreview,
) {
  let items: Vec<Line> = preview
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
  f.render_widget(Paragraph::new(items).block(block), area);
}

// ── Popups ─────────────────────────────────────────────────────

fn draw_password_popup(f: &mut Frame, app: &App) {
  let area = centered_rect_fixed(50, 3, f.area());
  f.render_widget(Clear, area);

  let block = Block::default()
    .title(" Enter Password ")
    .borders(Borders::ALL)
    .fg(Color::Yellow);
  let inner = block.inner(area);
  let w = inner.width.saturating_sub(1) as usize;
  let pw_len = app.password_input.len();

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

  f.render_widget(Paragraph::new(masked).block(block), area);
}

fn draw_extract_dest_popup(f: &mut Frame, app: &App) {
  let area = centered_rect(60, 15, f.area());
  f.render_widget(Clear, area);
  let block = Block::default()
    .title(" Extract to ")
    .borders(Borders::ALL)
    .fg(Color::Cyan);
  f.render_widget(
    Paragraph::new(app.extract_dest_input.as_str())
      .block(block)
      .fg(Color::White),
    area,
  );
}

fn draw_properties_popup(f: &mut Frame, app: &App) {
  let area = centered_rect(60, 40, f.area());
  f.render_widget(Clear, area);
  let block = Block::default()
    .title(" File Properties ")
    .borders(Borders::ALL)
    .fg(Color::Green)
    .padding(Padding::horizontal(1));

  if let Some(ref entry) = app.properties_entry {
    let size_str = format!("{} bytes", entry.size);
    let comp_str = format!("{} bytes", entry.compressed_size);
    let type_str = if entry.is_dir { "Directory" } else { "File" };
    let lines = vec![
      prop_line("Name:      ", &entry.name),
      prop_line("Size:      ", &size_str),
      prop_line("Compressed:", &comp_str),
      prop_line("Method:    ", &entry.method),
      prop_line("Type:      ", type_str),
    ];
    f.render_widget(Paragraph::new(lines).block(block), area);
  }
}

fn draw_help_popup(f: &mut Frame, app: &mut App) {
  let area = centered_rect(55, 55, f.area());
  f.render_widget(Clear, area);
  let block = Block::default()
    .title(" Help ")
    .borders(Borders::ALL)
    .fg(Color::Cyan)
    .padding(Padding::horizontal(1));

  let lines = vec![
    Line::from(""),
    heading_line(" Navigation", Color::Yellow),
    Line::from("  ↑/k/j        Move cursor"),
    Line::from("  Enter         Open dir / Preview file"),
    Line::from("  Backspace     Go to parent directory"),
    Line::from("  Esc           Back / Quit at root"),
    Line::from("  Tab           Switch focus (Left/Right)"),
    Line::from(""),
    heading_line(" Actions", Color::Yellow),
    Line::from("  e             Extract selected file"),
    Line::from("  E             Extract all files"),
    Line::from("  p             Show file properties"),
    Line::from(""),
    heading_line(" Preview", Color::Yellow),
    Line::from("  Directories: shown automatically"),
    Line::from("  Files: Enter or Click (if selected) to load"),
    Line::from("  Focus right (Tab), then ↑/↓/PgUp/PgDn scroll"),
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

  let inner = block.inner(area);
  let visible = inner.height as usize;
  let scroll = app.help_scroll.min(lines.len().saturating_sub(1));
  let end = (scroll + visible).min(lines.len());
  let clipped: Vec<Line> = lines[scroll..end].to_vec();

  f.render_widget(Paragraph::new(clipped).block(block.clone()), area);

  let mut sb_state = ScrollbarState::default()
    .content_length(lines.len())
    .position(scroll)
    .viewport_content_length(visible);
  render_scrollbar(f, area, &mut sb_state);
}

fn heading_line(s: &str, color: Color) -> Line<'_> {
  Line::from(Span::styled(s, Style::default().fg(color)))
}

fn prop_line<'a>(label: &'a str, val: &'a str) -> Line<'a> {
  Line::from(vec![Span::styled(label, dim()), Span::raw(val.to_string())])
}

// ── Status bar ─────────────────────────────────────────────────

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
  let msg = app.status_msg.as_deref().unwrap_or(
        "[Enter]Open/Preview [e]Extract [E]All [Tab]Focus [P/U]Scroll [?]Help [q]Quit  | Click: select · Click ..: back"
    );
  let style = if app.status_msg.is_some() {
    Style::default().fg(Color::Yellow)
  } else {
    dim()
  };
  f.render_widget(Paragraph::new(msg).style(style), area);
}

// ── Layout helpers ─────────────────────────────────────────────

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

fn focus_style(active: bool) -> Style {
  Style::default().fg(if active { Color::Cyan } else { Color::DarkGray })
}

fn dim() -> Style {
  Style::default().fg(Color::DarkGray)
}

fn render_scrollbar(f: &mut Frame, area: Rect, state: &mut ratatui::widgets::ScrollbarState) {
  let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
    .begin_symbol(None)
    .end_symbol(None);
  f.render_stateful_widget(sb, area, state);
}

// ── Shared ─────────────────────────────────────────────────────

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
