use kaya_protocol::{Packet, PacketType};
use kaya_shared::{now_millis, validate_room_name, DEFAULT_ROOM};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub timestamp: String,
    pub room: Option<String>,
    pub from_node: String,
    pub from_callsign: String,
    pub target_node: Option<String>,
    pub body: String,
    pub direct: bool,
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub members: HashSet<String>,
    pub messages: Vec<ChatMessage>,
    pub local_joined: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomSummary {
    pub name: String,
    pub member_count: usize,
    pub local_joined: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteOutcome {
    RoomCreated {
        node_id: String,
        callsign: String,
        room: String,
    },
    Joined {
        node_id: String,
        callsign: String,
        room: String,
    },
    Left {
        node_id: String,
        room: Option<String>,
    },
    MembersRequested {
        node_id: String,
        room: String,
    },
    MembersResponse {
        room: String,
        members: Vec<String>,
    },
    RoomMessage(ChatMessage),
    DirectMessage(ChatMessage),
    Ignored,
}

#[derive(Debug)]
pub struct RoomStore {
    own_node_id: String,
    own_callsign: String,
    current_room: String,
    rooms: HashMap<String, Room>,
    direct_messages: Vec<ChatMessage>,
}

impl RoomStore {
    pub fn new(own_node_id: impl Into<String>, own_callsign: impl Into<String>) -> Self {
        let mut store = Self {
            own_node_id: own_node_id.into(),
            own_callsign: own_callsign.into(),
            current_room: DEFAULT_ROOM.to_string(),
            rooms: HashMap::new(),
            direct_messages: Vec::new(),
        };
        let _ = store.create(DEFAULT_ROOM);
        let _ = store.join(DEFAULT_ROOM);
        store
    }

    pub fn create(&mut self, room: &str) -> kaya_shared::Result<String> {
        let room = validate_room_name(room)?;
        self.rooms.entry(room.clone()).or_insert_with(|| Room {
            name: room.clone(),
            members: HashSet::new(),
            messages: Vec::new(),
            local_joined: false,
        });
        Ok(room)
    }

    pub fn join(&mut self, room: &str) -> kaya_shared::Result<String> {
        let room = self.create(room)?;
        let own_node_id = self.own_node_id.clone();
        let room_state = self.room_mut(&room)?;
        room_state.local_joined = true;
        room_state.members.insert(own_node_id);
        self.current_room = room.clone();
        Ok(room)
    }

    pub fn leave(&mut self, room: &str) -> kaya_shared::Result<String> {
        let room = validate_room_name(room)?;
        if let Some(room_state) = self.rooms.get_mut(&room) {
            room_state.local_joined = false;
            room_state.members.remove(&self.own_node_id);
        }
        if self.current_room == room {
            if !self.is_joined(DEFAULT_ROOM) {
                let _ = self.join(DEFAULT_ROOM)?;
            }
            self.current_room = DEFAULT_ROOM.to_string();
        }
        Ok(room)
    }

    pub fn current_room(&self) -> &str {
        &self.current_room
    }

    pub fn is_joined(&self, room: &str) -> bool {
        let Ok(room) = validate_room_name(room) else {
            return false;
        };
        self.rooms
            .get(&room)
            .map(|room| room.local_joined)
            .unwrap_or(false)
    }

    pub fn room_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.rooms.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn summaries(&self) -> Vec<RoomSummary> {
        let mut summaries: Vec<_> = self
            .rooms
            .values()
            .map(|room| RoomSummary {
                name: room.name.clone(),
                member_count: room.members.len(),
                local_joined: room.local_joined,
            })
            .collect();
        summaries.sort_by(|left, right| left.name.cmp(&right.name));
        summaries
    }

    pub fn current_members(&self) -> Vec<String> {
        self.members(&self.current_room)
    }

    pub fn members(&self, room: &str) -> Vec<String> {
        let Ok(room) = validate_room_name(room) else {
            return Vec::new();
        };
        let mut members: Vec<_> = self
            .rooms
            .get(&room)
            .map(|room| room.members.iter().cloned().collect())
            .unwrap_or_default();
        members.sort();
        members
    }

    pub fn add_local_room_message(&mut self, body: impl Into<String>) -> ChatMessage {
        let message = ChatMessage {
            timestamp: now_millis().to_string(),
            room: Some(self.current_room.clone()),
            from_node: self.own_node_id.clone(),
            from_callsign: self.own_callsign.clone(),
            target_node: None,
            body: body.into(),
            direct: false,
            encrypted: false,
        };
        if let Ok(room) = self.room_mut(&self.current_room.clone()) {
            room.messages.push(message.clone());
        }
        message
    }

    pub fn add_local_direct_message(
        &mut self,
        target_node: impl Into<String>,
        body: impl Into<String>,
    ) -> ChatMessage {
        let message = ChatMessage {
            timestamp: now_millis().to_string(),
            room: None,
            from_node: self.own_node_id.clone(),
            from_callsign: self.own_callsign.clone(),
            target_node: Some(target_node.into()),
            body: body.into(),
            direct: true,
            encrypted: false,
        };
        self.direct_messages.push(message.clone());
        message
    }

    pub fn route_packet(&mut self, packet: &Packet) -> RouteOutcome {
        match packet.packet_type {
            PacketType::Hello
            | PacketType::Heartbeat
            | PacketType::JoinRoom
            | PacketType::RoomJoin => self.observe_join(packet),
            PacketType::RoomAnnounce => self.observe_room_announce(packet),
            PacketType::RoomLeave | PacketType::Leave => self.observe_leave(packet),
            PacketType::RoomMembersRequest => {
                let Some(room) = packet.room.clone() else {
                    return RouteOutcome::Ignored;
                };
                RouteOutcome::MembersRequested {
                    node_id: packet.node_id.clone(),
                    room,
                }
            }
            PacketType::RoomMembersResponse => {
                let Some(room) = packet.room.clone() else {
                    return RouteOutcome::Ignored;
                };
                let room_state = match self.room_mut(&room) {
                    Ok(room_state) => room_state,
                    Err(_) => return RouteOutcome::Ignored,
                };
                let members = packet.members();
                for member in &members {
                    room_state.members.insert(member.clone());
                }
                RouteOutcome::MembersResponse { room, members }
            }
            PacketType::RoomMessage => self.observe_room_message(packet),
            PacketType::DirectMessage => self.observe_direct_message(packet),
            PacketType::DmAck
            | PacketType::DmSessionRequest
            | PacketType::DmSessionAccept
            | PacketType::DirectMessageEncrypted
            | PacketType::FileOffer
            | PacketType::FileAccept
            | PacketType::FileReject
            | PacketType::FileChunk
            | PacketType::FileChunkEncrypted
            | PacketType::FileChunkAck
            | PacketType::FileTransferComplete
            | PacketType::FileTransferCancel
            | PacketType::FileTransferError
            | PacketType::PresenceUpdate
            | PacketType::Ping
            | PacketType::Pong
            | PacketType::System
            | PacketType::Error => RouteOutcome::Ignored,
        }
    }

    pub fn current_messages(&self) -> &[ChatMessage] {
        self.rooms
            .get(&self.current_room)
            .map(|room| room.messages.as_slice())
            .unwrap_or(&[])
    }

    pub fn direct_messages(&self) -> &[ChatMessage] {
        &self.direct_messages
    }

    fn observe_room_announce(&mut self, packet: &Packet) -> RouteOutcome {
        let Some(room) = packet.room.clone() else {
            return RouteOutcome::Ignored;
        };
        if self.create_remote_room(&room).is_err() {
            return RouteOutcome::Ignored;
        }
        RouteOutcome::RoomCreated {
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
            room,
        }
    }

    fn observe_join(&mut self, packet: &Packet) -> RouteOutcome {
        let Some(room) = packet.room.clone() else {
            return RouteOutcome::Ignored;
        };
        let room_state = match self.room_mut(&room) {
            Ok(room_state) => room_state,
            Err(_) => return RouteOutcome::Ignored,
        };
        room_state.members.insert(packet.node_id.clone());
        RouteOutcome::Joined {
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
            room,
        }
    }

    fn observe_leave(&mut self, packet: &Packet) -> RouteOutcome {
        if packet.packet_type == PacketType::Leave {
            for room in self.rooms.values_mut() {
                room.members.remove(&packet.node_id);
            }
        } else if let Some(room) = &packet.room {
            if let Some(room_state) = self.rooms.get_mut(room) {
                room_state.members.remove(&packet.node_id);
            }
        }
        RouteOutcome::Left {
            node_id: packet.node_id.clone(),
            room: packet.room.clone(),
        }
    }

    fn observe_room_message(&mut self, packet: &Packet) -> RouteOutcome {
        let Some(room) = packet.room.clone() else {
            return RouteOutcome::Ignored;
        };
        if !self.is_joined(&room) {
            return RouteOutcome::Ignored;
        }
        let Some(body) = packet.body() else {
            return RouteOutcome::Ignored;
        };
        let message = ChatMessage {
            timestamp: packet.timestamp.clone(),
            room: Some(room.clone()),
            from_node: packet.node_id.clone(),
            from_callsign: packet.callsign.clone(),
            target_node: None,
            body: body.to_string(),
            direct: false,
            encrypted: false,
        };
        let room_state = match self.room_mut(&room) {
            Ok(room_state) => room_state,
            Err(_) => return RouteOutcome::Ignored,
        };
        room_state.members.insert(packet.node_id.clone());
        room_state.messages.push(message.clone());
        RouteOutcome::RoomMessage(message)
    }

    fn observe_direct_message(&mut self, packet: &Packet) -> RouteOutcome {
        let target = packet.target_node.as_deref().unwrap_or_default();
        if target != self.own_node_id && !target.eq_ignore_ascii_case(&self.own_callsign) {
            return RouteOutcome::Ignored;
        }
        let Some(body) = packet.body() else {
            return RouteOutcome::Ignored;
        };
        let message = ChatMessage {
            timestamp: packet.timestamp.clone(),
            room: None,
            from_node: packet.node_id.clone(),
            from_callsign: packet.callsign.clone(),
            target_node: Some(target.to_string()),
            body: body.to_string(),
            direct: true,
            encrypted: false,
        };
        self.direct_messages.push(message.clone());
        RouteOutcome::DirectMessage(message)
    }

    fn create_remote_room(&mut self, room: &str) -> kaya_shared::Result<String> {
        let room = validate_room_name(room)?;
        self.rooms.entry(room.clone()).or_insert_with(|| Room {
            name: room.clone(),
            members: HashSet::new(),
            messages: Vec::new(),
            local_joined: false,
        });
        Ok(room)
    }

    fn room_mut(&mut self, room: &str) -> kaya_shared::Result<&mut Room> {
        let room = validate_room_name(room)?;
        self.rooms.entry(room.clone()).or_insert_with(|| Room {
            name: room.clone(),
            members: HashSet::new(),
            messages: Vec::new(),
            local_joined: false,
        });
        self.rooms
            .get_mut(&room)
            .ok_or(kaya_shared::KayaError::InvalidRoomName(room))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_default_room() {
        let store = RoomStore::new("KY-000001", "Helio");
        assert_eq!(store.current_room(), "geral");
        assert_eq!(store.room_names(), vec!["geral"]);
        assert!(store.is_joined("geral"));
    }

    #[test]
    fn creates_and_leaves_rooms() {
        let mut store = RoomStore::new("KY-000001", "Helio");
        assert_eq!(store.create("semana-info").unwrap(), "semana-info");
        assert_eq!(store.join("semana-info").unwrap(), "semana-info");
        assert_eq!(store.current_room(), "semana-info");

        store.leave("semana-info").unwrap();

        assert_eq!(store.current_room(), "geral");
        assert!(!store.is_joined("semana-info"));
    }

    #[test]
    fn routes_room_messages_only_for_joined_rooms() {
        let mut store = RoomStore::new("KY-000001", "Helio");
        let packet = Packet::room_message("KY-71AF92", "Ana", "semana-info", "recebido");

        assert_eq!(store.route_packet(&packet), RouteOutcome::Ignored);
        store.join("semana-info").unwrap();
        assert!(matches!(
            store.route_packet(&packet),
            RouteOutcome::RoomMessage(_)
        ));
        assert_eq!(store.current_messages()[0].body, "recebido");
    }

    #[test]
    fn accepts_direct_messages_for_own_node() {
        let mut store = RoomStore::new("KY-000001", "Helio");
        let packet = Packet::direct_message("KY-71AF92", "Bruno", "KY-000001", "teste privado");

        let outcome = store.route_packet(&packet);

        assert!(matches!(outcome, RouteOutcome::DirectMessage(_)));
        assert_eq!(store.direct_messages()[0].body, "teste privado");
    }

    #[test]
    fn applies_membership_response() {
        let mut store = RoomStore::new("KY-000001", "Helio");
        let packet = Packet::room_members_response(
            "KY-71AF92",
            "Ana",
            "geral",
            vec!["KY-71AF92".into(), "KY-BB0022".into()],
        );

        assert!(matches!(
            store.route_packet(&packet),
            RouteOutcome::MembersResponse { .. }
        ));
        assert_eq!(store.members("geral").len(), 3);
    }
}
