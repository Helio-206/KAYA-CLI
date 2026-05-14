use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferProgress {
    pub bytes_received: u64,
    pub chunks_received: u32,
    pub total_chunks: u32,
    pub percent: f64,
    pub speed_bytes_per_sec: u64,
    pub eta_secs: Option<u64>,
}

impl TransferProgress {
    pub fn new(bytes_received: u64, chunks_received: u32, total_chunks: u32) -> Self {
        let percent = if total_chunks == 0 {
            100.0
        } else {
            (chunks_received as f64 / total_chunks as f64) * 100.0
        };
        Self {
            bytes_received,
            chunks_received,
            total_chunks,
            percent,
            speed_bytes_per_sec: 0,
            eta_secs: None,
        }
    }
}
