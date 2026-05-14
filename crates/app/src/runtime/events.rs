use super::Runtime;
use kaya_events::KayaEvent;
use kaya_protocol::{Packet, PacketType};
use kaya_rooms::{ChatMessage, RouteOutcome};
use kaya_security::{
    encrypted_payload_from_packet, packet_requires_signature_validation,
    session_accept_from_packet, session_request_from_packet, verify_packet_signature,
    SignatureStatus, TrustObservation,
};
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
            KayaEvent::IdentityLoaded {
                node_id,
                fingerprint,
            } => {
                self.ui_state
                    .push_log(format!("identity loaded {node_id} {fingerprint}"));
                self.sync_security_to_ui();
            }
            KayaEvent::IdentityCreated {
                node_id,
                fingerprint,
            } => {
                self.ui_state
                    .push_log(format!("identity created {node_id} {fingerprint}"));
                self.system_message(format!("identity fingerprint {fingerprint}"));
                self.sync_security_to_ui();
            }
            KayaEvent::PacketSignatureValid {
                node_id,
                fingerprint,
            } => {
                self.ui_state
                    .push_log(format!("signature valid {node_id} {fingerprint}"));
            }
            KayaEvent::PacketSignatureInvalid { node_id, reason } => {
                self.ui_state
                    .push_log(format!("signature invalid {node_id}: {reason}"));
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
                    encrypted: false,
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
                    encrypted: false,
                };
                self.push_chat_message(&message, local);
                self.persist_chat_message(&message);
            }
            KayaEvent::EncryptedMessageReceived {
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
                    encrypted: true,
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
                    encrypted: false,
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
            KayaEvent::PeerTrusted {
                node_id,
                callsign,
                fingerprint,
            } => {
                self.system_message(format!("trusted {callsign} {node_id} {fingerprint}"));
                self.sync_peers_to_ui();
            }
            KayaEvent::PeerBlocked {
                node_id,
                callsign,
                fingerprint,
            } => {
                self.system_message(format!("blocked {callsign} {node_id} {fingerprint}"));
                self.sync_peers_to_ui();
            }
            KayaEvent::SecureSessionStarted {
                peer_node_id,
                session_id,
            } => {
                self.ui_state
                    .push_log(format!("secure session active {peer_node_id} {session_id}"));
                self.system_message(format!("secure session active with {peer_node_id}"));
                self.sync_security_to_ui();
            }
            KayaEvent::SecureSessionClosed {
                peer_node_id,
                session_id,
            } => {
                let suffix = session_id
                    .map(|value| format!(" {value}"))
                    .unwrap_or_default();
                self.system_message(format!("secure session closed with {peer_node_id}{suffix}"));
                self.sync_security_to_ui();
            }
            KayaEvent::FileOfferReceived {
                file_id,
                file_name,
                from_node,
                from_callsign,
                size_bytes,
                encrypted,
            } => {
                let fingerprint = self
                    .trust_store
                    .get(&from_node)
                    .map(|peer| peer.fingerprint.clone())
                    .unwrap_or_else(|| "--".into());
                let mode = if encrypted {
                    "encrypted"
                } else {
                    "unencrypted"
                };
                self.system_message(format!(
                    "{from_callsign} offers file: {file_name}, {} bytes, {mode}, fingerprint {fingerprint}. Use /accept-file {file_id} or /reject-file {file_id}",
                    size_bytes
                ));
                self.sync_files_to_ui();
            }
            KayaEvent::FileOfferSent {
                file_id,
                file_name,
                target_callsign,
                encrypted,
                ..
            } => {
                let mode = if encrypted {
                    "encrypted"
                } else {
                    "unencrypted"
                };
                self.system_message(format!(
                    "offered file {file_name} to {target_callsign}: {file_id} [{mode}]"
                ));
                self.sync_files_to_ui();
            }
            KayaEvent::FileAccepted {
                file_id, callsign, ..
            } => {
                self.ui_state
                    .push_log(format!("file accepted {file_id} by {callsign}"));
                self.sync_files_to_ui();
            }
            KayaEvent::FileRejected {
                file_id,
                callsign,
                reason,
                ..
            } => {
                self.system_message(format!(
                    "file {file_id} rejected by {callsign}: {}",
                    reason.unwrap_or_else(|| "no reason".into())
                ));
                self.sync_files_to_ui();
            }
            KayaEvent::FileChunkReceived {
                file_id,
                chunk_index,
                total_chunks,
            } => {
                self.ui_state.push_log(format!(
                    "file chunk {file_id} {}/{}",
                    chunk_index + 1,
                    total_chunks
                ));
                self.sync_files_to_ui();
            }
            KayaEvent::FileChunkAcked {
                file_id,
                chunk_index,
            } => {
                self.ui_state
                    .push_log(format!("file chunk ack {file_id} {chunk_index}"));
            }
            KayaEvent::FileTransferProgress { file_id, .. } => {
                self.sync_files_to_ui();
                self.ui_state.push_log(format!("file progress {file_id}"));
            }
            KayaEvent::FileTransferCompleted { file_id, path } => {
                self.system_message(format!(
                    "file transfer completed {file_id} {}",
                    path.unwrap_or_else(|| "--".into())
                ));
                self.sync_files_to_ui();
            }
            KayaEvent::FileTransferCancelled { file_id, reason } => {
                self.system_message(format!(
                    "file transfer cancelled {file_id}: {}",
                    reason.unwrap_or_else(|| "no reason".into())
                ));
                self.sync_files_to_ui();
            }
            KayaEvent::FileTransferFailed { file_id, reason } => {
                self.system_message(format!("file transfer failed {file_id}: {reason}"));
                self.sync_files_to_ui();
            }
            KayaEvent::FileHashVerified { file_id, sha256 } => {
                self.ui_state
                    .push_log(format!("file hash verified {file_id} {sha256}"));
                self.sync_files_to_ui();
            }
            KayaEvent::FileHashMismatch { file_id } => {
                self.system_message(format!("file hash mismatch {file_id}"));
                self.sync_files_to_ui();
            }
            KayaEvent::SecurityWarning { node_id, message } => {
                self.ui_state.security_warnings += 1;
                let source = node_id.unwrap_or_else(|| "unknown".into());
                self.ui_state
                    .push_log(format!("security warning {source}: {message}"));
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

            if !self.inspect_packet_security(&packet) {
                return;
            }

            self.observe_peer(&packet);
            self.remember_peer(&packet);
            self.sync_peers_to_ui();
            if self.route_security_packet(&packet).await {
                return;
            }
            if self.route_file_packet(&packet).await {
                return;
            }
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

    async fn route_security_packet(&mut self, packet: &Packet) -> bool {
        match packet.packet_type {
            PacketType::DmSessionRequest => {
                if !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(request) = session_request_from_packet(packet) else {
                    self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: "malformed secure session request".into(),
                    });
                    return true;
                };
                if !self.packet_fingerprint_matches(&packet.node_id, &request.fingerprint) {
                    return true;
                }
                match self.sessions.accept_request(
                    &packet.node_id,
                    &request.session_id,
                    &request.x25519_public_key,
                    &request.fingerprint,
                ) {
                    Ok(accept) => {
                        self.publish(KayaEvent::SecureSessionStarted {
                            peer_node_id: packet.node_id.clone(),
                            session_id: accept.session_id.clone(),
                        });
                        self.send_packet(Packet::dm_session_accept(
                            self.node_id.clone(),
                            self.callsign.clone(),
                            packet.node_id.clone(),
                            accept.session_id,
                            accept.x25519_public_key,
                            accept.fingerprint,
                        ))
                        .await;
                    }
                    Err(err) => self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: err.to_string(),
                    }),
                }
                true
            }
            PacketType::DmSessionAccept => {
                if !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(accept) = session_accept_from_packet(packet) else {
                    self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: "malformed secure session accept".into(),
                    });
                    return true;
                };
                if !self.packet_fingerprint_matches(&packet.node_id, &accept.fingerprint) {
                    return true;
                }
                match self.sessions.complete_accept(
                    &packet.node_id,
                    &accept.session_id,
                    &accept.x25519_public_key,
                    &accept.fingerprint,
                ) {
                    Ok(()) => {
                        self.publish(KayaEvent::SecureSessionStarted {
                            peer_node_id: packet.node_id.clone(),
                            session_id: accept.session_id,
                        });
                        self.flush_pending_secure_messages(&packet.node_id).await;
                    }
                    Err(err) => self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: err.to_string(),
                    }),
                }
                true
            }
            PacketType::DirectMessageEncrypted => {
                if !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(payload) = encrypted_payload_from_packet(packet) else {
                    self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: "malformed encrypted dm".into(),
                    });
                    return true;
                };
                if !self.packet_fingerprint_matches(&packet.node_id, &payload.sender_fingerprint) {
                    return true;
                }
                match self.sessions.decrypt(&packet.node_id, &payload) {
                    Ok(body) => {
                        self.publish(KayaEvent::EncryptedMessageReceived {
                            from_node: packet.node_id.clone(),
                            from_callsign: packet.callsign.clone(),
                            target_node: self.node_id.clone(),
                            body,
                            local: false,
                        });
                    }
                    Err(err) => self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: err.to_string(),
                    }),
                }
                true
            }
            _ => false,
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

    fn inspect_packet_security(&mut self, packet: &Packet) -> bool {
        if self.trust_store.is_blocked(&packet.node_id) {
            self.publish(KayaEvent::SecurityWarning {
                node_id: Some(packet.node_id.clone()),
                message: "blocked peer packet rejected".into(),
            });
            return false;
        }

        match verify_packet_signature(packet) {
            SignatureStatus::Valid { fingerprint } => {
                self.publish(KayaEvent::PacketSignatureValid {
                    node_id: packet.node_id.clone(),
                    fingerprint: fingerprint.clone(),
                });
                match self
                    .trust_store
                    .record_seen(&packet.node_id, &packet.callsign, &fingerprint)
                {
                    Ok(TrustObservation::FingerprintChanged { previous, current }) => {
                        self.publish(KayaEvent::SecurityWarning {
                            node_id: Some(packet.node_id.clone()),
                            message: format!("fingerprint changed {previous} -> {current}"),
                        });
                    }
                    Ok(TrustObservation::New | TrustObservation::Updated) => {}
                    Err(err) => self.publish(KayaEvent::SecurityWarning {
                        node_id: Some(packet.node_id.clone()),
                        message: err.to_string(),
                    }),
                }
                true
            }
            SignatureStatus::Missing if secure_packet_requires_signature(packet.packet_type) => {
                self.publish(KayaEvent::SecurityWarning {
                    node_id: Some(packet.node_id.clone()),
                    message: "unsigned secure packet rejected".into(),
                });
                false
            }
            SignatureStatus::Missing => true,
            SignatureStatus::Invalid { reason } => {
                let required = packet_requires_signature_validation(packet.packet_type);
                self.publish(KayaEvent::PacketSignatureInvalid {
                    node_id: packet.node_id.clone(),
                    reason: reason.clone(),
                });
                self.publish(KayaEvent::SecurityWarning {
                    node_id: Some(packet.node_id.clone()),
                    message: if required {
                        "invalid required packet signature rejected".into()
                    } else {
                        "invalid packet signature rejected".into()
                    },
                });
                false
            }
        }
    }

    pub(super) fn packet_targets_local_node(&self, packet: &Packet) -> bool {
        packet.target_node.as_deref() == Some(self.node_id.as_str())
            || packet
                .target_node
                .as_deref()
                .map(|target| target.eq_ignore_ascii_case(&self.callsign))
                .unwrap_or(false)
    }

    pub(super) fn packet_fingerprint_matches(&self, node_id: &str, fingerprint: &str) -> bool {
        let matches = self
            .trust_store
            .get(node_id)
            .map(|peer| peer.fingerprint == fingerprint)
            .unwrap_or(true);
        if !matches {
            self.publish(KayaEvent::SecurityWarning {
                node_id: Some(node_id.to_string()),
                message: "packet fingerprint does not match signed identity".into(),
            });
        }
        matches
    }
}

fn secure_packet_requires_signature(packet_type: PacketType) -> bool {
    matches!(
        packet_type,
        PacketType::DmSessionRequest
            | PacketType::DmSessionAccept
            | PacketType::DirectMessageEncrypted
            | PacketType::FileChunkEncrypted
    )
}
