use super::Runtime;
use kaya_events::KayaEvent;
use kaya_protocol::{Packet, PacketType};
use kaya_rooms::{ChatMessage, RouteOutcome};
use tracing::{debug, error, info, info_span, Instrument};

impl Runtime {
    pub(super) async fn handle_event(&mut self, event: KayaEvent) {
        self.diagnostics.counters.increment(event.kind());

        match event {
            KayaEvent::PacketReceived {
                packet,
                source,
                bytes,
            } => self.handle_packet_received(packet, source, bytes).await,
            KayaEvent::PacketSent {
                packet_id,
                packet_type,
                bytes,
            } => {
                self.ui_state.packets_tx += 1;
                self.ui_state.bytes_tx += bytes as u64;
                self.ui_state
                    .push_log(format!("tx {packet_type:?} {packet_id} bytes={bytes}"));
            }
            KayaEvent::PeerDiscovered { node_id, callsign } => {
                self.ui_state
                    .push_log(format!("peer discovered {callsign} {node_id}"));
            }
            KayaEvent::PeerTimedOut { node_id } => {
                self.ui_state.push_log(format!("peer timeout {node_id}"));
            }
            KayaEvent::RoomJoined {
                node_id,
                callsign,
                room,
                local,
            } => self.apply_room_joined(node_id, callsign, room, local),
            KayaEvent::RoomCreated {
                node_id,
                callsign,
                room,
                local,
            } => {
                if local {
                    self.ui_state.push_log(format!("created room #{room}"));
                    self.system_message(format!("created #{room}"));
                } else {
                    self.ui_state
                        .push_log(format!("peer {callsign} {node_id} announced #{room}"));
                }
                self.sync_peers_to_ui();
            }
            KayaEvent::RoomLeft { node_id, room } => {
                let room = room
                    .map(|value| format!(" from #{value}"))
                    .unwrap_or_default();
                self.ui_state.push_log(format!("peer {node_id} left{room}"));
            }
            KayaEvent::RoomMessageReceived {
                room,
                from_node,
                from_callsign,
                body,
                local,
            } => {
                let message = ChatMessage {
                    timestamp: kaya_shared::now_millis().to_string(),
                    room: Some(room),
                    from_node,
                    from_callsign,
                    target_node: None,
                    body,
                    direct: false,
                };
                self.push_chat_message(&message, local);
                self.persist_chat_message(&message);
            }
            KayaEvent::DirectMessageReceived {
                from_node,
                from_callsign,
                target_node,
                body,
                local,
            } => {
                let message = ChatMessage {
                    timestamp: kaya_shared::now_millis().to_string(),
                    room: None,
                    from_node,
                    from_callsign,
                    target_node: Some(target_node),
                    body,
                    direct: true,
                };
                self.push_chat_message(&message, local);
                self.persist_chat_message(&message);
            }
            KayaEvent::DirectMessageSent {
                target_node,
                target_callsign,
                body,
            } => {
                let message = ChatMessage {
                    timestamp: kaya_shared::now_millis().to_string(),
                    room: None,
                    from_node: self.node_id.clone(),
                    from_callsign: self.callsign.clone(),
                    target_node: Some(target_node),
                    body,
                    direct: true,
                };
                self.push_chat_message(&message, true);
                self.persist_chat_message(&message);
                self.ui_state
                    .push_log(format!("dm sent to {target_callsign}"));
            }
            KayaEvent::PresenceUpdated {
                node_id,
                callsign,
                presence,
            } => {
                if node_id == self.node_id {
                    self.presence = presence;
                    self.ui_state.presence = presence;
                    self.system_message(format!("presence set to {presence}"));
                } else {
                    self.ui_state
                        .push_log(format!("presence {callsign} {node_id}: {presence}"));
                }
                self.sync_peers_to_ui();
            }
            KayaEvent::ErrorOccurred { scope, message } => {
                self.diagnostics.malformed_packets += u64::from(scope == "transport.rx");
                error!(%scope, %message, "runtime error");
                self.ui_state.push_log(format!("{scope}: {message}"));
                self.system_message(format!("{scope}: {message}"));
            }
            KayaEvent::NetworkStarted { multicast_addr } => {
                info!(%multicast_addr, "network started");
                self.ui_state.status = "CONNECTED".into();
                self.ui_state
                    .push_log(format!("network started {multicast_addr}"));
            }
            KayaEvent::ShutdownInitiated { reason } => {
                self.ui_state.status = "SHUTDOWN".into();
                self.ui_state
                    .push_log(format!("shutdown initiated: {reason}"));
            }
        }
    }

    async fn handle_packet_received(&mut self, packet: Packet, source: String, bytes: usize) {
        self.ui_state.packets_rx += 1;
        self.ui_state.bytes_rx += bytes as u64;

        let span = info_span!(
            "packet.received",
            packet_id = %packet.packet_id,
            packet_type = ?packet.packet_type,
            source = %source,
            node_id = %packet.node_id,
        );

        async {
            if !self.dedup.observe(packet.packet_id) {
                self.diagnostics.duplicate_packets += 1;
                debug!("duplicate packet dropped");
                self.ui_state
                    .push_log(format!("duplicate packet dropped {}", packet.packet_id));
                return;
            }

            if packet.node_id == self.node_id {
                debug!("loopback packet ignored");
                return;
            }

            self.observe_peer(&packet);
            self.remember_peer(&packet);
            self.sync_peers_to_ui();
            self.route_packet(&packet).await;

            for packet in self.state_sync_for(&packet) {
                self.send_packet(packet).await;
            }
        }
        .instrument(span)
        .await;
    }

    fn apply_room_joined(&mut self, node_id: String, callsign: String, room: String, local: bool) {
        if local {
            self.ui_state.current_room = room.clone();
            self.ui_state.space = room.clone();
            self.ui_state.push_log(format!("joined room #{room}"));
            self.system_message(format!("joined #{room}"));
        } else {
            self.ui_state
                .push_log(format!("peer {callsign} {node_id} present in #{room}"));
        }
        self.sync_peers_to_ui();
    }

    fn observe_peer(&mut self, packet: &Packet) {
        if let Some(event) = self.peers.observe_packet(packet) {
            self.publish_peer_event(event);
        }
    }

    async fn route_packet(&mut self, packet: &Packet) {
        match self.rooms.route_packet(packet) {
            RouteOutcome::RoomMessage(message) => self.publish_room_message(message),
            RouteOutcome::DirectMessage(message) => {
                self.publish(KayaEvent::DirectMessageReceived {
                    from_node: message.from_node,
                    from_callsign: message.from_callsign,
                    target_node: message.target_node.unwrap_or_else(|| self.node_id.clone()),
                    body: message.body,
                    local: false,
                });
                self.send_packet(Packet::dm_ack(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    packet.node_id.clone(),
                    packet.packet_id,
                ))
                .await;
            }
            RouteOutcome::Joined {
                node_id,
                callsign,
                room,
            } => {
                if matches!(
                    packet.packet_type,
                    PacketType::Hello | PacketType::JoinRoom | PacketType::RoomJoin
                ) {
                    self.publish(KayaEvent::RoomJoined {
                        node_id,
                        callsign,
                        room,
                        local: false,
                    });
                }
            }
            RouteOutcome::RoomCreated {
                node_id,
                callsign,
                room,
            } => {
                self.publish(KayaEvent::RoomCreated {
                    node_id,
                    callsign,
                    room,
                    local: false,
                });
                self.sync_peers_to_ui();
            }
            RouteOutcome::Left { node_id, room } => {
                self.publish(KayaEvent::RoomLeft { node_id, room });
            }
            RouteOutcome::MembersRequested { node_id, room } => {
                if self.rooms.is_joined(&room) {
                    self.send_packet(Packet::room_members_response(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        room.clone(),
                        self.rooms.members(&room),
                    ))
                    .await;
                } else {
                    self.ui_state
                        .push_log(format!("ignored members request from {node_id}"));
                }
            }
            RouteOutcome::MembersResponse { room, members } => {
                self.ui_state
                    .push_log(format!("synced {} members for #{room}", members.len()));
                self.sync_peers_to_ui();
            }
            RouteOutcome::Ignored => {}
        }
    }

    fn publish_room_message(&self, message: ChatMessage) {
        let Some(room) = message.room.clone() else {
            return;
        };
        if room == self.rooms.current_room() {
            self.publish(KayaEvent::RoomMessageReceived {
                room,
                from_node: message.from_node,
                from_callsign: message.from_callsign,
                body: message.body,
                local: false,
            });
        }
    }
}
