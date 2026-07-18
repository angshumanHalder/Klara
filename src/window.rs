use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use winit::window::Window;

use crate::{
    layout::{LayoutNode, Rect, SplitDirection},
    pane::Pane,
};

pub const STATUS_BAR_HEIGHT: f32 = 24.0;
const CELL_W: f32 = 8.0;
const CELL_H: f32 = 16.0;

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn next_id() -> String {
    NEXT_ID
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .to_string()
}

fn rect_to_grid(rect: Rect) -> (usize, usize) {
    let rows = (rect.height / CELL_H).max(1.0) as usize;
    let cols = (rect.width / CELL_W).max(1.0) as usize;
    (rows, cols)
}

pub struct WindowManager {
    pub root: LayoutNode,
    pub active: Arc<Mutex<Pane>>,
    pub width: f32,
    pub height: f32,
    window: Option<Arc<Window>>,
}

impl WindowManager {
    pub fn new(width: f32, height: f32, window: Option<Arc<Window>>) -> anyhow::Result<Self> {
        let content = Rect {
            x: 0.0,
            y: 0.0,
            width,
            height: height - STATUS_BAR_HEIGHT,
        };
        let (rows, cols) = rect_to_grid(content);
        let pane = Arc::new(Mutex::new(Pane::new(
            next_id(),
            rows,
            cols,
            window.clone(),
        )?));
        Ok(Self {
            root: LayoutNode::Leaf(Arc::clone(&pane)),
            active: pane,
            width,
            height,
            window,
        })
    }

    pub fn content_rect(&self) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: self.width,
            height: self.height - STATUS_BAR_HEIGHT,
        }
    }

    pub fn pane_layouts(&self) -> Vec<(Arc<Mutex<Pane>>, Rect)> {
        self.root.calculate_layouts(self.content_rect())
    }

    pub fn split_pane(&mut self, direction: SplitDirection) -> anyhow::Result<()> {
        let target = Arc::as_ptr(&self.active);
        let active_rect = self
            .pane_layouts()
            .into_iter()
            .find(|(p, _)| Arc::as_ptr(p) == target)
            .map(|(_, r)| r)
            .unwrap_or(self.content_rect());

        let child_rect = match direction {
            SplitDirection::Vertical => Rect {
                width: active_rect.width / 2.0,
                ..active_rect
            },
            SplitDirection::Horizontal => Rect {
                height: active_rect.height / 2.0,
                ..active_rect
            },
        };

        let (rows, cols) = rect_to_grid(child_rect);
        let new_pane = Arc::new(Mutex::new(Pane::new(
            next_id(),
            rows,
            cols,
            self.window.clone(),
        )?));

        let old_root =
            std::mem::replace(&mut self.root, LayoutNode::Leaf(Arc::clone(&self.active)));
        self.root = old_root.split_leaf(target, direction, Arc::clone(&new_pane));
        self.active = new_pane;
        Ok(())
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }
}

#[cfg(test)]
mod test {
    use crate::window::WindowManager;

    #[test]
    fn test_new_creats_single_pane() {
        let wm = WindowManager::new(800.0, 600.0, None).unwrap();
        assert_eq!(wm.pane_layouts().len(), 1);
    }

    #[test]
    fn test_split_pane_vertical() {
        let mut wm = WindowManager::new(800.0, 600.0, None).unwrap();
        wm.split_pane(crate::layout::SplitDirection::Vertical)
            .unwrap();
        let layouts = wm.pane_layouts();
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].1.width, layouts[1].1.width);
    }
}
