use crate::errors::{FileTransferError, FileTransferResult};
use crate::hashing::{sha256_hex, verify_sha256};
use crate::metadata::FileMetadata;
use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChunk {
    pub file_id: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub chunk_hash: String,
    pub payload: Vec<u8>,
    pub timestamp: String,
}

impl FileChunk {
    pub fn new(
        file_id: impl Into<String>,
        chunk_index: u32,
        total_chunks: u32,
        payload: Vec<u8>,
    ) -> Self {
        let chunk_hash = sha256_hex(&payload);
        Self {
            file_id: file_id.into(),
            chunk_index,
            total_chunks,
            chunk_hash,
            payload,
            timestamp: now_millis().to_string(),
        }
    }

    pub fn validate(&self) -> FileTransferResult<()> {
        if self.chunk_index >= self.total_chunks {
            return Err(FileTransferError::InvalidChunk {
                file_id: self.file_id.clone(),
                reason: "chunk index out of range".into(),
            });
        }
        if !verify_sha256(&self.payload, &self.chunk_hash) {
            return Err(FileTransferError::InvalidChunk {
                file_id: self.file_id.clone(),
                reason: "chunk hash mismatch".into(),
            });
        }
        Ok(())
    }
}

pub fn chunk_bytes(
    file_id: &str,
    bytes: &[u8],
    chunk_size: usize,
) -> FileTransferResult<Vec<FileChunk>> {
    if chunk_size == 0 {
        return Err(FileTransferError::InvalidChunkSize);
    }
    let total_chunks = if bytes.is_empty() {
        0
    } else {
        bytes.len().div_ceil(chunk_size) as u32
    };
    Ok(bytes
        .chunks(chunk_size)
        .enumerate()
        .map(|(index, payload)| {
            FileChunk::new(file_id, index as u32, total_chunks, payload.to_vec())
        })
        .collect())
}

pub fn reassemble_chunks(
    metadata: &FileMetadata,
    chunks: &[FileChunk],
) -> FileTransferResult<Vec<u8>> {
    let mut by_index: BTreeMap<u32, &FileChunk> = BTreeMap::new();
    for chunk in chunks {
        chunk.validate()?;
        if chunk.file_id != metadata.file_id {
            return Err(FileTransferError::InvalidChunk {
                file_id: chunk.file_id.clone(),
                reason: "chunk belongs to another file".into(),
            });
        }
        if chunk.total_chunks != metadata.total_chunks {
            return Err(FileTransferError::InvalidChunk {
                file_id: chunk.file_id.clone(),
                reason: "total chunk count mismatch".into(),
            });
        }
        by_index.insert(chunk.chunk_index, chunk);
    }

    let mut bytes = Vec::with_capacity(metadata.file_size as usize);
    for index in 0..metadata.total_chunks {
        let Some(chunk) = by_index.get(&index) else {
            return Err(FileTransferError::MissingChunk {
                file_id: metadata.file_id.clone(),
                chunk_index: index,
            });
        };
        bytes.extend_from_slice(&chunk.payload);
    }
    if bytes.len() as u64 != metadata.file_size || !verify_sha256(&bytes, &metadata.sha256) {
        return Err(FileTransferError::HashMismatch {
            file_id: metadata.file_id.clone(),
        });
    }
    Ok(bytes)
}
