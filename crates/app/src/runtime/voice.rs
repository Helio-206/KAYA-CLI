use super::Runtime;
use crate::runtime::voice_media::{pcm_has_voice_activity, VoiceMediaRuntime, VoiceRuntimeEvent};
use kaya_events::KayaEvent;
use kaya_protocol::{
    Packet, PacketType, VoiceFramePayload, VoiceHeartbeatPayload, VoiceStartPayload,
    VoiceStopPayload,
};
use kaya_voice::{OpusCodecConfig, OpusFrameCodec, PushToTalkState};

impl Runtime {
    pub(super) async fn join_voice(&mut self, room: &str) {
        if !self.config.voice.enabled {
            self.system_message("voice is disabled in config");
            return;
        }
        if !self.rooms.is_joined(room) {
            self.system_message(format!("join #{room} before /voice-join"));
            return;
        }

        let session = match self.voice.join(room) {
            Ok(session) => session.clone(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };

        self.voice.active_speakers.clear();
        self.start_voice_media();
        self.publish(KayaEvent::VoiceJoined {
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
            room: session.room.clone(),
            session_id: session.session_id.clone(),
            local: true,
        });
        self.sync_voice_to_ui();
        self.send_packet(Packet::voice_start(
            self.node_id.clone(),
            self.callsign.clone(),
            session.room.clone(),
            self.voice_start_payload(&session),
        ))
        .await;
        if let Some(packet) = self.voice_heartbeat_packet() {
            self.send_packet(packet).await;
        }
    }

    pub(super) async fn leave_voice(&mut self, reason: &str) {
        let session = match self.voice.leave() {
            Ok(session) => session,
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };

        self.voice.active_speakers.clear();
        self.stop_voice_media();
        self.publish(KayaEvent::VoiceLeft {
            node_id: self.node_id.clone(),
            room: session.room.clone(),
            session_id: Some(session.session_id.clone()),
            local: true,
        });
        self.sync_voice_to_ui();
        self.send_packet(Packet::voice_stop(
            self.node_id.clone(),
            self.callsign.clone(),
            session.room,
            VoiceStopPayload {
                session_id: session.session_id,
                reason: reason.to_string(),
            },
        ))
        .await;
    }

    pub(super) async fn set_voice_muted(&mut self, muted: bool) {
        match self.voice.set_muted(muted) {
            Ok(()) => {
                self.sync_voice_to_ui();
                self.system_message(if muted { "voice muted" } else { "voice unmuted" });
                if let Some(packet) = self.voice_heartbeat_packet() {
                    self.send_packet(packet).await;
                }
            }
            Err(err) => self.system_message(err.to_string()),
        }
    }

    pub(super) async fn toggle_voice_ptt(&mut self) {
        match self.voice.toggle_ptt() {
            Ok(state) => {
                self.sync_voice_to_ui();
                self.system_message(match state {
                    PushToTalkState::Idle => "push-to-talk released",
                    PushToTalkState::Holding => "push-to-talk engaged",
                });
                if let Some(packet) = self.voice_heartbeat_packet() {
                    self.send_packet(packet).await;
                }
            }
            Err(err) => self.system_message(err.to_string()),
        }
    }

    pub(super) async fn set_voice_ptt_holding(&mut self, holding: bool) {
        let previous = self.voice.current.as_ref().map(|session| session.ptt);
        let Ok(state) = self.voice.set_ptt(holding) else {
            return;
        };
        if previous == Some(state) {
            return;
        }
        self.sync_voice_to_ui();
        if let Some(packet) = self.voice_heartbeat_packet() {
            self.send_packet(packet).await;
        }
    }

    pub(super) fn show_voice_status(&mut self) {
        let devices = kaya_voice::list_audio_devices()
            .map(|devices| devices.len().to_string())
            .unwrap_or_else(|err| format!("unavailable ({err})"));
        if let Some(session) = &self.voice.current {
            let speakers = if self.voice.active_speakers.is_empty() {
                "none".to_string()
            } else {
                self.voice
                    .active_speakers
                    .values()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            self.system_message(format!(
                "voice room=#{} session={} codec={} devices={} muted={} ptt={} frames_tx={} frames_rx={} lost={} speakers={}",
                session.room,
                session.session_id,
                kaya_voice::OpusFrameCodec::backend_name(),
                devices,
                session.muted,
                matches!(session.ptt, PushToTalkState::Holding),
                self.voice.frames_tx,
                self.voice.frames_rx,
                self.voice.packets_lost,
                speakers,
            ));
        } else if self.voice.enabled {
            self.system_message("voice ready; join with /voice-join <room>");
        } else {
            self.system_message("voice disabled in config");
        }
    }

    pub(super) fn voice_heartbeat_packet(&self) -> Option<Packet> {
        let session = self.voice.current.as_ref()?;
        Some(Packet::voice_heartbeat(
            self.node_id.clone(),
            self.callsign.clone(),
            session.room.clone(),
            VoiceHeartbeatPayload {
                session_id: session.session_id.clone(),
                muted: session.muted,
                push_to_talk: matches!(session.ptt, PushToTalkState::Holding),
                speaking: matches!(session.ptt, PushToTalkState::Holding) && !session.muted,
                packets_lost: self.voice.packets_lost,
            },
        ))
    }

    pub(super) async fn route_voice_packet(&mut self, packet: &Packet) -> bool {
        let Some(room) = packet.room.as_deref() else {
            return matches!(
                packet.packet_type,
                PacketType::VoiceStart
                    | PacketType::VoiceStop
                    | PacketType::VoiceFrame
                    | PacketType::VoiceHeartbeat
            );
        };

        if !self.monitors_voice_room(room) {
            return matches!(
                packet.packet_type,
                PacketType::VoiceStart
                    | PacketType::VoiceStop
                    | PacketType::VoiceFrame
                    | PacketType::VoiceHeartbeat
            );
        }

        match packet.packet_type {
            PacketType::VoiceStart => {
                let Ok(payload) = serde_json::from_value::<VoiceStartPayload>(packet.payload.clone())
                else {
                    self.system_message("voice start payload malformed");
                    return true;
                };
                self.voice.observe_speaker(packet.node_id.clone(), packet.callsign.clone());
                self.publish(KayaEvent::VoiceJoined {
                    node_id: packet.node_id.clone(),
                    callsign: packet.callsign.clone(),
                    room: room.to_string(),
                    session_id: payload.session_id,
                    local: false,
                });
                self.sync_voice_to_ui();
                true
            }
            PacketType::VoiceStop => {
                let Ok(payload) = serde_json::from_value::<VoiceStopPayload>(packet.payload.clone())
                else {
                    self.system_message("voice stop payload malformed");
                    return true;
                };
                self.voice.remove_speaker(&packet.node_id);
                self.publish(KayaEvent::VoiceLeft {
                    node_id: packet.node_id.clone(),
                    room: room.to_string(),
                    session_id: Some(payload.session_id),
                    local: false,
                });
                self.sync_voice_to_ui();
                true
            }
            PacketType::VoiceHeartbeat => {
                let Ok(payload) =
                    serde_json::from_value::<VoiceHeartbeatPayload>(packet.payload.clone())
                else {
                    self.system_message("voice heartbeat payload malformed");
                    return true;
                };
                if payload.speaking && !payload.muted {
                    self.voice.observe_speaker(packet.node_id.clone(), packet.callsign.clone());
                } else {
                    self.voice.remove_speaker(&packet.node_id);
                }
                self.sync_voice_to_ui();
                true
            }
            PacketType::VoiceFrame => {
                if packet.target_node.is_some() && !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(payload) = serde_json::from_value::<VoiceFramePayload>(packet.payload.clone())
                else {
                    self.system_message("voice frame payload malformed");
                    return true;
                };
                self.voice.record_rx_frame();
                self.voice.observe_speaker(packet.node_id.clone(), packet.callsign.clone());
                if packet.node_id != self.node_id {
                    match self.voice_codec().and_then(|codec| codec.decode_pcm_i16(&payload.opus_payload)) {
                        Ok(samples) => {
                            if let Some(media) = &self.voice_media {
                                media.queue_playback(samples);
                            }
                        }
                        Err(err) => self.publish(KayaEvent::ErrorOccurred {
                            scope: "voice.decode".into(),
                            message: err.to_string(),
                        }),
                    }
                }
                self.sync_voice_to_ui();
                true
            }
            _ => false,
        }
    }

    fn monitors_voice_room(&self, room: &str) -> bool {
        self.voice
            .current
            .as_ref()
            .map(|session| session.room == room)
            .unwrap_or_else(|| self.rooms.current_room() == room)
    }

    pub(super) fn voice_start_payload(
        &self,
        session: &kaya_voice::VoiceRoomSession,
    ) -> VoiceStartPayload {
        VoiceStartPayload {
            session_id: session.session_id.clone(),
            codec: kaya_voice::OpusFrameCodec::backend_name().into(),
            bitrate: self.config.voice.opus_bitrate,
            frame_ms: self.config.voice.opus_frame_ms,
            muted: session.muted,
            push_to_talk: matches!(session.ptt, PushToTalkState::Holding),
            encrypted: false,
        }
    }

    pub(super) async fn handle_voice_runtime_event(&mut self, event: VoiceRuntimeEvent) {
        match event {
            VoiceRuntimeEvent::CapturedPcm { samples } => {
                let Some(session) = self.voice.current.clone() else {
                    return;
                };
                if session.muted {
                    return;
                }

                let speaking = matches!(session.ptt, PushToTalkState::Holding)
                    || pcm_has_voice_activity(&samples);
                if !speaking {
                    return;
                }

                let sequence = match self.voice.next_sequence() {
                    Ok(sequence) => sequence,
                    Err(err) => {
                        self.publish(KayaEvent::ErrorOccurred {
                            scope: "voice.sequence".into(),
                            message: err.to_string(),
                        });
                        return;
                    }
                };
                let payload = match self.voice_codec().and_then(|codec| codec.encode_pcm_i16(&samples)) {
                    Ok(payload) => payload,
                    Err(err) => {
                        self.publish(KayaEvent::ErrorOccurred {
                            scope: "voice.encode".into(),
                            message: err.to_string(),
                        });
                        return;
                    }
                };

                self.sync_voice_to_ui();
                self.send_packet(Packet::voice_frame(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    session.room,
                    VoiceFramePayload {
                        session_id: session.session_id,
                        sequence,
                        timestamp: kaya_shared::now_millis().to_string(),
                        opus_payload: payload,
                        encrypted: false,
                    },
                ))
                .await;
            }
            VoiceRuntimeEvent::BackendError { scope, message } => {
                self.publish(KayaEvent::ErrorOccurred {
                    scope: scope.into(),
                    message,
                });
            }
        }
    }

    fn start_voice_media(&mut self) {
        if self.voice_media.is_some() {
            return;
        }
        self.voice_media = Some(VoiceMediaRuntime::start(&self.config.voice));
    }

    pub(super) fn stop_voice_media(&mut self) {
        if let Some(runtime) = self.voice_media.take() {
            runtime.stop();
        }
    }

    fn voice_codec(&self) -> Result<OpusFrameCodec, kaya_voice::VoiceError> {
        OpusFrameCodec::new(OpusCodecConfig {
            bitrate: self.config.voice.opus_bitrate,
            frame_ms: self.config.voice.opus_frame_ms,
        })
    }
}