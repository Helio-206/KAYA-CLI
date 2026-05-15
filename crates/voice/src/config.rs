use crate::errors::{VoiceError, VoiceResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub input_device: String,
    pub output_device: String,
    pub opus_bitrate: u32,
    pub opus_frame_ms: u16,
    pub push_to_talk_key: String,
    pub allow_mesh: bool,
    pub allow_relay: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            input_device: "default".into(),
            output_device: "default".into(),
            opus_bitrate: 24_000,
            opus_frame_ms: 20,
            push_to_talk_key: "space".into(),
            allow_mesh: true,
            allow_relay: false,
        }
    }
}

impl VoiceConfig {
    pub fn validate(&self) -> VoiceResult<()> {
        if self.input_device.trim().is_empty() {
            return Err(VoiceError::Codec("input_device cannot be empty".into()));
        }
        if self.output_device.trim().is_empty() {
            return Err(VoiceError::Codec("output_device cannot be empty".into()));
        }
        if self.opus_bitrate == 0 {
            return Err(VoiceError::Codec("opus_bitrate must be greater than zero".into()));
        }
        if !matches!(self.opus_frame_ms, 10 | 20 | 40 | 60) {
            return Err(VoiceError::Codec(
                "opus_frame_ms must be one of 10, 20, 40 or 60".into(),
            ));
        }
        if self.push_to_talk_key.trim().is_empty() {
            return Err(VoiceError::Codec(
                "push_to_talk_key cannot be empty".into(),
            ));
        }
        Ok(())
    }
}
