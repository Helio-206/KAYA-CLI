use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceFrameSecurity {
    Unencrypted,
    Encrypted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceFrame {
    pub session_id: String,
    pub sequence: u64,
    pub timestamp: String,
    pub opus_payload: Vec<u8>,
    pub security: VoiceFrameSecurity,
}

impl VoiceFrame {
    pub fn new(
        session_id: impl Into<String>,
        sequence: u64,
        opus_payload: Vec<u8>,
        security: VoiceFrameSecurity,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            sequence,
            timestamp: now_millis().to_string(),
            opus_payload,
            security,
        }
    }
}
