use super::Runtime;
use kaya_commands::{Command, ParsedInput};
use kaya_events::KayaEvent;
use kaya_peer::TargetResolution;
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
                self.show_rooms();
            }
            Command::Create { room } => self.create_room(&room).await,
            Command::Join { room } => {
                self.join_room(&room).await;
            }
            Command::Leave { room } => self.leave_room(&room).await,
            Command::Current => {
                self.system_message(format!("current room: #{}", self.rooms.current_room()));
            }
            Command::RoomMessage { body } => self.send_room_message(body).await,
            Command::Msg { target, body } => self.send_direct_message(target, body).await,
            Command::Presence { status } => self.set_presence(status).await,
            Command::History { room } => self.show_history(room.as_deref()),
            Command::DmHistory { peer } => self.show_dm_history(&peer),
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

    async fn create_room(&mut self, room: &str) {
        match self.rooms.create(room) {
            Ok(room) => {
                self.publish(KayaEvent::RoomCreated {
                    node_id: self.node_id.clone(),
                    callsign: self.callsign.clone(),
                    room: room.clone(),
                    local: true,
                });
                self.send_packet(Packet::room_announce(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    room,
                ))
                .await;
            }
            Err(err) => self.system_message(format!("{err}")),
        }
    }

    async fn join_room(&mut self, room: &str) {
        let Ok(room) = self.rooms.join(room) else {
            self.system_message("invalid room name");
            return;
        };
        self.config.last_room = Some(self.rooms.current_room().to_string());
        self.publish(KayaEvent::RoomJoined {
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
            room: room.clone(),
            local: true,
        });
        self.send_packet(Packet::room_join(
            self.node_id.clone(),
            self.callsign.clone(),
            room.clone(),
        ))
        .await;
        self.send_packet(Packet::room_members_request(
            self.node_id.clone(),
            self.callsign.clone(),
            room,
        ))
        .await;
    }

    async fn leave_room(&mut self, room: &str) {
        match self.rooms.leave(room) {
            Ok(room) => {
                self.publish(KayaEvent::RoomLeft {
                    node_id: self.node_id.clone(),
                    room: Some(room.clone()),
                });
                self.send_packet(Packet::room_leave(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    room,
                ))
                .await;
                self.sync_peers_to_ui();
            }
            Err(err) => self.system_message(format!("{err}")),
        }
    }

    async fn send_room_message(&mut self, body: String) {
        if !self.rooms.is_joined(self.rooms.current_room()) {
            self.system_message(format!(
                "cannot send: not joined to #{}",
                self.rooms.current_room()
            ));
            return;
        }
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
        let target = match self.peers.resolve_target_checked(&target) {
            TargetResolution::Found(peer) => peer,
            TargetResolution::NotFound(target) => {
                self.system_message(format!("dm target not found: {target}"));
                return;
            }
            TargetResolution::DuplicateCallsign { callsign, matches } => {
                self.system_message(format!(
                    "callsign {callsign} is ambiguous: {}",
                    matches.join(", ")
                ));
                return;
            }
        };

        self.rooms
            .add_local_direct_message(target.node_id.clone(), body.clone());
        self.publish(KayaEvent::DirectMessageSent {
            target_node: target.node_id.clone(),
            target_callsign: target.callsign.clone(),
            body: body.clone(),
        });
        self.send_packet(Packet::direct_message(
            self.node_id.clone(),
            self.callsign.clone(),
            target.node_id,
            body,
        ))
        .await;
    }

    async fn set_presence(&mut self, status: kaya_shared::PresenceStatus) {
        self.presence = status;
        self.publish(KayaEvent::PresenceUpdated {
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
            presence: status,
        });
        self.send_packet(Packet::presence_update(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
            status,
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
                format!(
                    "{} {} {} {}",
                    peer.callsign, peer.node_id, peer.presence, status
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(summary);
    }

    fn show_rooms(&mut self) {
        let summary = self
            .rooms
            .summaries()
            .into_iter()
            .map(|room| {
                let marker = if room.name == self.rooms.current_room() {
                    "current"
                } else if room.local_joined {
                    "joined"
                } else {
                    "known"
                };
                format!("#{} {} peers {}", room.name, marker, room.member_count)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(if summary.is_empty() {
            "rooms: none".into()
        } else {
            format!("rooms: {summary}")
        });
    }

    fn show_history(&mut self, room: Option<&str>) {
        let room = room
            .map(ToString::to_string)
            .unwrap_or_else(|| self.rooms.current_room().to_string());
        match self.store.list_room_history(&room, 12) {
            Ok(records) if records.is_empty() => {
                self.system_message(format!("no local history for #{room}"));
            }
            Ok(records) => {
                for record in records {
                    self.system_message(format!(
                        "history #{} {}: {}",
                        record.room.unwrap_or_else(|| room.clone()),
                        record.from,
                        record.body
                    ));
                }
            }
            Err(err) => self.system_message(format!("{err}")),
        }
    }

    fn show_dm_history(&mut self, peer: &str) {
        match self.store.list_dm_history(peer, 12) {
            Ok(records) if records.is_empty() => {
                self.system_message(format!("no local dm history for {peer}"));
            }
            Ok(records) => {
                for record in records {
                    self.system_message(format!(
                        "dm-history {} -> {}: {}",
                        record.from,
                        record.target.unwrap_or_else(|| "me".into()),
                        record.body
                    ));
                }
            }
            Err(err) => self.system_message(format!("{err}")),
        }
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
