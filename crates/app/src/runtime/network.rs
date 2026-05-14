use super::Runtime;
use kaya_events::KayaEvent;
use kaya_protocol::{Packet, PacketType};
use kaya_security::sign_packet;
use tokio::task::JoinHandle;
use tracing::{info_span, warn, Instrument};

impl Runtime {
    pub(super) fn spawn_network_reader(&self) -> JoinHandle<()> {
        let transport = self.transport.clone();
        let bus = self.bus.clone();
        tokio::spawn(async move {
            let multicast_addr = transport.multicast_addr().to_string();
            let _ = bus.publish(KayaEvent::NetworkStarted { multicast_addr });

            loop {
                match transport.recv_packet().await {
                    Ok((packet, addr, bytes)) => {
                        let _ = bus.publish(KayaEvent::PacketReceived {
                            packet,
                            source: addr.to_string(),
                            bytes,
                        });
                    }
                    Err(err) => {
                        let _ = bus.publish(KayaEvent::ErrorOccurred {
                            scope: "transport.rx".into(),
                            message: err.to_string(),
                        });
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
            match self.transport.send_packet(&packet).await {
                Ok(bytes) => {
                    self.publish(KayaEvent::PacketSent {
                        packet_id,
                        packet_type,
                        bytes,
                    });
                }
                Err(err) => {
                    warn!(%err, "packet send failed");
                    self.publish(KayaEvent::ErrorOccurred {
                        scope: "transport.tx".into(),
                        message: err.to_string(),
                    });
                }
            }
        }
        .instrument(span)
        .await;
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
