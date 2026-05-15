use thiserror::Error;

pub type VoiceResult<T> = std::result::Result<T, VoiceError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VoiceError {
    #[error("voice is disabled")]
    Disabled,
    #[error("audio error: {0}")]
    Audio(String),
    #[error("not joined to a voice room")]
    NotJoined,
    #[error("already joined to voice room {0}")]
    AlreadyJoined(String),
    #[error("invalid voice frame: {0}")]
    InvalidFrame(String),
    #[error("opus codec error: {0}")]
    Codec(String),
}
