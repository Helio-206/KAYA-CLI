use super::Runtime;
use kaya_events::KayaEvent;
use kaya_protocol::{Packet, PacketType};
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
        self.send_packet(Packet::join_room(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
    }

    pub(super) async fn send_packet(&mut self, packet: Packet) {
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
            kaya_peer::PeerEvent::Updated(_) => {}
        }
    }

    pub(super) fn heartbeat_packet(&self) -> Packet {
        Packet::heartbeat(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        )
    }

    pub(super) fn leave_packet(&self) -> Packet {
        Packet::leave(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        )
    }

    pub(super) fn pong_for(&self, packet: &Packet) -> Option<Packet> {
        (packet.packet_type == PacketType::Hello).then(|| {
            Packet::pong(
                self.node_id.clone(),
                self.callsign.clone(),
                packet.node_id.clone(),
                packet.packet_id,
            )
        })
    }
}
