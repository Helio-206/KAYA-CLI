use crate::frame::VoiceFrame;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct JitterBuffer {
    max_frames: usize,
    frames: BTreeMap<u64, VoiceFrame>,
    last_played: Option<u64>,
}

impl JitterBuffer {
    pub fn new(max_frames: usize) -> Self {
        Self {
            max_frames: max_frames.max(1),
            frames: BTreeMap::new(),
            last_played: None,
        }
    }

    pub fn push(&mut self, frame: VoiceFrame) -> bool {
        if self
            .last_played
            .map(|last| frame.sequence <= last)
            .unwrap_or(false)
        {
            return false;
        }
        self.frames.insert(frame.sequence, frame);
        while self.frames.len() > self.max_frames {
            let Some(first) = self.frames.keys().next().copied() else {
                break;
            };
            self.frames.remove(&first);
        }
        true
    }

    pub fn pop_next(&mut self) -> Option<VoiceFrame> {
        let sequence = self.frames.keys().next().copied()?;
        let frame = self.frames.remove(&sequence)?;
        self.last_played = Some(sequence);
        Some(frame)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::VoiceFrameSecurity;

    #[test]
    fn jitter_buffer_orders_and_drops_late_frames() {
        let mut jitter = JitterBuffer::new(4);
        jitter.push(VoiceFrame::new("voice-1", 2, vec![2], VoiceFrameSecurity::Encrypted));
        jitter.push(VoiceFrame::new("voice-1", 1, vec![1], VoiceFrameSecurity::Encrypted));

        assert_eq!(jitter.pop_next().unwrap().sequence, 1);
        assert!(!jitter.push(VoiceFrame::new(
            "voice-1",
            1,
            vec![1],
            VoiceFrameSecurity::Encrypted
        )));
        assert_eq!(jitter.pop_next().unwrap().sequence, 2);
    }
}
