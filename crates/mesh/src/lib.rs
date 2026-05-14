pub mod diagnostics;
pub mod discovery;
pub mod errors;
pub mod policy;
pub mod relay;
pub mod route;
pub mod scoring;
pub mod table;
pub mod ttl;

pub use diagnostics::MeshDiagnostics;
pub use discovery::{RouteAnnouncement, RouteDescriptor, RouteRequest, RouteResponse};
pub use errors::{MeshError, MeshResult};
pub use policy::MeshPolicy;
pub use relay::{decide_relay, MeshEnvelope, RelayDecision, RelayDropReason, SeenMeshPackets};
pub use route::{RouteEntry, RouteEntrySpec, RouteSource};
pub use scoring::score_route;
pub use table::RoutingTable;
pub use ttl::{decrement_ttl, DEFAULT_MESH_TTL, MESH_VERSION};

use kaya_protocol::Packet;
use kaya_shared::now_millis;

#[derive(Debug, Clone)]
pub struct MeshState {
    pub own_node_id: String,
    pub policy: MeshPolicy,
    pub table: RoutingTable,
    pub seen: SeenMeshPackets,
    pub diagnostics: MeshDiagnostics,
    last_route_discovered: Option<String>,
    current_route_trace: Vec<String>,
}

impl MeshState {
    pub fn new(own_node_id: impl Into<String>, policy: MeshPolicy) -> Self {
        let own_node_id = own_node_id.into();
        Self {
            own_node_id,
            table: RoutingTable::new(policy.route_expiry_ms()),
            seen: SeenMeshPackets::new(policy.max_seen_packets),
            diagnostics: MeshDiagnostics::default(),
            policy,
            last_route_discovered: None,
            current_route_trace: Vec::new(),
        }
    }

    pub fn observe_direct_peer(
        &mut self,
        node_id: &str,
        callsign: &str,
        trusted: bool,
        encrypted_capable: bool,
        latency_ms: Option<u64>,
    ) {
        let entry = RouteEntry::from_spec(RouteEntrySpec {
            destination_node: node_id.to_string(),
            destination_callsign: Some(callsign.to_string()),
            next_hop: node_id.to_string(),
            hop_count: 1,
            trusted,
            encrypted_capable,
            source: RouteSource::Direct,
            latency_ms,
        });
        self.table.upsert(entry);
    }

    pub fn observe_route(&mut self, entry: RouteEntry) {
        let destination = entry.destination_node.clone();
        self.table.upsert(entry);
        self.last_route_discovered = Some(destination);
        self.diagnostics.routes_discovered += 1;
    }

    pub fn expire_routes(&mut self) -> Vec<RouteEntry> {
        let expired = self.table.expire(now_millis());
        self.diagnostics.routes_expired += expired.len() as u64;
        expired
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.seen.clear();
        self.current_route_trace.clear();
        self.last_route_discovered = None;
    }

    pub fn best_route(&self, target: &str) -> Option<&RouteEntry> {
        self.table.best_route(target)
    }

    pub fn build_envelope(
        &self,
        destination_node: &str,
        next_hop: &str,
        inner_packet: Packet,
    ) -> MeshEnvelope {
        MeshEnvelope::new(
            &self.own_node_id,
            destination_node,
            &self.own_node_id,
            Some(next_hop.to_string()),
            self.policy.max_ttl,
            inner_packet,
        )
    }

    pub fn accept_seen(&mut self, mesh_packet_id: &str) -> bool {
        self.seen.observe(mesh_packet_id)
    }

    pub fn mark_relayed(&mut self, envelope: &MeshEnvelope) {
        self.diagnostics.relayed_packets += 1;
        self.diagnostics.total_hops += envelope.hop_count as u64;
        self.current_route_trace = envelope.route_trace.clone();
    }

    pub fn mark_delivered(&mut self, envelope: &MeshEnvelope) {
        self.diagnostics.delivered_packets += 1;
        self.current_route_trace = envelope.route_trace.clone();
    }

    pub fn mark_dropped(&mut self, reason: RelayDropReason) {
        self.diagnostics.dropped_packets += 1;
        match reason {
            RelayDropReason::Duplicate => self.diagnostics.duplicate_drops += 1,
            RelayDropReason::TtlExpired => self.diagnostics.ttl_drops += 1,
            RelayDropReason::BlockedPeer | RelayDropReason::PolicyDenied => {
                self.diagnostics.policy_drops += 1;
            }
            RelayDropReason::LoopDetected => self.diagnostics.loop_drops += 1,
            RelayDropReason::NoRoute => self.diagnostics.no_route_drops += 1,
            RelayDropReason::NotNextHop => {}
        }
    }

    pub fn diagnostics_snapshot(&self) -> MeshDiagnostics {
        let mut diagnostics = self.diagnostics.clone();
        diagnostics.enabled = self.policy.enabled;
        diagnostics.routes = self.table.len() as u64;
        diagnostics.last_route_discovered = self.last_route_discovered.clone();
        diagnostics.current_route_trace = self.current_route_trace.clone();
        diagnostics
    }
}
