use thiserror::Error;

pub type FileTransferResult<T> = std::result::Result<T, FileTransferError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FileTransferError {
    #[error("file transfer is disabled")]
    Disabled,
    #[error("file is too large: {actual} bytes exceeds {max} bytes")]
    FileTooLarge { max: u64, actual: u64 },
    #[error("invalid file name: {0}")]
    InvalidFileName(String),
    #[error("invalid chunk size")]
    InvalidChunkSize,
    #[error("invalid chunk for file {file_id}: {reason}")]
    InvalidChunk { file_id: String, reason: String },
    #[error("missing chunk {chunk_index} for file {file_id}")]
    MissingChunk { file_id: String, chunk_index: u32 },
    #[error("hash mismatch for file {file_id}")]
    HashMismatch { file_id: String },
    #[error("unknown file transfer {0}")]
    UnknownTransfer(String),
    #[error("invalid transfer state for {file_id}: {state}")]
    InvalidState { file_id: String, state: String },
    #[error("io error: {0}")]
    Io(String),
    #[error("json error: {0}")]
    Json(String),
}

impl From<std::io::Error> for FileTransferError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for FileTransferError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}
