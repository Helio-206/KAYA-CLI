use super::Runtime;
use kaya_commands::{Command, ParsedInput};
use kaya_events::KayaEvent;
use kaya_protocol::Packet;
use kaya_shared::Result;

impl Runtime {
    pub(super) async fn handle_input(&mut self, input: String) -> Result<bool> {
        match self.commands.parse(&input) {
            Ok(ParsedInput::Empty) => Ok(false),
            Ok(ParsedInput::Message(body)) => {
                self.send_room_message(body).await;
                Ok(false)
            }
            Ok(ParsedInput::Command(command)) => self.handle_command(command).await,
            Err(err) => {
                self.publish(KayaEvent::ErrorOccurred {
                    scope: "commands".into(),
                    message: err.to_string(),
                });
                Ok(false)
            }
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Help => self.system_message(self.commands.help_text()),
            Command::Who => self.show_who(),
            Command::Rooms => {
                self.system_message(format!("rooms: {}", self.rooms.room_names().join(", ")))
            }
            Command::Join { room } | Command::Room { room: Some(room) } => {
                self.join_room(&room).await;
            }
            Command::Room { room: None } => {
                self.system_message(format!("current room: #{}", self.rooms.current_room()));
            }
            Command::Msg { target, body } => self.send_direct_message(target, body).await,
            Command::Status => self.show_status(),
            Command::Logs => {
                self.ui_state.show_logs = !self.ui_state.show_logs;
                self.ui_state
                    .push_log(format!("logs visible: {}", self.ui_state.show_logs));
            }
            Command::Clear => {
                self.ui_state.clear_messages();
                self.ui_state.push_log("traffic panel cleared");
            }
            Command::Exit => return Ok(true),
        }

        Ok(false)
    }

    async fn join_room(&mut self, room: &str) {
        self.rooms.join(room);
        self.config.last_room = Some(self.rooms.current_room().to_string());
        self.publish(KayaEvent::RoomJoined {
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
            room: self.rooms.current_room().to_string(),
            local: true,
        });
        self.send_packet(Packet::join_room(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
    }

    async fn send_room_message(&mut self, body: String) {
        let room = self.rooms.current_room().to_string();
        self.rooms.add_local_room_message(body.clone());
        self.publish(KayaEvent::RoomMessageReceived {
            room: room.clone(),
            from_node: self.node_id.clone(),
            from_callsign: self.callsign.clone(),
            body: body.clone(),
            local: true,
        });
        self.send_packet(Packet::room_message(
            self.node_id.clone(),
            self.callsign.clone(),
            room,
            body,
        ))
        .await;
    }

    async fn send_direct_message(&mut self, target: String, body: String) {
        let target_node = self
            .peers
            .resolve_target(&target)
            .map(|peer| peer.node_id.clone())
            .unwrap_or_else(|| target.clone());

        self.rooms
            .add_local_direct_message(target_node.clone(), body.clone());
        self.publish(KayaEvent::DirectMessageReceived {
            from_node: self.node_id.clone(),
            from_callsign: self.callsign.clone(),
            target_node: target.clone(),
            body: body.clone(),
            local: true,
        });
        self.send_packet(Packet::direct_message(
            self.node_id.clone(),
            self.callsign.clone(),
            target_node,
            body,
        ))
        .await;
    }

    fn show_who(&mut self) {
        let peers = self.peers.snapshots();
        if peers.is_empty() {
            self.system_message("no peers discovered");
            return;
        }

        let summary = peers
            .into_iter()
            .map(|peer| {
                let status = if peer.online { "online" } else { "offline" };
                format!("{} {} {}", peer.callsign, peer.node_id, status)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(summary);
    }

    fn show_status(&mut self) {
        self.system_message(format!(
            "node={} room=#{} peers={} packets_tx={} packets_rx={} events={}",
            self.node_id,
            self.rooms.current_room(),
            self.peers.online_count(),
            self.ui_state.packets_tx,
            self.ui_state.packets_rx,
            self.diagnostics.counters.total()
        ));
    }
}
