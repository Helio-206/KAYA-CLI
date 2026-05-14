use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("relay io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("relay protocol error: {0}")]
    Protocol(String),
    #[error("relay channel closed: {0}")]
    ChannelClosed(String),
    #[error("relay policy denied: {0}")]
    Policy(String),
    #[error("relay registration failed: {0}")]
    Registration(String),
    #[error("relay malformed frame: {0}")]
    MalformedFrame(String),
}

pub type RelayResult<T> = std::result::Result<T, RelayError>;
