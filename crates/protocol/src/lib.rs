use kaya_shared::{
    is_valid_node_id, normalize_room, now_millis, KayaError, Result, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

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

    pub fn validate(&self) -> Result<()> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(KayaError::InvalidPacket(format!(
                "unsupported protocol version {}",
                self.protocol_version
            )));
        }

        if self.packet_id.is_nil() {
            return Err(KayaError::InvalidPacket("packet_id cannot be nil".into()));
        }

        if !is_valid_node_id(&self.node_id) {
            return Err(KayaError::InvalidPacket(format!(
                "invalid node_id {}",
                self.node_id
            )));
        }

        if self.callsign.trim().is_empty() {
            return Err(KayaError::InvalidPacket("callsign cannot be empty".into()));
        }

        if self.timestamp.trim().is_empty() || self.timestamp.parse::<u64>().is_err() {
            return Err(KayaError::InvalidPacket(
                "timestamp must be unix milliseconds".into(),
            ));
        }

        match self.packet_type {
            PacketType::Hello
            | PacketType::Heartbeat
            | PacketType::Leave
            | PacketType::JoinRoom
            | PacketType::RoomMessage => {
                if self.room.as_deref().unwrap_or_default().trim().is_empty() {
                    return Err(KayaError::InvalidPacket(format!(
                        "{:?} requires room",
                        self.packet_type
                    )));
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
                    return Err(KayaError::InvalidPacket(format!(
                        "{:?} requires target_node",
                        self.packet_type
                    )));
                }
            }
            PacketType::System | PacketType::Error => {}
        }

        if matches!(
            self.packet_type,
            PacketType::RoomMessage | PacketType::DirectMessage
        ) && self.body().unwrap_or_default().trim().is_empty()
        {
            return Err(KayaError::InvalidPacket(
                "message body cannot be empty".into(),
            ));
        }

        Ok(())
    }
}

pub fn encode(packet: &Packet) -> Result<Vec<u8>> {
    packet.validate()?;
    serde_json::to_vec(packet).map_err(Into::into)
}

pub fn decode(bytes: &[u8]) -> Result<Packet> {
    let packet: Packet = serde_json::from_slice(bytes)?;
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
        assert!(decode(&serde_json::to_vec(&packet).unwrap()).is_err());
    }

    #[test]
    fn packet_type_uses_expected_json_names() {
        let packet = Packet::join_room("KY-71AF92", "Helio", "semana-info");
        let value: Value = serde_json::from_slice(&encode(&packet).unwrap()).unwrap();
        assert_eq!(value["type"], "JOIN_ROOM");
    }
}
