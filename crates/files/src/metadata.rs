use crate::errors::{FileTransferError, FileTransferResult};
use crate::hashing::sha256_hex;
use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};
use uuid::Uuid;

pub const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTransferConfig {
    pub enabled: bool,
    pub max_file_size_bytes: u64,
    pub chunk_size: usize,
    pub accept_from_unknown: bool,
    pub download_dir: Option<String>,
}

impl Default for FileTransferConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size_bytes: DEFAULT_MAX_FILE_SIZE_BYTES,
            chunk_size: crate::DEFAULT_CHUNK_SIZE,
            accept_from_unknown: true,
            download_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: Option<String>,
    pub sha256: String,
    pub chunk_size: usize,
    pub total_chunks: u32,
    pub sender_node_id: String,
    pub sender_callsign: String,
    pub created_at: String,
    pub dangerous_extension: bool,
}

impl FileMetadata {
    pub fn from_bytes(
        file_name: &str,
        bytes: &[u8],
        mime_type: Option<String>,
        sender_node_id: &str,
        sender_callsign: &str,
        config: &FileTransferConfig,
    ) -> FileTransferResult<Self> {
        if !config.enabled {
            return Err(FileTransferError::Disabled);
        }
        if config.chunk_size == 0 {
            return Err(FileTransferError::InvalidChunkSize);
        }
        let file_size = bytes.len() as u64;
        if file_size > config.max_file_size_bytes {
            return Err(FileTransferError::FileTooLarge {
                max: config.max_file_size_bytes,
                actual: file_size,
            });
        }
        let file_name = safe_file_name(file_name)?;
        let sha256 = sha256_hex(bytes);
        let total_chunks = total_chunks(file_size, config.chunk_size);
        let file_id = generate_file_id(&sha256, &file_name, sender_node_id);

        Ok(Self {
            file_id,
            file_name: file_name.clone(),
            file_size,
            mime_type,
            sha256,
            chunk_size: config.chunk_size,
            total_chunks,
            sender_node_id: sender_node_id.to_string(),
            sender_callsign: sender_callsign.to_string(),
            created_at: now_millis().to_string(),
            dangerous_extension: dangerous_extension_warning(&file_name),
        })
    }
}

pub fn validate_file_name(file_name: &str) -> FileTransferResult<()> {
    safe_file_name(file_name).map(|_| ())
}

pub fn safe_file_name(file_name: &str) -> FileTransferResult<String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(FileTransferError::InvalidFileName(
            "file name cannot be empty".into(),
        ));
    }
    if trimmed.contains('\0') || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(FileTransferError::InvalidFileName(
            "path separators are not allowed".into(),
        ));
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(FileTransferError::InvalidFileName(
            "absolute paths are not allowed".into(),
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(FileTransferError::InvalidFileName(
            "path traversal is not allowed".into(),
        ));
    }
    if matches!(trimmed, "." | "..") {
        return Err(FileTransferError::InvalidFileName(
            "dot path names are not allowed".into(),
        ));
    }
    Ok(trimmed.to_string())
}

pub fn dangerous_extension_warning(file_name: &str) -> bool {
    let Some((_, extension)) = file_name.rsplit_once('.') else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "exe" | "bat" | "cmd" | "com" | "scr" | "ps1" | "vbs" | "js" | "sh" | "app"
    )
}

fn total_chunks(file_size: u64, chunk_size: usize) -> u32 {
    if file_size == 0 {
        0
    } else {
        file_size.div_ceil(chunk_size as u64) as u32
    }
}

fn generate_file_id(sha256: &str, file_name: &str, sender_node_id: &str) -> String {
    let seed = format!("{sha256}:{file_name}:{sender_node_id}:{}", Uuid::new_v4());
    format!(
        "KF-{}",
        &sha256_hex(seed.as_bytes())[..12].to_ascii_uppercase()
    )
}
