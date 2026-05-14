use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshDiagnostics {
    pub enabled: bool,
    pub routes: u64,
    pub routes_discovered: u64,
    pub routes_expired: u64,
    pub relayed_packets: u64,
    pub delivered_packets: u64,
    pub dropped_packets: u64,
    pub ttl_drops: u64,
    pub duplicate_drops: u64,
    pub policy_drops: u64,
    pub loop_drops: u64,
    pub no_route_drops: u64,
    pub total_hops: u64,
    pub last_route_discovered: Option<String>,
    pub current_route_trace: Vec<String>,
}

impl MeshDiagnostics {
    pub fn avg_hop_count(&self) -> u64 {
        self.total_hops
            .checked_div(self.relayed_packets)
            .unwrap_or_default()
    }
}
