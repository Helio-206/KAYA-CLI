use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};

use crate::scoring::score_route;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteSource {
    Direct,
    Announce,
    Response,
    RelayTrace,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub destination_node: String,
    pub destination_callsign: Option<String>,
    pub next_hop: String,
    pub hop_count: u8,
    pub score: i64,
    pub last_seen: u64,
    pub expires_at: u64,
    pub trusted: bool,
    pub encrypted_capable: bool,
    pub source: RouteSource,
    pub latency_ms: Option<u64>,
    pub failure_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEntrySpec {
    pub destination_node: String,
    pub destination_callsign: Option<String>,
    pub next_hop: String,
    pub hop_count: u8,
    pub trusted: bool,
    pub encrypted_capable: bool,
    pub source: RouteSource,
    pub latency_ms: Option<u64>,
}

impl RouteEntry {
    pub fn from_spec(spec: RouteEntrySpec) -> Self {
        let now = now_millis();
        let mut entry = Self {
            destination_node: spec.destination_node,
            destination_callsign: spec.destination_callsign,
            next_hop: spec.next_hop,
            hop_count: spec.hop_count,
            score: 0,
            last_seen: now,
            expires_at: now + 120_000,
            trusted: spec.trusted,
            encrypted_capable: spec.encrypted_capable,
            source: spec.source,
            latency_ms: spec.latency_ms,
            failure_count: 0,
        };
        entry.recalculate_score(now);
        entry
    }

    pub fn with_expiry(mut self, expiry_ms: u64) -> Self {
        self.expires_at = self.last_seen.saturating_add(expiry_ms);
        self
    }

    pub fn recalculate_score(&mut self, now: u64) {
        self.score = score_route(
            self.hop_count,
            self.trusted,
            self.encrypted_capable,
            self.latency_ms,
            self.failure_count,
            now.saturating_sub(self.last_seen),
        );
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at <= now
    }
}
