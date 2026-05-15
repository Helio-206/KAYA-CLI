use crate::config::VoiceConfig;
use crate::errors::{VoiceError, VoiceResult};
use kaya_shared::{now_millis, validate_room_name};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushToTalkState {
    #[default]
    Idle,
    Holding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomSession {
    pub room: String,
    pub session_id: String,
    pub joined_at: String,
    pub muted: bool,
    pub ptt: PushToTalkState,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceState {
    pub enabled: bool,
    pub current: Option<VoiceRoomSession>,
    pub active_speakers: BTreeMap<String, String>,
    pub frames_rx: u64,
    pub frames_tx: u64,
    pub packets_lost: u64,
}

impl VoiceState {
    pub fn new(config: &VoiceConfig) -> Self {
        Self {
            enabled: config.enabled,
            current: None,
            active_speakers: BTreeMap::new(),
            frames_rx: 0,
            frames_tx: 0,
            packets_lost: 0,
        }
    }

    pub fn join(&mut self, room: &str) -> VoiceResult<&VoiceRoomSession> {
        if !self.enabled {
            return Err(VoiceError::Disabled);
        }
        if let Some(current) = &self.current {
            return Err(VoiceError::AlreadyJoined(current.room.clone()));
        }
        let room = validate_room_name(room).map_err(|err| VoiceError::InvalidFrame(err.to_string()))?;
        self.current = Some(VoiceRoomSession {
            room,
            session_id: format!("voice-{}", Uuid::new_v4()),
            joined_at: now_millis().to_string(),
            muted: false,
            ptt: PushToTalkState::Idle,
            sequence: 0,
        });
        Ok(self.current.as_ref().expect("voice session inserted"))
    }

    pub fn leave(&mut self) -> VoiceResult<VoiceRoomSession> {
        self.current.take().ok_or(VoiceError::NotJoined)
    }

    pub fn set_muted(&mut self, muted: bool) -> VoiceResult<()> {
        let session = self.current.as_mut().ok_or(VoiceError::NotJoined)?;
        session.muted = muted;
        if muted {
            session.ptt = PushToTalkState::Idle;
        }
        Ok(())
    }

    pub fn toggle_ptt(&mut self) -> VoiceResult<PushToTalkState> {
        let session = self.current.as_mut().ok_or(VoiceError::NotJoined)?;
        if session.muted {
            session.ptt = PushToTalkState::Idle;
            return Ok(session.ptt);
        }
        session.ptt = match session.ptt {
            PushToTalkState::Idle => PushToTalkState::Holding,
            PushToTalkState::Holding => PushToTalkState::Idle,
        };
        Ok(session.ptt)
    }

    pub fn set_ptt(&mut self, holding: bool) -> VoiceResult<PushToTalkState> {
        let session = self.current.as_mut().ok_or(VoiceError::NotJoined)?;
        if session.muted {
            session.ptt = PushToTalkState::Idle;
            return Ok(session.ptt);
        }
        session.ptt = if holding {
            PushToTalkState::Holding
        } else {
            PushToTalkState::Idle
        };
        Ok(session.ptt)
    }

    pub fn next_sequence(&mut self) -> VoiceResult<u64> {
        let session = self.current.as_mut().ok_or(VoiceError::NotJoined)?;
        let sequence = session.sequence;
        session.sequence = session.sequence.saturating_add(1);
        self.frames_tx = self.frames_tx.saturating_add(1);
        Ok(sequence)
    }

    pub fn record_rx_frame(&mut self) {
        self.frames_rx = self.frames_rx.saturating_add(1);
    }

    pub fn record_packet_loss(&mut self, lost: u64) {
        self.packets_lost = self.packets_lost.saturating_add(lost);
    }

    pub fn observe_speaker(&mut self, node_id: impl Into<String>, callsign: impl Into<String>) {
        self.active_speakers.insert(node_id.into(), callsign.into());
    }

    pub fn remove_speaker(&mut self, node_id: &str) {
        self.active_speakers.remove(node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_session_lifecycle_and_mute_ptt_work() {
        let mut state = VoiceState::new(&VoiceConfig::default());

        assert_eq!(state.join("semana-info").unwrap().room, "semana-info");
        assert_eq!(state.toggle_ptt().unwrap(), PushToTalkState::Holding);
        assert_eq!(state.set_ptt(false).unwrap(), PushToTalkState::Idle);
        state.set_muted(true).unwrap();
        assert_eq!(state.current.as_ref().unwrap().ptt, PushToTalkState::Idle);
        assert_eq!(state.leave().unwrap().room, "semana-info");
    }
}
