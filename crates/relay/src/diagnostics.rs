use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
struct RelayDiagnosticsInner {
    accepted_connections: AtomicU64,
    disconnected_peers: AtomicU64,
    forwarded_packets: AtomicU64,
    broadcast_packets: AtomicU64,
    malformed_frames: AtomicU64,
    heartbeat_timeouts: AtomicU64,
}

#[derive(Debug, Clone, Default)]
pub struct RelayDiagnostics {
    inner: Arc<RelayDiagnosticsInner>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelayDiagnosticsSnapshot {
    pub accepted_connections: u64,
    pub disconnected_peers: u64,
    pub forwarded_packets: u64,
    pub broadcast_packets: u64,
    pub malformed_frames: u64,
    pub heartbeat_timeouts: u64,
}

impl RelayDiagnostics {
    pub fn inc_accepted(&self) {
        self.inner
            .accepted_connections
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_disconnected(&self) {
        self.inner
            .disconnected_peers
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_forwarded(&self) {
        self.inner.forwarded_packets.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_broadcast(&self) {
        self.inner.broadcast_packets.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_malformed(&self) {
        self.inner.malformed_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_heartbeat_timeouts(&self) {
        self.inner
            .heartbeat_timeouts
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> RelayDiagnosticsSnapshot {
        RelayDiagnosticsSnapshot {
            accepted_connections: self.inner.accepted_connections.load(Ordering::Relaxed),
            disconnected_peers: self.inner.disconnected_peers.load(Ordering::Relaxed),
            forwarded_packets: self.inner.forwarded_packets.load(Ordering::Relaxed),
            broadcast_packets: self.inner.broadcast_packets.load(Ordering::Relaxed),
            malformed_frames: self.inner.malformed_frames.load(Ordering::Relaxed),
            heartbeat_timeouts: self.inner.heartbeat_timeouts.load(Ordering::Relaxed),
        }
    }
}
