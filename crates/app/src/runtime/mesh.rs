use super::Runtime;
use kaya_events::KayaEvent;
use kaya_mesh::{
    decide_relay, MeshEnvelope, RelayDecision, RelayDropReason, RouteEntry, RouteEntrySpec,
    RouteSource,
};
use kaya_protocol::{Packet, PacketType, RouteDescriptorPayload, RouteResponsePayload};
use kaya_security::{sign_packet, TrustStatus};
use kaya_shared::{is_valid_node_id, KayaError};
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub(super) struct MeshTarget {
    pub node_id: String,
    pub callsign: String,
}

impl Runtime {
    pub(super) fn route_announce_packet(&self) -> Packet {
        Packet::route_announce(
            self.node_id.clone(),
            self.callsign.clone(),
            self.mesh_route_descriptors(),
        )
    }

    pub(super) async fn send_packet_routed(&mut self, mut packet: Packet, destination_node: &str) {
        if self.direct_peer_online(destination_node) {
            self.send_packet(packet).await;
            return;
        }

        let route = self.mesh.best_route(destination_node).cloned();

        if route.is_none() {
            if self.trust_store.is_blocked(destination_node) {
                self.publish(KayaEvent::RelayDenied {
                    source_node: self.node_id.clone(),
                    destination_node: destination_node.to_string(),
                    reason: "blocked peer in relay path".into(),
                });
                return;
            }
            if let Err(err) = sign_packet(&mut packet, &self.identity) {
                self.publish(KayaEvent::SecurityWarning {
                    node_id: Some(self.node_id.clone()),
                    message: format!("outgoing relay packet signing failed: {err}"),
                });
                return;
            }
            if self.send_packet_via_relay(destination_node, packet.room.clone(), &packet) {
                return;
            }
            self.send_route_request(destination_node).await;
            self.system_message(format!(
                "no mesh route to {destination_node}; route request sent"
            ));
            return;
        }

        let route = route.expect("route checked above");

        if self.trust_store.is_blocked(&route.next_hop)
            || self.trust_store.is_blocked(destination_node)
        {
            self.publish(KayaEvent::RelayDenied {
                source_node: self.node_id.clone(),
                destination_node: destination_node.to_string(),
                reason: "blocked peer in route".into(),
            });
            return;
        }

        if let Err(err) = sign_packet(&mut packet, &self.identity) {
            self.publish(KayaEvent::SecurityWarning {
                node_id: Some(self.node_id.clone()),
                message: format!("inner mesh packet signing failed: {err}"),
            });
            return;
        }

        let envelope = self
            .mesh
            .build_envelope(destination_node, &route.next_hop, packet);
        let Ok(payload) = envelope.to_value() else {
            self.publish(KayaEvent::RouteError {
                destination_node: destination_node.to_string(),
                reason: "failed to encode mesh envelope".into(),
            });
            return;
        };
        self.send_packet(Packet::mesh_relay(
            self.node_id.clone(),
            self.callsign.clone(),
            route.next_hop,
            payload,
        ))
        .await;
    }

    pub(super) async fn send_route_request(&mut self, destination_node: &str) {
        if !self.mesh.policy.enabled || !is_valid_node_id(destination_node) {
            return;
        }
        let now = kaya_shared::now_millis();
        if self
            .pending_route_requests
            .get(destination_node)
            .map(|requested_at| {
                now.saturating_sub(*requested_at) < self.timeouts.route_discovery_ms
            })
            .unwrap_or(false)
        {
            return;
        }
        let request_id = Uuid::new_v4().to_string();
        self.pending_route_requests
            .insert(destination_node.to_string(), now);
        self.publish(KayaEvent::RouteRequestSent {
            destination_node: destination_node.to_string(),
            request_id: request_id.clone(),
        });
        self.send_packet(Packet::route_request(
            self.node_id.clone(),
            self.callsign.clone(),
            request_id,
            destination_node.to_string(),
            self.mesh.policy.max_ttl,
        ))
        .await;
    }

    pub(super) async fn route_mesh_packet(&mut self, packet: &Packet) -> bool {
        match packet.packet_type {
            PacketType::RouteAnnounce => {
                self.receive_route_announce(packet);
                true
            }
            PacketType::RouteRequest => {
                self.receive_route_request(packet).await;
                true
            }
            PacketType::RouteResponse => {
                self.receive_route_response(packet);
                true
            }
            PacketType::MeshRelay => {
                self.receive_mesh_relay(packet).await;
                true
            }
            PacketType::RouteError => {
                let destination = payload_str(packet, "destination_node").unwrap_or("--");
                let reason = payload_str(packet, "reason").unwrap_or("route error");
                self.publish(KayaEvent::RouteError {
                    destination_node: destination.to_string(),
                    reason: reason.to_string(),
                });
                true
            }
            PacketType::RoutePing => {
                if self.packet_targets_local_node(packet) {
                    self.send_packet_routed(
                        Packet::route_pong(
                            self.node_id.clone(),
                            self.callsign.clone(),
                            packet.node_id.clone(),
                            packet.packet_id,
                        ),
                        &packet.node_id,
                    )
                    .await;
                }
                true
            }
            PacketType::RoutePong => true,
            _ => false,
        }
    }

    pub(super) fn resolve_mesh_target(&self, target: &str) -> Option<MeshTarget> {
        let route = self.mesh.best_route(target)?;
        Some(MeshTarget {
            node_id: route.destination_node.clone(),
            callsign: route
                .destination_callsign
                .clone()
                .unwrap_or_else(|| route.destination_node.clone()),
        })
    }

    pub(super) fn show_routes(&mut self) {
        let entries = self.mesh.table.entries();
        if entries.is_empty() {
            self.system_message("mesh routes: none");
            return;
        }
        let summary = entries
            .into_iter()
            .take(8)
            .map(|route| {
                format!(
                    "{} via {} hops={} score={} source={:?}",
                    route.destination_callsign.unwrap_or(route.destination_node),
                    route.next_hop,
                    route.hop_count,
                    route.score,
                    route.source
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(format!("mesh routes: {summary}"));
    }

    pub(super) fn show_route(&mut self, target: &str) {
        let Some(route) = self.mesh.best_route(target).cloned() else {
            self.system_message(format!("no mesh route to {target}"));
            return;
        };
        self.system_message(format!(
            "route {} via {} hops={} score={} trusted={} encrypted_capable={} source={:?}",
            route.destination_node,
            route.next_hop,
            route.hop_count,
            route.score,
            route.trusted,
            route.encrypted_capable,
            route.source
        ));
    }

    pub(super) fn show_mesh_status(&mut self) {
        let diagnostics = self.mesh.diagnostics_snapshot();
        self.system_message(format!(
            "mesh enabled={} routes={} relayed={} delivered={} dropped={} avg_hops={} last={}",
            diagnostics.enabled,
            diagnostics.routes,
            diagnostics.relayed_packets,
            diagnostics.delivered_packets,
            diagnostics.dropped_packets,
            diagnostics.avg_hop_count(),
            diagnostics
                .last_route_discovered
                .unwrap_or_else(|| "--".into())
        ));
    }

    pub(super) fn clear_mesh(&mut self) {
        self.mesh.clear();
        self.sync_mesh_to_ui();
        self.system_message("mesh routing table cleared");
    }

    pub(super) fn observe_mesh_peer(&mut self, packet: &Packet) {
        if packet.node_id == self.node_id || self.trust_store.is_blocked(&packet.node_id) {
            return;
        }
        let trusted = self.trust_store.status(&packet.node_id) == TrustStatus::Trusted;
        let encrypted_capable = self.sessions.has_active(&packet.node_id);
        let latency_ms = self
            .peers
            .get(&packet.node_id)
            .and_then(|peer| peer.latency_ms);
        self.mesh.observe_direct_peer(
            &packet.node_id,
            &packet.callsign,
            trusted,
            encrypted_capable,
            latency_ms,
        );
        self.sync_mesh_to_ui();
    }

    fn receive_route_announce(&mut self, packet: &Packet) {
        if !self.mesh.policy.enabled || self.trust_store.is_blocked(&packet.node_id) {
            return;
        }
        let Some(routes) = packet
            .payload
            .get("routes")
            .and_then(serde_json::Value::as_array)
        else {
            return;
        };
        for route in routes {
            let Ok(descriptor) = serde_json::from_value::<RouteDescriptorPayload>(route.clone())
            else {
                continue;
            };
            if descriptor.destination_node == self.node_id
                || descriptor.destination_node == packet.node_id
            {
                continue;
            }
            if self.trust_store.is_blocked(&descriptor.destination_node) {
                continue;
            }
            let entry = RouteEntry::from_spec(RouteEntrySpec {
                destination_node: descriptor.destination_node,
                destination_callsign: descriptor.destination_callsign,
                next_hop: packet.node_id.clone(),
                hop_count: descriptor.hop_count.saturating_add(1),
                trusted: descriptor.trusted,
                encrypted_capable: descriptor.encrypted_capable,
                source: RouteSource::Announce,
                latency_ms: self
                    .peers
                    .get(&packet.node_id)
                    .and_then(|peer| peer.latency_ms),
            });
            self.observe_route_entry(entry);
        }
        self.sync_mesh_to_ui();
    }

    async fn receive_route_request(&mut self, packet: &Packet) {
        if !self.mesh.policy.enabled {
            return;
        }
        let Some(destination_node) = payload_str(packet, "destination_node").map(str::to_string)
        else {
            return;
        };
        let Some(request_id) = payload_str(packet, "request_id").map(str::to_string) else {
            return;
        };
        if destination_node == self.node_id {
            self.send_route_response(
                &packet.node_id,
                RouteResponsePayload {
                    request_id,
                    destination_node,
                    destination_callsign: Some(self.callsign.clone()),
                    next_hop: self.node_id.clone(),
                    hop_count: 1,
                    score: 10_000,
                    route_trace: vec![self.node_id.clone()],
                    trusted: true,
                    encrypted_capable: true,
                },
            )
            .await;
            return;
        }

        let Some(route) = self.mesh.best_route(&destination_node).cloned() else {
            return;
        };
        self.send_route_response(
            &packet.node_id,
            RouteResponsePayload {
                request_id,
                destination_node: route.destination_node,
                destination_callsign: route.destination_callsign,
                next_hop: self.node_id.clone(),
                hop_count: route.hop_count.saturating_add(1),
                score: route.score,
                route_trace: vec![self.node_id.clone(), route.next_hop],
                trusted: route.trusted,
                encrypted_capable: route.encrypted_capable,
            },
        )
        .await;
    }

    async fn send_route_response(&mut self, target_node: &str, response: RouteResponsePayload) {
        self.send_packet_routed(
            Packet::route_response(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.to_string(),
                response,
            ),
            target_node,
        )
        .await;
    }

    fn receive_route_response(&mut self, packet: &Packet) {
        if !self.packet_targets_local_node(packet) || self.trust_store.is_blocked(&packet.node_id) {
            return;
        }
        let Ok(response) = serde_json::from_value::<RouteResponsePayload>(packet.payload.clone())
        else {
            return;
        };
        if response.destination_node == self.node_id {
            return;
        }
        self.pending_route_requests
            .remove(&response.destination_node);
        let entry = RouteEntry::from_spec(RouteEntrySpec {
            destination_node: response.destination_node.clone(),
            destination_callsign: response.destination_callsign,
            next_hop: packet.node_id.clone(),
            hop_count: response.hop_count.max(1),
            trusted: response.trusted,
            encrypted_capable: response.encrypted_capable,
            source: RouteSource::Response,
            latency_ms: self
                .peers
                .get(&packet.node_id)
                .and_then(|peer| peer.latency_ms),
        });
        self.observe_route_entry(entry);
        self.publish(KayaEvent::RouteResponseReceived {
            destination_node: response.destination_node,
            next_hop: packet.node_id.clone(),
            hop_count: response.hop_count,
        });
        self.sync_mesh_to_ui();
    }

    async fn receive_mesh_relay(&mut self, packet: &Packet) {
        let envelope = match MeshEnvelope::decode(packet.payload.clone()) {
            Ok(envelope) => envelope,
            Err(err) => {
                self.publish(KayaEvent::MeshPacketDropped {
                    mesh_packet_id: "unknown".into(),
                    reason: err.to_string(),
                });
                return;
            }
        };

        if !self.mesh.accept_seen(&envelope.mesh_packet_id) {
            self.drop_mesh_packet(&envelope, RelayDropReason::Duplicate);
            return;
        }

        if self.trust_store.is_blocked(&envelope.source_node) {
            self.drop_mesh_packet(&envelope, RelayDropReason::BlockedPeer);
            return;
        }

        if envelope.destination_node == self.node_id {
            self.mesh.mark_delivered(&envelope);
            self.publish(KayaEvent::MeshPacketDelivered {
                mesh_packet_id: envelope.mesh_packet_id.clone(),
                source_node: envelope.source_node.clone(),
                route_trace: envelope.route_trace.clone(),
            });
            self.learn_route_from_trace(&envelope, &packet.node_id);
            self.sync_mesh_to_ui();
            self.handle_mesh_inner_packet(*envelope.inner_packet).await;
            return;
        }

        let next_hop = self
            .mesh
            .best_route(&envelope.destination_node)
            .map(|route| route.next_hop.as_str());
        let blocked = self.trust_store.is_blocked(&packet.node_id)
            || self.trust_store.is_blocked(&envelope.destination_node);
        match decide_relay(
            &envelope,
            &self.node_id,
            &self.mesh.policy,
            blocked,
            next_hop,
        ) {
            RelayDecision::Deliver => {
                self.mesh.mark_delivered(&envelope);
                self.handle_mesh_inner_packet(*envelope.inner_packet).await;
            }
            RelayDecision::Relay { next_hop } => {
                match envelope.relay(&self.node_id, Some(next_hop.clone())) {
                    Ok(relayed) => {
                        self.mesh.mark_relayed(&relayed);
                        self.publish(KayaEvent::MeshPacketRelayed {
                            mesh_packet_id: relayed.mesh_packet_id.clone(),
                            destination_node: relayed.destination_node.clone(),
                            next_hop: next_hop.clone(),
                            hop_count: relayed.hop_count,
                        });
                        let Ok(payload) = relayed.to_value() else {
                            self.publish(KayaEvent::RouteError {
                                destination_node: envelope.destination_node,
                                reason: "failed to encode relayed envelope".into(),
                            });
                            return;
                        };
                        self.send_packet(Packet::mesh_relay(
                            self.node_id.clone(),
                            self.callsign.clone(),
                            next_hop,
                            payload,
                        ))
                        .await;
                    }
                    Err(err) => {
                        warn!(%err, "mesh relay failed");
                        self.drop_mesh_packet(&envelope, RelayDropReason::TtlExpired);
                    }
                }
            }
            RelayDecision::Drop(reason) => {
                self.drop_mesh_packet(&envelope, reason);
            }
        }
        self.sync_mesh_to_ui();
    }

    async fn handle_mesh_inner_packet(&mut self, packet: Packet) {
        if packet.node_id == self.node_id {
            return;
        }
        if !self.inspect_packet_security(&packet) {
            return;
        }
        self.remember_peer(&packet);
        self.sync_security_to_ui();
        if self.route_security_packet(&packet).await {
            return;
        }
        if let Err(err) = self.validate_mesh_packet_is_control_only(&packet) {
            self.publish(KayaEvent::FileTransferFailed {
                file_id: packet
                    .payload
                    .get("file_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                reason: err.to_string(),
            });
            return;
        }
        if self.route_file_packet(&packet).await {
            return;
        }
        self.route_packet(&packet).await;
    }

    fn mesh_route_descriptors(&self) -> Vec<RouteDescriptorPayload> {
        let mut routes: Vec<RouteDescriptorPayload> = self
            .peers
            .snapshots()
            .into_iter()
            .filter(|peer| peer.online && !self.trust_store.is_blocked(&peer.node_id))
            .map(|peer| RouteDescriptorPayload {
                destination_node: peer.node_id.clone(),
                destination_callsign: Some(peer.callsign.clone()),
                hop_count: 1,
                score: 10_000,
                trusted: self.trust_store.status(&peer.node_id) == TrustStatus::Trusted,
                encrypted_capable: self.sessions.has_active(&peer.node_id),
            })
            .collect();

        routes.extend(
            self.mesh
                .table
                .entries()
                .into_iter()
                .filter(|route| {
                    route.destination_node != self.node_id
                        && !self.trust_store.is_blocked(&route.destination_node)
                })
                .map(|route| RouteDescriptorPayload {
                    destination_node: route.destination_node,
                    destination_callsign: route.destination_callsign,
                    hop_count: route.hop_count,
                    score: route.score,
                    trusted: route.trusted,
                    encrypted_capable: route.encrypted_capable,
                }),
        );
        routes
    }

    fn direct_peer_online(&self, node_id: &str) -> bool {
        self.peers
            .get(node_id)
            .map(|peer| peer.online)
            .unwrap_or(false)
    }

    fn observe_route_entry(&mut self, entry: RouteEntry) {
        let destination_node = entry.destination_node.clone();
        let next_hop = entry.next_hop.clone();
        let hop_count = entry.hop_count;
        self.mesh.observe_route(entry);
        self.publish(KayaEvent::RouteDiscovered {
            destination_node,
            next_hop,
            hop_count,
        });
    }

    fn learn_route_from_trace(&mut self, envelope: &MeshEnvelope, previous_hop: &str) {
        if envelope.source_node == self.node_id
            || self.trust_store.is_blocked(&envelope.source_node)
        {
            return;
        }
        let callsign = envelope.inner_packet.callsign.clone();
        let entry = RouteEntry::from_spec(RouteEntrySpec {
            destination_node: envelope.source_node.clone(),
            destination_callsign: Some(callsign),
            next_hop: previous_hop.to_string(),
            hop_count: envelope.hop_count.max(1),
            trusted: self.trust_store.status(&envelope.source_node) == TrustStatus::Trusted,
            encrypted_capable: envelope.inner_packet.packet_type
                == PacketType::DirectMessageEncrypted,
            source: RouteSource::RelayTrace,
            latency_ms: self
                .peers
                .get(previous_hop)
                .and_then(|peer| peer.latency_ms),
        });
        self.observe_route_entry(entry);
    }

    fn drop_mesh_packet(&mut self, envelope: &MeshEnvelope, reason: RelayDropReason) {
        self.mesh.mark_dropped(reason);
        let reason_text = format!("{reason:?}");
        self.publish(KayaEvent::MeshPacketDropped {
            mesh_packet_id: envelope.mesh_packet_id.clone(),
            reason: reason_text.clone(),
        });
        self.publish(KayaEvent::RelayDenied {
            source_node: envelope.source_node.clone(),
            destination_node: envelope.destination_node.clone(),
            reason: reason_text,
        });
        debug!(
            mesh_packet_id = %envelope.mesh_packet_id,
            source_node = %envelope.source_node,
            destination_node = %envelope.destination_node,
            "mesh packet dropped"
        );
    }

    pub(super) fn validate_mesh_packet_is_control_only(
        &self,
        packet: &Packet,
    ) -> Result<(), KayaError> {
        match packet.packet_type {
            PacketType::FileChunk | PacketType::FileChunkEncrypted | PacketType::FileChunkAck => {
                Err(KayaError::Transport(
                    "file chunks over mesh not enabled yet".into(),
                ))
            }
            _ => Ok(()),
        }
    }
}

fn payload_str<'a>(packet: &'a Packet, field: &str) -> Option<&'a str> {
    packet
        .payload
        .get(field)
        .and_then(serde_json::Value::as_str)
}
