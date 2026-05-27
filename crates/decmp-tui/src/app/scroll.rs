use crate::app::App;

impl App {
  pub fn scroll_preview_up(&mut self) {
    self.preview.scroll = self.preview.scroll.saturating_sub(1);
  }

  pub fn scroll_preview_down(&mut self) {
    if !self.preview.content.lines.is_empty()
      && self.preview.scroll < self.preview.content.lines.len().saturating_sub(1)
    {
      self.preview.scroll += 1;
    }
  }

  pub fn scroll_preview_page_up(&mut self) {
    for _ in 0..20 {
      self.scroll_preview_up();
    }
  }

  pub fn scroll_preview_page_down(&mut self) {
    for _ in 0..20 {
      self.scroll_preview_down();
    }
  }
}
