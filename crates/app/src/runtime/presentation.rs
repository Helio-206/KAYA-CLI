use super::Runtime;
use kaya_events::KayaEvent;
use kaya_persistence::{HistoryRecord, KnownPeer};
use kaya_protocol::Packet;
use kaya_rooms::ChatMessage;
use kaya_shared::now_millis;
use kaya_ui::{UiMessage, UiPeer};
use tracing::error;

impl Runtime {
    pub(super) fn system_message(&mut self, body: impl Into<String>) {
        self.ui_state.push_message(UiMessage {
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
            from: message.from_callsign.clone(),
            body: message.body.clone(),
            direct: message.direct,
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
                online: peer.online,
                latency_ms: peer.latency_ms,
            })
            .collect();
    }

    pub(super) fn publish(&self, event: KayaEvent) {
        if let Err(err) = self.bus.publish(event) {
            error!(%err, "event publish failed");
        }
    }
}
