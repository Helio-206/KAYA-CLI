use kaya_protocol::{Packet, PacketType};
use kaya_shared::{normalize_room, DEFAULT_ROOM};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub room: Option<String>,
    pub from_node: String,
    pub from_callsign: String,
    pub target_node: Option<String>,
    pub body: String,
    pub direct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub members: HashSet<String>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteOutcome {
    Joined {
        node_id: String,
        callsign: String,
        room: String,
    },
    Left {
        node_id: String,
        room: Option<String>,
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
        store.join(DEFAULT_ROOM);
        store
    }

    pub fn join(&mut self, room: &str) {
        let room = normalize_room(room);
        self.rooms.entry(room.clone()).or_insert_with(|| Room {
            name: room.clone(),
            members: HashSet::new(),
            messages: Vec::new(),
        });
        self.current_room = room;
    }

    pub fn current_room(&self) -> &str {
        &self.current_room
    }

    pub fn room_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.rooms.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn add_local_room_message(&mut self, body: impl Into<String>) -> ChatMessage {
        let message = ChatMessage {
            room: Some(self.current_room.clone()),
            from_node: self.own_node_id.clone(),
            from_callsign: self.own_callsign.clone(),
            target_node: None,
            body: body.into(),
            direct: false,
        };
        self.room_mut(&self.current_room.clone())
            .messages
            .push(message.clone());
        message
    }

    pub fn add_local_direct_message(
        &mut self,
        target_node: impl Into<String>,
        body: impl Into<String>,
    ) -> ChatMessage {
        let message = ChatMessage {
            room: None,
            from_node: self.own_node_id.clone(),
            from_callsign: self.own_callsign.clone(),
            target_node: Some(target_node.into()),
            body: body.into(),
            direct: true,
        };
        self.direct_messages.push(message.clone());
        message
    }

    pub fn route_packet(&mut self, packet: &Packet) -> RouteOutcome {
        match packet.packet_type {
            PacketType::Hello | PacketType::Heartbeat | PacketType::JoinRoom => {
                let Some(room) = packet.room.clone() else {
                    return RouteOutcome::Ignored;
                };
                self.room_mut(&room).members.insert(packet.node_id.clone());
                RouteOutcome::Joined {
                    node_id: packet.node_id.clone(),
                    callsign: packet.callsign.clone(),
                    room,
                }
            }
            PacketType::Leave => {
                for room in self.rooms.values_mut() {
                    room.members.remove(&packet.node_id);
                }
                RouteOutcome::Left {
                    node_id: packet.node_id.clone(),
                    room: packet.room.clone(),
                }
            }
            PacketType::RoomMessage => {
                let Some(room) = packet.room.clone() else {
                    return RouteOutcome::Ignored;
                };
                let Some(body) = packet.body() else {
                    return RouteOutcome::Ignored;
                };
                let message = ChatMessage {
                    room: Some(room.clone()),
                    from_node: packet.node_id.clone(),
                    from_callsign: packet.callsign.clone(),
                    target_node: None,
                    body: body.to_string(),
                    direct: false,
                };
                let room_state = self.room_mut(&room);
                room_state.members.insert(packet.node_id.clone());
                room_state.messages.push(message.clone());
                RouteOutcome::RoomMessage(message)
            }
            PacketType::DirectMessage => {
                let target = packet.target_node.as_deref().unwrap_or_default();
                if target != self.own_node_id && !target.eq_ignore_ascii_case(&self.own_callsign) {
                    return RouteOutcome::Ignored;
                }
                let Some(body) = packet.body() else {
                    return RouteOutcome::Ignored;
                };
                let message = ChatMessage {
                    room: None,
                    from_node: packet.node_id.clone(),
                    from_callsign: packet.callsign.clone(),
                    target_node: Some(target.to_string()),
                    body: body.to_string(),
                    direct: true,
                };
                self.direct_messages.push(message.clone());
                RouteOutcome::DirectMessage(message)
            }
            PacketType::Ping | PacketType::Pong | PacketType::System | PacketType::Error => {
                RouteOutcome::Ignored
            }
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

    fn room_mut(&mut self, room: &str) -> &mut Room {
        let room = normalize_room(room);
        self.rooms.entry(room.clone()).or_insert_with(|| Room {
            name: room,
            members: HashSet::new(),
            messages: Vec::new(),
        })
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
    }

    #[test]
    fn routes_room_messages_to_room_history() {
        let mut store = RoomStore::new("KY-000001", "Helio");
        let packet = Packet::room_message("KY-71AF92", "Ana", "semana-info", "recebido");

        let outcome = store.route_packet(&packet);

        assert!(matches!(outcome, RouteOutcome::RoomMessage(_)));
        store.join("semana-info");
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
    fn ignores_direct_messages_for_other_nodes() {
        let mut store = RoomStore::new("KY-000001", "Helio");
        let packet = Packet::direct_message("KY-71AF92", "Bruno", "KY-222222", "segredo");

        assert_eq!(store.route_packet(&packet), RouteOutcome::Ignored);
        assert!(store.direct_messages().is_empty());
    }
}
