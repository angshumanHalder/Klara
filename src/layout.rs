use std::sync::{Arc, Mutex};

use crate::pane::Pane;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum SplitDirection {
    Vertical,
    Horizontal,
}

pub enum LayoutNode {
    Leaf(Arc<Mutex<Pane>>),
    Split {
        direction: SplitDirection,
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub fn calculate_layouts(&self, rect: Rect) -> Vec<(Arc<Mutex<Pane>>, Rect)> {
        match self {
            LayoutNode::Leaf(pane) => vec![(Arc::clone(pane), rect)],
            LayoutNode::Split {
                direction,
                left,
                right,
            } => {
                let (lr, rr) = match direction {
                    SplitDirection::Vertical => {
                        let half = rect.width / 2.0;
                        (
                            Rect {
                                x: rect.x,
                                y: rect.y,
                                width: half,
                                height: rect.height,
                            },
                            Rect {
                                x: rect.x + half,
                                y: rect.y,
                                width: rect.width - half,
                                height: rect.height,
                            },
                        )
                    }
                    SplitDirection::Horizontal => {
                        let half = rect.height / 2.0;
                        (
                            Rect {
                                x: rect.x,
                                y: rect.y,
                                width: rect.width,
                                height: half,
                            },
                            Rect {
                                x: rect.x,
                                y: rect.y + half,
                                width: rect.width,
                                height: rect.height - half,
                            },
                        )
                    }
                };
                let mut out = left.calculate_layouts(lr);
                out.extend(right.calculate_layouts(rr));
                out
            }
        }
    }

    pub fn split_leaf(
        self,
        target: *const Mutex<Pane>,
        direction: SplitDirection,
        new_pane: Arc<Mutex<Pane>>,
    ) -> Self {
        match self {
            LayoutNode::Leaf(ref pane) if Arc::as_ptr(pane) == target => LayoutNode::Split {
                direction,
                left: Box::new(self),
                right: Box::new(LayoutNode::Leaf(new_pane)),
            },
            LayoutNode::Split {
                direction: d,
                left,
                right,
            } => LayoutNode::Split {
                direction: d,
                left: Box::new(left.split_leaf(target, direction, Arc::clone(&new_pane))),
                right: Box::new(right.split_leaf(target, direction, new_pane)),
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_pane() -> Arc<Mutex<Pane>> {
        Arc::new(Mutex::new(
            Pane::new("test".to_string(), 24, 80, None).unwrap(),
        ))
    }

    #[test]
    fn test_calculate_layouts_single_leaf() {
        let pane = make_pane();
        let root = LayoutNode::Leaf(Arc::clone(&pane));
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let layouts = root.calculate_layouts(rect);
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].1.width, 800.0);
    }

    #[test]
    fn test_split_leaf_vertical() {
        let pane = make_pane();
        let target = Arc::as_ptr(&pane);
        let root = LayoutNode::Leaf(Arc::clone(&pane));
        let new_pane = make_pane();
        let root = root.split_leaf(target, SplitDirection::Vertical, new_pane);
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let layouts = root.calculate_layouts(rect);
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].1.width, 400.0);
        assert_eq!(layouts[1].1.x, 400.0);
    }
}
