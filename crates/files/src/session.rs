use crate::chunk::{chunk_bytes, reassemble_chunks, FileChunk};
use crate::errors::{FileTransferError, FileTransferResult};
use crate::metadata::{FileMetadata, FileTransferConfig};
use crate::progress::TransferProgress;
use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Offered,
    Accepted,
    Transferring,
    Paused,
    Completed,
    Rejected,
    Cancelled,
    Failed,
    Corrupted,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Offered => f.write_str("offered"),
            TransferStatus::Accepted => f.write_str("accepted"),
            TransferStatus::Transferring => f.write_str("transferring"),
            TransferStatus::Paused => f.write_str("paused"),
            TransferStatus::Completed => f.write_str("completed"),
            TransferStatus::Rejected => f.write_str("rejected"),
            TransferStatus::Cancelled => f.write_str("cancelled"),
            TransferStatus::Failed => f.write_str("failed"),
            TransferStatus::Corrupted => f.write_str("corrupted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferSecurity {
    Unencrypted,
    Encrypted,
}

impl std::fmt::Display for TransferSecurity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferSecurity::Unencrypted => f.write_str("unencrypted"),
            TransferSecurity::Encrypted => f.write_str("encrypted"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferSession {
    pub file_id: String,
    pub metadata: FileMetadata,
    pub peer_node_id: String,
    pub peer_callsign: String,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub security: TransferSecurity,
    pub signed: bool,
    pub trusted: bool,
    pub bytes_received: u64,
    pub chunks_received: u32,
    pub total_chunks: u32,
    pub created_at: String,
    pub updated_at: String,
    pub completed_path: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingFileRequest {
    pub path: PathBuf,
    pub sender_node_id: String,
    pub sender_callsign: String,
    pub peer_node_id: String,
    pub peer_callsign: String,
    pub security: TransferSecurity,
}

impl TransferSession {
    pub fn progress(&self) -> TransferProgress {
        TransferProgress::new(self.bytes_received, self.chunks_received, self.total_chunks)
    }

    fn set_status(&mut self, status: TransferStatus) {
        self.status = status;
        self.updated_at = now_millis().to_string();
    }
}

#[derive(Debug, Default)]
pub struct FileTransferManager {
    sessions: HashMap<String, TransferSession>,
    outgoing_chunks: HashMap<String, Vec<FileChunk>>,
    incoming_chunks: HashMap<String, BTreeMap<u32, FileChunk>>,
}

impl FileTransferManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare_outgoing(
        &mut self,
        request: OutgoingFileRequest,
        config: &FileTransferConfig,
    ) -> FileTransferResult<&TransferSession> {
        let bytes = fs::read(&request.path)?;
        let file_name = request
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("payload.bin");
        let metadata = FileMetadata::from_bytes(
            file_name,
            &bytes,
            None,
            &request.sender_node_id,
            &request.sender_callsign,
            config,
        )?;
        let chunks = chunk_bytes(&metadata.file_id, &bytes, metadata.chunk_size)?;
        let file_id = metadata.file_id.clone();
        let session = new_session(
            metadata,
            &request.peer_node_id,
            &request.peer_callsign,
            TransferDirection::Outgoing,
            request.security,
            true,
            false,
        );
        self.outgoing_chunks.insert(file_id.clone(), chunks);
        self.sessions.insert(file_id.clone(), session);
        self.session(&file_id)
    }

    pub fn receive_offer(
        &mut self,
        metadata: FileMetadata,
        peer_node_id: &str,
        peer_callsign: &str,
        security: TransferSecurity,
        signed: bool,
        trusted: bool,
    ) -> &TransferSession {
        let file_id = metadata.file_id.clone();
        let session = new_session(
            metadata,
            peer_node_id,
            peer_callsign,
            TransferDirection::Incoming,
            security,
            signed,
            trusted,
        );
        self.sessions.insert(file_id.clone(), session);
        self.sessions.get(&file_id).expect("session inserted")
    }

    pub fn accept(&mut self, file_id: &str) -> FileTransferResult<()> {
        let session = self.session_mut(file_id)?;
        if session.status != TransferStatus::Offered {
            return Err(FileTransferError::InvalidState {
                file_id: file_id.to_string(),
                state: session.status.to_string(),
            });
        }
        session.set_status(TransferStatus::Accepted);
        Ok(())
    }

    pub fn mark_outgoing_accepted(&mut self, file_id: &str) -> FileTransferResult<()> {
        let session = self.session_mut(file_id)?;
        session.set_status(TransferStatus::Accepted);
        Ok(())
    }

    pub fn reject(&mut self, file_id: &str) -> FileTransferResult<()> {
        let session = self.session_mut(file_id)?;
        session.set_status(TransferStatus::Rejected);
        Ok(())
    }

    pub fn cancel(&mut self, file_id: &str) -> FileTransferResult<()> {
        let session = self.session_mut(file_id)?;
        session.set_status(TransferStatus::Cancelled);
        Ok(())
    }

    pub fn fail(&mut self, file_id: &str, reason: impl Into<String>) -> FileTransferResult<()> {
        let session = self.session_mut(file_id)?;
        session.failure_reason = Some(reason.into());
        session.set_status(TransferStatus::Failed);
        Ok(())
    }

    pub fn outgoing_chunks(&self, file_id: &str) -> FileTransferResult<&[FileChunk]> {
        self.outgoing_chunks
            .get(file_id)
            .map(Vec::as_slice)
            .ok_or_else(|| FileTransferError::UnknownTransfer(file_id.to_string()))
    }

    pub fn receive_chunk(&mut self, chunk: FileChunk) -> FileTransferResult<Option<Vec<u8>>> {
        chunk.validate()?;
        let file_id = chunk.file_id.clone();
        let metadata = self.session(&file_id)?.metadata.clone();
        let chunks = self.incoming_chunks.entry(file_id.clone()).or_default();
        chunks.insert(chunk.chunk_index, chunk);
        let chunks_received = chunks.len() as u32;
        let bytes_received = chunks
            .values()
            .map(|chunk| chunk.payload.len() as u64)
            .sum();
        let is_complete = chunks_received == metadata.total_chunks;
        let session = self.session_mut(&file_id)?;
        session.bytes_received = bytes_received;
        session.chunks_received = chunks_received;
        session.set_status(TransferStatus::Transferring);

        if !is_complete {
            return Ok(None);
        }

        let ordered: Vec<FileChunk> = self
            .incoming_chunks
            .get(&file_id)
            .map(|chunks| chunks.values().cloned().collect())
            .unwrap_or_default();
        match reassemble_chunks(&metadata, &ordered) {
            Ok(bytes) => {
                let session = self.session_mut(&file_id)?;
                session.bytes_received = bytes.len() as u64;
                session.chunks_received = metadata.total_chunks;
                session.set_status(TransferStatus::Completed);
                Ok(Some(bytes))
            }
            Err(FileTransferError::HashMismatch { .. }) => {
                let session = self.session_mut(&file_id)?;
                session.set_status(TransferStatus::Corrupted);
                Err(FileTransferError::HashMismatch { file_id })
            }
            Err(err) => {
                let session = self.session_mut(&file_id)?;
                session.failure_reason = Some(err.to_string());
                session.set_status(TransferStatus::Failed);
                Err(err)
            }
        }
    }

    pub fn mark_completed_path(
        &mut self,
        file_id: &str,
        completed_path: impl Into<String>,
    ) -> FileTransferResult<()> {
        let session = self.session_mut(file_id)?;
        session.completed_path = Some(completed_path.into());
        session.set_status(TransferStatus::Completed);
        Ok(())
    }

    pub fn session(&self, file_id: &str) -> FileTransferResult<&TransferSession> {
        self.sessions
            .get(file_id)
            .ok_or_else(|| FileTransferError::UnknownTransfer(file_id.to_string()))
    }

    pub fn session_mut(&mut self, file_id: &str) -> FileTransferResult<&mut TransferSession> {
        self.sessions
            .get_mut(file_id)
            .ok_or_else(|| FileTransferError::UnknownTransfer(file_id.to_string()))
    }

    pub fn sessions(&self) -> Vec<TransferSession> {
        let mut sessions: Vec<_> = self.sessions.values().cloned().collect();
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        sessions
    }

    pub fn load_record(&mut self, session: TransferSession) {
        self.sessions.insert(session.file_id.clone(), session);
    }
}

fn new_session(
    metadata: FileMetadata,
    peer_node_id: &str,
    peer_callsign: &str,
    direction: TransferDirection,
    security: TransferSecurity,
    signed: bool,
    trusted: bool,
) -> TransferSession {
    let now = now_millis().to_string();
    TransferSession {
        file_id: metadata.file_id.clone(),
        total_chunks: metadata.total_chunks,
        metadata,
        peer_node_id: peer_node_id.to_string(),
        peer_callsign: peer_callsign.to_string(),
        direction,
        status: TransferStatus::Offered,
        security,
        signed,
        trusted,
        bytes_received: 0,
        chunks_received: 0,
        created_at: now.clone(),
        updated_at: now,
        completed_path: None,
        failure_reason: None,
    }
}
