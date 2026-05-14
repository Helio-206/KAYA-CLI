use super::Runtime;
use kaya_events::KayaEvent;
use kaya_files::{
    FileChunk, FileMetadata, OutgoingFileRequest, TransferDirection, TransferSecurity,
    TransferStatus,
};
use kaya_peer::TargetResolution;
use kaya_protocol::{
    FileChunkPayload, FileEncryptedChunkPayload, FileOfferPayload, Packet, PacketType,
};
use kaya_security::{decode_hex, EncryptedPayload, TrustStatus};

impl Runtime {
    pub(super) async fn send_file_offer(&mut self, target: String, path: String) {
        if !self.file_config.enabled {
            self.system_message("file transfer is disabled");
            return;
        }
        let target = match self.peers.resolve_target_checked(&target) {
            TargetResolution::Found(peer) => peer,
            TargetResolution::NotFound(target) => {
                let Some(peer) = self.resolve_mesh_target(&target) else {
                    if kaya_shared::is_valid_node_id(&target) {
                        self.send_route_request(&target).await;
                        self.system_message(format!(
                            "file target not found locally: {target}; route request sent"
                        ));
                    } else {
                        self.system_message(format!("file target not found: {target}"));
                    }
                    return;
                };
                self.send_file_offer_to(peer.node_id, peer.callsign, path)
                    .await;
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
            self.system_message(format!("file target is blocked: {}", target.node_id));
            return;
        }

        self.send_file_offer_to(target.node_id, target.callsign, path)
            .await;
    }

    async fn send_file_offer_to(
        &mut self,
        target_node: String,
        target_callsign: String,
        path: String,
    ) {
        if self.trust_store.is_blocked(&target_node) {
            self.system_message(format!("file target is blocked: {target_node}"));
            return;
        }
        let security = if self.sessions.has_active(&target_node) {
            TransferSecurity::Encrypted
        } else {
            TransferSecurity::Unencrypted
        };
        let session = match self.files.prepare_outgoing(
            OutgoingFileRequest {
                path: path.into(),
                sender_node_id: self.node_id.clone(),
                sender_callsign: self.callsign.clone(),
                peer_node_id: target_node.clone(),
                peer_callsign: target_callsign.clone(),
                security,
            },
            &self.file_config,
        ) {
            Ok(session) => session.clone(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        self.persist_file_session(&session.file_id);
        self.publish(KayaEvent::FileOfferSent {
            file_id: session.file_id.clone(),
            file_name: session.metadata.file_name.clone(),
            target_node: target_node.clone(),
            target_callsign: target_callsign.clone(),
            size_bytes: session.metadata.file_size,
            encrypted: security == TransferSecurity::Encrypted,
        });
        self.send_packet_routed(
            Packet::file_offer(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.clone(),
                file_offer_payload(&session.metadata, security == TransferSecurity::Encrypted),
            ),
            &target_node,
        )
        .await;
        self.sync_files_to_ui();
    }

    pub(super) async fn accept_file(&mut self, file_id: &str) {
        let session = match self.files.session(file_id) {
            Ok(session) => session.clone(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        if session.direction != TransferDirection::Incoming {
            self.system_message(format!("{file_id} is not an incoming offer"));
            return;
        }
        if let Err(err) = self.files.accept(file_id) {
            self.system_message(err.to_string());
            return;
        }
        self.persist_file_session(file_id);
        self.publish(KayaEvent::FileAccepted {
            file_id: file_id.to_string(),
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
        });
        self.send_packet_routed(
            Packet::file_accept(
                self.node_id.clone(),
                self.callsign.clone(),
                session.peer_node_id.clone(),
                file_id.to_string(),
            ),
            &session.peer_node_id,
        )
        .await;
        self.sync_files_to_ui();
    }

    pub(super) async fn reject_file(&mut self, file_id: &str) {
        let session = match self.files.session(file_id) {
            Ok(session) => session.clone(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        let _ = self.files.reject(file_id);
        self.persist_file_session(file_id);
        self.publish(KayaEvent::FileRejected {
            file_id: file_id.to_string(),
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
            reason: Some("operator rejected".into()),
        });
        self.send_packet_routed(
            Packet::file_reject(
                self.node_id.clone(),
                self.callsign.clone(),
                session.peer_node_id.clone(),
                file_id.to_string(),
                "operator rejected",
            ),
            &session.peer_node_id,
        )
        .await;
        self.sync_files_to_ui();
    }

    pub(super) async fn cancel_file(&mut self, file_id: &str) {
        let session = match self.files.session(file_id) {
            Ok(session) => session.clone(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        let _ = self.files.cancel(file_id);
        self.persist_file_session(file_id);
        self.publish(KayaEvent::FileTransferCancelled {
            file_id: file_id.to_string(),
            reason: Some("operator cancelled".into()),
        });
        self.send_packet_routed(
            Packet::file_transfer_cancel(
                self.node_id.clone(),
                self.callsign.clone(),
                session.peer_node_id.clone(),
                file_id.to_string(),
                "operator cancelled",
            ),
            &session.peer_node_id,
        )
        .await;
        self.sync_files_to_ui();
    }

    pub(super) fn show_files(&mut self) {
        let sessions = self.files.sessions();
        if sessions.is_empty() {
            self.system_message("no file transfers");
            return;
        }
        let summary = sessions
            .into_iter()
            .take(8)
            .map(|session| {
                let progress = session.progress();
                format!(
                    "{} {} {} {:.0}% {}",
                    session.file_id,
                    session.metadata.file_name,
                    session.peer_callsign,
                    progress.percent,
                    session.status
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(summary);
    }

    pub(super) fn show_file_info(&mut self, file_id: &str) {
        match self.files.session(file_id) {
            Ok(session) => {
                self.system_message(format!(
                    "{} {} size={} chunks={}/{} status={} security={} path={}",
                    session.file_id,
                    session.metadata.file_name,
                    session.metadata.file_size,
                    session.chunks_received,
                    session.total_chunks,
                    session.status,
                    session.security,
                    session.completed_path.as_deref().unwrap_or("--")
                ));
            }
            Err(err) => self.system_message(err.to_string()),
        }
    }

    pub(super) fn show_files_folder(&mut self) {
        self.system_message(format!(
            "completed files: {}",
            self.file_store.completed_dir().display()
        ));
    }

    pub(super) async fn route_file_packet(&mut self, packet: &Packet) -> bool {
        match packet.packet_type {
            PacketType::FileOffer => self.receive_file_offer(packet).await,
            PacketType::FileAccept => self.receive_file_accept(packet).await,
            PacketType::FileReject => self.receive_file_reject(packet),
            PacketType::FileChunk => self.receive_file_chunk(packet, false).await,
            PacketType::FileChunkEncrypted => self.receive_file_chunk(packet, true).await,
            PacketType::FileChunkAck => self.receive_file_ack(packet),
            PacketType::FileTransferComplete => self.receive_file_complete(packet),
            PacketType::FileTransferCancel => self.receive_file_cancel(packet),
            PacketType::FileTransferError => self.receive_file_error(packet),
            _ => return false,
        }
        true
    }

    async fn receive_file_offer(&mut self, packet: &Packet) {
        if !self.packet_targets_local_node(packet) {
            return;
        }
        if !self.file_config.enabled {
            self.send_packet_routed(
                Packet::file_reject(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    packet.node_id.clone(),
                    payload_str(packet, "file_id")
                        .unwrap_or("unknown")
                        .to_string(),
                    "file transfer disabled",
                ),
                &packet.node_id,
            )
            .await;
            return;
        }
        let payload: FileOfferPayload = match serde_json::from_value(packet.payload.clone()) {
            Ok(payload) => payload,
            Err(err) => {
                self.security_warning(Some(packet.node_id.clone()), err.to_string());
                return;
            }
        };
        let metadata = metadata_from_offer(&payload);
        let signed = packet.public_key.is_some() && packet.signature.is_some();
        let trusted = self.trust_store.status(&packet.node_id) == TrustStatus::Trusted;
        if !trusted && !self.file_config.accept_from_unknown {
            self.send_packet_routed(
                Packet::file_reject(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    packet.node_id.clone(),
                    payload.file_id,
                    "unknown peer not allowed",
                ),
                &packet.node_id,
            )
            .await;
            return;
        }
        let security = if payload.encrypted {
            TransferSecurity::Encrypted
        } else {
            TransferSecurity::Unencrypted
        };
        let session = self
            .files
            .receive_offer(
                metadata,
                &packet.node_id,
                &packet.callsign,
                security,
                signed,
                trusted,
            )
            .clone();
        self.persist_file_session(&session.file_id);
        self.publish(KayaEvent::FileOfferReceived {
            file_id: session.file_id.clone(),
            file_name: session.metadata.file_name.clone(),
            from_node: packet.node_id.clone(),
            from_callsign: packet.callsign.clone(),
            size_bytes: session.metadata.file_size,
            encrypted: security == TransferSecurity::Encrypted,
        });
        self.sync_files_to_ui();
    }

    async fn receive_file_accept(&mut self, packet: &Packet) {
        if !self.packet_targets_local_node(packet) {
            return;
        }
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        if let Err(err) = self.files.mark_outgoing_accepted(&file_id) {
            self.system_message(err.to_string());
            return;
        }
        self.persist_file_session(&file_id);
        self.publish(KayaEvent::FileAccepted {
            file_id: file_id.clone(),
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
        });
        self.send_file_chunks(&file_id, &packet.node_id).await;
    }

    fn receive_file_reject(&mut self, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let _ = self.files.reject(&file_id);
        self.persist_file_session(&file_id);
        self.publish(KayaEvent::FileRejected {
            file_id,
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
            reason: payload_str(packet, "reason").map(str::to_string),
        });
    }

    async fn receive_file_chunk(&mut self, packet: &Packet, encrypted: bool) {
        if !self.packet_targets_local_node(packet) {
            return;
        }
        let chunk = if encrypted {
            match self.decrypt_file_chunk_packet(packet) {
                Ok(chunk) => chunk,
                Err(err) => {
                    self.security_warning(Some(packet.node_id.clone()), err.to_string());
                    return;
                }
            }
        } else {
            match file_chunk_from_packet(packet) {
                Ok(chunk) => chunk,
                Err(err) => {
                    self.security_warning(Some(packet.node_id.clone()), err.to_string());
                    return;
                }
            }
        };
        let file_id = chunk.file_id.clone();
        let chunk_index = chunk.chunk_index;
        let total_chunks = chunk.total_chunks;
        match self.files.receive_chunk(chunk) {
            Ok(Some(bytes)) => {
                self.publish(KayaEvent::FileChunkReceived {
                    file_id: file_id.clone(),
                    chunk_index,
                    total_chunks,
                });
                self.publish_file_progress(&file_id);
                let session = match self.files.session(&file_id) {
                    Ok(session) => session.clone(),
                    Err(err) => {
                        self.security_warning(Some(packet.node_id.clone()), err.to_string());
                        return;
                    }
                };
                match self.file_store.save_completed(&session, &bytes) {
                    Ok(path) => {
                        let path_string = path.display().to_string();
                        let _ = self
                            .files
                            .mark_completed_path(&file_id, path_string.clone());
                        self.persist_file_session(&file_id);
                        self.publish(KayaEvent::FileHashVerified {
                            file_id: file_id.clone(),
                            sha256: session.metadata.sha256,
                        });
                        self.publish(KayaEvent::FileTransferCompleted {
                            file_id: file_id.clone(),
                            path: Some(path_string),
                        });
                        self.send_packet_routed(
                            Packet::file_transfer_complete(
                                self.node_id.clone(),
                                self.callsign.clone(),
                                packet.node_id.clone(),
                                file_id.clone(),
                            ),
                            &packet.node_id,
                        )
                        .await;
                    }
                    Err(err) => self.publish(KayaEvent::FileTransferFailed {
                        file_id: file_id.clone(),
                        reason: err.to_string(),
                    }),
                }
            }
            Ok(None) => {
                self.publish(KayaEvent::FileChunkReceived {
                    file_id: file_id.clone(),
                    chunk_index,
                    total_chunks,
                });
                self.publish_file_progress(&file_id);
            }
            Err(err) => {
                self.publish(KayaEvent::FileHashMismatch {
                    file_id: file_id.clone(),
                });
                self.publish(KayaEvent::FileTransferFailed {
                    file_id: file_id.clone(),
                    reason: err.to_string(),
                });
                self.send_packet_routed(
                    Packet::file_transfer_error(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        packet.node_id.clone(),
                        file_id.clone(),
                        err.to_string(),
                    ),
                    &packet.node_id,
                )
                .await;
            }
        }
        self.send_packet_routed(
            Packet::file_chunk_ack(
                self.node_id.clone(),
                self.callsign.clone(),
                packet.node_id.clone(),
                file_id,
                chunk_index,
            ),
            &packet.node_id,
        )
        .await;
        self.sync_files_to_ui();
    }

    fn receive_file_ack(&mut self, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let chunk_index = packet
            .payload
            .get("chunk_index")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as u32;
        self.publish(KayaEvent::FileChunkAcked {
            file_id,
            chunk_index,
        });
    }

    fn receive_file_complete(&mut self, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        if let Ok(session) = self.files.session_mut(&file_id) {
            session.status = TransferStatus::Completed;
        }
        self.persist_file_session(&file_id);
        self.publish(KayaEvent::FileTransferCompleted {
            file_id,
            path: None,
        });
        self.sync_files_to_ui();
    }

    fn receive_file_cancel(&mut self, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let _ = self.files.cancel(&file_id);
        self.persist_file_session(&file_id);
        self.publish(KayaEvent::FileTransferCancelled {
            file_id,
            reason: payload_str(packet, "reason").map(str::to_string),
        });
        self.sync_files_to_ui();
    }

    fn receive_file_error(&mut self, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let reason = payload_str(packet, "reason")
            .unwrap_or("remote file transfer error")
            .to_string();
        let _ = self.files.fail(&file_id, reason.clone());
        self.persist_file_session(&file_id);
        self.publish(KayaEvent::FileTransferFailed { file_id, reason });
        self.sync_files_to_ui();
    }

    async fn send_file_chunks(&mut self, file_id: &str, peer_node_id: &str) {
        if !self.direct_peer_connected(peer_node_id)
            && self
                .peers
                .get(peer_node_id)
                .map(|peer| !peer.online)
                .unwrap_or(true)
        {
            let reason = "file chunks over mesh not enabled yet".to_string();
            let _ = self.files.fail(file_id, reason.clone());
            self.persist_file_session(file_id);
            self.publish(KayaEvent::FileTransferFailed {
                file_id: file_id.to_string(),
                reason: reason.clone(),
            });
            self.send_packet_routed(
                Packet::file_transfer_error(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    peer_node_id.to_string(),
                    file_id.to_string(),
                    reason,
                ),
                peer_node_id,
            )
            .await;
            return;
        }
        let session = match self.files.session(file_id) {
            Ok(session) => session.clone(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        let chunks = match self.files.outgoing_chunks(file_id) {
            Ok(chunks) => chunks.to_vec(),
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        for chunk in chunks {
            if session.security == TransferSecurity::Encrypted {
                match self
                    .sessions
                    .encrypt_file_chunk(peer_node_id, &chunk.payload)
                {
                    Ok(payload) => {
                        self.send_packet_routed(
                            Packet::file_chunk_encrypted(
                                self.node_id.clone(),
                                self.callsign.clone(),
                                peer_node_id.to_string(),
                                FileEncryptedChunkPayload {
                                    file_id: chunk.file_id.clone(),
                                    chunk_index: chunk.chunk_index,
                                    total_chunks: chunk.total_chunks,
                                    chunk_hash: chunk.chunk_hash,
                                    session_id: payload.session_id,
                                    nonce: payload.nonce,
                                    ciphertext: payload.ciphertext,
                                    sender_fingerprint: payload.sender_fingerprint,
                                    timestamp: payload.timestamp,
                                },
                            ),
                            peer_node_id,
                        )
                        .await;
                    }
                    Err(err) => {
                        self.security_warning(Some(peer_node_id.to_string()), err.to_string());
                        return;
                    }
                }
            } else {
                self.send_packet_routed(
                    Packet::file_chunk(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        peer_node_id.to_string(),
                        FileChunkPayload {
                            file_id: chunk.file_id,
                            chunk_index: chunk.chunk_index,
                            total_chunks: chunk.total_chunks,
                            chunk_hash: chunk.chunk_hash,
                            payload: kaya_security::encode_hex(&chunk.payload),
                            timestamp: chunk.timestamp,
                        },
                    ),
                    peer_node_id,
                )
                .await;
            }
        }
        self.sync_files_to_ui();
    }

    fn decrypt_file_chunk_packet(&mut self, packet: &Packet) -> kaya_shared::Result<FileChunk> {
        let payload: FileEncryptedChunkPayload = serde_json::from_value(packet.payload.clone())?;
        if !self.packet_fingerprint_matches(&packet.node_id, &payload.sender_fingerprint) {
            return Err(kaya_shared::KayaError::Security(
                "file chunk fingerprint mismatch".into(),
            ));
        }
        let bytes = self.sessions.decrypt_file_chunk(
            &packet.node_id,
            &EncryptedPayload {
                session_id: payload.session_id,
                nonce: payload.nonce,
                ciphertext: payload.ciphertext,
                sender_fingerprint: payload.sender_fingerprint,
                timestamp: payload.timestamp.clone(),
            },
        )?;
        Ok(FileChunk {
            file_id: payload.file_id,
            chunk_index: payload.chunk_index,
            total_chunks: payload.total_chunks,
            chunk_hash: payload.chunk_hash,
            payload: bytes,
            timestamp: payload.timestamp,
        })
    }

    fn publish_file_progress(&mut self, file_id: &str) {
        if let Ok(session) = self.files.session(file_id) {
            self.publish(KayaEvent::FileTransferProgress {
                file_id: file_id.to_string(),
                bytes_received: session.bytes_received,
                chunks_received: session.chunks_received,
                total_chunks: session.total_chunks,
            });
        }
    }

    pub(super) fn persist_file_session(&mut self, file_id: &str) {
        let Ok(session) = self.files.session(file_id) else {
            return;
        };
        if let Err(err) = self.file_store.save_record(session) {
            self.publish(KayaEvent::ErrorOccurred {
                scope: "files.metadata".into(),
                message: err.to_string(),
            });
        }
    }

    fn security_warning(&self, node_id: Option<String>, message: String) {
        self.publish(KayaEvent::SecurityWarning { node_id, message });
    }

    pub(super) fn expire_stale_file_transfers(&mut self, cutoff_ms: u64) {
        let stale_ids: Vec<_> = self
            .files
            .sessions()
            .into_iter()
            .filter(|session| {
                matches!(
                    session.status,
                    TransferStatus::Offered
                        | TransferStatus::Accepted
                        | TransferStatus::Transferring
                        | TransferStatus::Paused
                ) && session.updated_at.parse::<u64>().unwrap_or(u64::MAX) < cutoff_ms
            })
            .map(|session| session.file_id)
            .collect();

        for file_id in stale_ids {
            let reason = format!(
                "file transfer idle timeout after {}ms",
                self.timeouts.file_transfer_idle_ms
            );
            let _ = self.files.fail(&file_id, reason.clone());
            self.persist_file_session(&file_id);
            self.publish(KayaEvent::FileTransferFailed { file_id, reason });
        }
        self.sync_files_to_ui();
    }
}

fn file_offer_payload(metadata: &FileMetadata, encrypted: bool) -> FileOfferPayload {
    FileOfferPayload {
        file_id: metadata.file_id.clone(),
        file_name: metadata.file_name.clone(),
        file_size: metadata.file_size,
        mime_type: metadata.mime_type.clone(),
        sha256: metadata.sha256.clone(),
        chunk_size: metadata.chunk_size,
        total_chunks: metadata.total_chunks,
        sender_node_id: metadata.sender_node_id.clone(),
        sender_callsign: metadata.sender_callsign.clone(),
        created_at: metadata.created_at.clone(),
        dangerous_extension: metadata.dangerous_extension,
        encrypted,
    }
}

fn metadata_from_offer(payload: &FileOfferPayload) -> FileMetadata {
    FileMetadata {
        file_id: payload.file_id.clone(),
        file_name: payload.file_name.clone(),
        file_size: payload.file_size,
        mime_type: payload.mime_type.clone(),
        sha256: payload.sha256.clone(),
        chunk_size: payload.chunk_size,
        total_chunks: payload.total_chunks,
        sender_node_id: payload.sender_node_id.clone(),
        sender_callsign: payload.sender_callsign.clone(),
        created_at: payload.created_at.clone(),
        dangerous_extension: payload.dangerous_extension,
    }
}

fn file_chunk_from_packet(packet: &Packet) -> kaya_shared::Result<FileChunk> {
    let payload: FileChunkPayload = serde_json::from_value(packet.payload.clone())?;
    Ok(FileChunk {
        file_id: payload.file_id,
        chunk_index: payload.chunk_index,
        total_chunks: payload.total_chunks,
        chunk_hash: payload.chunk_hash,
        payload: decode_hex(&payload.payload)?,
        timestamp: payload.timestamp,
    })
}

fn payload_str<'a>(packet: &'a Packet, field: &str) -> Option<&'a str> {
    packet
        .payload
        .get(field)
        .and_then(serde_json::Value::as_str)
}
