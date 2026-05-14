use super::Runtime;
use kaya_events::KayaEvent;
use kaya_protocol::{Packet, PacketType};
use kaya_relay::{RelayClient, RelayPolicy, RelayRegistration};
use kaya_security::sign_packet;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tracing::{info_span, warn, Instrument};

impl Runtime {
    pub(super) fn spawn_network_reader(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
        network_recv_ms: u64,
    ) -> JoinHandle<()> {
        let transport = self.transport.clone();
        let bus = self.bus.clone();
        tokio::spawn(async move {
            let multicast_addr = transport.multicast_addr().to_string();
            let _ = bus.publish(KayaEvent::NetworkStarted { multicast_addr });
            let recv_timeout = Duration::from_millis(network_recv_ms);

            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_ok() && *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    received = time::timeout(recv_timeout, transport.recv_packet()) => {
                        match received {
                            Ok(Ok((packet, addr, bytes))) => {
                                let _ = bus.publish(KayaEvent::PacketReceived {
                                    packet,
                                    source: addr.to_string(),
                                    bytes,
                                });
                            }
                            Ok(Err(err)) => {
                                let _ = bus.publish(KayaEvent::ErrorOccurred {
                                    scope: "transport.rx".into(),
                                    message: err.to_string(),
                                });
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        })
    }

    pub(super) async fn connect_relay(&mut self, shutdown_rx: watch::Receiver<bool>) {
        if !self.config.relay.enabled {
            return;
        }

        let Some(url) = self
            .config
            .relay
            .url
            .as_ref()
            .filter(|url| !url.trim().is_empty())
            .cloned()
        else {
            self.system_message("relay enabled but relay.url is empty");
            return;
        };

        match RelayClient::connect(
            &url,
            RelayRegistration {
                node_id: self.node_id.clone(),
                callsign: self.callsign.clone(),
                fingerprint: self.identity.fingerprint.clone(),
                capabilities: vec!["rooms".into(), "dm".into(), "mesh".into(), "files".into()],
            },
            RelayPolicy {
                allow_unknown: self.file_config.accept_from_unknown,
                max_clients: 100,
                heartbeat_interval_ms: self.config.relay.heartbeat_interval_ms,
                connection_timeout_ms: self.config.relay.connection_timeout_ms,
                rooms: kaya_relay::RelayRoomPolicy {
                    enabled: self.config.relay.rooms.enabled,
                    broadcast: self.config.relay.rooms.broadcast,
                },
                file_transfer: kaya_relay::RelayFileTransferPolicy {
                    enabled: self.config.relay.file_transfer.enabled,
                    allow_chunks: self.config.relay.file_transfer.allow_chunks,
                    max_file_size_mb: self.config.relay.file_transfer.max_file_size_mb,
                },
            },
        )
        .await
        {
            Ok(client) => {
                self.relay_tx = Some(client.sender());
                self.relay_task = Some(self.spawn_relay_reader(client, shutdown_rx, url.clone()));
                self.system_message(format!("relay dialing {url}"));
            }
            Err(err) => {
                self.system_message(format!("relay connection failed: {err}"));
            }
        }
    }

    fn spawn_relay_reader(
        &self,
        mut client: RelayClient,
        mut shutdown_rx: watch::Receiver<bool>,
        relay_url: String,
    ) -> JoinHandle<()> {
        let bus = self.bus.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_ok() && *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    received = client.recv() => {
                        match received {
                            Some(packet) => {
                                let bytes = serde_json::to_vec(&packet)
                                    .map(|buffer| buffer.len())
                                    .unwrap_or_default();
                                let _ = bus.publish(KayaEvent::PacketReceived {
                                    packet,
                                    source: format!("relay:{relay_url}"),
                                    bytes,
                                });
                            }
                            None => {
                                let _ = bus.publish(KayaEvent::ErrorOccurred {
                                    scope: "relay.rx".into(),
                                    message: format!("relay stream closed {relay_url}"),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        })
    }

    pub(super) async fn bootstrap(&mut self) {
        self.ui_state.push_log(format!(
            "node {} initialized as {}",
            self.node_id, self.callsign
        ));
        if self.identity_created {
            self.publish(KayaEvent::IdentityCreated {
                node_id: self.node_id.clone(),
                fingerprint: self.identity.fingerprint.clone(),
            });
        } else {
            self.publish(KayaEvent::IdentityLoaded {
                node_id: self.node_id.clone(),
                fingerprint: self.identity.fingerprint.clone(),
            });
        }
        self.publish(KayaEvent::RoomJoined {
            node_id: self.node_id.clone(),
            callsign: self.callsign.clone(),
            room: self.rooms.current_room().to_string(),
            local: true,
        });
        self.send_packet(Packet::hello(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
        self.send_packet(Packet::presence_update(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
            self.presence,
        ))
        .await;
        self.send_packet(Packet::room_announce(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
        self.send_packet(Packet::room_join(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
        self.send_packet(Packet::room_members_request(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
        self.send_packet(self.route_announce_packet()).await;
    }

    pub(super) async fn send_packet(&mut self, mut packet: Packet) {
        if let Err(err) = sign_packet(&mut packet, &self.identity) {
            self.publish(KayaEvent::SecurityWarning {
                node_id: Some(self.node_id.clone()),
                message: format!("outgoing packet signing failed: {err}"),
            });
            return;
        }
        let packet_id = packet.packet_id;
        let packet_type = packet.packet_type;
        let span = info_span!("packet.send", %packet_id, packet_type = ?packet_type);

        async {
            match time::timeout(
                Duration::from_millis(self.timeouts.packet_send_ms),
                self.transport.send_packet(&packet),
            )
            .await
            {
                Ok(Ok(bytes)) => {
                    self.publish(KayaEvent::PacketSent {
                        packet_id,
                        packet_type,
                        bytes,
                    });
                }
                Ok(Err(err)) => {
                    warn!(%err, "packet send failed");
                    self.publish(KayaEvent::ErrorOccurred {
                        scope: "transport.tx".into(),
                        message: err.to_string(),
                    });
                }
                Err(_) => {
                    self.publish(KayaEvent::ErrorOccurred {
                        scope: "transport.tx".into(),
                        message: format!(
                            "packet send timed out after {}ms",
                            self.timeouts.packet_send_ms
                        ),
                    });
                }
            }

            self.mirror_packet_to_relay(&packet);
        }
        .instrument(span)
        .await;
    }

    pub(super) fn send_packet_via_relay(
        &mut self,
        destination_node: &str,
        room: Option<String>,
        packet: &Packet,
    ) -> bool {
        let Some(relay_tx) = &self.relay_tx else {
            return false;
        };

        let Ok(inner_packet) = serde_json::to_value(packet) else {
            self.publish(KayaEvent::ErrorOccurred {
                scope: "relay.tx".into(),
                message: "failed to encode relay payload".into(),
            });
            return false;
        };

        relay_tx
            .send(Packet::relay_forward(
                self.node_id.clone(),
                self.callsign.clone(),
                destination_node.to_string(),
                room,
                inner_packet,
            ))
            .map_err(|err| {
                self.publish(KayaEvent::ErrorOccurred {
                    scope: "relay.tx".into(),
                    message: err.to_string(),
                });
            })
            .is_ok()
    }

    fn mirror_packet_to_relay(&mut self, packet: &Packet) {
        if !self.config.relay.enabled
            || !self.config.relay.rooms.enabled
            || !self.config.relay.rooms.bridge_local
        {
            return;
        }

        if !matches!(
            packet.packet_type,
            PacketType::Hello
                | PacketType::Heartbeat
                | PacketType::Leave
                | PacketType::JoinRoom
                | PacketType::RoomAnnounce
                | PacketType::RoomJoin
                | PacketType::RoomLeave
                | PacketType::RoomMembersRequest
                | PacketType::RoomMembersResponse
                | PacketType::PresenceUpdate
                | PacketType::RoomMessage
        ) {
            return;
        }

        let _ = self.send_packet_via_relay("*", packet.room.clone(), packet);
    }

    pub(super) fn resolve_relay_target(&self, target: &str) -> Option<(String, String)> {
        if let Some(peer) = self.relay_peers.get(target) {
            return Some((peer.node_id.clone(), peer.callsign.clone()));
        }
        if kaya_shared::is_valid_node_id(target) && self.relay_tx.is_some() {
            return Some((target.to_string(), target.to_string()));
        }
        let mut matches = self
            .relay_peers
            .values()
            .filter(|peer| peer.callsign.eq_ignore_ascii_case(target));
        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some((first.node_id.clone(), first.callsign.clone()))
    }

    pub(super) fn publish_peer_event(&mut self, event: kaya_peer::PeerEvent) {
        match event {
            kaya_peer::PeerEvent::Discovered(node_id) => {
                let callsign = self
                    .peers
                    .get(&node_id)
                    .map(|peer| peer.callsign.clone())
                    .unwrap_or_default();
                self.publish(KayaEvent::PeerDiscovered { node_id, callsign });
            }
            kaya_peer::PeerEvent::TimedOut(node_id) => {
                self.publish(KayaEvent::PeerTimedOut { node_id });
            }
            kaya_peer::PeerEvent::Left(node_id) => self.publish(KayaEvent::RoomLeft {
                node_id,
                room: None,
            }),
            kaya_peer::PeerEvent::PresenceChanged(node_id, presence) => {
                let callsign = self
                    .peers
                    .get(&node_id)
                    .map(|peer| peer.callsign.clone())
                    .unwrap_or_default();
                self.publish(KayaEvent::PresenceUpdated {
                    node_id,
                    callsign,
                    presence,
                });
            }
            kaya_peer::PeerEvent::Updated(_) => {}
        }
    }

    pub(super) fn heartbeat_packet(&self) -> Packet {
        Packet::heartbeat(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
            self.presence,
        )
    }

    pub(super) fn leave_packet(&self) -> Packet {
        Packet::leave(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        )
    }

    pub(super) fn state_sync_for(&self, packet: &Packet) -> Vec<Packet> {
        if packet.packet_type != PacketType::Hello {
            return Vec::new();
        }

        let mut packets = vec![
            Packet::pong(
                self.node_id.clone(),
                self.callsign.clone(),
                packet.node_id.clone(),
                packet.packet_id,
            ),
            Packet::presence_update(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
                self.presence,
            ),
            Packet::room_join(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
            ),
            self.route_announce_packet(),
        ];

        for room in self.rooms.summaries() {
            let room_name = room.name.clone();
            packets.push(Packet::room_announce(
                self.node_id.clone(),
                self.callsign.clone(),
                room_name.clone(),
            ));
            if room.local_joined {
                packets.push(Packet::room_members_response(
                    self.node_id.clone(),
                    self.callsign.clone(),
                    room_name.clone(),
                    self.rooms.members(&room_name),
                ));
            }
        }

        packets
    }
}
