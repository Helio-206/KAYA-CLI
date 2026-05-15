use crate::errors::{VoiceError, VoiceResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpusCodecConfig {
    pub bitrate: u32,
    pub frame_ms: u16,
}

impl Default for OpusCodecConfig {
    fn default() -> Self {
        Self {
            bitrate: 24_000,
            frame_ms: 20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpusFrameCodec {
    config: OpusCodecConfig,
}

impl OpusFrameCodec {
    pub const fn backend_name() -> &'static str {
        "pcm-fallback"
    }

    pub fn new(config: OpusCodecConfig) -> VoiceResult<Self> {
        if config.bitrate == 0 {
            return Err(VoiceError::Codec("bitrate must be greater than zero".into()));
        }
        if !matches!(config.frame_ms, 10 | 20 | 40 | 60) {
            return Err(VoiceError::Codec(
                "frame_ms must be one of 10, 20, 40 or 60".into(),
            ));
        }
        Ok(Self { config })
    }

    pub fn encode_pcm_i16(&self, pcm: &[i16]) -> VoiceResult<Vec<u8>> {
        if pcm.is_empty() {
            return Err(VoiceError::Codec("pcm frame cannot be empty".into()));
        }
        let mut encoded = Vec::with_capacity(pcm.len() * 2);
        for sample in pcm {
            encoded.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(encoded)
    }

    pub fn decode_pcm_i16(&self, opus_payload: &[u8]) -> VoiceResult<Vec<i16>> {
        if opus_payload.is_empty() || opus_payload.len() % 2 != 0 {
            return Err(VoiceError::Codec("invalid encoded frame size".into()));
        }
        Ok(opus_payload
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect())
    }

    pub fn config(&self) -> OpusCodecConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_codec_roundtrips_pcm_frames() {
        let codec = OpusFrameCodec::new(OpusCodecConfig::default()).unwrap();
        let pcm = vec![0_i16, 123, -456, 2048];

        let encoded = codec.encode_pcm_i16(&pcm).unwrap();
        let decoded = codec.decode_pcm_i16(&encoded).unwrap();

        assert_eq!(decoded, pcm);
    }
}
