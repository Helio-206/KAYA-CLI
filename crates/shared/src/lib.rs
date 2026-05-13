use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_ROOM: &str = "geral";
pub const MULTICAST_IPV4: &str = "239.71.0.1";
pub const MULTICAST_PORT: u16 = 42424;
pub const MAX_PACKET_BYTES: usize = 64 * 1024;
pub const PEER_TIMEOUT_SECS: u64 = 12;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    pub fn generate() -> Self {
        let raw = Uuid::new_v4().simple().to_string();
        Self(format!("KY-{}", raw[..6].to_ascii_uppercase()))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if is_valid_node_id(&value) {
            Ok(Self(value))
        } else {
            Err(KayaError::InvalidNodeId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<NodeId> for String {
    fn from(value: NodeId) -> Self {
        value.0
    }
}

pub fn is_valid_node_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('K'))
        && matches!(chars.next(), Some('Y'))
        && matches!(chars.next(), Some('-'))
        && chars.clone().count() == 6
        && chars.all(|ch| ch.is_ascii_hexdigit() && ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn normalize_room(room: &str) -> String {
    room.trim().trim_start_matches('#').to_ascii_lowercase()
}

pub fn normalize_callsign(callsign: &str) -> String {
    callsign.trim().to_string()
}

#[derive(Debug)]
pub enum KayaError {
    InvalidNodeId(String),
    InvalidPacket(String),
    InvalidCommand(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Storage(String),
}

impl fmt::Display for KayaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KayaError::InvalidNodeId(value) => write!(f, "invalid KAYA node id: {value}"),
            KayaError::InvalidPacket(value) => write!(f, "invalid KAYA packet: {value}"),
            KayaError::InvalidCommand(value) => write!(f, "invalid KAYA command: {value}"),
            KayaError::Io(err) => write!(f, "io error: {err}"),
            KayaError::Json(err) => write!(f, "json error: {err}"),
            KayaError::Storage(err) => write!(f, "storage error: {err}"),
        }
    }
}

impl std::error::Error for KayaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KayaError::Io(err) => Some(err),
            KayaError::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for KayaError {
    fn from(value: std::io::Error) -> Self {
        KayaError::Io(value)
    }
}

impl From<serde_json::Error> for KayaError {
    fn from(value: serde_json::Error) -> Self {
        KayaError::Json(value)
    }
}

pub type Result<T> = std::result::Result<T, KayaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_match_kaya_format() {
        let id = NodeId::generate();
        assert!(is_valid_node_id(id.as_str()));
        assert_eq!(id.as_str().len(), 9);
        assert!(id.as_str().starts_with("KY-"));
    }

    #[test]
    fn rejects_malformed_node_ids() {
        assert!(!is_valid_node_id("KY-12345"));
        assert!(!is_valid_node_id("ky-123456"));
        assert!(!is_valid_node_id("KY-XYZ123"));
    }

    #[test]
    fn normalizes_room_names() {
        assert_eq!(normalize_room(" #Semana-Info "), "semana-info");
    }
}
