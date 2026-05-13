use kaya_protocol::{Packet, PacketType};
use kaya_shared::PEER_TIMEOUT_SECS;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub node_id: String,
    pub callsign: String,
    pub rooms: HashSet<String>,
    pub last_seen: Instant,
    pub latency_ms: Option<u64>,
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerSnapshot {
    pub node_id: String,
    pub callsign: String,
    pub rooms: Vec<String>,
    pub latency_ms: Option<u64>,
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerEvent {
    Discovered(String),
    Updated(String),
    Left(String),
    TimedOut(String),
}

#[derive(Debug)]
pub struct PeerRegistry {
    own_node_id: String,
    timeout: Duration,
    peers: HashMap<String, Peer>,
}

impl PeerRegistry {
    pub fn new(own_node_id: impl Into<String>) -> Self {
        Self {
            own_node_id: own_node_id.into(),
            timeout: Duration::from_secs(PEER_TIMEOUT_SECS),
            peers: HashMap::new(),
        }
    }

    pub fn with_timeout(own_node_id: impl Into<String>, timeout: Duration) -> Self {
        Self {
            own_node_id: own_node_id.into(),
            timeout,
            peers: HashMap::new(),
        }
    }

    pub fn observe_packet(&mut self, packet: &Packet) -> Option<PeerEvent> {
        self.observe_packet_at(packet, Instant::now())
    }

    pub fn observe_packet_at(&mut self, packet: &Packet, now: Instant) -> Option<PeerEvent> {
        if packet.node_id == self.own_node_id {
            return None;
        }

        if packet.packet_type == PacketType::Leave {
            return self.mark_left(&packet.node_id);
        }

        let mut discovered = false;
        let peer = self.peers.entry(packet.node_id.clone()).or_insert_with(|| {
            discovered = true;
            Peer {
                node_id: packet.node_id.clone(),
                callsign: packet.callsign.clone(),
                rooms: HashSet::new(),
                last_seen: now,
                latency_ms: None,
                online: true,
            }
        });

        peer.callsign = packet.callsign.clone();
        peer.last_seen = now;
        peer.online = true;
        if let Some(room) = &packet.room {
            peer.rooms.insert(room.clone());
        }

        if discovered {
            Some(PeerEvent::Discovered(peer.node_id.clone()))
        } else {
            Some(PeerEvent::Updated(peer.node_id.clone()))
        }
    }

    pub fn mark_left(&mut self, node_id: &str) -> Option<PeerEvent> {
        let peer = self.peers.get_mut(node_id)?;
        peer.online = false;
        Some(PeerEvent::Left(node_id.to_string()))
    }

    pub fn prune(&mut self) -> Vec<PeerEvent> {
        self.prune_at(Instant::now())
    }

    pub fn prune_at(&mut self, now: Instant) -> Vec<PeerEvent> {
        let mut events = Vec::new();
        for peer in self.peers.values_mut() {
            if peer.online && now.duration_since(peer.last_seen) > self.timeout {
                peer.online = false;
                events.push(PeerEvent::TimedOut(peer.node_id.clone()));
            }
        }
        events
    }

    pub fn resolve_target(&self, target: &str) -> Option<&Peer> {
        self.peers.get(target).or_else(|| {
            self.peers
                .values()
                .find(|peer| peer.callsign.eq_ignore_ascii_case(target))
        })
    }

    pub fn snapshots(&self) -> Vec<PeerSnapshot> {
        let mut peers: Vec<_> = self
            .peers
            .values()
            .map(|peer| {
                let mut rooms: Vec<_> = peer.rooms.iter().cloned().collect();
                rooms.sort();
                PeerSnapshot {
                    node_id: peer.node_id.clone(),
                    callsign: peer.callsign.clone(),
                    rooms,
                    latency_ms: peer.latency_ms,
                    online: peer.online,
                }
            })
            .collect();
        peers.sort_by(|left, right| left.callsign.cmp(&right.callsign));
        peers
    }

    pub fn online_count(&self) -> usize {
        self.peers.values().filter(|peer| peer.online).count()
    }

    pub fn get(&self, node_id: &str) -> Option<&Peer> {
        self.peers.get(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_peer_from_packet() {
        let mut registry = PeerRegistry::new("KY-000001");
        let packet = Packet::hello("KY-71AF92", "Ana", "geral");

        let event = registry.observe_packet(&packet);

        assert_eq!(event, Some(PeerEvent::Discovered("KY-71AF92".into())));
        assert_eq!(registry.online_count(), 1);
        assert_eq!(registry.resolve_target("Ana").unwrap().node_id, "KY-71AF92");
    }

    #[test]
    fn ignores_own_packets() {
        let mut registry = PeerRegistry::new("KY-71AF92");
        let packet = Packet::hello("KY-71AF92", "Helio", "geral");

        assert_eq!(registry.observe_packet(&packet), None);
        assert_eq!(registry.online_count(), 0);
    }

    #[test]
    fn marks_peer_offline_after_timeout() {
        let start = Instant::now();
        let mut registry = PeerRegistry::with_timeout("KY-000001", Duration::from_secs(2));
        let packet = Packet::heartbeat("KY-71AF92", "Ana", "geral");

        registry.observe_packet_at(&packet, start);
        let events = registry.prune_at(start + Duration::from_secs(3));

        assert_eq!(events, vec![PeerEvent::TimedOut("KY-71AF92".into())]);
        assert!(!registry.get("KY-71AF92").unwrap().online);
    }
}
