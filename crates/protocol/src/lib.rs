use kaya_shared::{
    is_valid_node_id, normalize_room, now_millis, validate_room_name, PresenceStatus,
    MAX_PACKET_BYTES, PROTOCOL_VERSION,
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
    RoomAnnounce,
    RoomJoin,
    RoomLeave,
    RoomMembersRequest,
    RoomMembersResponse,
    RoomMessage,
    DirectMessage,
    DmAck,
    DmSessionRequest,
    DmSessionAccept,
    DirectMessageEncrypted,
    PresenceUpdate,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedDirectMessagePayload {
    pub session_id: String,
    pub nonce: String,
    pub ciphertext: String,
    pub sender_fingerprint: String,
    pub timestamp: String,
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
            public_key: None,
            signature: None,
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
        presence: PresenceStatus,
    ) -> Self {
        Self::new(
            PacketType::Heartbeat,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({ "presence": presence.as_str() }),
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
        Self::room_join(node_id, callsign, room)
    }

    pub fn room_announce(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RoomAnnounce,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({}),
        )
    }

    pub fn room_join(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RoomJoin,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({}),
        )
    }

    pub fn room_leave(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RoomLeave,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({}),
        )
    }

    pub fn room_members_request(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RoomMembersRequest,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({}),
        )
    }

    pub fn room_members_response(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
        members: Vec<String>,
    ) -> Self {
        Self::new(
            PacketType::RoomMembersResponse,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({ "members": members }),
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

    pub fn dm_ack(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        packet_id: Uuid,
    ) -> Self {
        Self::new(
            PacketType::DmAck,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({ "ack": packet_id }),
        )
    }

    pub fn dm_session_request(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        session_id: impl Into<String>,
        x25519_public_key: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::DmSessionRequest,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({
                "session_id": session_id.into(),
                "x25519_public_key": x25519_public_key.into(),
                "fingerprint": fingerprint.into()
            }),
        )
    }

    pub fn dm_session_accept(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        session_id: impl Into<String>,
        x25519_public_key: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::DmSessionAccept,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({
                "session_id": session_id.into(),
                "x25519_public_key": x25519_public_key.into(),
                "fingerprint": fingerprint.into()
            }),
        )
    }

    pub fn direct_message_encrypted(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        payload: EncryptedDirectMessagePayload,
    ) -> Self {
        Self::new(
            PacketType::DirectMessageEncrypted,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!(payload),
        )
    }

    pub fn presence_update(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
        presence: PresenceStatus,
    ) -> Self {
        Self::new(
            PacketType::PresenceUpdate,
            node_id,
            callsign,
            Some(room.into()),
            None,
            json!({ "presence": presence.as_str() }),
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

    pub fn presence(&self) -> Option<PresenceStatus> {
        self.payload
            .get("presence")
            .and_then(Value::as_str)
            .and_then(PresenceStatus::parse)
    }

    pub fn members(&self) -> Vec<String> {
        self.payload
            .get("members")
            .and_then(Value::as_array)
            .map(|members| {
                members
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default()
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
            | PacketType::RoomAnnounce
            | PacketType::RoomJoin
            | PacketType::RoomLeave
            | PacketType::RoomMembersRequest
            | PacketType::RoomMembersResponse
            | PacketType::PresenceUpdate
            | PacketType::RoomMessage => {
                let room = self.room.as_deref().unwrap_or_default();
                if room.trim().is_empty() {
                    return Err(ProtocolError::MissingField {
                        packet_type: self.packet_type,
                        field: "room",
                    });
                }
                validate_room_name(room).map_err(|err| ProtocolError::Decode(err.to_string()))?;
            }
            PacketType::DirectMessage | PacketType::DmAck | PacketType::Ping | PacketType::Pong => {
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
            PacketType::DmSessionRequest
            | PacketType::DmSessionAccept
            | PacketType::DirectMessageEncrypted => {
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

        if self.packet_type == PacketType::PresenceUpdate && self.presence().is_none() {
            return Err(ProtocolError::MissingField {
                packet_type: self.packet_type,
                field: "payload.presence",
            });
        }

        match self.packet_type {
            PacketType::DmSessionRequest | PacketType::DmSessionAccept => {
                for field in ["session_id", "x25519_public_key", "fingerprint"] {
                    if !payload_has_str(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            PacketType::DirectMessageEncrypted => {
                for field in [
                    "session_id",
                    "nonce",
                    "ciphertext",
                    "sender_fingerprint",
                    "timestamp",
                ] {
                    if !payload_has_str(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

fn payload_has_str(payload: &Value, field: &'static str) -> bool {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
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
        assert_eq!(value["type"], "ROOM_JOIN");
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

    #[test]
    fn phase_two_packets_validate() {
        let packets = [
            Packet::room_announce("KY-71AF92", "Helio", "semana-info"),
            Packet::room_join("KY-71AF92", "Helio", "semana-info"),
            Packet::room_leave("KY-71AF92", "Helio", "semana-info"),
            Packet::room_members_request("KY-71AF92", "Helio", "semana-info"),
            Packet::room_members_response(
                "KY-71AF92",
                "Helio",
                "semana-info",
                vec!["KY-71AF92".into()],
            ),
            Packet::presence_update("KY-71AF92", "Helio", "semana-info", PresenceStatus::Busy),
            Packet::dm_session_request(
                "KY-71AF92",
                "Helio",
                "KY-AAAAAA",
                "session-1",
                "xkey",
                "KAYA-FP: 8A19-FC90-B2D1",
            ),
            Packet::dm_session_accept(
                "KY-AAAAAA",
                "Ana",
                "KY-71AF92",
                "session-1",
                "xkey",
                "KAYA-FP: 8A19-FC90-B2D1",
            ),
            Packet::direct_message_encrypted(
                "KY-71AF92",
                "Helio",
                "KY-AAAAAA",
                EncryptedDirectMessagePayload {
                    session_id: "session-1".into(),
                    nonce: "nonce".into(),
                    ciphertext: "ciphertext".into(),
                    sender_fingerprint: "KAYA-FP: 8A19-FC90-B2D1".into(),
                    timestamp: "123".into(),
                },
            ),
        ];

        for packet in packets {
            packet.validate().expect("phase two packet validates");
        }
    }

    #[test]
    fn presence_update_requires_valid_presence() {
        let mut packet =
            Packet::presence_update("KY-71AF92", "Helio", "geral", PresenceStatus::Online);
        packet.payload = json!({ "presence": "sleeping" });

        assert!(matches!(
            packet.validate(),
            Err(ProtocolError::MissingField {
                field: "payload.presence",
                ..
            })
        ));
    }

    #[test]
    fn signed_packet_envelope_roundtrips() {
        let mut packet = Packet::hello("KY-71AF92", "Helio", "geral");
        packet.public_key = Some("public".into());
        packet.signature = Some("signature".into());

        let value: Value = serde_json::from_slice(&encode(&packet).unwrap()).unwrap();
        assert_eq!(value["public_key"], "public");
        assert_eq!(value["signature"], "signature");
    }
}
