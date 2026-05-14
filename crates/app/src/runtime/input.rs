use super::Runtime;
use kaya_commands::{Command, ParsedInput};
use kaya_events::KayaEvent;
use kaya_peer::TargetResolution;
use kaya_protocol::{EncryptedDirectMessagePayload, Packet};
use kaya_security::{TrustStatus, FINGERPRINT_PREFIX};
use kaya_shared::Result;

struct SecurityTarget {
    node_id: String,
    callsign: String,
    fingerprint: String,
}

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
            Command::Who { fingerprints } => self.show_who(fingerprints),
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
            Command::SecureMsg { target, body } => {
                self.send_secure_direct_message(target, body).await
            }
            Command::SendFile { target, path } => self.send_file_offer(target, path).await,
            Command::AcceptFile { file_id } => self.accept_file(&file_id).await,
            Command::RejectFile { file_id } => self.reject_file(&file_id).await,
            Command::Files => self.show_files(),
            Command::CancelFile { file_id } => self.cancel_file(&file_id).await,
            Command::OpenFolder => self.show_files_folder(),
            Command::FileInfo { file_id } => self.show_file_info(&file_id),
            Command::Presence { status } => self.set_presence(status).await,
            Command::Identity => self.show_identity(),
            Command::Fingerprint => self.system_message(self.identity.fingerprint.clone()),
            Command::Trust { peer } => self.set_peer_trust_status(&peer, TrustStatus::Trusted),
            Command::Untrust { peer } => self.set_peer_trust_status(&peer, TrustStatus::Unknown),
            Command::Block { peer } => self.set_peer_trust_status(&peer, TrustStatus::Blocked),
            Command::TrustList => self.show_trust_list(),
            Command::Sessions => self.show_sessions(),
            Command::CloseSession { peer } => self.close_secure_session(&peer),
            Command::Routes => self.show_routes(),
            Command::Route { node_id } => self.show_route(&node_id),
            Command::MeshStatus => self.show_mesh_status(),
            Command::MeshClear => self.clear_mesh(),
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
                if let Some(peer) = self.resolve_mesh_target(&target) {
                    self.send_direct_message_to(peer.node_id, peer.callsign, body)
                        .await;
                    return;
                }
                if kaya_shared::is_valid_node_id(&target) {
                    self.send_route_request(&target).await;
                    self.system_message(format!(
                        "dm target not found locally: {target}; route request sent"
                    ));
                } else {
                    self.system_message(format!("dm target not found: {target}"));
                }
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

        if self.trust_store.is_blocked(&target.node_id) {
            self.system_message(format!("dm target is blocked: {}", target.node_id));
            return;
        }

        self.send_direct_message_to(target.node_id, target.callsign, body)
            .await;
    }

    async fn send_secure_direct_message(&mut self, target: String, body: String) {
        let target = match self.peers.resolve_target_checked(&target) {
            TargetResolution::Found(peer) => peer,
            TargetResolution::NotFound(target) => {
                if let Some(peer) = self.resolve_mesh_target(&target) {
                    self.send_secure_direct_message_to(peer.node_id, peer.callsign, body)
                        .await;
                    return;
                }
                if kaya_shared::is_valid_node_id(&target) {
                    self.send_route_request(&target).await;
                    self.system_message(format!(
                        "secure dm target not found locally: {target}; route request sent"
                    ));
                } else {
                    self.system_message(format!("secure dm target not found: {target}"));
                }
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

        if self.trust_store.is_blocked(&target.node_id) {
            self.system_message(format!("secure dm target is blocked: {}", target.node_id));
            return;
        }

        self.send_secure_direct_message_to(target.node_id, target.callsign, body)
            .await;
    }

    async fn send_direct_message_to(
        &mut self,
        target_node: String,
        target_callsign: String,
        body: String,
    ) {
        if self.trust_store.is_blocked(&target_node) {
            self.system_message(format!("dm target is blocked: {target_node}"));
            return;
        }

        if self.sessions.has_active(&target_node) {
            self.send_encrypted_message(target_node, target_callsign, body)
                .await;
            return;
        }

        self.rooms
            .add_local_direct_message(target_node.clone(), body.clone());
        self.publish(KayaEvent::DirectMessageSent {
            target_node: target_node.clone(),
            target_callsign: target_callsign.clone(),
            body: body.clone(),
        });
        self.send_packet_routed(
            Packet::direct_message(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.clone(),
                body,
            ),
            &target_node,
        )
        .await;
    }

    async fn send_secure_direct_message_to(
        &mut self,
        target_node: String,
        target_callsign: String,
        body: String,
    ) {
        if self.trust_store.is_blocked(&target_node) {
            self.system_message(format!("secure dm target is blocked: {target_node}"));
            return;
        }

        if self.sessions.has_active(&target_node) {
            self.send_encrypted_message(target_node, target_callsign, body)
                .await;
            return;
        }

        let request = self.sessions.start_request(&target_node);
        self.pending_secure_messages
            .entry(target_node.clone())
            .or_default()
            .push(body);
        self.send_packet_routed(
            Packet::dm_session_request(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.clone(),
                request.session_id,
                request.x25519_public_key,
                request.fingerprint,
            ),
            &target_node,
        )
        .await;
        self.system_message(format!(
            "secure session requested with {}; message queued",
            target_callsign
        ));
        self.sync_security_to_ui();
    }

    pub(super) async fn flush_pending_secure_messages(&mut self, peer_node_id: &str) {
        let Some(messages) = self.pending_secure_messages.remove(peer_node_id) else {
            return;
        };
        let callsign = self
            .peers
            .get(peer_node_id)
            .map(|peer| peer.callsign.clone())
            .unwrap_or_else(|| peer_node_id.to_string());
        for body in messages {
            self.send_encrypted_message(peer_node_id.to_string(), callsign.clone(), body)
                .await;
        }
    }

    async fn send_encrypted_message(
        &mut self,
        target_node: String,
        target_callsign: String,
        body: String,
    ) {
        match self.sessions.encrypt(&target_node, &body) {
            Ok(payload) => {
                self.publish(KayaEvent::EncryptedMessageReceived {
                    from_node: self.node_id.clone(),
                    from_callsign: self.callsign.clone(),
                    target_node: target_node.clone(),
                    body: body.clone(),
                    local: true,
                });
                self.send_packet_routed(
                    Packet::direct_message_encrypted(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        target_node.clone(),
                        EncryptedDirectMessagePayload {
                            session_id: payload.session_id,
                            nonce: payload.nonce,
                            ciphertext: payload.ciphertext,
                            sender_fingerprint: payload.sender_fingerprint,
                            timestamp: payload.timestamp,
                        },
                    ),
                    &target_node,
                )
                .await;
                self.ui_state
                    .push_log(format!("encrypted dm sent to {target_callsign}"));
                self.sync_security_to_ui();
            }
            Err(err) => self.publish(KayaEvent::SecurityWarning {
                node_id: Some(target_node),
                message: err.to_string(),
            }),
        }
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

    fn show_who(&mut self, fingerprints: bool) {
        let peers = self.peers.snapshots();
        if peers.is_empty() {
            self.system_message("no peers discovered");
            return;
        }

        let summary = peers
            .into_iter()
            .filter(|peer| !self.trust_store.is_blocked(&peer.node_id))
            .map(|peer| {
                let status = if peer.online { "online" } else { "offline" };
                if fingerprints {
                    let fingerprint = self
                        .trust_store
                        .get(&peer.node_id)
                        .map(|peer| short_fingerprint(&peer.fingerprint))
                        .unwrap_or_else(|| "--".into());
                    let trust = self.trust_store.status(&peer.node_id);
                    format!(
                        "{} {} {} {} {} {}",
                        peer.callsign, peer.node_id, peer.presence, status, fingerprint, trust
                    )
                } else {
                    format!(
                        "{} {} {} {}",
                        peer.callsign, peer.node_id, peer.presence, status
                    )
                }
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
            "node={} room=#{} peers={} routes={} packets_tx={} packets_rx={} events={} secure_sessions={}",
            self.node_id,
            self.rooms.current_room(),
            self.peers.online_count(),
            self.mesh.table.len(),
            self.ui_state.packets_tx,
            self.ui_state.packets_rx,
            self.diagnostics.counters.total(),
            self.sessions.active_count()
        ));
    }

    fn show_identity(&mut self) {
        self.system_message(format!(
            "identity node={} callsign={} fingerprint={} ed25519={} x25519={}",
            self.node_id,
            self.callsign,
            self.identity.fingerprint,
            short_key(&self.identity.ed25519_public_key_hex()),
            short_key(&self.identity.x25519_public_key_hex())
        ));
    }

    fn set_peer_trust_status(&mut self, peer: &str, status: TrustStatus) {
        let Some(target) = self.resolve_security_target(peer) else {
            return;
        };
        match self.trust_store.set_status(&target.node_id, status) {
            Ok(()) => match status {
                TrustStatus::Trusted => self.publish(KayaEvent::PeerTrusted {
                    node_id: target.node_id,
                    callsign: target.callsign,
                    fingerprint: target.fingerprint,
                }),
                TrustStatus::Blocked => self.publish(KayaEvent::PeerBlocked {
                    node_id: target.node_id,
                    callsign: target.callsign,
                    fingerprint: target.fingerprint,
                }),
                TrustStatus::Unknown => {
                    self.system_message(format!("untrusted {}", target.node_id));
                    self.sync_peers_to_ui();
                }
            },
            Err(err) => self.system_message(err.to_string()),
        }
    }

    fn resolve_security_target(&mut self, target: &str) -> Option<SecurityTarget> {
        if let Some(peer) = self.trust_store.find(target) {
            return Some(SecurityTarget {
                node_id: peer.node_id.clone(),
                callsign: peer.callsign.clone(),
                fingerprint: peer.fingerprint.clone(),
            });
        }

        match self.peers.resolve_target_checked(target) {
            TargetResolution::Found(peer) => {
                let Some(record) = self.trust_store.get(&peer.node_id) else {
                    self.system_message(format!(
                        "peer {} has no fingerprint yet; wait for a signed packet",
                        peer.node_id
                    ));
                    return None;
                };
                Some(SecurityTarget {
                    node_id: peer.node_id,
                    callsign: peer.callsign,
                    fingerprint: record.fingerprint.clone(),
                })
            }
            TargetResolution::NotFound(target) => {
                self.system_message(format!("peer not found: {target}"));
                None
            }
            TargetResolution::DuplicateCallsign { callsign, matches } => {
                self.system_message(format!(
                    "callsign {callsign} is ambiguous: {}",
                    matches.join(", ")
                ));
                None
            }
        }
    }

    fn show_trust_list(&mut self) {
        let peers = self.trust_store.list();
        if peers.is_empty() {
            self.system_message("trust store is empty");
            return;
        }
        let summary = peers
            .into_iter()
            .map(|peer| {
                format!(
                    "{} {} {} {}",
                    peer.callsign,
                    peer.node_id,
                    short_fingerprint(&peer.fingerprint),
                    peer.trust_status
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(summary);
    }

    fn show_sessions(&mut self) {
        let sessions = self.sessions.views();
        if sessions.is_empty() {
            self.system_message("no secure sessions");
            return;
        }
        let summary = sessions
            .into_iter()
            .map(|session| {
                format!(
                    "{} {} {} count={}",
                    session.peer_node_id,
                    session.session_id,
                    session.status,
                    session.message_counter
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(summary);
    }

    fn close_secure_session(&mut self, peer: &str) {
        let Some(target) = self.resolve_security_target(peer) else {
            return;
        };
        let session_id = self
            .sessions
            .views()
            .into_iter()
            .find(|session| session.peer_node_id == target.node_id)
            .map(|session| session.session_id);
        if self.sessions.close(&target.node_id) {
            self.publish(KayaEvent::SecureSessionClosed {
                peer_node_id: target.node_id,
                session_id,
            });
        } else {
            self.system_message(format!("no secure session with {}", target.node_id));
        }
    }
}

fn short_fingerprint(fingerprint: &str) -> String {
    fingerprint
        .strip_prefix(FINGERPRINT_PREFIX)
        .unwrap_or(fingerprint)
        .to_string()
}

fn short_key(key: &str) -> String {
    let head = key.get(..8).unwrap_or(key);
    let tail = key.get(key.len().saturating_sub(8)..).unwrap_or_default();
    format!("{head}...{tail}")
}
