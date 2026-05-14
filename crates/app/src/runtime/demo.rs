use super::Runtime;
use kaya_events::KayaEvent;
use kaya_files::{FileMetadata, FileTransferManager, TransferSecurity};
use kaya_mesh::{MeshEnvelope, RouteEntry, RouteEntrySpec, RouteSource};
use kaya_peer::PeerRegistry;
use kaya_protocol::Packet;
use kaya_rooms::RouteOutcome;
use kaya_shared::PresenceStatus;
use kaya_ui::UiState;
use std::time::Duration;

impl Runtime {
    pub(super) fn demo_reset(&mut self) {
        if !self.ensure_demo_mode() {
            return;
        }

        self.peers = PeerRegistry::with_timeout(
            &self.node_id,
            Duration::from_secs(self.config.peer_timeout_secs),
        );
        self.rooms = kaya_rooms::RoomStore::new(&self.node_id, &self.callsign);
        let _ = self.rooms.join(self.config.active_room());
        self.files = FileTransferManager::new();
        self.mesh.clear();
        self.pending_secure_messages.clear();
        self.pending_route_requests.clear();
        self.ui_state = UiState::new(&self.node_id, &self.callsign, self.rooms.current_room());
        self.ui_state.identity_fingerprint = self.identity.short_fingerprint();
        self.ui_state.status = "DEMO".into();
        self.ui_state
            .push_log("demo state reset; use /demo-peers to seed the scene");
        self.sync_peers_to_ui();
        self.sync_security_to_ui();
        self.sync_files_to_ui();
        self.sync_mesh_to_ui();
    }

    pub(super) fn demo_seed_peers(&mut self, count: usize) {
        if !self.ensure_demo_mode() {
            return;
        }

        let room = self.rooms.current_room().to_string();
        for index in 0..count.max(1) {
            let node_id = demo_node_id(index);
            let callsign = demo_callsign(index).to_string();
            self.demo_register_peer(&node_id, &callsign, &room, PresenceStatus::Online);
        }
        self.system_message(format!("demo peers ready: {}", count.max(1)));
    }

    pub(super) fn demo_seed_messages(&mut self, room: &str, count: usize) {
        if !self.ensure_demo_mode() {
            return;
        }

        if self.peers.online_count() == 0 {
            self.demo_seed_peers(3);
        }

        let peers = self.peers.snapshots();
        for index in 0..count.max(1) {
            let peer = &peers[index % peers.len()];
            let packet = Packet::room_message(
                peer.node_id.clone(),
                peer.callsign.clone(),
                room.to_string(),
                demo_room_message(index),
            );
            self.demo_route_packet(packet);
        }
        self.system_message(format!("demo traffic generated for #{room}"));
    }

    pub(super) fn demo_mesh_route(&mut self) {
        if !self.ensure_demo_mode() {
            return;
        }

        if self.peers.online_count() < 2 {
            self.demo_seed_peers(3);
        }

        let peers = self.peers.snapshots();
        let relay = &peers[0];
        let destination = &peers[1];

        let entry = RouteEntry::from_spec(RouteEntrySpec {
            destination_node: destination.node_id.clone(),
            destination_callsign: Some(destination.callsign.clone()),
            next_hop: relay.node_id.clone(),
            hop_count: 2,
            trusted: true,
            encrypted_capable: true,
            source: RouteSource::Manual,
            latency_ms: Some(12),
        })
        .with_expiry(self.mesh.policy.route_expiry_ms());
        self.mesh.observe_route(entry);

        let base = MeshEnvelope::new(
            &relay.node_id,
            &destination.node_id,
            &relay.node_id,
            Some(self.node_id.clone()),
            self.mesh.policy.max_ttl,
            Packet::direct_message(
                relay.node_id.clone(),
                relay.callsign.clone(),
                destination.node_id.clone(),
                "[SECURE][MESH: 2 hops] teste",
            ),
        );
        let relayed = base
            .relay(&self.node_id, Some(destination.node_id.clone()))
            .unwrap_or(base.clone());
        let delivered = relayed
            .relay(&destination.node_id, None)
            .unwrap_or(relayed.clone());

        self.mesh.mark_relayed(&relayed);
        self.mesh.mark_delivered(&delivered);
        self.publish(KayaEvent::RouteDiscovered {
            destination_node: destination.node_id.clone(),
            next_hop: relay.node_id.clone(),
            hop_count: 2,
        });
        self.publish(KayaEvent::MeshPacketRelayed {
            mesh_packet_id: relayed.mesh_packet_id.clone(),
            destination_node: destination.node_id.clone(),
            next_hop: relay.node_id.clone(),
            hop_count: 2,
        });
        self.publish(KayaEvent::MeshPacketDelivered {
            mesh_packet_id: delivered.mesh_packet_id,
            source_node: relay.node_id.clone(),
            route_trace: delivered.route_trace.clone(),
        });
        self.publish(KayaEvent::EncryptedMessageReceived {
            from_node: relay.node_id.clone(),
            from_callsign: relay.callsign.clone(),
            target_node: self.node_id.clone(),
            body: "[SECURE][MESH: 2 hops] Ana -> Helio: teste".into(),
            local: false,
        });
        self.sync_mesh_to_ui();
    }

    pub(super) fn demo_file_offer(&mut self) {
        if !self.ensure_demo_mode() {
            return;
        }

        if self.peers.online_count() == 0 {
            self.demo_seed_peers(2);
        }

        let Some(peer) = self.peers.snapshots().into_iter().next() else {
            self.system_message("demo file offer requires at least one demo peer");
            return;
        };
        let metadata = match FileMetadata::from_bytes(
            "briefing-demo.pdf",
            b"KAYA demo packet: encrypted file offer",
            Some("application/pdf".into()),
            &peer.node_id,
            &peer.callsign,
            &self.file_config,
        ) {
            Ok(metadata) => metadata,
            Err(err) => {
                self.system_message(err.to_string());
                return;
            }
        };
        let trusted = self.trust_store.status(&peer.node_id) == kaya_security::TrustStatus::Trusted;
        let session = self
            .files
            .receive_offer(
                metadata.clone(),
                &peer.node_id,
                &peer.callsign,
                TransferSecurity::Encrypted,
                true,
                trusted,
            )
            .clone();
        self.persist_file_session(&session.file_id);
        self.publish(KayaEvent::FileOfferReceived {
            file_id: session.file_id,
            file_name: session.metadata.file_name,
            from_node: peer.node_id,
            from_callsign: peer.callsign,
            size_bytes: session.metadata.file_size,
            encrypted: true,
        });
    }

    pub(super) fn demo_security_warning(&mut self) {
        if !self.ensure_demo_mode() {
            return;
        }

        if self.peers.online_count() == 0 {
            self.demo_seed_peers(1);
        }

        let Some(peer) = self.peers.snapshots().into_iter().next() else {
            self.system_message("demo security warning requires at least one demo peer");
            return;
        };
        self.publish(KayaEvent::SecurityWarning {
            node_id: Some(peer.node_id.clone()),
            message: format!(
                "fingerprint changed for {} {}; review trust before sending secure data",
                peer.callsign, peer.node_id
            ),
        });
        self.system_message(format!(
            "trust warning: {} {} fingerprint needs operator confirmation",
            peer.callsign, peer.node_id
        ));
    }

    fn ensure_demo_mode(&mut self) -> bool {
        if self.demo_mode {
            return true;
        }
        self.system_message("demo commands require startup with --demo or --profile demo");
        false
    }

    fn demo_register_peer(
        &mut self,
        node_id: &str,
        callsign: &str,
        room: &str,
        presence: PresenceStatus,
    ) {
        let packet = Packet::heartbeat(
            node_id.to_string(),
            callsign.to_string(),
            room.to_string(),
            presence,
        );
        self.observe_peer(&packet);
        self.remember_peer(&packet);
        let _ = self
            .trust_store
            .record_seen(node_id, callsign, &demo_fingerprint(node_id));
        let outcome = self.rooms.route_packet(&packet);
        self.apply_demo_route_outcome(outcome);
        self.sync_peers_to_ui();
    }

    fn demo_route_packet(&mut self, packet: Packet) {
        self.observe_peer(&packet);
        self.remember_peer(&packet);
        let outcome = self.rooms.route_packet(&packet);
        self.apply_demo_route_outcome(outcome);
        self.sync_peers_to_ui();
    }

    fn apply_demo_route_outcome(&mut self, outcome: RouteOutcome) {
        match outcome {
            RouteOutcome::RoomMessage(message) => self.publish_room_message(message),
            RouteOutcome::DirectMessage(message) => {
                self.publish(KayaEvent::DirectMessageReceived {
                    from_node: message.from_node,
                    from_callsign: message.from_callsign,
                    target_node: message.target_node.unwrap_or_else(|| self.node_id.clone()),
                    body: message.body,
                    local: false,
                });
            }
            RouteOutcome::Joined {
                node_id,
                callsign,
                room,
            } => self.apply_room_joined(node_id, callsign, room, false),
            RouteOutcome::RoomCreated {
                node_id,
                callsign,
                room,
            } => self.publish(KayaEvent::RoomCreated {
                node_id,
                callsign,
                room,
                local: false,
            }),
            RouteOutcome::Left { node_id, room } => {
                self.publish(KayaEvent::RoomLeft { node_id, room });
            }
            RouteOutcome::MembersRequested { .. }
            | RouteOutcome::MembersResponse { .. }
            | RouteOutcome::Ignored => {}
        }
    }
}

fn demo_node_id(index: usize) -> String {
    format!("KY-D{:05X}", index + 1)
}

fn demo_callsign(index: usize) -> &'static str {
    const CALLSIGNS: &[&str] = &[
        "Ana", "Bruno", "Carla", "Davi", "Eva", "Fabio", "Gaia", "Iris",
    ];
    CALLSIGNS[index % CALLSIGNS.len()]
}

fn demo_fingerprint(node_id: &str) -> String {
    let suffix = node_id.strip_prefix("KY-").unwrap_or(node_id);
    format!("KAYA-FP: DE{}-MO{}-01", &suffix[0..2], &suffix[2..4])
}

fn demo_room_message(index: usize) -> String {
    const BODIES: &[&str] = &[
        "alguem recebe?",
        "recebido com baixa latencia",
        "painel tecnico atualizado",
        "mesh route estabilizada",
        "secure dm pronto para demo",
    ];
    BODIES[index % BODIES.len()].to_string()
}
