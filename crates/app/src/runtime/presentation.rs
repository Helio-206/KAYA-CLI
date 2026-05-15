use super::Runtime;
use kaya_events::KayaEvent;
use kaya_persistence::{HistoryRecord, KnownPeer};
use kaya_protocol::Packet;
use kaya_rooms::ChatMessage;
use kaya_shared::now_millis;
use kaya_ui::{UiFileTransfer, UiMeshDiagnostics, UiMessage, UiPeer, UiRoom};
use tracing::error;

impl Runtime {
    pub(super) fn system_message(&mut self, body: impl Into<String>) {
        self.ui_state.push_message(UiMessage {
            timestamp: now_millis().to_string(),
            room: Some(self.rooms.current_room().to_string()),
            from: "system".into(),
            target: None,
            body: body.into(),
            direct: false,
            encrypted: false,
            local: false,
        });
    }

    pub(super) fn push_chat_message(&mut self, message: &ChatMessage, local: bool) {
        self.ui_state.push_message(UiMessage {
            timestamp: message.timestamp.clone(),
            room: message.room.clone(),
            from: message.from_callsign.clone(),
            target: message.target_node.clone(),
            body: message.body.clone(),
            direct: message.direct,
            encrypted: message.encrypted,
            local,
        });
    }

    pub(super) fn persist_chat_message(&mut self, message: &ChatMessage) {
        let record = HistoryRecord {
            timestamp: now_millis().to_string(),
            room: message.room.clone(),
            target: message.target_node.clone(),
            from: message.from_callsign.clone(),
            body: message.body.clone(),
            direct: message.direct,
            encrypted: message.encrypted,
            event: false,
        };
        if let Err(err) = self.store.append_history(&record) {
            self.publish(KayaEvent::ErrorOccurred {
                scope: "persistence.history".into(),
                message: err.to_string(),
            });
        }
    }

    pub(super) fn remember_peer(&mut self, packet: &Packet) {
        let peer = KnownPeer {
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
            fingerprint: self
                .trust_store
                .get(&packet.node_id)
                .map(|peer| peer.fingerprint.clone()),
            last_seen: packet.timestamp.clone(),
        };
        if let Err(err) = self.store.remember_peer(&peer) {
            self.publish(KayaEvent::ErrorOccurred {
                scope: "persistence.peers".into(),
                message: err.to_string(),
            });
        }
    }

    pub(super) fn sync_peers_to_ui(&mut self) {
        self.ui_state.peers = self
            .peers
            .snapshots()
            .into_iter()
            .filter(|peer| !self.trust_store.is_blocked(&peer.node_id))
            .map(|peer| UiPeer {
                fingerprint: self
                    .trust_store
                    .get(&peer.node_id)
                    .map(|known| short_fingerprint(&known.fingerprint)),
                trust_status: self.trust_store.status(&peer.node_id).to_string(),
                node_id: peer.node_id,
                callsign: peer.callsign,
                presence: peer.presence,
                online: peer.online,
                latency_ms: peer.latency_ms,
            })
            .collect();
        self.ui_state.rooms = self
            .rooms
            .summaries()
            .into_iter()
            .map(|room| UiRoom {
                current: room.name == self.rooms.current_room(),
                name: room.name,
                member_count: room.member_count,
                joined: room.local_joined,
            })
            .collect();
        let peers = self.peers.snapshots();
        self.ui_state.current_members = self
            .rooms
            .current_members()
            .into_iter()
            .map(|node_id| {
                if node_id == self.node_id {
                    format!("{} {}", self.callsign, node_id)
                } else {
                    peers
                        .iter()
                        .find(|peer| peer.node_id == node_id)
                        .map(|peer| format!("{} {}", peer.callsign, peer.node_id))
                        .unwrap_or(node_id)
                }
            })
            .collect();
        self.ui_state.current_room = self.rooms.current_room().to_string();
        self.ui_state.space = self.rooms.current_room().to_string();
        self.ui_state.presence = self.presence;
        self.sync_security_to_ui();
        self.sync_voice_to_ui();
        self.sync_files_to_ui();
        self.sync_mesh_to_ui();
    }

    pub(super) fn sync_security_to_ui(&mut self) {
        self.ui_state.identity_fingerprint = self.identity.short_fingerprint();
        self.ui_state.trusted_peers = self.trust_store.trusted_count();
        self.ui_state.blocked_peers = self.trust_store.blocked_count();
        self.ui_state.secure_sessions = self.sessions.active_count();
    }

    pub(super) fn sync_voice_to_ui(&mut self) {
        self.ui_state.voice.enabled = self.voice.enabled;
        self.ui_state.voice.room = self.voice.current.as_ref().map(|session| session.room.clone());
        self.ui_state.voice.session_id = self
            .voice
            .current
            .as_ref()
            .map(|session| session.session_id.clone());
        self.ui_state.voice.muted = self
            .voice
            .current
            .as_ref()
            .map(|session| session.muted)
            .unwrap_or(false);
        self.ui_state.voice.push_to_talk = self
            .voice
            .current
            .as_ref()
            .map(|session| matches!(session.ptt, kaya_voice::PushToTalkState::Holding))
            .unwrap_or(false);
        self.ui_state.voice.active_speakers = self.voice.active_speakers.values().cloned().collect();
        self.ui_state.voice.active_speakers.sort();
        self.ui_state.voice.frames_rx = self.voice.frames_rx;
        self.ui_state.voice.frames_tx = self.voice.frames_tx;
        self.ui_state.voice.packets_lost = self.voice.packets_lost;
    }

    pub(super) fn sync_files_to_ui(&mut self) {
        self.ui_state.files = self
            .files
            .sessions()
            .into_iter()
            .map(|session| {
                let progress = session.progress();
                UiFileTransfer {
                    file_id: session.file_id,
                    file_name: session.metadata.file_name,
                    peer: session.peer_callsign,
                    percent: progress.percent,
                    status: session.status.to_string(),
                    security: session.security.to_string(),
                    trusted: session.trusted,
                    signed: session.signed,
                    hash_ok: match session.status {
                        kaya_files::TransferStatus::Completed => Some(true),
                        kaya_files::TransferStatus::Corrupted => Some(false),
                        _ => None,
                    },
                }
            })
            .collect();
    }

    pub(super) fn sync_mesh_to_ui(&mut self) {
        let diagnostics = self.mesh.diagnostics_snapshot();
        self.ui_state.mesh = UiMeshDiagnostics {
            enabled: diagnostics.enabled,
            routes: diagnostics.routes,
            relayed_packets: diagnostics.relayed_packets,
            delivered_packets: diagnostics.delivered_packets,
            dropped_packets: diagnostics.dropped_packets,
            avg_hop_count: diagnostics.avg_hop_count(),
            last_route_discovered: diagnostics.last_route_discovered,
            current_route_trace: diagnostics.current_route_trace,
        };
    }

    pub(super) fn publish(&self, event: KayaEvent) {
        if let Err(err) = self.bus.publish(event) {
            error!(%err, "event publish failed");
        }
    }
}

fn short_fingerprint(fingerprint: &str) -> String {
    fingerprint
        .strip_prefix(kaya_security::FINGERPRINT_PREFIX)
        .unwrap_or(fingerprint)
        .to_string()
}
