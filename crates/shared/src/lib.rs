use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_ROOM: &str = "geral";
pub const MULTICAST_IPV4: &str = "239.71.0.1";
pub const MULTICAST_PORT: u16 = 42424;
pub const MAX_PACKET_BYTES: usize = 64 * 1024;
pub const MIN_PACKET_BYTES: usize = 2;
pub const PEER_TIMEOUT_SECS: u64 = 12;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 3;
pub const EVENT_CHANNEL_CAPACITY: usize = 1024;
pub const MAX_ROOM_NAME_LEN: usize = 48;

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

pub fn monotonic_uptime_secs(started_at: SystemTime) -> u64 {
    SystemTime::now()
        .duration_since(started_at)
        .unwrap_or_default()
        .as_secs()
}

pub fn normalize_room(room: &str) -> String {
    room.trim().trim_start_matches('#').to_ascii_lowercase()
}

pub fn validate_room_name(room: &str) -> Result<String> {
    let room = normalize_room(room);
    if room.is_empty() {
        return Err(KayaError::InvalidRoomName(
            "room name cannot be empty".into(),
        ));
    }
    if room.len() > MAX_ROOM_NAME_LEN {
        return Err(KayaError::InvalidRoomName(format!(
            "room name cannot exceed {MAX_ROOM_NAME_LEN} characters"
        )));
    }
    if !room
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(KayaError::InvalidRoomName(format!(
            "room name contains invalid characters: {room}"
        )));
    }
    Ok(room)
}

pub fn normalize_callsign(callsign: &str) -> String {
    callsign.trim().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    Invisible,
    Offline,
}

impl PresenceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PresenceStatus::Online => "online",
            PresenceStatus::Away => "away",
            PresenceStatus::Busy => "busy",
            PresenceStatus::Invisible => "invisible",
            PresenceStatus::Offline => "offline",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "online" => Some(Self::Online),
            "away" => Some(Self::Away),
            "busy" => Some(Self::Busy),
            "invisible" => Some(Self::Invisible),
            "offline" => Some(Self::Offline),
            _ => None,
        }
    }
}

impl fmt::Display for PresenceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error)]
pub enum KayaError {
    #[error("invalid KAYA node id: {0}")]
    InvalidNodeId(String),
    #[error("invalid KAYA packet: {0}")]
    InvalidPacket(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("event channel closed: {0}")]
    ChannelClosed(String),
    #[error("invalid KAYA command: {0}")]
    InvalidCommand(String),
    #[error("invalid room name: {0}")]
    InvalidRoomName(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("storage error: {0}")]
    Storage(String),
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

    #[test]
    fn validates_room_names() {
        assert_eq!(validate_room_name("#Semana-Info").unwrap(), "semana-info");
        assert!(validate_room_name("bad room").is_err());
        assert!(validate_room_name("").is_err());
    }

    #[test]
    fn parses_presence_status() {
        assert_eq!(PresenceStatus::parse("busy"), Some(PresenceStatus::Busy));
        assert_eq!(PresenceStatus::Busy.to_string(), "busy");
    }
}
