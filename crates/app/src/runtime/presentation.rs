use super::Runtime;
use kaya_events::KayaEvent;
use kaya_persistence::{HistoryRecord, KnownPeer};
use kaya_protocol::Packet;
use kaya_rooms::ChatMessage;
use kaya_shared::now_millis;
use kaya_ui::{UiMessage, UiPeer, UiRoom};
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
            .map(|peer| UiPeer {
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
    }

    pub(super) fn publish(&self, event: KayaEvent) {
        if let Err(err) = self.bus.publish(event) {
            error!(%err, "event publish failed");
        }
    }
}
