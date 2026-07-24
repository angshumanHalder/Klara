use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

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

    #[error("pane process is no longer available")]
    ProcessUnavailable,

    #[error("pane is not running")]
    NotRunning,

    #[error("PTY master is no longer available")]
    MasterUnavailable,

    #[error("PTY reader did not stop within {timeout_ms} ms")]
    ReaderShutdownTimeout { timeout_ms: u64 },

    #[error("PTY reader thread panicked")]
    ReaderThreadPanicked,
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

    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn Child + Send + Sync>>,
    writer: Option<Box<dyn Write + Send>>,
    state: Arc<Mutex<PaneState>>,

    reader_task: Option<ReaderTask>,
    shutting_down: Arc<AtomicBool>,
}

struct ReaderTask {
    handle: JoinHandle<()>,
    finished: Receiver<()>,
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
        let shutting_down = Arc::new(AtomicBool::new(false));

        let reader_task = spawn_reader(
            reader,
            Arc::clone(&grid),
            Arc::clone(&state),
            Arc::clone(&shutting_down),
            window,
        );

        Ok(Pane {
            id,
            grid,
            rows,
            cols,
            master: Some(pair.master),
            child: Some(child),
            writer: Some(writer),
            state,
            shutting_down,
            reader_task: Some(reader_task),
        })
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<(), PaneError> {
        if self.state()? != PaneState::Running {
            return Err(PaneError::NotRunning);
        }

        let writer = self.writer.as_mut().ok_or(PaneError::ProcessUnavailable)?;
        writer
            .write_all(data)
            .map_err(|source| PaneError::WriteInput { source })?;

        writer
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

        self.master
            .as_ref()
            .ok_or(PaneError::MasterUnavailable)?
            .resize(size)
            .map_err(|source| PaneError::Pty {
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

        let child = self.child.as_mut().ok_or(PaneError::ProcessUnavailable)?;

        let exit_status = child.try_wait().map_err(|source| PaneError::Pty {
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
        self.child.as_ref().and_then(|child| child.process_id())
    }

    pub fn shutdown(&mut self) -> Result<PaneState, PaneError> {
        self.shutting_down.store(true, Ordering::Release);
        let current_state = self.state()?;

        self.writer.take();

        if matches!(current_state, PaneState::Exited { .. }) {
            self.child.take();
            self.master.take();
            self.finish_reader(Duration::from_secs(1))?;

            return Ok(current_state);
        }

        let child = self.child.as_mut().ok_or(PaneError::ProcessUnavailable)?;

        let status = match child.try_wait().map_err(|source| PaneError::Pty {
            operation: "poll child process during shutdown",
            source: source.into(),
        })? {
            Some(status) => status,
            None => {
                child.kill().map_err(|source| PaneError::Pty {
                    operation: "terminate child process",
                    source: source.into(),
                })?;
                child.wait().map_err(|source| PaneError::Pty {
                    operation: "reap child process",
                    source: source.into(),
                })?
            }
        };

        let next_state = PaneState::Exited {
            code: status.exit_code(),
            success: status.success(),
        };

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| PaneError::LifecycleStatePoisoned)?;

            *state = next_state.clone();
        }

        self.child.take();
        self.master.take();

        self.finish_reader(Duration::from_secs(1))?;

        Ok(next_state)
    }

    fn finish_reader(&mut self, timeout: Duration) -> Result<(), PaneError> {
        let Some(reader_task) = self.reader_task.take() else {
            return Ok(());
        };

        match reader_task.finished.recv_timeout(timeout) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => reader_task
                .handle
                .join()
                .map_err(|_| PaneError::ReaderThreadPanicked),
            Err(RecvTimeoutError::Timeout) => {
                let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
                self.reader_task = Some(reader_task);
                Err(PaneError::ReaderShutdownTimeout { timeout_ms })
            }
        }
    }

    fn force_cleanup(&mut self) {
        self.shutting_down.store(true, Ordering::Release);

        self.writer.take();

        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // already reaped
                }
                Ok(None) | Err(_) => {
                    if let Err(error) = child.kill() {
                        log::error!(
                            "pane {} failed to terminate child during fallback cleanup: {error}",
                            self.id
                        );
                    }

                    if let Err(error) = child.wait() {
                        log::error!(
                            "pane {} failed to reap child during fallback cleanup: {error}",
                            self.id
                        );
                    }
                }
            }
        }

        self.master.take();

        if let Err(error) = self.finish_reader(Duration::from_millis(100)) {
            log::error!(
                "pane {} failed to stop reader during fallback cleanup: {error}",
                self.id
            );
        }
    }

    #[cfg(test)]
    fn is_shutdown_complete(&self) -> bool {
        self.child.is_none()
            && self.writer.is_none()
            && self.master.is_none()
            && self.reader_task.is_none()
    }
}

impl Drop for Pane {
    fn drop(&mut self) {
        let already_clean = self.child.is_none()
            && self.writer.is_none()
            && self.master.is_none()
            && self.reader_task.is_none();

        if already_clean {
            return;
        }

        if let Err(error) = self.shutdown() {
            log::error!(
                "pane {} failed normal shutdown during drop: {error}",
                self.id
            );
            self.force_cleanup();
        }
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
    shutting_down: Arc<AtomicBool>,
    window: Option<Arc<Window>>,
) -> ReaderTask {
    let (finished_tx, finished) = mpsc::sync_channel(1);

    let handle = std::thread::spawn(move || {
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
                    if !shutting_down.load(Ordering::Acquire) {
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

        let _ = finished_tx.send(());
    });

    ReaderTask { handle, finished }
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
    fn newly_created_pane_can_be_shutdown() {
        let mut pane = Pane::new("test".into(), 24, 80, None).unwrap();

        assert_eq!(pane.state().unwrap(), PaneState::Running);
        assert!(pane.process_id().is_some());

        let state = pane.shutdown().unwrap();

        assert!(matches!(state, PaneState::Exited { .. }));
        assert_eq!(pane.state().unwrap(), state);
        assert!(pane.process_id().is_none());
        assert!(pane.is_shutdown_complete());
        assert!(matches!(
            pane.write_input(b"test"),
            Err(PaneError::NotRunning)
        ));
    }

    #[test]
    fn shutdown_is_idempotent() {
        let mut pane = Pane::new("test".into(), 24, 80, None).unwrap();

        let first = pane.shutdown().unwrap();
        let second = pane.shutdown().unwrap();

        assert_eq!(first, second);
        assert!(pane.is_shutdown_complete());
    }

    #[test]
    fn dropping_running_pane_performs_cleanup() {
        let pane = Pane::new("test".into(), 24, 80, None).unwrap();

        assert!(pane.process_id().is_some());
        drop(pane);
    }
}
