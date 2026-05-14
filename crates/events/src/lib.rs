use kaya_protocol::{Packet, PacketType};
use kaya_shared::{KayaError, Result, EVENT_CHANNEL_CAPACITY};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<KayaEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn system_default() -> Self {
        Self::new(EVENT_CHANNEL_CAPACITY)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<KayaEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: KayaEvent) -> Result<usize> {
        self.sender
            .send(event)
            .map_err(|err| KayaError::ChannelClosed(err.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KayaEvent {
    PeerDiscovered {
        node_id: String,
        callsign: String,
    },
    PeerTimedOut {
        node_id: String,
    },
    PacketReceived {
        packet: Packet,
        source: String,
        bytes: usize,
    },
    PacketSent {
        packet_id: Uuid,
        packet_type: PacketType,
        bytes: usize,
    },
    IdentityLoaded {
        node_id: String,
        fingerprint: String,
    },
    IdentityCreated {
        node_id: String,
        fingerprint: String,
    },
    PacketSignatureValid {
        node_id: String,
        fingerprint: String,
    },
    PacketSignatureInvalid {
        node_id: String,
        reason: String,
    },
    RoomJoined {
        node_id: String,
        callsign: String,
        room: String,
        local: bool,
    },
    RoomCreated {
        node_id: String,
        callsign: String,
        room: String,
        local: bool,
    },
    RoomLeft {
        node_id: String,
        room: Option<String>,
    },
    RoomMessageReceived {
        room: String,
        from_node: String,
        from_callsign: String,
        body: String,
        local: bool,
    },
    DirectMessageSent {
        target_node: String,
        target_callsign: String,
        body: String,
    },
    DirectMessageReceived {
        from_node: String,
        from_callsign: String,
        target_node: String,
        body: String,
        local: bool,
    },
    EncryptedMessageReceived {
        from_node: String,
        from_callsign: String,
        target_node: String,
        body: String,
        local: bool,
    },
    PresenceUpdated {
        node_id: String,
        callsign: String,
        presence: kaya_shared::PresenceStatus,
    },
    PeerTrusted {
        node_id: String,
        callsign: String,
        fingerprint: String,
    },
    PeerBlocked {
        node_id: String,
        callsign: String,
        fingerprint: String,
    },
    SecureSessionStarted {
        peer_node_id: String,
        session_id: String,
    },
    SecureSessionClosed {
        peer_node_id: String,
        session_id: Option<String>,
    },
    FileOfferReceived {
        file_id: String,
        file_name: String,
        from_node: String,
        from_callsign: String,
        size_bytes: u64,
        encrypted: bool,
    },
    FileOfferSent {
        file_id: String,
        file_name: String,
        target_node: String,
        target_callsign: String,
        size_bytes: u64,
        encrypted: bool,
    },
    FileAccepted {
        file_id: String,
        node_id: String,
        callsign: String,
    },
    FileRejected {
        file_id: String,
        node_id: String,
        callsign: String,
        reason: Option<String>,
    },
    FileChunkReceived {
        file_id: String,
        chunk_index: u32,
        total_chunks: u32,
    },
    FileChunkAcked {
        file_id: String,
        chunk_index: u32,
    },
    FileTransferProgress {
        file_id: String,
        bytes_received: u64,
        chunks_received: u32,
        total_chunks: u32,
    },
    FileTransferCompleted {
        file_id: String,
        path: Option<String>,
    },
    FileTransferCancelled {
        file_id: String,
        reason: Option<String>,
    },
    FileTransferFailed {
        file_id: String,
        reason: String,
    },
    FileHashVerified {
        file_id: String,
        sha256: String,
    },
    FileHashMismatch {
        file_id: String,
    },
    RouteDiscovered {
        destination_node: String,
        next_hop: String,
        hop_count: u8,
    },
    RouteExpired {
        destination_node: String,
    },
    RouteRequestSent {
        destination_node: String,
        request_id: String,
    },
    RouteResponseReceived {
        destination_node: String,
        next_hop: String,
        hop_count: u8,
    },
    MeshPacketRelayed {
        mesh_packet_id: String,
        destination_node: String,
        next_hop: String,
        hop_count: u8,
    },
    MeshPacketDropped {
        mesh_packet_id: String,
        reason: String,
    },
    MeshPacketDelivered {
        mesh_packet_id: String,
        source_node: String,
        route_trace: Vec<String>,
    },
    RelayDenied {
        source_node: String,
        destination_node: String,
        reason: String,
    },
    RouteError {
        destination_node: String,
        reason: String,
    },
    MeshDiagnosticsUpdated,
    SecurityWarning {
        node_id: Option<String>,
        message: String,
    },
    ErrorOccurred {
        scope: String,
        message: String,
    },
    NetworkStarted {
        multicast_addr: String,
    },
    ShutdownInitiated {
        reason: String,
    },
}

impl KayaEvent {
    pub fn kind(&self) -> EventKind {
        match self {
            KayaEvent::PeerDiscovered { .. } => EventKind::PeerDiscovered,
            KayaEvent::PeerTimedOut { .. } => EventKind::PeerTimedOut,
            KayaEvent::PacketReceived { .. } => EventKind::PacketReceived,
            KayaEvent::PacketSent { .. } => EventKind::PacketSent,
            KayaEvent::IdentityLoaded { .. } => EventKind::IdentityLoaded,
            KayaEvent::IdentityCreated { .. } => EventKind::IdentityCreated,
            KayaEvent::PacketSignatureValid { .. } => EventKind::PacketSignatureValid,
            KayaEvent::PacketSignatureInvalid { .. } => EventKind::PacketSignatureInvalid,
            KayaEvent::RoomJoined { .. } => EventKind::RoomJoined,
            KayaEvent::RoomCreated { .. } => EventKind::RoomCreated,
            KayaEvent::RoomLeft { .. } => EventKind::RoomLeft,
            KayaEvent::RoomMessageReceived { .. } => EventKind::RoomMessageReceived,
            KayaEvent::DirectMessageSent { .. } => EventKind::DirectMessageSent,
            KayaEvent::DirectMessageReceived { .. } => EventKind::DirectMessageReceived,
            KayaEvent::EncryptedMessageReceived { .. } => EventKind::EncryptedMessageReceived,
            KayaEvent::PresenceUpdated { .. } => EventKind::PresenceUpdated,
            KayaEvent::PeerTrusted { .. } => EventKind::PeerTrusted,
            KayaEvent::PeerBlocked { .. } => EventKind::PeerBlocked,
            KayaEvent::SecureSessionStarted { .. } => EventKind::SecureSessionStarted,
            KayaEvent::SecureSessionClosed { .. } => EventKind::SecureSessionClosed,
            KayaEvent::FileOfferReceived { .. } => EventKind::FileOfferReceived,
            KayaEvent::FileOfferSent { .. } => EventKind::FileOfferSent,
            KayaEvent::FileAccepted { .. } => EventKind::FileAccepted,
            KayaEvent::FileRejected { .. } => EventKind::FileRejected,
            KayaEvent::FileChunkReceived { .. } => EventKind::FileChunkReceived,
            KayaEvent::FileChunkAcked { .. } => EventKind::FileChunkAcked,
            KayaEvent::FileTransferProgress { .. } => EventKind::FileTransferProgress,
            KayaEvent::FileTransferCompleted { .. } => EventKind::FileTransferCompleted,
            KayaEvent::FileTransferCancelled { .. } => EventKind::FileTransferCancelled,
            KayaEvent::FileTransferFailed { .. } => EventKind::FileTransferFailed,
            KayaEvent::FileHashVerified { .. } => EventKind::FileHashVerified,
            KayaEvent::FileHashMismatch { .. } => EventKind::FileHashMismatch,
            KayaEvent::RouteDiscovered { .. } => EventKind::RouteDiscovered,
            KayaEvent::RouteExpired { .. } => EventKind::RouteExpired,
            KayaEvent::RouteRequestSent { .. } => EventKind::RouteRequestSent,
            KayaEvent::RouteResponseReceived { .. } => EventKind::RouteResponseReceived,
            KayaEvent::MeshPacketRelayed { .. } => EventKind::MeshPacketRelayed,
            KayaEvent::MeshPacketDropped { .. } => EventKind::MeshPacketDropped,
            KayaEvent::MeshPacketDelivered { .. } => EventKind::MeshPacketDelivered,
            KayaEvent::RelayDenied { .. } => EventKind::RelayDenied,
            KayaEvent::RouteError { .. } => EventKind::RouteError,
            KayaEvent::MeshDiagnosticsUpdated => EventKind::MeshDiagnosticsUpdated,
            KayaEvent::SecurityWarning { .. } => EventKind::SecurityWarning,
            KayaEvent::ErrorOccurred { .. } => EventKind::ErrorOccurred,
            KayaEvent::NetworkStarted { .. } => EventKind::NetworkStarted,
            KayaEvent::ShutdownInitiated { .. } => EventKind::ShutdownInitiated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventKind {
    PeerDiscovered,
    PeerTimedOut,
    PacketReceived,
    PacketSent,
    IdentityLoaded,
    IdentityCreated,
    PacketSignatureValid,
    PacketSignatureInvalid,
    RoomCreated,
    RoomJoined,
    RoomLeft,
    RoomMessageReceived,
    DirectMessageSent,
    DirectMessageReceived,
    EncryptedMessageReceived,
    PresenceUpdated,
    PeerTrusted,
    PeerBlocked,
    SecureSessionStarted,
    SecureSessionClosed,
    FileOfferReceived,
    FileOfferSent,
    FileAccepted,
    FileRejected,
    FileChunkReceived,
    FileChunkAcked,
    FileTransferProgress,
    FileTransferCompleted,
    FileTransferCancelled,
    FileTransferFailed,
    FileHashVerified,
    FileHashMismatch,
    RouteDiscovered,
    RouteExpired,
    RouteRequestSent,
    RouteResponseReceived,
    MeshPacketRelayed,
    MeshPacketDropped,
    MeshPacketDelivered,
    RelayDenied,
    RouteError,
    MeshDiagnosticsUpdated,
    SecurityWarning,
    ErrorOccurred,
    NetworkStarted,
    ShutdownInitiated,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::PeerDiscovered => "peer.discovered",
            EventKind::PeerTimedOut => "peer.timed_out",
            EventKind::PacketReceived => "packet.received",
            EventKind::PacketSent => "packet.sent",
            EventKind::IdentityLoaded => "identity.loaded",
            EventKind::IdentityCreated => "identity.created",
            EventKind::PacketSignatureValid => "packet.signature.valid",
            EventKind::PacketSignatureInvalid => "packet.signature.invalid",
            EventKind::RoomCreated => "room.created",
            EventKind::RoomJoined => "room.joined",
            EventKind::RoomLeft => "room.left",
            EventKind::RoomMessageReceived => "room.message",
            EventKind::DirectMessageSent => "dm.sent",
            EventKind::DirectMessageReceived => "dm.message",
            EventKind::EncryptedMessageReceived => "dm.encrypted.message",
            EventKind::PresenceUpdated => "presence.updated",
            EventKind::PeerTrusted => "peer.trusted",
            EventKind::PeerBlocked => "peer.blocked",
            EventKind::SecureSessionStarted => "secure.session.started",
            EventKind::SecureSessionClosed => "secure.session.closed",
            EventKind::FileOfferReceived => "file.offer.received",
            EventKind::FileOfferSent => "file.offer.sent",
            EventKind::FileAccepted => "file.accepted",
            EventKind::FileRejected => "file.rejected",
            EventKind::FileChunkReceived => "file.chunk.received",
            EventKind::FileChunkAcked => "file.chunk.acked",
            EventKind::FileTransferProgress => "file.progress",
            EventKind::FileTransferCompleted => "file.completed",
            EventKind::FileTransferCancelled => "file.cancelled",
            EventKind::FileTransferFailed => "file.failed",
            EventKind::FileHashVerified => "file.hash.verified",
            EventKind::FileHashMismatch => "file.hash.mismatch",
            EventKind::RouteDiscovered => "route.discovered",
            EventKind::RouteExpired => "route.expired",
            EventKind::RouteRequestSent => "route.request.sent",
            EventKind::RouteResponseReceived => "route.response.received",
            EventKind::MeshPacketRelayed => "mesh.packet.relayed",
            EventKind::MeshPacketDropped => "mesh.packet.dropped",
            EventKind::MeshPacketDelivered => "mesh.packet.delivered",
            EventKind::RelayDenied => "mesh.relay.denied",
            EventKind::RouteError => "route.error",
            EventKind::MeshDiagnosticsUpdated => "mesh.diagnostics.updated",
            EventKind::SecurityWarning => "security.warning",
            EventKind::ErrorOccurred => "error",
            EventKind::NetworkStarted => "network.started",
            EventKind::ShutdownInitiated => "shutdown",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventCounters {
    counters: BTreeMap<EventKind, u64>,
}

impl EventCounters {
    pub fn increment(&mut self, kind: EventKind) {
        *self.counters.entry(kind).or_default() += 1;
    }

    pub fn get(&self, kind: EventKind) -> u64 {
        self.counters.get(&kind).copied().unwrap_or_default()
    }

    pub fn total(&self) -> u64 {
        self.counters.values().sum()
    }

    pub fn snapshot(&self) -> Vec<(String, u64)> {
        self.counters
            .iter()
            .map(|(kind, count)| (kind.as_str().to_string(), *count))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bus_delivers_events_to_subscribers() {
        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();

        bus.publish(KayaEvent::NetworkStarted {
            multicast_addr: "239.71.0.1:42424".into(),
        })
        .expect("event published");

        let received = rx.recv().await.expect("event received");
        assert_eq!(received.kind(), EventKind::NetworkStarted);
    }

    #[test]
    fn counters_track_event_kinds() {
        let mut counters = EventCounters::default();

        counters.increment(EventKind::PacketReceived);
        counters.increment(EventKind::PacketReceived);
        counters.increment(EventKind::ErrorOccurred);

        assert_eq!(counters.get(EventKind::PacketReceived), 2);
        assert_eq!(counters.total(), 3);
    }
}
