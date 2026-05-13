use kaya_shared::{
    is_valid_node_id, normalize_room, now_millis, MAX_PACKET_BYTES, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

const MAX_FUTURE_SKEW_MS: u64 = 5 * 60 * 1000;

pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("unsupported protocol version {actual}; expected {expected}")]
    UnsupportedVersion { actual: u16, expected: u16 },
    #[error("packet_id cannot be nil")]
    NilPacketId,
    #[error("invalid node_id {0}")]
    InvalidNodeId(String),
    #[error("callsign cannot be empty")]
    EmptyCallsign,
    #[error("timestamp must be a non-zero unix millisecond value")]
    InvalidTimestamp,
    #[error("timestamp is too far in the future")]
    FutureTimestamp,
    #[error("{packet_type:?} requires {field}")]
    MissingField {
        packet_type: PacketType,
        field: &'static str,
    },
    #[error("payload must be a JSON object")]
    InvalidPayload,
    #[error("message body cannot be empty")]
    EmptyMessageBody,
    #[error("packet exceeds {max} bytes: {actual}")]
    PacketTooLarge { max: usize, actual: usize },
    #[error("packet is too small to be valid JSON")]
    PacketTooSmall,
    #[error("packet decode failed: {0}")]
    Decode(String),
    #[error("packet encode failed: {0}")]
    Encode(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PacketType {
    Hello,
    Heartbeat,
    Leave,
    JoinRoom,
    RoomMessage,
    DirectMessage,
    Ping,
    Pong,
    System,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Packet {
    pub protocol_version: u16,
    pub packet_id: Uuid,
    #[serde(rename = "type")]
    pub packet_type: PacketType,
    pub node_id: String,
    pub callsign: String,
    pub timestamp: String,
    pub room: Option<String>,
    pub target_node: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

impl Packet {
    pub fn new(
        packet_type: PacketType,
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: Option<String>,
        target_node: Option<String>,
        payload: Value,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            packet_id: Uuid::new_v4(),
            packet_type,
            node_id: node_id.into(),
            callsign: callsign.into(),
            timestamp: now_millis().to_string(),
            room: room.map(|room| normalize_room(&room)),
            target_node,
            payload,
        }
    }

    pub fn hello(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::Hello,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({ "capabilities": ["rooms", "dm", "presence"], "agent": "kaya-cli" }),
        )
    }

    pub fn heartbeat(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::Heartbeat,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({ "status": "online" }),
        )
    }

    pub fn leave(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::Leave,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({}),
        )
    }

    pub fn join_room(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::JoinRoom,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({}),
        )
    }

    pub fn room_message(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RoomMessage,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({ "body": body.into() }),
        )
    }

    pub fn direct_message(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::DirectMessage,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({ "body": body.into() }),
        )
    }

    pub fn ping(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::Ping,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({}),
        )
    }

    pub fn pong(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        packet_id: Uuid,
    ) -> Self {
        Self::new(
            PacketType::Pong,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({ "reply_to": packet_id }),
        )
    }

    pub fn system(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::System,
            node_id,
            callsign,
            None,
            None,
            json!({ "message": message.into() }),
        )
    }

    pub fn error(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::Error,
            node_id,
            callsign,
            None,
            None,
            json!({ "message": message.into() }),
        )
    }

    pub fn body(&self) -> Option<&str> {
        self.payload.get("body").and_then(Value::as_str)
    }

    pub fn message(&self) -> Option<&str> {
        self.payload.get("message").and_then(Value::as_str)
    }

    pub fn validate(&self) -> ProtocolResult<()> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                actual: self.protocol_version,
                expected: PROTOCOL_VERSION,
            });
        }

        if self.packet_id.is_nil() {
            return Err(ProtocolError::NilPacketId);
        }

        if !is_valid_node_id(&self.node_id) {
            return Err(ProtocolError::InvalidNodeId(self.node_id.clone()));
        }

        if self.callsign.trim().is_empty() {
            return Err(ProtocolError::EmptyCallsign);
        }

        let timestamp = self
            .timestamp
            .trim()
            .parse::<u64>()
            .map_err(|_| ProtocolError::InvalidTimestamp)?;
        if timestamp == 0 {
            return Err(ProtocolError::InvalidTimestamp);
        }
        if timestamp > now_millis().saturating_add(MAX_FUTURE_SKEW_MS) {
            return Err(ProtocolError::FutureTimestamp);
        }

        if !self.payload.is_object() {
            return Err(ProtocolError::InvalidPayload);
        }

        match self.packet_type {
            PacketType::Hello
            | PacketType::Heartbeat
            | PacketType::Leave
            | PacketType::JoinRoom
            | PacketType::RoomMessage => {
                if self.room.as_deref().unwrap_or_default().trim().is_empty() {
                    return Err(ProtocolError::MissingField {
                        packet_type: self.packet_type,
                        field: "room",
                    });
                }
            }
            PacketType::DirectMessage | PacketType::Ping | PacketType::Pong => {
                if self
                    .target_node
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    return Err(ProtocolError::MissingField {
                        packet_type: self.packet_type,
                        field: "target_node",
                    });
                }
            }
            PacketType::System | PacketType::Error => {}
        }

        if matches!(
            self.packet_type,
            PacketType::RoomMessage | PacketType::DirectMessage
        ) && self.body().unwrap_or_default().trim().is_empty()
        {
            return Err(ProtocolError::EmptyMessageBody);
        }

        Ok(())
    }
}

pub fn encode(packet: &Packet) -> ProtocolResult<Vec<u8>> {
    packet.validate()?;
    serde_json::to_vec(packet).map_err(|err| ProtocolError::Encode(err.to_string()))
}

pub fn decode(bytes: &[u8]) -> ProtocolResult<Packet> {
    decode_with_limit(bytes, MAX_PACKET_BYTES)
}

pub fn decode_with_limit(bytes: &[u8], max_bytes: usize) -> ProtocolResult<Packet> {
    if bytes.len() < kaya_shared::MIN_PACKET_BYTES {
        return Err(ProtocolError::PacketTooSmall);
    }
    if bytes.len() > max_bytes {
        return Err(ProtocolError::PacketTooLarge {
            max: max_bytes,
            actual: bytes.len(),
        });
    }

    let packet: Packet =
        serde_json::from_slice(bytes).map_err(|err| ProtocolError::Decode(err.to_string()))?;
    packet.validate()?;
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_message_serializes_with_protocol_shape() {
        let packet = Packet::room_message("KY-71AF92", "Ana", "geral", "alguem recebe?");
        let encoded = encode(&packet).expect("encoded packet");
        let decoded = decode(&encoded).expect("decoded packet");

        assert_eq!(decoded.protocol_version, 1);
        assert_eq!(decoded.packet_type, PacketType::RoomMessage);
        assert_eq!(decoded.room.as_deref(), Some("geral"));
        assert_eq!(decoded.body(), Some("alguem recebe?"));
    }

    #[test]
    fn direct_messages_require_target_and_body() {
        let mut packet = Packet::direct_message("KY-71AF92", "Bruno", "KY-AAAAAA", "teste privado");
        packet.validate().expect("valid dm");

        packet.target_node = None;
        assert!(packet.validate().is_err());
    }

    #[test]
    fn rejects_wrong_protocol_version() {
        let mut packet = Packet::hello("KY-71AF92", "Helio", "geral");
        packet.protocol_version = 99;
        let bytes = serde_json::to_vec(&packet).expect("packet json");
        assert!(matches!(
            decode(&bytes),
            Err(ProtocolError::UnsupportedVersion { actual: 99, .. })
        ));
    }

    #[test]
    fn packet_type_uses_expected_json_names() {
        let packet = Packet::join_room("KY-71AF92", "Helio", "semana-info");
        let value: Value = serde_json::from_slice(&encode(&packet).unwrap()).unwrap();
        assert_eq!(value["type"], "JOIN_ROOM");
    }

    #[test]
    fn rejects_unknown_packet_types() {
        let mut value = serde_json::to_value(Packet::hello("KY-71AF92", "Helio", "geral")).unwrap();
        value["type"] = Value::String("UNKNOWN".into());
        let bytes = serde_json::to_vec(&value).unwrap();

        assert!(matches!(decode(&bytes), Err(ProtocolError::Decode(_))));
    }

    #[test]
    fn rejects_non_object_payloads() {
        let mut packet = Packet::hello("KY-71AF92", "Helio", "geral");
        packet.payload = Value::String("bad".into());

        assert!(matches!(
            packet.validate(),
            Err(ProtocolError::InvalidPayload)
        ));
    }

    #[test]
    fn rejects_oversized_packets_before_json_decode() {
        let bytes = vec![b' '; 32];
        assert!(matches!(
            decode_with_limit(&bytes, 8),
            Err(ProtocolError::PacketTooLarge { max: 8, actual: 32 })
        ));
    }
}
