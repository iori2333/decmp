use std::collections::BTreeMap;

use decmp_core::ArchiveEntry;

#[derive(Debug, Clone)]
pub enum DirNode {
  File(ArchiveEntry),
  Dir(DirTree),
}

#[derive(Debug, Clone, Default)]
pub struct DirTree {
  pub children: BTreeMap<String, DirNode>,
}

impl DirTree {
  pub fn from_entries(entries: &[ArchiveEntry]) -> Self {
    let mut root = DirTree::default();
    for entry in entries {
      let name = entry.name.strip_prefix("./").unwrap_or(&entry.name);
      let name = name.trim_end_matches('/');
      if name.is_empty() {
        continue;
      }
      let parts: Vec<&str> = name.split('/').collect();
      root.insert(&parts, entry.clone());
    }
    root
  }

  fn insert(&mut self, parts: &[&str], entry: ArchiveEntry) {
    if parts.is_empty() {
      return;
    }

    if parts.len() == 1 {
      let name = parts[0].to_string();
      if entry.is_dir {
        self
          .children
          .entry(name)
          .or_insert_with(|| DirNode::Dir(DirTree::default()));
      } else {
        self.children.insert(name, DirNode::File(entry));
      }
      return;
    }

    let dir_name = parts[0].to_string();
    let dir = self
      .children
      .entry(dir_name)
      .or_insert_with(|| DirNode::Dir(DirTree::default()));

    if let DirNode::Dir(subtree) = dir {
      subtree.insert(&parts[1..], entry);
    }
  }

  pub fn sorted_entries(&self) -> Vec<(&String, &DirNode)> {
    let mut entries: Vec<_> = self.children.iter().collect();
    entries.sort_by(|a, b| {
      let a_is_dir = matches!(a.1, DirNode::Dir(_));
      let b_is_dir = matches!(b.1, DirNode::Dir(_));
      match (a_is_dir, b_is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.cmp(b.0),
      }
    });
    entries
  }
}

pub fn is_binary_content(data: &[u8]) -> bool {
  let check_len = data.len().min(8000);
  let sample = &data[..check_len];

  if sample.contains(&0) {
    return true;
  }

  let non_printable = sample
    .iter()
    .filter(|&&b| b != b'\n' && b != b'\r' && b != b'\t' && b < 0x20)
    .count();

  non_printable > check_len / 10
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(name: &str, size: u64, is_dir: bool) -> ArchiveEntry {
    ArchiveEntry {
      name: name.to_string(),
      size,
      compressed_size: size / 2,
      is_dir,
      method: "deflate".to_string(),
      modified: None,
    }
  }

  #[test]
  fn test_flat_entries() {
    let entries = vec![entry("a.txt", 100, false), entry("b.txt", 200, false)];
    let tree = DirTree::from_entries(&entries);
    assert_eq!(tree.children.len(), 2);
  }

  #[test]
  fn test_nested_dirs() {
    let entries = vec![
      entry("src/", 0, true),
      entry("src/main.rs", 500, false),
      entry("src/lib.rs", 300, false),
      entry("src/utils/", 0, true),
      entry("src/utils/mod.rs", 100, false),
    ];
    let tree = DirTree::from_entries(&entries);
    assert_eq!(tree.children.len(), 1);

    let src = tree.children.get("src").unwrap();
    match src {
      DirNode::Dir(subtree) => {
        assert_eq!(subtree.children.len(), 3); // main.rs, lib.rs, utils
      }
      _ => panic!("expected dir"),
    }
  }

  #[test]
  fn test_sorted_dirs_first() {
    let entries = vec![
      entry("zebra.txt", 100, false),
      entry("alpha/", 0, true),
      entry("beta.txt", 200, false),
    ];
    let tree = DirTree::from_entries(&entries);
    let sorted = tree.sorted_entries();
    assert_eq!(sorted.len(), 3);
    assert!(matches!(sorted[0].1, DirNode::Dir(_)));
    assert_eq!(sorted[0].0, "alpha");
  }

  #[test]
  fn test_empty_tree() {
    let tree = DirTree::from_entries(&[]);
    assert!(tree.children.is_empty());
  }

  #[test]
  fn test_root_files_and_dirs() {
    let entries = vec![
      entry("file.txt", 50, false),
      entry("dir/", 0, true),
      entry("dir/nested.txt", 30, false),
    ];
    let tree = DirTree::from_entries(&entries);
    let sorted = tree.sorted_entries();
    assert_eq!(sorted.len(), 2);
    assert_eq!(sorted[0].0, "dir");
    assert_eq!(sorted[1].0, "file.txt");
  }

  #[test]
  fn test_strip_dot_slash_prefix() {
    let entries = vec![
      entry("./", 0, true),
      entry("./health.c", 100, false),
      entry("./server.go", 200, false),
      entry("./src/", 0, true),
      entry("./src/main.rs", 300, false),
    ];
    let tree = DirTree::from_entries(&entries);
    let sorted = tree.sorted_entries();
    // "./" is skipped; we get health.c, server.go, src/
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0, "src");
    assert_eq!(sorted[1].0, "health.c");
    assert_eq!(sorted[2].0, "server.go");
  }

  #[test]
  fn test_is_binary_content() {
    assert!(!is_binary_content(b""));
    assert!(!is_binary_content(b"Hello, world!\nThis is text."));
    assert!(!is_binary_content(
      "fn main() {\n  println!(\"你好\");\n}\n".as_bytes()
    ));

    assert!(is_binary_content(b"text\0with\0nulls"));

    let binary = [0u8; 1024];
    assert!(is_binary_content(&binary));

    let mut jpeg_like = Vec::with_capacity(512);
    for i in 0..512 {
      jpeg_like.push((i * 73 + 17) as u8);
    }
    assert!(is_binary_content(&jpeg_like));
  }
}
