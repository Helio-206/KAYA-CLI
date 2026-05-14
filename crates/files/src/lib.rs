mod chunk;
mod errors;
mod hashing;
mod metadata;
mod progress;
mod session;
mod store;

pub use chunk::{chunk_bytes, reassemble_chunks, FileChunk, DEFAULT_CHUNK_SIZE};
pub use errors::{FileTransferError, FileTransferResult};
pub use hashing::{sha256_hex, verify_sha256};
pub use metadata::{
    dangerous_extension_warning, safe_file_name, validate_file_name, FileMetadata,
    FileTransferConfig,
};
pub use progress::TransferProgress;
pub use session::{
    FileTransferManager, OutgoingFileRequest, TransferDirection, TransferSecurity, TransferSession,
    TransferStatus,
};
pub use store::{FileStore, StoredTransferRecord};
