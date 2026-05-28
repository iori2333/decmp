use std::cell::Cell;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::context::AppContext;

pub struct ScrollState {
  pub v_scroll: usize,
  pub h_scroll: usize,
  pub v_scrollbar: ScrollbarState,
  pub last_viewport: Cell<usize>,
}

impl Default for ScrollState {
  fn default() -> Self {
    Self {
      v_scroll: 0,
      h_scroll: 0,
      v_scrollbar: ScrollbarState::default(),
      last_viewport: Cell::new(0),
    }
  }
}

impl ScrollState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn reset(&mut self) {
    self.v_scroll = 0;
    self.h_scroll = 0;
    self.v_scrollbar = ScrollbarState::default();
    self.last_viewport.set(0);
  }
}

pub trait Scrollable {
  fn supports_vertical_scroll(&self) -> bool {
    false
  }
  fn supports_horizontal_scroll(&self) -> bool {
    false
  }

  #[allow(dead_code)]
  fn scroll_state(&self) -> &ScrollState;
  fn scroll_state_mut(&mut self) -> &mut ScrollState;

  fn content_line_count(&self, _ctx: &AppContext) -> usize {
    0
  }

  fn scroll_up(&mut self, _ctx: &AppContext) {
    if self.supports_vertical_scroll() {
      self.scroll_state_mut().v_scroll = self.scroll_state_mut().v_scroll.saturating_sub(1);
    }
  }

  fn scroll_down(&mut self, ctx: &AppContext) {
    if !self.supports_vertical_scroll() {
      return;
    }
    let total = self.content_line_count(ctx);
    let viewport = self.scroll_state().last_viewport.get().max(1);
    let max = total.saturating_sub(viewport);
    if self.scroll_state().v_scroll < max {
      self.scroll_state_mut().v_scroll += 1;
    }
  }

  fn scroll_page_up(&mut self, ctx: &AppContext) {
    for _ in 0..20 {
      self.scroll_up(ctx);
    }
  }

  fn scroll_page_down(&mut self, ctx: &AppContext) {
    for _ in 0..20 {
      self.scroll_down(ctx);
    }
  }

  fn scroll_left(&mut self) {
    if self.supports_horizontal_scroll() {
      self.scroll_state_mut().h_scroll = self.scroll_state_mut().h_scroll.saturating_sub(4);
    }
  }

  fn scroll_right(&mut self) {
    if self.supports_horizontal_scroll() {
      self.scroll_state_mut().h_scroll += 4;
    }
  }

  fn render_scrollbar(
    &mut self,
    area: Rect,
    viewport_height: usize,
    frame: &mut Frame,
    ctx: &AppContext,
  ) {
    if !self.supports_vertical_scroll() {
      return;
    }
    let total = self.content_line_count(ctx);
    self.scroll_state_mut().last_viewport.set(viewport_height);
    if total == 0 || total <= viewport_height {
      return;
    }
    let s = self.scroll_state_mut();
    let max_scroll = total.saturating_sub(viewport_height);
    if s.v_scroll > max_scroll {
      s.v_scroll = max_scroll;
    }
    let scrollbar_content = max_scroll + 1;
    s.v_scrollbar = s
      .v_scrollbar
      .content_length(scrollbar_content)
      .position(s.v_scroll)
      .viewport_content_length(1);
    let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
      .begin_symbol(None)
      .end_symbol(None);
    frame.render_stateful_widget(sb, area, &mut s.v_scrollbar);
  }
}
