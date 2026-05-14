use kaya_protocol::Packet;
use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

use crate::errors::{MeshError, MeshResult};
use crate::policy::MeshPolicy;
use crate::ttl::{decrement_ttl, MESH_VERSION};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshEnvelope {
    pub mesh_version: u16,
    pub mesh_packet_id: String,
    pub source_node: String,
    pub destination_node: String,
    pub previous_hop: String,
    pub next_hop: Option<String>,
    pub ttl: u8,
    pub hop_count: u8,
    pub route_trace: Vec<String>,
    pub created_at: String,
    pub inner_packet: Box<Packet>,
}

impl MeshEnvelope {
    pub fn new(
        source_node: &str,
        destination_node: &str,
        previous_hop: &str,
        next_hop: Option<String>,
        ttl: u8,
        inner_packet: Packet,
    ) -> Self {
        Self {
            mesh_version: MESH_VERSION,
            mesh_packet_id: Uuid::new_v4().to_string(),
            source_node: source_node.to_string(),
            destination_node: destination_node.to_string(),
            previous_hop: previous_hop.to_string(),
            next_hop,
            ttl,
            hop_count: 0,
            route_trace: vec![source_node.to_string()],
            created_at: now_millis().to_string(),
            inner_packet: Box::new(inner_packet),
        }
    }

    pub fn relay(&self, relay_node: &str, next_hop: Option<String>) -> MeshResult<Self> {
        let ttl = decrement_ttl(self.ttl).ok_or(MeshError::TtlExpired)?;
        let mut relay = self.clone();
        relay.previous_hop = relay_node.to_string();
        relay.next_hop = next_hop;
        relay.ttl = ttl;
        relay.hop_count = relay.hop_count.saturating_add(1);
        relay.route_trace.push(relay_node.to_string());
        Ok(relay)
    }

    pub fn should_be_seen_by(&self, node_id: &str) -> bool {
        self.next_hop.as_deref() == Some(node_id) || self.destination_node == node_id
    }

    pub fn decode(value: serde_json::Value) -> MeshResult<Self> {
        serde_json::from_value(value).map_err(|err| MeshError::Decode(err.to_string()))
    }

    pub fn to_value(&self) -> MeshResult<serde_json::Value> {
        serde_json::to_value(self).map_err(|err| MeshError::Decode(err.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayDropReason {
    Duplicate,
    TtlExpired,
    BlockedPeer,
    PolicyDenied,
    LoopDetected,
    NoRoute,
    NotNextHop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayDecision {
    Deliver,
    Relay { next_hop: String },
    Drop(RelayDropReason),
}

pub fn decide_relay(
    envelope: &MeshEnvelope,
    own_node_id: &str,
    policy: &MeshPolicy,
    blocked: bool,
    known_route_next_hop: Option<&str>,
) -> RelayDecision {
    if !policy.enabled {
        return RelayDecision::Drop(RelayDropReason::PolicyDenied);
    }
    if envelope.destination_node == own_node_id {
        return RelayDecision::Deliver;
    }
    if envelope.source_node == own_node_id
        || envelope.route_trace.iter().any(|hop| hop == own_node_id)
    {
        return RelayDecision::Drop(RelayDropReason::LoopDetected);
    }
    if envelope.next_hop.as_deref() != Some(own_node_id) {
        return RelayDecision::Drop(RelayDropReason::NotNextHop);
    }
    if envelope.ttl <= 1 {
        return RelayDecision::Drop(RelayDropReason::TtlExpired);
    }
    if blocked && !policy.allow_relay_for_blocked {
        return RelayDecision::Drop(RelayDropReason::BlockedPeer);
    }
    match known_route_next_hop {
        Some(next_hop) => RelayDecision::Relay {
            next_hop: next_hop.to_string(),
        },
        None => RelayDecision::Drop(RelayDropReason::NoRoute),
    }
}

#[derive(Debug, Clone)]
pub struct SeenMeshPackets {
    max: usize,
    order: VecDeque<String>,
    seen: HashSet<String>,
}

impl SeenMeshPackets {
    pub fn new(max: usize) -> Self {
        Self {
            max: max.max(1),
            order: VecDeque::new(),
            seen: HashSet::new(),
        }
    }

    pub fn observe(&mut self, packet_id: &str) -> bool {
        if self.seen.contains(packet_id) {
            return false;
        }
        self.seen.insert(packet_id.to_string());
        self.order.push_back(packet_id.to_string());
        while self.order.len() > self.max {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        true
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.seen.clear();
    }
}
