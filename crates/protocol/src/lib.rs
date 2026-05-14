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
    RelayRegister,
    RelayRegistered,
    RelayPeerList,
    RelayForward,
    RelayDelivered,
    RelayError,
    RelayHeartbeat,
    RelayDisconnect,
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
    FileOffer,
    FileAccept,
    FileReject,
    FileChunk,
    FileChunkEncrypted,
    FileChunkAck,
    FileTransferComplete,
    FileTransferCancel,
    FileTransferError,
    RouteAnnounce,
    RouteRequest,
    RouteResponse,
    MeshRelay,
    RouteError,
    RoutePing,
    RoutePong,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOfferPayload {
    pub file_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: Option<String>,
    pub sha256: String,
    pub chunk_size: usize,
    pub total_chunks: u32,
    pub sender_node_id: String,
    pub sender_callsign: String,
    pub created_at: String,
    pub dangerous_extension: bool,
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChunkPayload {
    pub file_id: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub chunk_hash: String,
    pub payload: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEncryptedChunkPayload {
    pub file_id: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub chunk_hash: String,
    pub session_id: String,
    pub nonce: String,
    pub ciphertext: String,
    pub sender_fingerprint: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDescriptorPayload {
    pub destination_node: String,
    pub destination_callsign: Option<String>,
    pub hop_count: u8,
    pub score: i64,
    pub trusted: bool,
    pub encrypted_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAnnouncePayload {
    pub routes: Vec<RouteDescriptorPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRequestPayload {
    pub request_id: String,
    pub destination_node: String,
    pub ttl: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteResponsePayload {
    pub request_id: String,
    pub destination_node: String,
    pub destination_callsign: Option<String>,
    pub next_hop: String,
    pub hop_count: u8,
    pub score: i64,
    pub route_trace: Vec<String>,
    pub trusted: bool,
    pub encrypted_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayRegisterPayload {
    pub fingerprint: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayRegisteredPayload {
    pub relay_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayPeerDescriptor {
    pub node_id: String,
    pub callsign: String,
    pub fingerprint: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayPeerListPayload {
    pub peers: Vec<RelayPeerDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayForwardPayload {
    pub destination_node: String,
    pub room: Option<String>,
    pub inner_packet: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayDeliveredPayload {
    pub destination_node: String,
    pub relay_packet_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayHeartbeatPayload {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayDisconnectPayload {
    pub reason: String,
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

    pub fn relay_register(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        fingerprint: impl Into<String>,
        capabilities: Vec<String>,
    ) -> Self {
        Self::new(
            PacketType::RelayRegister,
            node_id,
            callsign,
            None,
            None,
            json!(RelayRegisterPayload {
                fingerprint: fingerprint.into(),
                capabilities,
            }),
        )
    }

    pub fn relay_registered(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        relay_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RelayRegistered,
            node_id,
            callsign,
            None,
            None,
            json!(RelayRegisteredPayload {
                relay_id: relay_id.into(),
                message: message.into(),
            }),
        )
    }

    pub fn relay_peer_list(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        peers: Vec<RelayPeerDescriptor>,
    ) -> Self {
        Self::new(
            PacketType::RelayPeerList,
            node_id,
            callsign,
            None,
            None,
            json!(RelayPeerListPayload { peers }),
        )
    }

    pub fn relay_forward(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        destination_node: impl Into<String>,
        room: Option<String>,
        inner_packet: Value,
    ) -> Self {
        let destination_node = destination_node.into();
        Self::new(
            PacketType::RelayForward,
            node_id,
            callsign,
            room.clone(),
            Some(destination_node.clone()),
            json!(RelayForwardPayload {
                destination_node,
                room,
                inner_packet,
            }),
        )
    }

    pub fn relay_delivered(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        destination_node: impl Into<String>,
        relay_packet_id: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RelayDelivered,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!(RelayDeliveredPayload {
                destination_node: destination_node.into(),
                relay_packet_id: relay_packet_id.into(),
            }),
        )
    }

    pub fn relay_error(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: Option<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RelayError,
            node_id,
            callsign,
            None,
            target_node,
            json!(RelayErrorPayload {
                code: code.into(),
                message: message.into(),
            }),
        )
    }

    pub fn relay_heartbeat(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RelayHeartbeat,
            node_id,
            callsign,
            None,
            None,
            json!(RelayHeartbeatPayload {
                status: status.into(),
            }),
        )
    }

    pub fn relay_disconnect(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RelayDisconnect,
            node_id,
            callsign,
            None,
            None,
            json!(RelayDisconnectPayload {
                reason: reason.into(),
            }),
        )
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

    pub fn file_offer(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        payload: FileOfferPayload,
    ) -> Self {
        Self::new(
            PacketType::FileOffer,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!(payload),
        )
    }

    pub fn file_accept(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Self {
        Self::file_control(
            PacketType::FileAccept,
            node_id,
            callsign,
            target_node,
            file_id,
            None,
        )
    }

    pub fn file_reject(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::file_control(
            PacketType::FileReject,
            node_id,
            callsign,
            target_node,
            file_id,
            Some(reason.into()),
        )
    }

    pub fn file_chunk(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        payload: FileChunkPayload,
    ) -> Self {
        Self::new(
            PacketType::FileChunk,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!(payload),
        )
    }

    pub fn file_chunk_encrypted(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        payload: FileEncryptedChunkPayload,
    ) -> Self {
        Self::new(
            PacketType::FileChunkEncrypted,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!(payload),
        )
    }

    pub fn file_chunk_ack(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
        chunk_index: u32,
    ) -> Self {
        Self::new(
            PacketType::FileChunkAck,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({ "file_id": file_id.into(), "chunk_index": chunk_index }),
        )
    }

    pub fn file_transfer_complete(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Self {
        Self::file_control(
            PacketType::FileTransferComplete,
            node_id,
            callsign,
            target_node,
            file_id,
            None,
        )
    }

    pub fn file_transfer_cancel(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::file_control(
            PacketType::FileTransferCancel,
            node_id,
            callsign,
            target_node,
            file_id,
            Some(reason.into()),
        )
    }

    pub fn file_transfer_error(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::file_control(
            PacketType::FileTransferError,
            node_id,
            callsign,
            target_node,
            file_id,
            Some(reason.into()),
        )
    }

    pub fn route_announce(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        routes: Vec<RouteDescriptorPayload>,
    ) -> Self {
        Self::new(
            PacketType::RouteAnnounce,
            node_id,
            callsign,
            None,
            None,
            json!(RouteAnnouncePayload { routes }),
        )
    }

    pub fn route_request(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        request_id: impl Into<String>,
        destination_node: impl Into<String>,
        ttl: u8,
    ) -> Self {
        Self::new(
            PacketType::RouteRequest,
            node_id,
            callsign,
            None,
            None,
            json!(RouteRequestPayload {
                request_id: request_id.into(),
                destination_node: destination_node.into(),
                ttl,
            }),
        )
    }

    pub fn route_response(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        response: RouteResponsePayload,
    ) -> Self {
        Self::new(
            PacketType::RouteResponse,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!(response),
        )
    }

    pub fn mesh_relay(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        next_hop: impl Into<String>,
        envelope: Value,
    ) -> Self {
        Self::new(
            PacketType::MeshRelay,
            node_id,
            callsign,
            None,
            Some(next_hop.into()),
            envelope,
        )
    }

    pub fn route_error(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        destination_node: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RouteError,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({
                "destination_node": destination_node.into(),
                "reason": reason.into()
            }),
        )
    }

    pub fn route_ping(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
    ) -> Self {
        Self::new(
            PacketType::RoutePing,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({}),
        )
    }

    pub fn route_pong(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        packet_id: Uuid,
    ) -> Self {
        Self::new(
            PacketType::RoutePong,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            json!({ "reply_to": packet_id }),
        )
    }

    fn file_control(
        packet_type: PacketType,
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        target_node: impl Into<String>,
        file_id: impl Into<String>,
        reason: Option<String>,
    ) -> Self {
        let mut payload = json!({ "file_id": file_id.into() });
        if let Some(reason) = reason {
            payload["reason"] = Value::String(reason);
        }
        Self::new(
            packet_type,
            node_id,
            callsign,
            None,
            Some(target_node.into()),
            payload,
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
            | PacketType::DirectMessageEncrypted
            | PacketType::RelayForward
            | PacketType::RelayDelivered
            | PacketType::RelayError
            | PacketType::FileOffer
            | PacketType::FileAccept
            | PacketType::FileReject
            | PacketType::FileChunk
            | PacketType::FileChunkEncrypted
            | PacketType::FileChunkAck
            | PacketType::FileTransferComplete
            | PacketType::FileTransferCancel
            | PacketType::FileTransferError
            | PacketType::RouteResponse
            | PacketType::MeshRelay
            | PacketType::RouteError
            | PacketType::RoutePing
            | PacketType::RoutePong => {
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
            PacketType::RouteAnnounce
            | PacketType::RouteRequest
            | PacketType::RelayRegister
            | PacketType::RelayRegistered
            | PacketType::RelayPeerList
            | PacketType::RelayHeartbeat
            | PacketType::RelayDisconnect
            | PacketType::System
            | PacketType::Error => {}
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
            PacketType::RelayRegister => {
                require_payload_str(&self.payload, self.packet_type, "fingerprint")?;
                require_payload_array(&self.payload, self.packet_type, "capabilities")?;
            }
            PacketType::RelayRegistered => {
                require_payload_str(&self.payload, self.packet_type, "relay_id")?;
                require_payload_str(&self.payload, self.packet_type, "message")?;
            }
            PacketType::RelayPeerList => {
                require_payload_array(&self.payload, self.packet_type, "peers")?;
            }
            PacketType::RelayForward => {
                require_payload_str(&self.payload, self.packet_type, "destination_node")?;
                require_payload_object(&self.payload, self.packet_type, "inner_packet")?;
            }
            PacketType::RelayDelivered => {
                require_payload_str(&self.payload, self.packet_type, "destination_node")?;
                require_payload_str(&self.payload, self.packet_type, "relay_packet_id")?;
            }
            PacketType::RelayError => {
                require_payload_str(&self.payload, self.packet_type, "code")?;
                require_payload_str(&self.payload, self.packet_type, "message")?;
            }
            PacketType::RelayHeartbeat => {
                require_payload_str(&self.payload, self.packet_type, "status")?;
            }
            PacketType::RelayDisconnect => {
                require_payload_str(&self.payload, self.packet_type, "reason")?;
            }
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
            PacketType::FileOffer => {
                for field in [
                    "file_id",
                    "file_name",
                    "sha256",
                    "sender_node_id",
                    "sender_callsign",
                    "created_at",
                ] {
                    if !payload_has_str(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
                for field in ["file_size", "chunk_size", "total_chunks"] {
                    if !payload_has_number(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            PacketType::FileChunk => {
                for field in ["file_id", "chunk_hash", "payload", "timestamp"] {
                    if !payload_has_str(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
                for field in ["chunk_index", "total_chunks"] {
                    if !payload_has_number(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            PacketType::FileChunkEncrypted => {
                for field in [
                    "file_id",
                    "chunk_hash",
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
                for field in ["chunk_index", "total_chunks"] {
                    if !payload_has_number(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            PacketType::FileAccept
            | PacketType::FileReject
            | PacketType::FileChunkAck
            | PacketType::FileTransferComplete
            | PacketType::FileTransferCancel
            | PacketType::FileTransferError => {
                require_payload_str(&self.payload, self.packet_type, "file_id")?;
            }
            PacketType::RouteAnnounce => {
                require_payload_array(&self.payload, self.packet_type, "routes")?;
            }
            PacketType::RouteRequest => {
                require_payload_str(&self.payload, self.packet_type, "request_id")?;
                require_payload_str(&self.payload, self.packet_type, "destination_node")?;
                if !payload_has_number(&self.payload, "ttl") {
                    return Err(ProtocolError::MissingField {
                        packet_type: self.packet_type,
                        field: "ttl",
                    });
                }
            }
            PacketType::RouteResponse => {
                for field in ["request_id", "destination_node", "next_hop"] {
                    require_payload_str(&self.payload, self.packet_type, field)?;
                }
                require_payload_array(&self.payload, self.packet_type, "route_trace")?;
                for field in ["hop_count", "score"] {
                    if self.payload.get(field).and_then(Value::as_i64).is_none() {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            PacketType::MeshRelay => {
                for field in [
                    "mesh_packet_id",
                    "source_node",
                    "destination_node",
                    "previous_hop",
                    "route_trace",
                    "inner_packet",
                ] {
                    require_payload_str_or_object(&self.payload, self.packet_type, field)?;
                }
                for field in ["ttl", "hop_count"] {
                    if !payload_has_number(&self.payload, field) {
                        return Err(ProtocolError::MissingField {
                            packet_type: self.packet_type,
                            field,
                        });
                    }
                }
            }
            PacketType::RouteError => {
                require_payload_str(&self.payload, self.packet_type, "destination_node")?;
                require_payload_str(&self.payload, self.packet_type, "reason")?;
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

fn require_payload_str(
    payload: &Value,
    packet_type: PacketType,
    field: &'static str,
) -> ProtocolResult<()> {
    if payload_has_str(payload, field) {
        Ok(())
    } else {
        Err(ProtocolError::MissingField { packet_type, field })
    }
}

fn payload_has_number(payload: &Value, field: &'static str) -> bool {
    payload.get(field).and_then(Value::as_u64).is_some()
}

fn require_payload_array(
    payload: &Value,
    packet_type: PacketType,
    field: &'static str,
) -> ProtocolResult<()> {
    if payload.get(field).and_then(Value::as_array).is_some() {
        Ok(())
    } else {
        Err(ProtocolError::MissingField { packet_type, field })
    }
}

fn require_payload_str_or_object(
    payload: &Value,
    packet_type: PacketType,
    field: &'static str,
) -> ProtocolResult<()> {
    let Some(value) = payload.get(field) else {
        return Err(ProtocolError::MissingField { packet_type, field });
    };
    if value.is_string() || value.is_object() || value.is_array() || value.is_number() {
        Ok(())
    } else {
        Err(ProtocolError::MissingField { packet_type, field })
    }
}

fn require_payload_object(
    payload: &Value,
    packet_type: PacketType,
    field: &'static str,
) -> ProtocolResult<()> {
    if payload.get(field).map(Value::is_object).unwrap_or(false) {
        Ok(())
    } else {
        Err(ProtocolError::MissingField { packet_type, field })
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
            Packet::file_offer(
                "KY-71AF92",
                "Helio",
                "KY-AAAAAA",
                FileOfferPayload {
                    file_id: "KF-ABCDEF123456".into(),
                    file_name: "report.pdf".into(),
                    file_size: 42,
                    mime_type: Some("application/pdf".into()),
                    sha256: "a".repeat(64),
                    chunk_size: 65536,
                    total_chunks: 1,
                    sender_node_id: "KY-71AF92".into(),
                    sender_callsign: "Helio".into(),
                    created_at: "123".into(),
                    dangerous_extension: false,
                    encrypted: false,
                },
            ),
            Packet::file_chunk(
                "KY-71AF92",
                "Helio",
                "KY-AAAAAA",
                FileChunkPayload {
                    file_id: "KF-ABCDEF123456".into(),
                    chunk_index: 0,
                    total_chunks: 1,
                    chunk_hash: "a".repeat(64),
                    payload: "abcd".into(),
                    timestamp: "123".into(),
                },
            ),
            Packet::file_chunk_encrypted(
                "KY-71AF92",
                "Helio",
                "KY-AAAAAA",
                FileEncryptedChunkPayload {
                    file_id: "KF-ABCDEF123456".into(),
                    chunk_index: 0,
                    total_chunks: 1,
                    chunk_hash: "a".repeat(64),
                    session_id: "session-1".into(),
                    nonce: "nonce".into(),
                    ciphertext: "ciphertext".into(),
                    sender_fingerprint: "KAYA-FP: 8A19-FC90-B2D1".into(),
                    timestamp: "123".into(),
                },
            ),
            Packet::file_accept("KY-AAAAAA", "Ana", "KY-71AF92", "KF-ABCDEF123456"),
            Packet::file_chunk_ack("KY-AAAAAA", "Ana", "KY-71AF92", "KF-ABCDEF123456", 0),
            Packet::file_transfer_complete("KY-AAAAAA", "Ana", "KY-71AF92", "KF-ABCDEF123456"),
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

    #[test]
    fn phase_five_mesh_packets_validate() {
        let route = RouteDescriptorPayload {
            destination_node: "KY-A91C0D".into(),
            destination_callsign: Some("Bruno".into()),
            hop_count: 2,
            score: 9000,
            trusted: false,
            encrypted_capable: true,
        };
        Packet::route_announce("KY-71AF92", "Helio", vec![route])
            .validate()
            .unwrap();
        Packet::route_request("KY-71AF92", "Helio", "request-1", "KY-A91C0D", 5)
            .validate()
            .unwrap();
        Packet::route_response(
            "KY-AAAAAA",
            "Ana",
            "KY-71AF92",
            RouteResponsePayload {
                request_id: "request-1".into(),
                destination_node: "KY-A91C0D".into(),
                destination_callsign: Some("Bruno".into()),
                next_hop: "KY-AAAAAA".into(),
                hop_count: 2,
                score: 9000,
                route_trace: vec!["KY-AAAAAA".into(), "KY-A91C0D".into()],
                trusted: true,
                encrypted_capable: true,
            },
        )
        .validate()
        .unwrap();

        let inner = Packet::direct_message("KY-71AF92", "Helio", "KY-A91C0D", "teste via mesh");
        Packet::mesh_relay(
            "KY-AAAAAA",
            "Ana",
            "KY-A91C0D",
            json!({
                "mesh_version": 1,
                "mesh_packet_id": "mesh-1",
                "source_node": "KY-71AF92",
                "destination_node": "KY-A91C0D",
                "previous_hop": "KY-AAAAAA",
                "next_hop": "KY-A91C0D",
                "ttl": 4,
                "hop_count": 1,
                "route_trace": ["KY-71AF92", "KY-AAAAAA"],
                "created_at": "123",
                "inner_packet": inner,
            }),
        )
        .validate()
        .unwrap();
    }
}
