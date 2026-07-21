use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use thiserror::Error;
use winit::window::Window;

use crate::terminal::{TerminalError, grid::Grid};

#[derive(Debug, Error)]
pub enum PaneError {
    #[error(
        "PTY dimension `{field}` is out of range: received {value}, maximum is {}",
        u16::MAX
    )]
    DimensionOutOfRange { field: &'static str, value: usize },

    #[error("failed to {operation}")]
    Pty {
        operation: &'static str,
        #[source]
        source: anyhow::Error,
    },

    #[error("failed to write input to PTY")]
    WriteInput {
        #[source]
        source: std::io::Error,
    },

    #[error("terminal state lock is poisoned")]
    TerminalStatePoisoned,

    #[error(transparent)]
    Terminal(#[from] TerminalError),

    #[error("pane lifecycle state lock is poisoned")]
    LifecycleStatePoisoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneState {
    Running,
    Exited { code: u32, success: bool },
    Failed { message: String },
}

pub struct Pane {
    pub id: String,
    pub grid: Arc<Mutex<Grid>>,
    pub rows: usize,
    pub cols: usize,

    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    state: Arc<Mutex<PaneState>>,
}

impl Pane {
    pub fn new(
        id: String,
        rows: usize,
        cols: usize,
        window: Option<Arc<Window>>,
    ) -> Result<Self, PaneError> {
        let size = pty_size(rows, cols, 0, 0)?;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let pty_system = native_pty_system();

        let pair = pty_system.openpty(size).map_err(|source| PaneError::Pty {
            operation: "open PTY",
            source,
        })?;

        let mut command = CommandBuilder::new(&shell);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "klara");

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|source| PaneError::Pty {
                operation: "spawn shell",
                source,
            })?;

        let writer = pair.master.take_writer().map_err(|source| PaneError::Pty {
            operation: "open PTY writer",
            source,
        })?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|source| PaneError::Pty {
                operation: "open PTY reader",
                source,
            })?;

        let grid = Arc::new(Mutex::new(Grid::new(rows, cols)));
        let state = Arc::new(Mutex::new(PaneState::Running));

        spawn_reader(reader, Arc::clone(&grid), Arc::clone(&state), window);

        Ok(Pane {
            id,
            grid,
            rows,
            cols,
            master: pair.master,
            child,
            writer,
            state,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<(), PaneError> {
        self.writer
            .write_all(data)
            .map_err(|source| PaneError::WriteInput { source })?;

        self.writer
            .flush()
            .map_err(|source| PaneError::WriteInput { source })
    }

    pub fn resize(
        &mut self,
        rows: usize,
        cols: usize,
        pixel_width: usize,
        pixel_height: usize,
    ) -> Result<(), PaneError> {
        let size = pty_size(rows, cols, pixel_width, pixel_height)?;

        self.master.resize(size).map_err(|source| PaneError::Pty {
            operation: "resize PTY",
            source,
        })?;

        let mut grid = self
            .grid
            .lock()
            .map_err(|_| PaneError::TerminalStatePoisoned)?;

        grid.resize(rows, cols)?;

        self.rows = rows;
        self.cols = cols;

        Ok(())
    }

    pub fn poll_child(&mut self) -> Result<PaneState, PaneError> {
        let current_state = self.state()?;

        if matches!(current_state, PaneState::Exited { .. }) {
            return Ok(current_state);
        }

        let exit_status = self.child.try_wait().map_err(|source| PaneError::Pty {
            operation: "poll child process",
            source: source.into(),
        })?;

        let next_state = match exit_status {
            Some(status) => PaneState::Exited {
                code: status.exit_code(),
                success: status.success(),
            },
            None => current_state.clone(),
        };

        if next_state != current_state {
            let mut state = self
                .state
                .lock()
                .map_err(|_| PaneError::LifecycleStatePoisoned)?;

            *state = next_state.clone();
        }

        Ok(next_state)
    }

    pub fn state(&self) -> Result<PaneState, PaneError> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| PaneError::LifecycleStatePoisoned)
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }
}

fn to_pty_dimension(field: &'static str, value: usize) -> Result<u16, PaneError> {
    u16::try_from(value).map_err(|_| PaneError::DimensionOutOfRange { field, value })
}

fn pty_size(
    rows: usize,
    cols: usize,
    pixel_width: usize,
    pixel_height: usize,
) -> Result<PtySize, PaneError> {
    Ok(PtySize {
        rows: to_pty_dimension("rows", rows)?,
        cols: to_pty_dimension("cols", cols)?,
        pixel_width: to_pty_dimension("pixel_width", pixel_width)?,
        pixel_height: to_pty_dimension("pixel_height", pixel_height)?,
    })
}

fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    grid: Arc<Mutex<Grid>>,
    state: Arc<Mutex<PaneState>>,
    window: Option<Arc<Window>>,
) {
    std::thread::spawn(move || {
        let mut parser = vte::Parser::new();
        let mut buf = [0u8; 4096];
        loop {
            let bytes_read = match reader.read(&mut buf) {
                Ok(0) => {
                    if let Some(window) = &window {
                        window.request_redraw();
                    }
                    break;
                }
                Ok(bytes_read) => bytes_read,
                Err(error) => {
                    let message = format!("PTY reader failed: {error}");
                    log::error!("{message}");

                    match state.lock() {
                        Ok(mut state) => {
                            *state = PaneState::Failed { message };
                        }
                        Err(lock_error) => {
                            log::error!("pane lifecycle state lock is poisoned: {lock_error}");
                        }
                    }

                    break;
                }
            };

            let dirty = {
                let mut grid = match grid.lock() {
                    Ok(grid) => grid,
                    Err(error) => {
                        log::error!("terminal state lock is poisoned: {error}");
                        break;
                    }
                };

                for &byte in &buf[..bytes_read] {
                    parser.advance(&mut *grid, byte);
                }

                grid.dirty.iter().any(|&row| row)
            };

            if dirty && let Some(window) = &window {
                window.request_redraw();
            }
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pty_size_rejects_dimensions_larger_than_u16() {
        let value = usize::from(u16::MAX) + 1;

        let error = pty_size(value, 80, 0, 0).unwrap_err();

        assert!(
            matches!(error, PaneError::DimensionOutOfRange { field: "rows", value: rejected } if rejected == value)
        );
    }

    #[test]
    fn pty_size_reports_invalid_pixel_dimension() {
        let value = usize::from(u16::MAX) + 1;

        let error = pty_size(24, 80, value, 16).unwrap_err();

        assert!(matches!(
        error,
        PaneError::DimensionOutOfRange { field: "pixel_width", value: rejected } if rejected == value
        ));
    }

    #[test]
    fn newly_created_pane_is_running() {
        let pane = Pane::new("test".into(), 24, 80, None).unwrap();

        assert_eq!(pane.state().unwrap(), PaneState::Running);
        assert!(pane.process_id().is_some());
    }
}
