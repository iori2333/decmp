use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::tile::TileId;

pub enum LayoutNode {
  Split {
    direction: Direction,
    constraints: Vec<Constraint>,
    children: Vec<LayoutNode>,
  },
  Leaf {
    tile_id: TileId,
    visible: bool,
  },
}

impl LayoutNode {
  pub fn resolve(&self, area: Rect) -> HashMap<TileId, Rect> {
    let mut map = HashMap::new();
    self.resolve_into(area, &mut map);
    map
  }

  fn resolve_into(&self, area: Rect, map: &mut HashMap<TileId, Rect>) {
    match self {
      LayoutNode::Split {
        direction,
        constraints,
        children,
      } => {
        let layout = Layout::new(*direction, constraints.clone());
        let chunks = layout.split(area);
        for (child, chunk) in children.iter().zip(chunks.iter()) {
          child.resolve_into(*chunk, map);
        }
      }
      LayoutNode::Leaf { tile_id, visible } => {
        if *visible {
          map.insert(*tile_id, area);
        }
      }
    }
  }

  pub fn default_layout() -> Self {
    LayoutNode::Split {
      direction: Direction::Vertical,
      constraints: vec![Constraint::Min(1), Constraint::Length(1)],
      children: vec![
        LayoutNode::Split {
          direction: Direction::Horizontal,
          constraints: vec![Constraint::Percentage(50), Constraint::Percentage(50)],
          children: vec![
            LayoutNode::Leaf {
              tile_id: TileId::FileList,
              visible: true,
            },
            LayoutNode::Split {
              direction: Direction::Vertical,
              constraints: vec![Constraint::Percentage(30), Constraint::Percentage(70)],
              children: vec![
                LayoutNode::Leaf {
                  tile_id: TileId::Properties,
                  visible: true,
                },
                LayoutNode::Leaf {
                  tile_id: TileId::Preview,
                  visible: true,
                },
              ],
            },
          ],
        },
        LayoutNode::Leaf {
          tile_id: TileId::StatusBar,
          visible: true,
        },
      ],
    }
  }
}
