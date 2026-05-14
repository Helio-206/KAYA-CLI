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
    PresenceUpdated {
        node_id: String,
        callsign: String,
        presence: kaya_shared::PresenceStatus,
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
            KayaEvent::RoomJoined { .. } => EventKind::RoomJoined,
            KayaEvent::RoomCreated { .. } => EventKind::RoomCreated,
            KayaEvent::RoomLeft { .. } => EventKind::RoomLeft,
            KayaEvent::RoomMessageReceived { .. } => EventKind::RoomMessageReceived,
            KayaEvent::DirectMessageSent { .. } => EventKind::DirectMessageSent,
            KayaEvent::DirectMessageReceived { .. } => EventKind::DirectMessageReceived,
            KayaEvent::PresenceUpdated { .. } => EventKind::PresenceUpdated,
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
    RoomCreated,
    RoomJoined,
    RoomLeft,
    RoomMessageReceived,
    DirectMessageSent,
    DirectMessageReceived,
    PresenceUpdated,
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
            EventKind::RoomCreated => "room.created",
            EventKind::RoomJoined => "room.joined",
            EventKind::RoomLeft => "room.left",
            EventKind::RoomMessageReceived => "room.message",
            EventKind::DirectMessageSent => "dm.sent",
            EventKind::DirectMessageReceived => "dm.message",
            EventKind::PresenceUpdated => "presence.updated",
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
