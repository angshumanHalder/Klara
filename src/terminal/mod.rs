use thiserror::Error;

pub mod cell;
pub use emulator::{CursorStyle, Terminal};
mod emulator;
mod row;
mod screen;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalError {
    #[error("terminal dimensions must be non-zero, received {rows} row and {cols} columns")]
    InvalidSize { rows: usize, cols: usize },
}
