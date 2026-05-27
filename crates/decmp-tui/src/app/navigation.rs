use crate::app::{App, Focus};
use crate::tree::{DirNode, DirTree};

impl App {
  pub fn current_tree(&self) -> &DirTree {
    let mut tree = &self.archive.tree;
    for part in &self.nav.current_path {
      if let Some(DirNode::Dir(subtree)) = tree.children.get(part) {
        tree = subtree;
      }
    }
    tree
  }

  pub fn current_display_path(&self) -> String {
    if self.nav.current_path.is_empty() {
      String::new()
    } else {
      self.nav.current_path.join("/") + "/"
    }
  }

  pub fn display_entries(&self) -> Vec<(String, Option<&DirNode>)> {
    let mut result = Vec::new();
    if !self.nav.current_path.is_empty() {
      result.push((String::from(".."), None));
    }
    for (name, node) in self.current_tree().sorted_entries() {
      result.push((name.clone(), Some(node)));
    }
    result
  }

  pub(crate) fn selected_info(&self) -> Option<(String, bool)> {
    let idx = self.nav.list_state.selected()?;
    let entries = self.display_entries();
    let (name, node) = entries.get(idx)?;
    let is_dir = name == ".." || matches!(node, Some(DirNode::Dir(_)));
    Some((name.clone(), is_dir))
  }

  pub fn move_up(&mut self) {
    if self.nav.focus == Focus::Right {
      self.scroll_preview_up();
      return;
    }
    self.navigate_list(-1);
  }

  pub fn move_down(&mut self) {
    if self.nav.focus == Focus::Right {
      self.scroll_preview_down();
      return;
    }
    self.navigate_list(1);
  }

  fn navigate_list(&mut self, delta: isize) {
    let len = self.display_entries().len();
    if len == 0 {
      return;
    }
    let i = self.nav.list_state.selected().unwrap_or(0);
    let new = (i as isize + delta).clamp(0, len as isize - 1) as usize;
    self.nav.list_state.select(Some(new));
    self.update_side_preview();
  }

  pub fn enter_selected(&mut self) {
    let Some((name, is_dir)) = self.selected_info() else {
      return;
    };
    if name == ".." || is_dir {
      if name == ".." {
        self.go_up();
      } else {
        self.navigate_into(&name);
      }
    } else {
      self.load_preview();
    }
  }

  pub fn go_up(&mut self) {
    if self.nav.current_path.pop().is_some() {
      self.nav.list_state.select(Some(0));
      self.update_side_preview();
    }
  }

  fn navigate_into(&mut self, name: &str) {
    self.nav.current_path.push(name.to_string());
    self.nav.list_state.select(Some(0));
  }
}
