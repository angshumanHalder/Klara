use thiserror::Error;
use winit::error::EventLoopError;

use crate::config::ConfigError;

#[derive(Debug, Error)]
pub enum KlaraError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("window event loop failed")]
    EventLoop(#[from] EventLoopError),
}
