use crate::errors::{RelayError, RelayResult};
use kaya_protocol::{Packet, RelayPeerDescriptor};
use kaya_shared::now_millis;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayPeerInfo {
    pub node_id: String,
    pub callsign: String,
    pub fingerprint: String,
    pub capabilities: Vec<String>,
    pub connected_at_ms: u64,
    pub last_seen_ms: u64,
}

struct RegistryEntry {
    info: RelayPeerInfo,
    tx: mpsc::UnboundedSender<Packet>,
}

#[derive(Default)]
pub struct RelayRegistry {
    peers: HashMap<String, RegistryEntry>,
}

impl RelayRegistry {
    pub fn register(
        &mut self,
        node_id: String,
        callsign: String,
        fingerprint: String,
        capabilities: Vec<String>,
        tx: mpsc::UnboundedSender<Packet>,
    ) -> RelayResult<RelayPeerInfo> {
        let now = now_millis();
        let info = RelayPeerInfo {
            node_id: node_id.clone(),
            callsign,
            fingerprint,
            capabilities,
            connected_at_ms: now,
            last_seen_ms: now,
        };
        self.peers.insert(
            node_id,
            RegistryEntry {
                info: info.clone(),
                tx,
            },
        );
        Ok(info)
    }

    pub fn unregister(&mut self, node_id: &str) -> Option<RelayPeerInfo> {
        self.peers.remove(node_id).map(|entry| entry.info)
    }

    pub fn update_seen(&mut self, node_id: &str) {
        if let Some(entry) = self.peers.get_mut(node_id) {
            entry.info.last_seen_ms = now_millis();
        }
    }

    pub fn peer_list(&self) -> Vec<RelayPeerDescriptor> {
        let mut peers: Vec<_> = self
            .peers
            .values()
            .map(|entry| RelayPeerDescriptor {
                node_id: entry.info.node_id.clone(),
                callsign: entry.info.callsign.clone(),
                fingerprint: entry.info.fingerprint.clone(),
                capabilities: entry.info.capabilities.clone(),
            })
            .collect();
        peers.sort_by(|left, right| left.callsign.cmp(&right.callsign));
        peers
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn contains(&self, node_id: &str) -> bool {
        self.peers.contains_key(node_id)
    }

    pub fn send_to(&self, node_id: &str, packet: Packet) -> RelayResult<()> {
        let Some(entry) = self.peers.get(node_id) else {
            return Err(RelayError::Registration(format!(
                "peer not connected: {node_id}"
            )));
        };
        entry
            .tx
            .send(packet)
            .map_err(|err| RelayError::ChannelClosed(err.to_string()))
    }

    pub fn broadcast_except(&self, excluded_node_id: &str, packet: &Packet) -> RelayResult<usize> {
        let mut sent = 0;
        for (node_id, entry) in &self.peers {
            if node_id == excluded_node_id {
                continue;
            }
            entry
                .tx
                .send(packet.clone())
                .map_err(|err| RelayError::ChannelClosed(err.to_string()))?;
            sent += 1;
        }
        Ok(sent)
    }

    pub fn cleanup_stale(&mut self, cutoff_ms: u64) -> Vec<RelayPeerInfo> {
        let stale: Vec<_> = self
            .peers
            .values()
            .filter(|entry| entry.info.last_seen_ms < cutoff_ms)
            .map(|entry| entry.info.node_id.clone())
            .collect();
        stale
            .into_iter()
            .filter_map(|node_id| self.unregister(&node_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_up_stale_peers() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut registry = RelayRegistry::default();
        let info = registry
            .register(
                "KY-71AF92".into(),
                "Ana".into(),
                "KAYA-FP: 11-22-33".into(),
                vec!["rooms".into()],
                tx,
            )
            .unwrap();
        let removed = registry.cleanup_stale(info.connected_at_ms + 1);

        assert_eq!(removed.len(), 1);
        assert!(!registry.contains("KY-71AF92"));
    }
}
