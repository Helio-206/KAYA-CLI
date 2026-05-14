use serde::{Deserialize, Serialize};

use crate::ttl::DEFAULT_MESH_TTL;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshPolicy {
    pub enabled: bool,
    pub max_ttl: u8,
    pub allow_relay_for_unknown: bool,
    pub allow_relay_for_blocked: bool,
    pub relay_encrypted_only: bool,
    pub route_expiry_seconds: u64,
    pub max_seen_packets: usize,
}

impl Default for MeshPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_ttl: DEFAULT_MESH_TTL,
            allow_relay_for_unknown: true,
            allow_relay_for_blocked: false,
            relay_encrypted_only: false,
            route_expiry_seconds: 120,
            max_seen_packets: 5000,
        }
    }
}

impl MeshPolicy {
    pub fn route_expiry_ms(&self) -> u64 {
        self.route_expiry_seconds.saturating_mul(1000)
    }
}
