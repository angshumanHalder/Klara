use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use winit::window::Window;

use crate::{
    layout::{LayoutNode, Rect, SplitDirection},
    pane::{Pane, PaneState},
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
            .find(|(pane, _)| Arc::as_ptr(pane) == target)
            .map(|(_, rect)| rect)
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

        self.resize_panes()?;

        Ok(())
    }

    pub fn resize(&mut self, width: f32, height: f32) -> anyhow::Result<()> {
        self.width = width;
        self.height = height;

        self.resize_panes()
    }

    fn resize_panes(&mut self) -> anyhow::Result<()> {
        let layouts = self.pane_layouts();

        for (pane, rect) in layouts {
            let (rows, cols) = rect_to_grid(rect);

            let mut pane = pane
                .lock()
                .map_err(|_| anyhow::anyhow!("pane lock is poisoned"))?;

            pane.resize(rows, cols, CELL_W as usize, CELL_H as usize)?;
        }

        Ok(())
    }

    pub fn poll_children(&mut self) -> anyhow::Result<Vec<(String, PaneState)>> {
        let mut transitions = Vec::new();

        for (pane, _) in self.pane_layouts() {
            let mut pane = pane
                .lock()
                .map_err(|_| anyhow::anyhow!("pane lock is poisoned"))?;

            let prev = pane.state()?;
            let curr = pane.poll_child()?;

            if curr != prev {
                transitions.push((pane.id.clone(), curr));
            }
        }

        Ok(transitions)
    }

    pub fn shutdown_all(&mut self) -> anyhow::Result<()> {
        let mut failures = Vec::new();

        for (pane, _) in self.pane_layouts() {
            let mut pane = match pane.lock() {
                Ok(pane) => pane,
                Err(_) => {
                    failures.push("pane lock is poisoned".to_string());
                    continue;
                }
            };

            if let Err(error) = pane.shutdown() {
                failures.push(format!("pane {} failed to shutdown: {error}", pane.id));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(failures.join("; ")))
        }
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

    #[test]
    fn resize_propagates_dimensions_to_single_pane() {
        let mut wm = WindowManager::new(800.0, 600.0, None).unwrap();

        wm.resize(640.0, 480.0).unwrap();

        let layouts = wm.pane_layouts();
        assert_eq!(layouts.len(), 1);

        let pane = layouts[0].0.lock().unwrap();
        assert_eq!(pane.rows, 28);
        assert_eq!(pane.cols, 80);

        let grid = pane.grid.lock().unwrap();

        assert_eq!(grid.rows, 28);
        assert_eq!(grid.cols, 80);
    }

    #[test]
    fn split_resizes_original_and_new_panes() {
        let mut wm = WindowManager::new(800.0, 600.0, None).unwrap();

        wm.split_pane(crate::layout::SplitDirection::Vertical)
            .unwrap();

        let layouts = wm.pane_layouts();
        assert_eq!(layouts.len(), 2);

        for (pane, _) in layouts {
            let pane = pane.lock().unwrap();

            assert_eq!(pane.rows, 36);
            assert_eq!(pane.cols, 50);

            let grid = pane.grid.lock().unwrap();

            assert_eq!(grid.rows, 36);
            assert_eq!(grid.cols, 50);
        }
    }
}
