use thiserror::Error;

pub mod cell;
pub mod grid;
mod row;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalError {
    #[error("terminal dimensions must be non-zero, received {rows} row and {cols} columns")]
    InvalidSize { rows: usize, cols: usize },
}
