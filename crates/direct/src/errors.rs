use thiserror::Error;

#[derive(Debug, Error)]
pub enum DirectError {
    #[error("direct io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("direct protocol error: {0}")]
    Protocol(String),
    #[error("direct malformed frame: {0}")]
    MalformedFrame(String),
    #[error("direct invalid handshake: {0}")]
    InvalidHandshake(String),
    #[error("direct channel closed: {0}")]
    ChannelClosed(String),
}

pub type DirectResult<T> = std::result::Result<T, DirectError>;
