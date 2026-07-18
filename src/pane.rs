use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use winit::window::Window;

use crate::terminal::grid::Grid;

pub struct Pane {
    pub id: String,
    pub grid: Arc<Mutex<Grid>>,
    pub rows: usize,
    pub cols: usize,
    writer: Box<dyn std::io::Write + Send>,
}

impl Pane {
    pub fn new(
        id: String,
        rows: usize,
        cols: usize,
        window: Option<Arc<Window>>,
    ) -> anyhow::Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let pty_stream = native_pty_system();
        let pair = pty_stream.openpty(PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "klara");

        let _child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        let grid = Arc::new(Mutex::new(Grid::new(rows, cols)));
        let grid_clone = grid.clone();

        std::thread::spawn(move || {
            let mut parser = vte::Parser::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let dirty = {
                            let mut g = grid_clone.lock().unwrap();
                            for &byte in &buf[..n] {
                                parser.advance(&mut *g, byte);
                            }
                            g.dirty.iter().any(|&d| d)
                        };
                        if dirty {
                            if let Some(w) = &window {
                                w.request_redraw();
                            }
                        }
                    }
                }
            }
        });

        Ok(Pane {
            id,
            grid,
            rows,
            cols,
            writer,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
    }
}
