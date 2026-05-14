use kaya_shared::now_millis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportType {
    Multicast,
    DirectTcp,
    Mesh,
    Relay,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Multicast => f.write_str("multicast"),
            TransportType::DirectTcp => f.write_str("direct_tcp"),
            TransportType::Mesh => f.write_str("mesh"),
            TransportType::Relay => f.write_str("relay"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Failed,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Connecting => f.write_str("connecting"),
            ConnectionState::Connected => f.write_str("connected"),
            ConnectionState::Disconnecting => f.write_str("disconnecting"),
            ConnectionState::Disconnected => f.write_str("disconnected"),
            ConnectionState::Failed => f.write_str("failed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectConnectionView {
    pub peer_node_id: String,
    pub peer_callsign: String,
    pub local_addr: String,
    pub remote_addr: String,
    pub connected_at_ms: u64,
    pub transport_type: TransportType,
    pub latency_ms: Option<u64>,
    pub encrypted_capable: bool,
    pub connection_state: ConnectionState,
}

impl DirectConnectionView {
    pub fn connected(
        peer_node_id: impl Into<String>,
        peer_callsign: impl Into<String>,
        local_addr: impl Into<String>,
        remote_addr: impl Into<String>,
        encrypted_capable: bool,
    ) -> Self {
        Self {
            peer_node_id: peer_node_id.into(),
            peer_callsign: peer_callsign.into(),
            local_addr: local_addr.into(),
            remote_addr: remote_addr.into(),
            connected_at_ms: now_millis(),
            transport_type: TransportType::DirectTcp,
            latency_ms: None,
            encrypted_capable,
            connection_state: ConnectionState::Connected,
        }
    }
}
