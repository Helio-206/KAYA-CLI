use async_trait::async_trait;
use kaya_commands::{Command, CommandRegistry, ParsedInput};
use kaya_events::KayaEvent;
use kaya_files::{
    FileChunk, FileMetadata, FileStore, FileTransferConfig, FileTransferManager,
    OutgoingFileRequest, TransferSecurity, TransferSession, TransferStatus,
};
use kaya_mesh::{
    decide_relay, MeshDiagnostics, MeshEnvelope, MeshPolicy, MeshState, RelayDecision,
    RelayDropReason, RouteEntry, RouteEntrySpec, RouteSource,
};
use kaya_peer::{PeerRegistry, PeerSnapshot, TargetResolution};
use kaya_persistence::{
    profile_data_dir, ConfigProfile, ConfigStore, KayaConfig as PersistedConfig, KnownPeer, Store,
    TimeoutSettings,
};
use kaya_protocol::{
    EncryptedDirectMessagePayload, FileChunkPayload, FileEncryptedChunkPayload, FileOfferPayload,
    Packet, PacketType, RouteDescriptorPayload, RouteResponsePayload,
};
use kaya_rooms::{ChatMessage, RoomStore, RoomSummary, RouteOutcome};
use kaya_security::{
    decode_hex, encode_hex, encrypted_payload_from_packet, packet_requires_signature_validation,
    session_accept_from_packet, session_request_from_packet, sign_packet, verify_packet_signature,
    EncryptedPayload, IdentityStore, LocalIdentity, SecureSessionManager, SecureSessionView,
    SignatureStatus, TrustObservation, TrustStatus, TrustStore,
};
use kaya_shared::{is_valid_node_id, now_millis, KayaError, PresenceStatus, Result};
use kaya_transport::{MulticastTransport, PacketDeduplicator, TransportConfig};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tokio::time;
use uuid::Uuid;

pub use kaya_commands::{Command as KayaCommand, CommandRegistry as KayaCommandRegistry};
pub use kaya_events::KayaEvent as KayaRuntimeEvent;
pub use kaya_mesh::MeshDiagnostics as KayaMeshDiagnostics;
pub use kaya_persistence::ConfigProfile as KayaProfile;
pub use kaya_rooms::ChatMessage as KayaMessage;
pub use kaya_security::TrustedPeer;

const DEFAULT_EVENT_CAPACITY: usize = 512;

#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub profile: ConfigProfile,
    pub data_dir: Option<PathBuf>,
    pub callsign: Option<String>,
    pub settings: PersistedConfig,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            profile: ConfigProfile::Default,
            data_dir: None,
            callsign: None,
            settings: PersistedConfig::default(),
        }
    }
}

impl CoreConfig {
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir
            .clone()
            .unwrap_or_else(|| profile_data_dir(self.profile))
    }
}

#[async_trait]
pub trait KayaTransport: Send + Sync {
    async fn send_packet(&self, packet: &Packet) -> Result<usize>;
    async fn recv_packet(&self) -> Result<(Packet, String, usize)>;
    fn label(&self) -> String;
}

#[derive(Clone)]
pub struct MulticastRuntimeTransport {
    inner: MulticastTransport,
}

impl MulticastRuntimeTransport {
    pub async fn bind(config: TransportConfig) -> Result<Self> {
        let inner = MulticastTransport::bind(config)
            .await
            .map_err(|err| KayaError::Transport(err.to_string()))?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl KayaTransport for MulticastRuntimeTransport {
    async fn send_packet(&self, packet: &Packet) -> Result<usize> {
        self.inner
            .send_packet(packet)
            .await
            .map_err(|err| KayaError::Transport(err.to_string()))
    }

    async fn recv_packet(&self) -> Result<(Packet, String, usize)> {
        let (packet, addr, bytes) = self
            .inner
            .recv_packet()
            .await
            .map_err(|err| KayaError::Transport(err.to_string()))?;
        Ok((packet, addr.to_string(), bytes))
    }

    fn label(&self) -> String {
        self.inner.multicast_addr().to_string()
    }
}

#[derive(Clone)]
pub struct MockTransport {
    incoming: Arc<Mutex<mpsc::UnboundedReceiver<(Packet, String, usize)>>>,
    outgoing: mpsc::UnboundedSender<Packet>,
    label: String,
}

#[derive(Clone)]
pub struct MockTransportHandle {
    incoming: mpsc::UnboundedSender<(Packet, String, usize)>,
    outgoing: Arc<Mutex<mpsc::UnboundedReceiver<Packet>>>,
}

impl MockTransport {
    pub fn pair() -> (Arc<Self>, MockTransportHandle) {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let transport = Arc::new(Self {
            incoming: Arc::new(Mutex::new(incoming_rx)),
            outgoing: outgoing_tx,
            label: "mock://kaya".into(),
        });
        let handle = MockTransportHandle {
            incoming: incoming_tx,
            outgoing: Arc::new(Mutex::new(outgoing_rx)),
        };
        (transport, handle)
    }
}

#[async_trait]
impl KayaTransport for MockTransport {
    async fn send_packet(&self, packet: &Packet) -> Result<usize> {
        self.outgoing
            .send(packet.clone())
            .map_err(|err| KayaError::ChannelClosed(err.to_string()))?;
        Ok(1)
    }

    async fn recv_packet(&self) -> Result<(Packet, String, usize)> {
        let mut incoming = self.incoming.lock().await;
        incoming
            .recv()
            .await
            .ok_or_else(|| KayaError::ChannelClosed("mock transport inbound closed".into()))
    }

    fn label(&self) -> String {
        self.label.clone()
    }
}

impl MockTransportHandle {
    pub fn inject(&self, packet: Packet) -> Result<()> {
        self.inject_from(packet, "mock://peer".into())
    }

    pub fn inject_from(&self, packet: Packet, source: String) -> Result<()> {
        self.incoming
            .send((packet, source, 1))
            .map_err(|err| KayaError::ChannelClosed(err.to_string()))
    }

    pub async fn next_sent(&self) -> Option<Packet> {
        self.outgoing.lock().await.recv().await
    }
}

#[derive(Debug, Clone)]
struct PendingSecureQueue {
    messages: Vec<String>,
}

struct CoreState {
    node_id: String,
    callsign: String,
    identity: LocalIdentity,
    identity_store: IdentityStore,
    identity_created: bool,
    peers: PeerRegistry,
    rooms: RoomStore,
    files: FileTransferManager,
    file_store: FileStore,
    file_config: FileTransferConfig,
    mesh: MeshState,
    store: Store,
    trust_store: TrustStore,
    sessions: SecureSessionManager,
    pending_secure_messages: HashMap<String, PendingSecureQueue>,
    pending_route_requests: HashMap<String, u64>,
    config_store: ConfigStore,
    config: PersistedConfig,
    timeouts: TimeoutSettings,
    commands: CommandRegistry,
    presence: PresenceStatus,
    dedup: PacketDeduplicator,
}

struct CoreBootstrap {
    identity: LocalIdentity,
    identity_created: bool,
    identity_store: IdentityStore,
    file_store: FileStore,
    store: Store,
    config_store: ConfigStore,
    config: PersistedConfig,
    profile: ConfigProfile,
    trust_store: TrustStore,
}

impl CoreState {
    fn from_bootstrap(bootstrap: CoreBootstrap) -> Self {
        let CoreBootstrap {
            identity,
            identity_created,
            identity_store,
            file_store,
            store,
            config_store,
            config,
            profile: _profile,
            trust_store,
        } = bootstrap;
        let node_id = identity.node_id.clone();
        let callsign = identity.callsign.clone();
        let mut rooms = RoomStore::new(&node_id, &callsign);
        if rooms.join(config.active_room()).is_err() {
            let _ = rooms.join(kaya_shared::DEFAULT_ROOM);
        }

        let file_config = file_transfer_config(&config);
        let mesh_policy = mesh_policy(&config);
        let mut files = FileTransferManager::new();
        if let Ok(records) = file_store.list_records() {
            for record in records {
                files.load_record(record.session);
            }
        }

        Self {
            node_id: node_id.clone(),
            callsign,
            identity: identity.clone(),
            identity_store,
            identity_created,
            peers: PeerRegistry::with_timeout(
                &node_id,
                Duration::from_secs(config.peer_timeout_secs),
            ),
            rooms,
            files,
            file_store,
            file_config,
            mesh: MeshState::new(&node_id, mesh_policy),
            store,
            trust_store,
            sessions: SecureSessionManager::new(identity),
            pending_secure_messages: HashMap::new(),
            pending_route_requests: HashMap::new(),
            config_store,
            timeouts: config.timeouts.clone(),
            config,
            commands: CommandRegistry::default(),
            presence: PresenceStatus::Online,
            dedup: PacketDeduplicator::new(4096),
        }
    }

    async fn bootstrap(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
    ) {
        let event = if self.identity_created {
            KayaEvent::IdentityCreated {
                node_id: self.node_id.clone(),
                fingerprint: self.identity.fingerprint.clone(),
            }
        } else {
            KayaEvent::IdentityLoaded {
                node_id: self.node_id.clone(),
                fingerprint: self.identity.fingerprint.clone(),
            }
        };
        publish(events, event);
        publish(
            events,
            KayaEvent::RoomJoined {
                node_id: self.node_id.clone(),
                callsign: self.callsign.clone(),
                room: self.rooms.current_room().to_string(),
                local: true,
            },
        );
        publish(
            events,
            KayaEvent::NetworkStarted {
                multicast_addr: transport.label(),
            },
        );

        self.send_packet(
            transport,
            events,
            Packet::hello(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
            ),
        )
        .await;
        self.send_packet(
            transport,
            events,
            Packet::presence_update(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
                self.presence,
            ),
        )
        .await;
        self.send_packet(
            transport,
            events,
            Packet::room_announce(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
            ),
        )
        .await;
        self.send_packet(
            transport,
            events,
            Packet::room_join(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
            ),
        )
        .await;
        self.send_packet(
            transport,
            events,
            Packet::room_members_request(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
            ),
        )
        .await;
        self.send_packet(transport, events, self.route_announce_packet())
            .await;
    }

    async fn shutdown(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
    ) -> Result<()> {
        self.send_packet(transport, events, self.leave_packet())
            .await;
        self.config.last_room = Some(self.rooms.current_room().to_string());
        self.config_store.save(&self.config)?;
        self.store.flush()?;
        publish(
            events,
            KayaEvent::ShutdownInitiated {
                reason: "operator requested shutdown".into(),
            },
        );
        Ok(())
    }

    async fn send_packet(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        mut packet: Packet,
    ) {
        if let Err(err) = sign_packet(&mut packet, &self.identity) {
            publish(
                events,
                KayaEvent::SecurityWarning {
                    node_id: Some(self.node_id.clone()),
                    message: format!("outgoing packet signing failed: {err}"),
                },
            );
            return;
        }

        let packet_id = packet.packet_id;
        let packet_type = packet.packet_type;
        match time::timeout(
            Duration::from_millis(self.timeouts.packet_send_ms),
            transport.send_packet(&packet),
        )
        .await
        {
            Ok(Ok(bytes)) => publish(
                events,
                KayaEvent::PacketSent {
                    packet_id,
                    packet_type,
                    bytes,
                },
            ),
            Ok(Err(err)) => publish(
                events,
                KayaEvent::ErrorOccurred {
                    scope: "transport.tx".into(),
                    message: err.to_string(),
                },
            ),
            Err(_) => publish(
                events,
                KayaEvent::ErrorOccurred {
                    scope: "transport.tx".into(),
                    message: format!(
                        "packet send timed out after {}ms",
                        self.timeouts.packet_send_ms
                    ),
                },
            ),
        }
    }

    async fn send_packet_routed(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        mut packet: Packet,
        destination_node: &str,
    ) {
        if self.direct_peer_online(destination_node) {
            self.send_packet(transport, events, packet).await;
            return;
        }

        let Some(route) = self.mesh.best_route(destination_node).cloned() else {
            self.send_route_request(transport, events, destination_node)
                .await;
            return;
        };

        if self.trust_store.is_blocked(&route.next_hop)
            || self.trust_store.is_blocked(destination_node)
        {
            publish(
                events,
                KayaEvent::RelayDenied {
                    source_node: self.node_id.clone(),
                    destination_node: destination_node.to_string(),
                    reason: "blocked peer in route".into(),
                },
            );
            return;
        }

        if let Err(err) = sign_packet(&mut packet, &self.identity) {
            publish(
                events,
                KayaEvent::SecurityWarning {
                    node_id: Some(self.node_id.clone()),
                    message: format!("inner mesh packet signing failed: {err}"),
                },
            );
            return;
        }

        let envelope = self
            .mesh
            .build_envelope(destination_node, &route.next_hop, packet);
        let Ok(payload) = envelope.to_value() else {
            publish(
                events,
                KayaEvent::RouteError {
                    destination_node: destination_node.to_string(),
                    reason: "failed to encode mesh envelope".into(),
                },
            );
            return;
        };

        self.send_packet(
            transport,
            events,
            Packet::mesh_relay(
                self.node_id.clone(),
                self.callsign.clone(),
                route.next_hop,
                payload,
            ),
        )
        .await;
    }

    async fn send_route_request(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        destination_node: &str,
    ) {
        if !self.mesh.policy.enabled || !is_valid_node_id(destination_node) {
            return;
        }
        let now = now_millis();
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
        publish(
            events,
            KayaEvent::RouteRequestSent {
                destination_node: destination_node.to_string(),
                request_id: request_id.clone(),
            },
        );

        self.send_packet(
            transport,
            events,
            Packet::route_request(
                self.node_id.clone(),
                self.callsign.clone(),
                request_id,
                destination_node.to_string(),
                self.mesh.policy.max_ttl,
            ),
        )
        .await;
    }

    async fn set_callsign(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        callsign: &str,
    ) -> Result<()> {
        self.identity.set_callsign(callsign);
        self.identity_store.save(&self.identity)?;
        self.callsign = self.identity.callsign.clone();
        self.rooms.set_own_callsign(self.callsign.clone());
        self.sessions.set_identity(self.identity.clone());
        self.config.nickname = Some(self.callsign.clone());
        self.config_store.save(&self.config)?;

        self.send_packet(
            transport,
            events,
            Packet::hello(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
            ),
        )
        .await;
        self.send_packet(
            transport,
            events,
            Packet::presence_update(
                self.node_id.clone(),
                self.callsign.clone(),
                self.rooms.current_room().to_string(),
                self.presence,
            ),
        )
        .await;
        Ok(())
    }

    async fn join_room(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        room: &str,
    ) -> Result<()> {
        let room = self.rooms.join(room)?;
        self.config.last_room = Some(self.rooms.current_room().to_string());
        publish(
            events,
            KayaEvent::RoomJoined {
                node_id: self.node_id.clone(),
                callsign: self.callsign.clone(),
                room: room.clone(),
                local: true,
            },
        );
        self.send_packet(
            transport,
            events,
            Packet::room_join(self.node_id.clone(), self.callsign.clone(), room.clone()),
        )
        .await;
        self.send_packet(
            transport,
            events,
            Packet::room_members_request(self.node_id.clone(), self.callsign.clone(), room),
        )
        .await;
        Ok(())
    }

    async fn send_room_message(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        room: Option<&str>,
        body: &str,
    ) -> Result<()> {
        let selected_room = room
            .map(str::to_string)
            .unwrap_or_else(|| self.rooms.current_room().to_string());
        let current_room = self.rooms.current_room().to_string();
        if room.is_some() && selected_room != current_room {
            self.rooms.join(&selected_room)?;
        }
        if !self.rooms.is_joined(self.rooms.current_room()) {
            return Err(KayaError::InvalidCommand(format!(
                "cannot send: not joined to #{}",
                self.rooms.current_room()
            )));
        }
        let room = self.rooms.current_room().to_string();
        self.rooms.add_local_room_message(body.to_string());
        let message = ChatMessage {
            timestamp: now_millis().to_string(),
            room: Some(room.clone()),
            from_node: self.node_id.clone(),
            from_callsign: self.callsign.clone(),
            target_node: None,
            body: body.to_string(),
            direct: false,
            encrypted: false,
        };
        self.persist_chat_message(&message, events);
        publish(
            events,
            KayaEvent::RoomMessageReceived {
                room: room.clone(),
                from_node: self.node_id.clone(),
                from_callsign: self.callsign.clone(),
                body: body.to_string(),
                local: true,
            },
        );
        self.send_packet(
            transport,
            events,
            Packet::room_message(
                self.node_id.clone(),
                self.callsign.clone(),
                room,
                body.to_string(),
            ),
        )
        .await;
        Ok(())
    }

    async fn send_direct_message(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        target: &str,
        body: &str,
    ) -> Result<()> {
        let (target_node, target_callsign) = self.resolve_message_target(target)?;
        if self.trust_store.is_blocked(&target_node) {
            return Err(KayaError::Security(format!(
                "dm target is blocked: {target_node}"
            )));
        }

        if self.sessions.has_active(&target_node) {
            return self
                .send_encrypted_message(
                    transport,
                    events,
                    target_node,
                    target_callsign,
                    body.to_string(),
                )
                .await;
        }

        self.rooms
            .add_local_direct_message(target_node.clone(), body.to_string());
        let message = ChatMessage {
            timestamp: now_millis().to_string(),
            room: None,
            from_node: self.node_id.clone(),
            from_callsign: self.callsign.clone(),
            target_node: Some(target_node.clone()),
            body: body.to_string(),
            direct: true,
            encrypted: false,
        };
        self.persist_chat_message(&message, events);
        publish(
            events,
            KayaEvent::DirectMessageSent {
                target_node: target_node.clone(),
                target_callsign: target_callsign.clone(),
                body: body.to_string(),
            },
        );
        self.send_packet_routed(
            transport,
            events,
            Packet::direct_message(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.clone(),
                body.to_string(),
            ),
            &target_node,
        )
        .await;
        Ok(())
    }

    async fn send_secure_direct_message(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        target: &str,
        body: &str,
    ) -> Result<()> {
        let (target_node, target_callsign) = self.resolve_message_target(target)?;
        if self.trust_store.is_blocked(&target_node) {
            return Err(KayaError::Security(format!(
                "secure dm target is blocked: {target_node}"
            )));
        }

        if self.sessions.has_active(&target_node) {
            return self
                .send_encrypted_message(
                    transport,
                    events,
                    target_node,
                    target_callsign,
                    body.to_string(),
                )
                .await;
        }

        let request = self.sessions.start_request(&target_node);
        self.pending_secure_messages
            .entry(target_node.clone())
            .and_modify(|queue| queue.messages.push(body.to_string()))
            .or_insert_with(|| PendingSecureQueue {
                messages: vec![body.to_string()],
            });
        self.send_packet_routed(
            transport,
            events,
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
        Ok(())
    }

    async fn send_encrypted_message(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        target_node: String,
        target_callsign: String,
        body: String,
    ) -> Result<()> {
        let encrypted = self.sessions.encrypt(&target_node, &body)?;
        let message = ChatMessage {
            timestamp: now_millis().to_string(),
            room: None,
            from_node: self.node_id.clone(),
            from_callsign: self.callsign.clone(),
            target_node: Some(target_node.clone()),
            body: body.clone(),
            direct: true,
            encrypted: true,
        };
        self.persist_chat_message(&message, events);
        publish(
            events,
            KayaEvent::EncryptedMessageReceived {
                from_node: self.node_id.clone(),
                from_callsign: self.callsign.clone(),
                target_node: target_node.clone(),
                body: body.clone(),
                local: true,
            },
        );
        self.send_packet_routed(
            transport,
            events,
            Packet::direct_message_encrypted(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.clone(),
                EncryptedDirectMessagePayload {
                    session_id: encrypted.session_id,
                    nonce: encrypted.nonce,
                    ciphertext: encrypted.ciphertext,
                    sender_fingerprint: encrypted.sender_fingerprint,
                    timestamp: encrypted.timestamp,
                },
            ),
            &target_node,
        )
        .await;
        publish(
            events,
            KayaEvent::DirectMessageSent {
                target_node,
                target_callsign,
                body,
            },
        );
        Ok(())
    }

    async fn send_file_offer(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        target: &str,
        path: impl Into<PathBuf>,
    ) -> Result<String> {
        if !self.file_config.enabled {
            return Err(KayaError::Config("file transfer is disabled".into()));
        }
        let (target_node, target_callsign) = self.resolve_message_target(target)?;
        if self.trust_store.is_blocked(&target_node) {
            return Err(KayaError::Security(format!(
                "file target is blocked: {target_node}"
            )));
        }
        let security = if self.sessions.has_active(&target_node) {
            TransferSecurity::Encrypted
        } else {
            TransferSecurity::Unencrypted
        };
        let session = self
            .files
            .prepare_outgoing(
                OutgoingFileRequest {
                    path: path.into(),
                    sender_node_id: self.node_id.clone(),
                    sender_callsign: self.callsign.clone(),
                    peer_node_id: target_node.clone(),
                    peer_callsign: target_callsign.clone(),
                    security,
                },
                &self.file_config,
            )?
            .clone();
        self.persist_file_session(&session.file_id, events);
        publish(
            events,
            KayaEvent::FileOfferSent {
                file_id: session.file_id.clone(),
                file_name: session.metadata.file_name.clone(),
                target_node: target_node.clone(),
                target_callsign: target_callsign.clone(),
                size_bytes: session.metadata.file_size,
                encrypted: security == TransferSecurity::Encrypted,
            },
        );
        self.send_packet_routed(
            transport,
            events,
            Packet::file_offer(
                self.node_id.clone(),
                self.callsign.clone(),
                target_node.clone(),
                file_offer_payload(&session.metadata, security == TransferSecurity::Encrypted),
            ),
            &target_node,
        )
        .await;
        Ok(session.file_id)
    }

    async fn set_peer_trust(
        &mut self,
        target: &str,
        status: TrustStatus,
        events: &broadcast::Sender<KayaEvent>,
    ) -> Result<()> {
        let peer = self
            .trust_store
            .find(target)
            .cloned()
            .ok_or_else(|| KayaError::Security(format!("peer not in trust store: {target}")))?;
        self.trust_store.set_status(&peer.node_id, status)?;
        let event = match status {
            TrustStatus::Blocked => KayaEvent::PeerBlocked {
                node_id: peer.node_id,
                callsign: peer.callsign,
                fingerprint: peer.fingerprint,
            },
            TrustStatus::Trusted => KayaEvent::PeerTrusted {
                node_id: peer.node_id,
                callsign: peer.callsign,
                fingerprint: peer.fingerprint,
            },
            TrustStatus::Unknown => KayaEvent::SecurityWarning {
                node_id: Some(peer.node_id),
                message: format!("trust reset for {}", peer.callsign),
            },
        };
        publish(events, event);
        Ok(())
    }

    async fn execute_input(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        input: &str,
    ) -> Result<bool> {
        match self.commands.parse(input)? {
            ParsedInput::Empty => Ok(false),
            ParsedInput::Message(body) => {
                self.send_room_message(transport, events, None, &body)
                    .await?;
                Ok(false)
            }
            ParsedInput::Command(command) => self.execute_command(transport, events, command).await,
        }
    }

    async fn execute_command(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        command: Command,
    ) -> Result<bool> {
        match command {
            Command::Join { room } => {
                self.join_room(transport, events, &room).await?;
            }
            Command::RoomMessage { body } => {
                self.send_room_message(transport, events, None, &body)
                    .await?;
            }
            Command::Msg { target, body } => {
                self.send_direct_message(transport, events, &target, &body)
                    .await?;
            }
            Command::SecureMsg { target, body } => {
                self.send_secure_direct_message(transport, events, &target, &body)
                    .await?;
            }
            Command::SendFile { target, path } => {
                let _ = self
                    .send_file_offer(transport, events, &target, path)
                    .await?;
            }
            Command::Trust { peer } => {
                self.set_peer_trust(&peer, TrustStatus::Trusted, events)
                    .await?;
            }
            Command::Untrust { peer } => {
                self.set_peer_trust(&peer, TrustStatus::Unknown, events)
                    .await?;
            }
            Command::Block { peer } => {
                self.set_peer_trust(&peer, TrustStatus::Blocked, events)
                    .await?;
            }
            Command::Presence { status } => {
                self.presence = status;
                publish(
                    events,
                    KayaEvent::PresenceUpdated {
                        node_id: self.node_id.clone(),
                        callsign: self.callsign.clone(),
                        presence: status,
                    },
                );
                self.send_packet(
                    transport,
                    events,
                    Packet::presence_update(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        self.rooms.current_room().to_string(),
                        status,
                    ),
                )
                .await;
            }
            Command::Route { node_id } => {
                self.send_route_request(transport, events, &node_id).await;
            }
            Command::Exit => return Ok(true),
            _ => {}
        }
        Ok(false)
    }

    async fn handle_packet_received(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: Packet,
        source: String,
        bytes: usize,
    ) {
        publish(
            events,
            KayaEvent::PacketReceived {
                packet: packet.clone(),
                source,
                bytes,
            },
        );
        if !self.dedup.observe(packet.packet_id) || packet.node_id == self.node_id {
            return;
        }
        if !self.inspect_packet_security(&packet, events) {
            return;
        }

        self.observe_peer(&packet, events);
        self.remember_peer(&packet, events);
        if self.route_mesh_packet(transport, events, &packet).await {
            return;
        }
        if self.route_security_packet(transport, events, &packet).await {
            return;
        }
        if self.route_file_packet(transport, events, &packet).await {
            return;
        }
        self.route_room_packet(transport, events, &packet).await;
        for packet in self.state_sync_for(&packet) {
            self.send_packet(transport, events, packet).await;
        }
    }

    async fn route_room_packet(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) {
        match self.rooms.route_packet(packet) {
            RouteOutcome::RoomMessage(message) => self.publish_room_message(message, events),
            RouteOutcome::DirectMessage(message) => {
                self.persist_chat_message(&message, events);
                publish(
                    events,
                    KayaEvent::DirectMessageReceived {
                        from_node: message.from_node,
                        from_callsign: message.from_callsign,
                        target_node: message.target_node.unwrap_or_else(|| self.node_id.clone()),
                        body: message.body,
                        local: false,
                    },
                );
                self.send_packet_routed(
                    transport,
                    events,
                    Packet::dm_ack(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        packet.node_id.clone(),
                        packet.packet_id,
                    ),
                    &packet.node_id,
                )
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
                    publish(
                        events,
                        KayaEvent::RoomJoined {
                            node_id,
                            callsign,
                            room,
                            local: false,
                        },
                    );
                }
            }
            RouteOutcome::RoomCreated {
                node_id,
                callsign,
                room,
            } => publish(
                events,
                KayaEvent::RoomCreated {
                    node_id,
                    callsign,
                    room,
                    local: false,
                },
            ),
            RouteOutcome::Left { node_id, room } => {
                publish(events, KayaEvent::RoomLeft { node_id, room });
            }
            RouteOutcome::MembersRequested { room, .. } => {
                if self.rooms.is_joined(&room) {
                    self.send_packet(
                        transport,
                        events,
                        Packet::room_members_response(
                            self.node_id.clone(),
                            self.callsign.clone(),
                            room.clone(),
                            self.rooms.members(&room),
                        ),
                    )
                    .await;
                }
            }
            RouteOutcome::MembersResponse { .. } | RouteOutcome::Ignored => {}
        }
    }

    async fn route_security_packet(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) -> bool {
        match packet.packet_type {
            PacketType::DmSessionRequest => {
                if !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(request) = session_request_from_packet(packet) else {
                    self.security_warning(
                        Some(packet.node_id.clone()),
                        "malformed secure session request".into(),
                        events,
                    );
                    return true;
                };
                if !self.packet_fingerprint_matches(&packet.node_id, &request.fingerprint, events) {
                    return true;
                }
                match self.sessions.accept_request(
                    &packet.node_id,
                    &request.session_id,
                    &request.x25519_public_key,
                    &request.fingerprint,
                ) {
                    Ok(accept) => {
                        publish(
                            events,
                            KayaEvent::SecureSessionStarted {
                                peer_node_id: packet.node_id.clone(),
                                session_id: accept.session_id.clone(),
                            },
                        );
                        self.send_packet_routed(
                            transport,
                            events,
                            Packet::dm_session_accept(
                                self.node_id.clone(),
                                self.callsign.clone(),
                                packet.node_id.clone(),
                                accept.session_id,
                                accept.x25519_public_key,
                                accept.fingerprint,
                            ),
                            &packet.node_id,
                        )
                        .await;
                    }
                    Err(err) => {
                        self.security_warning(Some(packet.node_id.clone()), err.to_string(), events)
                    }
                }
                true
            }
            PacketType::DmSessionAccept => {
                if !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(accept) = session_accept_from_packet(packet) else {
                    self.security_warning(
                        Some(packet.node_id.clone()),
                        "malformed secure session accept".into(),
                        events,
                    );
                    return true;
                };
                if !self.packet_fingerprint_matches(&packet.node_id, &accept.fingerprint, events) {
                    return true;
                }
                match self.sessions.complete_accept(
                    &packet.node_id,
                    &accept.session_id,
                    &accept.x25519_public_key,
                    &accept.fingerprint,
                ) {
                    Ok(()) => {
                        publish(
                            events,
                            KayaEvent::SecureSessionStarted {
                                peer_node_id: packet.node_id.clone(),
                                session_id: accept.session_id,
                            },
                        );
                        self.flush_pending_secure_messages(transport, events, &packet.node_id)
                            .await;
                    }
                    Err(err) => {
                        self.security_warning(Some(packet.node_id.clone()), err.to_string(), events)
                    }
                }
                true
            }
            PacketType::DirectMessageEncrypted => {
                if !self.packet_targets_local_node(packet) {
                    return true;
                }
                let Ok(payload) = encrypted_payload_from_packet(packet) else {
                    self.security_warning(
                        Some(packet.node_id.clone()),
                        "malformed encrypted dm".into(),
                        events,
                    );
                    return true;
                };
                if !self.packet_fingerprint_matches(
                    &packet.node_id,
                    &payload.sender_fingerprint,
                    events,
                ) {
                    return true;
                }
                match self.sessions.decrypt(&packet.node_id, &payload) {
                    Ok(body) => {
                        let message = ChatMessage {
                            timestamp: now_millis().to_string(),
                            room: None,
                            from_node: packet.node_id.clone(),
                            from_callsign: packet.callsign.clone(),
                            target_node: Some(self.node_id.clone()),
                            body: body.clone(),
                            direct: true,
                            encrypted: true,
                        };
                        self.persist_chat_message(&message, events);
                        publish(
                            events,
                            KayaEvent::EncryptedMessageReceived {
                                from_node: packet.node_id.clone(),
                                from_callsign: packet.callsign.clone(),
                                target_node: self.node_id.clone(),
                                body,
                                local: false,
                            },
                        );
                    }
                    Err(err) => {
                        self.security_warning(Some(packet.node_id.clone()), err.to_string(), events)
                    }
                }
                true
            }
            _ => false,
        }
    }

    async fn route_file_packet(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) -> bool {
        match packet.packet_type {
            PacketType::FileOffer => self.receive_file_offer(transport, events, packet).await,
            PacketType::FileAccept => self.receive_file_accept(transport, events, packet).await,
            PacketType::FileReject => self.receive_file_reject(events, packet),
            PacketType::FileChunk => {
                self.receive_file_chunk(transport, events, packet, false)
                    .await
            }
            PacketType::FileChunkEncrypted => {
                self.receive_file_chunk(transport, events, packet, true)
                    .await
            }
            PacketType::FileChunkAck => self.receive_file_ack(events, packet),
            PacketType::FileTransferComplete => self.receive_file_complete(events, packet),
            PacketType::FileTransferCancel => self.receive_file_cancel(events, packet),
            PacketType::FileTransferError => self.receive_file_error(events, packet),
            _ => return false,
        }
        true
    }

    async fn receive_file_offer(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) {
        if !self.packet_targets_local_node(packet) {
            return;
        }
        if !self.file_config.enabled {
            self.send_packet_routed(
                transport,
                events,
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
                self.security_warning(Some(packet.node_id.clone()), err.to_string(), events);
                return;
            }
        };
        let metadata = metadata_from_offer(&payload);
        let signed = packet.public_key.is_some() && packet.signature.is_some();
        let trusted = self.trust_store.status(&packet.node_id) == TrustStatus::Trusted;
        if !trusted && !self.file_config.accept_from_unknown {
            self.send_packet_routed(
                transport,
                events,
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
        self.persist_file_session(&session.file_id, events);
        publish(
            events,
            KayaEvent::FileOfferReceived {
                file_id: session.file_id.clone(),
                file_name: session.metadata.file_name.clone(),
                from_node: packet.node_id.clone(),
                from_callsign: packet.callsign.clone(),
                size_bytes: session.metadata.file_size,
                encrypted: security == TransferSecurity::Encrypted,
            },
        );
    }

    async fn receive_file_accept(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) {
        if !self.packet_targets_local_node(packet) {
            return;
        }
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        if self.files.mark_outgoing_accepted(&file_id).is_ok() {
            self.persist_file_session(&file_id, events);
            publish(
                events,
                KayaEvent::FileAccepted {
                    file_id: file_id.clone(),
                    node_id: packet.node_id.clone(),
                    callsign: packet.callsign.clone(),
                },
            );
            self.send_file_chunks(transport, events, &file_id, &packet.node_id)
                .await;
        }
    }

    fn receive_file_reject(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let _ = self.files.reject(&file_id);
        self.persist_file_session(&file_id, events);
        publish(
            events,
            KayaEvent::FileRejected {
                file_id,
                node_id: packet.node_id.clone(),
                callsign: packet.callsign.clone(),
                reason: payload_str(packet, "reason").map(str::to_string),
            },
        );
    }

    async fn receive_file_chunk(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
        encrypted: bool,
    ) {
        if !self.packet_targets_local_node(packet) {
            return;
        }
        let chunk = if encrypted {
            match self.decrypt_file_chunk_packet(packet, events) {
                Ok(chunk) => chunk,
                Err(err) => {
                    self.security_warning(Some(packet.node_id.clone()), err.to_string(), events);
                    return;
                }
            }
        } else {
            match file_chunk_from_packet(packet) {
                Ok(chunk) => chunk,
                Err(err) => {
                    self.security_warning(Some(packet.node_id.clone()), err.to_string(), events);
                    return;
                }
            }
        };
        let file_id = chunk.file_id.clone();
        let chunk_index = chunk.chunk_index;
        let total_chunks = chunk.total_chunks;
        match self.files.receive_chunk(chunk) {
            Ok(Some(bytes)) => {
                publish(
                    events,
                    KayaEvent::FileChunkReceived {
                        file_id: file_id.clone(),
                        chunk_index,
                        total_chunks,
                    },
                );
                self.publish_file_progress(&file_id, events);
                if let Ok(session) = self.files.session(&file_id).cloned() {
                    match self.file_store.save_completed(&session, &bytes) {
                        Ok(path) => {
                            let path_string = path.display().to_string();
                            let _ = self
                                .files
                                .mark_completed_path(&file_id, path_string.clone());
                            self.persist_file_session(&file_id, events);
                            publish(
                                events,
                                KayaEvent::FileHashVerified {
                                    file_id: file_id.clone(),
                                    sha256: session.metadata.sha256,
                                },
                            );
                            publish(
                                events,
                                KayaEvent::FileTransferCompleted {
                                    file_id: file_id.clone(),
                                    path: Some(path_string),
                                },
                            );
                            self.send_packet(
                                transport,
                                events,
                                Packet::file_transfer_complete(
                                    self.node_id.clone(),
                                    self.callsign.clone(),
                                    packet.node_id.clone(),
                                    file_id.clone(),
                                ),
                            )
                            .await;
                        }
                        Err(err) => publish(
                            events,
                            KayaEvent::FileTransferFailed {
                                file_id: file_id.clone(),
                                reason: err.to_string(),
                            },
                        ),
                    }
                }
            }
            Ok(None) => {
                publish(
                    events,
                    KayaEvent::FileChunkReceived {
                        file_id: file_id.clone(),
                        chunk_index,
                        total_chunks,
                    },
                );
                self.publish_file_progress(&file_id, events);
            }
            Err(err) => {
                publish(
                    events,
                    KayaEvent::FileHashMismatch {
                        file_id: file_id.clone(),
                    },
                );
                publish(
                    events,
                    KayaEvent::FileTransferFailed {
                        file_id: file_id.clone(),
                        reason: err.to_string(),
                    },
                );
                self.send_packet(
                    transport,
                    events,
                    Packet::file_transfer_error(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        packet.node_id.clone(),
                        file_id.clone(),
                        err.to_string(),
                    ),
                )
                .await;
            }
        }
        self.send_packet(
            transport,
            events,
            Packet::file_chunk_ack(
                self.node_id.clone(),
                self.callsign.clone(),
                packet.node_id.clone(),
                file_id,
                chunk_index,
            ),
        )
        .await;
    }

    fn receive_file_ack(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let chunk_index = packet
            .payload
            .get("chunk_index")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as u32;
        publish(
            events,
            KayaEvent::FileChunkAcked {
                file_id,
                chunk_index,
            },
        );
    }

    fn receive_file_complete(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        if let Ok(session) = self.files.session_mut(&file_id) {
            session.status = TransferStatus::Completed;
        }
        self.persist_file_session(&file_id, events);
        publish(
            events,
            KayaEvent::FileTransferCompleted {
                file_id,
                path: None,
            },
        );
    }

    fn receive_file_cancel(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let _ = self.files.cancel(&file_id);
        self.persist_file_session(&file_id, events);
        publish(
            events,
            KayaEvent::FileTransferCancelled {
                file_id,
                reason: payload_str(packet, "reason").map(str::to_string),
            },
        );
    }

    fn receive_file_error(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
        let Some(file_id) = payload_str(packet, "file_id").map(str::to_string) else {
            return;
        };
        let reason = payload_str(packet, "reason")
            .unwrap_or("remote file transfer error")
            .to_string();
        let _ = self.files.fail(&file_id, reason.clone());
        self.persist_file_session(&file_id, events);
        publish(events, KayaEvent::FileTransferFailed { file_id, reason });
    }

    async fn send_file_chunks(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        file_id: &str,
        peer_node_id: &str,
    ) {
        if self
            .peers
            .get(peer_node_id)
            .map(|peer| !peer.online)
            .unwrap_or(true)
        {
            let reason = "file chunks over mesh not enabled yet".to_string();
            let _ = self.files.fail(file_id, reason.clone());
            self.persist_file_session(file_id, events);
            publish(
                events,
                KayaEvent::FileTransferFailed {
                    file_id: file_id.to_string(),
                    reason: reason.clone(),
                },
            );
            self.send_packet_routed(
                transport,
                events,
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
            Err(_) => return,
        };
        let chunks = match self.files.outgoing_chunks(file_id) {
            Ok(chunks) => chunks.to_vec(),
            Err(_) => return,
        };

        for chunk in chunks {
            if session.security == TransferSecurity::Encrypted {
                match self
                    .sessions
                    .encrypt_file_chunk(peer_node_id, &chunk.payload)
                {
                    Ok(payload) => {
                        self.send_packet(
                            transport,
                            events,
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
                        )
                        .await;
                    }
                    Err(err) => {
                        self.security_warning(
                            Some(peer_node_id.to_string()),
                            err.to_string(),
                            events,
                        );
                        return;
                    }
                }
            } else {
                self.send_packet(
                    transport,
                    events,
                    Packet::file_chunk(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        peer_node_id.to_string(),
                        FileChunkPayload {
                            file_id: chunk.file_id,
                            chunk_index: chunk.chunk_index,
                            total_chunks: chunk.total_chunks,
                            chunk_hash: chunk.chunk_hash,
                            payload: encode_hex(&chunk.payload),
                            timestamp: chunk.timestamp,
                        },
                    ),
                )
                .await;
            }
        }
    }

    async fn route_mesh_packet(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) -> bool {
        match packet.packet_type {
            PacketType::RouteAnnounce => {
                self.receive_route_announce(events, packet);
                true
            }
            PacketType::RouteRequest => {
                self.receive_route_request(transport, events, packet).await;
                true
            }
            PacketType::RouteResponse => {
                self.receive_route_response(events, packet);
                true
            }
            PacketType::MeshRelay => {
                self.receive_mesh_relay(transport, events, packet).await;
                true
            }
            PacketType::RouteError => {
                publish(
                    events,
                    KayaEvent::RouteError {
                        destination_node: payload_str(packet, "destination_node")
                            .unwrap_or("--")
                            .to_string(),
                        reason: payload_str(packet, "reason")
                            .unwrap_or("route error")
                            .to_string(),
                    },
                );
                true
            }
            PacketType::RoutePing => {
                if self.packet_targets_local_node(packet) {
                    self.send_packet_routed(
                        transport,
                        events,
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

    fn receive_route_announce(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
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
            self.observe_route_entry(entry, events);
        }
    }

    async fn receive_route_request(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) {
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
                transport,
                events,
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
            transport,
            events,
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

    async fn send_route_response(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        target_node: &str,
        response: RouteResponsePayload,
    ) {
        self.send_packet_routed(
            transport,
            events,
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

    fn receive_route_response(&mut self, events: &broadcast::Sender<KayaEvent>, packet: &Packet) {
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
        self.observe_route_entry(entry, events);
        publish(
            events,
            KayaEvent::RouteResponseReceived {
                destination_node: response.destination_node,
                next_hop: packet.node_id.clone(),
                hop_count: response.hop_count,
            },
        );
    }

    async fn receive_mesh_relay(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        packet: &Packet,
    ) {
        let envelope = match MeshEnvelope::decode(packet.payload.clone()) {
            Ok(envelope) => envelope,
            Err(err) => {
                publish(
                    events,
                    KayaEvent::MeshPacketDropped {
                        mesh_packet_id: "unknown".into(),
                        reason: err.to_string(),
                    },
                );
                return;
            }
        };

        if !self.mesh.accept_seen(&envelope.mesh_packet_id) {
            self.drop_mesh_packet(events, &envelope, RelayDropReason::Duplicate);
            return;
        }
        if self.trust_store.is_blocked(&envelope.source_node) {
            self.drop_mesh_packet(events, &envelope, RelayDropReason::BlockedPeer);
            return;
        }

        if envelope.destination_node == self.node_id {
            self.mesh.mark_delivered(&envelope);
            publish(
                events,
                KayaEvent::MeshPacketDelivered {
                    mesh_packet_id: envelope.mesh_packet_id.clone(),
                    source_node: envelope.source_node.clone(),
                    route_trace: envelope.route_trace.clone(),
                },
            );
            self.learn_route_from_trace(&envelope, &packet.node_id, events);
            Box::pin(self.handle_packet_received(
                transport,
                events,
                *envelope.inner_packet,
                packet.node_id.clone(),
                0,
            ))
            .await;
            return;
        }

        let decision = decide_relay(
            &envelope,
            &self.node_id,
            &self.mesh.policy,
            self.trust_store.is_blocked(&packet.node_id),
            self.mesh
                .best_route(&envelope.destination_node)
                .map(|route| route.next_hop.as_str()),
        );
        match decision {
            RelayDecision::Drop(reason) => self.drop_mesh_packet(events, &envelope, reason),
            RelayDecision::Deliver => {
                self.mesh.mark_delivered(&envelope);
                publish(
                    events,
                    KayaEvent::MeshPacketDelivered {
                        mesh_packet_id: envelope.mesh_packet_id.clone(),
                        source_node: envelope.source_node.clone(),
                        route_trace: envelope.route_trace.clone(),
                    },
                );
            }
            RelayDecision::Relay { next_hop } => {
                let Ok(relayed) = envelope.relay(&self.node_id, Some(next_hop.clone())) else {
                    self.drop_mesh_packet(events, &envelope, RelayDropReason::TtlExpired);
                    return;
                };
                self.learn_route_from_trace(&relayed, &packet.node_id, events);
                self.mesh.mark_relayed(&relayed);
                publish(
                    events,
                    KayaEvent::MeshPacketRelayed {
                        mesh_packet_id: relayed.mesh_packet_id.clone(),
                        destination_node: relayed.destination_node.clone(),
                        next_hop: next_hop.clone(),
                        hop_count: relayed.hop_count,
                    },
                );
                if let Ok(payload) = relayed.to_value() {
                    self.send_packet(
                        transport,
                        events,
                        Packet::mesh_relay(
                            self.node_id.clone(),
                            self.callsign.clone(),
                            next_hop,
                            payload,
                        ),
                    )
                    .await;
                }
            }
        }
    }

    fn drop_mesh_packet(
        &mut self,
        events: &broadcast::Sender<KayaEvent>,
        envelope: &MeshEnvelope,
        reason: RelayDropReason,
    ) {
        self.mesh.mark_dropped(reason);
        publish(
            events,
            KayaEvent::MeshPacketDropped {
                mesh_packet_id: envelope.mesh_packet_id.clone(),
                reason: format!("{reason:?}"),
            },
        );
    }

    fn learn_route_from_trace(
        &mut self,
        envelope: &MeshEnvelope,
        previous_hop: &str,
        events: &broadcast::Sender<KayaEvent>,
    ) {
        if envelope.source_node == self.node_id
            || self.trust_store.is_blocked(&envelope.source_node)
        {
            return;
        }
        let entry = RouteEntry::from_spec(RouteEntrySpec {
            destination_node: envelope.source_node.clone(),
            destination_callsign: Some(envelope.inner_packet.callsign.clone()),
            next_hop: previous_hop.to_string(),
            hop_count: envelope.hop_count.max(1),
            trusted: self.trust_store.status(&envelope.source_node) == TrustStatus::Trusted,
            encrypted_capable: matches!(
                envelope.inner_packet.packet_type,
                PacketType::DirectMessageEncrypted
                    | PacketType::DmSessionRequest
                    | PacketType::DmSessionAccept
            ),
            source: RouteSource::RelayTrace,
            latency_ms: self
                .peers
                .get(previous_hop)
                .and_then(|peer| peer.latency_ms),
        });
        self.observe_route_entry(entry, events);
    }

    fn observe_route_entry(&mut self, entry: RouteEntry, events: &broadcast::Sender<KayaEvent>) {
        let destination_node = entry.destination_node.clone();
        let next_hop = entry.next_hop.clone();
        let hop_count = entry.hop_count;
        self.mesh.observe_route(entry);
        publish(
            events,
            KayaEvent::RouteDiscovered {
                destination_node,
                next_hop,
                hop_count,
            },
        );
    }

    fn observe_peer(&mut self, packet: &Packet, events: &broadcast::Sender<KayaEvent>) {
        if let Some(event) = self.peers.observe_packet(packet) {
            match event {
                kaya_peer::PeerEvent::Discovered(node_id) => publish(
                    events,
                    KayaEvent::PeerDiscovered {
                        node_id: node_id.clone(),
                        callsign: self
                            .peers
                            .get(&node_id)
                            .map(|peer| peer.callsign.clone())
                            .unwrap_or_default(),
                    },
                ),
                kaya_peer::PeerEvent::TimedOut(node_id) => {
                    publish(events, KayaEvent::PeerTimedOut { node_id });
                }
                kaya_peer::PeerEvent::Left(node_id) => {
                    publish(
                        events,
                        KayaEvent::RoomLeft {
                            node_id,
                            room: None,
                        },
                    );
                }
                kaya_peer::PeerEvent::PresenceChanged(node_id, presence) => publish(
                    events,
                    KayaEvent::PresenceUpdated {
                        node_id: node_id.clone(),
                        callsign: self
                            .peers
                            .get(&node_id)
                            .map(|peer| peer.callsign.clone())
                            .unwrap_or_default(),
                        presence,
                    },
                ),
                kaya_peer::PeerEvent::Updated(_) => {}
            }
        }
        self.observe_mesh_peer(packet);
    }

    fn observe_mesh_peer(&mut self, packet: &Packet) {
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
    }

    fn remember_peer(&mut self, packet: &Packet, events: &broadcast::Sender<KayaEvent>) {
        let peer = KnownPeer {
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
            fingerprint: self
                .trust_store
                .get(&packet.node_id)
                .map(|peer| peer.fingerprint.clone()),
            last_seen: packet.timestamp.clone(),
        };
        if let Err(err) = self.store.remember_peer(&peer) {
            publish(
                events,
                KayaEvent::ErrorOccurred {
                    scope: "persistence.peers".into(),
                    message: err.to_string(),
                },
            );
        }
    }

    fn inspect_packet_security(
        &mut self,
        packet: &Packet,
        events: &broadcast::Sender<KayaEvent>,
    ) -> bool {
        if self.trust_store.is_blocked(&packet.node_id) {
            self.security_warning(
                Some(packet.node_id.clone()),
                "blocked peer packet rejected".into(),
                events,
            );
            return false;
        }

        match verify_packet_signature(packet) {
            SignatureStatus::Valid { fingerprint } => {
                publish(
                    events,
                    KayaEvent::PacketSignatureValid {
                        node_id: packet.node_id.clone(),
                        fingerprint: fingerprint.clone(),
                    },
                );
                match self
                    .trust_store
                    .record_seen(&packet.node_id, &packet.callsign, &fingerprint)
                {
                    Ok(TrustObservation::FingerprintChanged { previous, current }) => self
                        .security_warning(
                            Some(packet.node_id.clone()),
                            format!("fingerprint changed {previous} -> {current}"),
                            events,
                        ),
                    Ok(TrustObservation::New | TrustObservation::Updated) => {}
                    Err(err) => {
                        self.security_warning(Some(packet.node_id.clone()), err.to_string(), events)
                    }
                }
                true
            }
            SignatureStatus::Missing if secure_packet_requires_signature(packet.packet_type) => {
                self.security_warning(
                    Some(packet.node_id.clone()),
                    "unsigned secure packet rejected".into(),
                    events,
                );
                false
            }
            SignatureStatus::Missing => true,
            SignatureStatus::Invalid { reason } => {
                publish(
                    events,
                    KayaEvent::PacketSignatureInvalid {
                        node_id: packet.node_id.clone(),
                        reason: reason.clone(),
                    },
                );
                let required = packet_requires_signature_validation(packet.packet_type);
                self.security_warning(
                    Some(packet.node_id.clone()),
                    if required {
                        "invalid required packet signature rejected".into()
                    } else {
                        "invalid packet signature rejected".into()
                    },
                    events,
                );
                false
            }
        }
    }

    async fn flush_pending_secure_messages(
        &mut self,
        transport: &dyn KayaTransport,
        events: &broadcast::Sender<KayaEvent>,
        peer_node_id: &str,
    ) {
        let target_callsign = self
            .resolve_mesh_target(peer_node_id)
            .map(|target| target.1)
            .or_else(|| {
                self.peers
                    .get(peer_node_id)
                    .map(|peer| peer.callsign.clone())
            })
            .unwrap_or_else(|| peer_node_id.to_string());
        if let Some(queued) = self.pending_secure_messages.remove(peer_node_id) {
            for message in queued.messages {
                let _ = self
                    .send_encrypted_message(
                        transport,
                        events,
                        peer_node_id.to_string(),
                        target_callsign.clone(),
                        message,
                    )
                    .await;
            }
        }
    }

    fn heartbeat_packet(&self) -> Packet {
        Packet::heartbeat(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
            self.presence,
        )
    }

    fn leave_packet(&self) -> Packet {
        Packet::leave(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        )
    }

    fn route_announce_packet(&self) -> Packet {
        Packet::route_announce(
            self.node_id.clone(),
            self.callsign.clone(),
            self.mesh_route_descriptors(),
        )
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

    fn state_sync_for(&self, packet: &Packet) -> Vec<Packet> {
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

    fn resolve_message_target(&self, target: &str) -> Result<(String, String)> {
        match self.peers.resolve_target_checked(target) {
            TargetResolution::Found(peer) => Ok((peer.node_id, peer.callsign)),
            TargetResolution::NotFound(target) => {
                if let Some(peer) = self.resolve_mesh_target(&target) {
                    Ok(peer)
                } else if is_valid_node_id(&target) {
                    Ok((target.clone(), target))
                } else {
                    Err(KayaError::InvalidCommand(format!(
                        "target not found: {target}"
                    )))
                }
            }
            TargetResolution::DuplicateCallsign { callsign, matches } => {
                Err(KayaError::InvalidCommand(format!(
                    "callsign {callsign} is ambiguous: {}",
                    matches.join(", ")
                )))
            }
        }
    }

    fn resolve_mesh_target(&self, target: &str) -> Option<(String, String)> {
        let route = self.mesh.best_route(target)?;
        Some((
            route.destination_node.clone(),
            route
                .destination_callsign
                .clone()
                .unwrap_or_else(|| route.destination_node.clone()),
        ))
    }

    fn direct_peer_online(&self, node_id: &str) -> bool {
        self.peers
            .get(node_id)
            .map(|peer| peer.online)
            .unwrap_or(false)
    }

    fn packet_targets_local_node(&self, packet: &Packet) -> bool {
        packet.target_node.as_deref() == Some(self.node_id.as_str())
            || packet
                .target_node
                .as_deref()
                .map(|target| target.eq_ignore_ascii_case(&self.callsign))
                .unwrap_or(false)
    }

    fn packet_fingerprint_matches(
        &self,
        node_id: &str,
        fingerprint: &str,
        events: &broadcast::Sender<KayaEvent>,
    ) -> bool {
        let matches = self
            .trust_store
            .get(node_id)
            .map(|peer| peer.fingerprint == fingerprint)
            .unwrap_or(true);
        if !matches {
            publish(
                events,
                KayaEvent::SecurityWarning {
                    node_id: Some(node_id.to_string()),
                    message: "packet fingerprint does not match signed identity".into(),
                },
            );
        }
        matches
    }

    fn publish_room_message(
        &mut self,
        message: ChatMessage,
        events: &broadcast::Sender<KayaEvent>,
    ) {
        let Some(room) = message.room.clone() else {
            return;
        };
        self.persist_chat_message(&message, events);
        if room == self.rooms.current_room() {
            publish(
                events,
                KayaEvent::RoomMessageReceived {
                    room,
                    from_node: message.from_node,
                    from_callsign: message.from_callsign,
                    body: message.body,
                    local: false,
                },
            );
        }
    }

    fn persist_chat_message(
        &mut self,
        message: &ChatMessage,
        events: &broadcast::Sender<KayaEvent>,
    ) {
        let record = kaya_persistence::HistoryRecord {
            timestamp: now_millis().to_string(),
            room: message.room.clone(),
            target: message.target_node.clone(),
            from: message.from_callsign.clone(),
            body: message.body.clone(),
            direct: message.direct,
            encrypted: message.encrypted,
            event: false,
        };
        if let Err(err) = self.store.append_history(&record) {
            publish(
                events,
                KayaEvent::ErrorOccurred {
                    scope: "persistence.history".into(),
                    message: err.to_string(),
                },
            );
        }
    }

    fn persist_file_session(&mut self, file_id: &str, events: &broadcast::Sender<KayaEvent>) {
        let Ok(session) = self.files.session(file_id) else {
            return;
        };
        if let Err(err) = self.file_store.save_record(session) {
            publish(
                events,
                KayaEvent::ErrorOccurred {
                    scope: "files.metadata".into(),
                    message: err.to_string(),
                },
            );
        }
    }

    fn publish_file_progress(&self, file_id: &str, events: &broadcast::Sender<KayaEvent>) {
        if let Ok(session) = self.files.session(file_id) {
            publish(
                events,
                KayaEvent::FileTransferProgress {
                    file_id: file_id.to_string(),
                    bytes_received: session.bytes_received,
                    chunks_received: session.chunks_received,
                    total_chunks: session.total_chunks,
                },
            );
        }
    }

    fn security_warning(
        &self,
        node_id: Option<String>,
        message: String,
        events: &broadcast::Sender<KayaEvent>,
    ) {
        publish(events, KayaEvent::SecurityWarning { node_id, message });
    }

    fn expire_idle_state(&mut self, events: &broadcast::Sender<KayaEvent>) {
        for event in self.peers.prune() {
            match event {
                kaya_peer::PeerEvent::TimedOut(node_id) => {
                    publish(events, KayaEvent::PeerTimedOut { node_id });
                }
                kaya_peer::PeerEvent::PresenceChanged(node_id, presence) => publish(
                    events,
                    KayaEvent::PresenceUpdated {
                        node_id: node_id.clone(),
                        callsign: self
                            .peers
                            .get(&node_id)
                            .map(|peer| peer.callsign.clone())
                            .unwrap_or_default(),
                        presence,
                    },
                ),
                kaya_peer::PeerEvent::Discovered(_)
                | kaya_peer::PeerEvent::Left(_)
                | kaya_peer::PeerEvent::Updated(_) => {}
            }
        }

        for route in self.mesh.expire_routes() {
            publish(
                events,
                KayaEvent::RouteExpired {
                    destination_node: route.destination_node,
                },
            );
        }

        let cutoff = now_millis().saturating_sub(self.timeouts.secure_session_ms);
        for session in self.sessions.expire_before(cutoff) {
            publish(
                events,
                KayaEvent::SecureSessionClosed {
                    peer_node_id: session.peer_node_id,
                    session_id: Some(session.session_id),
                },
            );
        }

        let stale_cutoff = now_millis().saturating_sub(self.timeouts.file_transfer_idle_ms);
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
                ) && session.updated_at.parse::<u64>().unwrap_or(u64::MAX) < stale_cutoff
            })
            .map(|session| session.file_id)
            .collect();
        for file_id in stale_ids {
            let reason = format!(
                "file transfer idle timeout after {}ms",
                self.timeouts.file_transfer_idle_ms
            );
            let _ = self.files.fail(&file_id, reason.clone());
            self.persist_file_session(&file_id, events);
            publish(events, KayaEvent::FileTransferFailed { file_id, reason });
        }
    }

    fn decrypt_file_chunk_packet(
        &mut self,
        packet: &Packet,
        events: &broadcast::Sender<KayaEvent>,
    ) -> Result<FileChunk> {
        let payload: FileEncryptedChunkPayload = serde_json::from_value(packet.payload.clone())?;
        if !self.packet_fingerprint_matches(&packet.node_id, &payload.sender_fingerprint, events) {
            return Err(KayaError::Security(
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
}

#[derive(Clone)]
pub struct KayaCore {
    inner: Arc<KayaCoreInner>,
}

struct KayaCoreInner {
    transport: Arc<dyn KayaTransport>,
    events: broadcast::Sender<KayaEvent>,
    state: Mutex<CoreState>,
    shutdown_tx: watch::Sender<bool>,
    network_task: Mutex<Option<JoinHandle<()>>>,
    maintenance_task: Mutex<Option<JoinHandle<()>>>,
}

impl KayaCore {
    pub async fn new(config: CoreConfig) -> Result<Self> {
        let runtime_config = transport_config(&config.settings)?;
        let transport = Arc::new(MulticastRuntimeTransport::bind(runtime_config).await?);
        Self::with_transport(config, transport).await
    }

    pub async fn with_transport(
        config: CoreConfig,
        transport: Arc<dyn KayaTransport>,
    ) -> Result<Self> {
        let data_dir = config.data_dir();
        let config_store = ConfigStore::new(&data_dir);
        let mut persisted = config_store.load_or_create()?;
        persisted.apply_profile(config.profile);
        if config.callsign.is_some() {
            persisted.nickname = config.callsign.clone();
        }
        if config.settings != PersistedConfig::default() {
            persisted = config.settings.clone();
            persisted.apply_profile(config.profile);
        }
        config_store.save(&persisted)?;

        let identity_store = IdentityStore::new(&data_dir);
        let identity_created = !identity_store.path().exists();
        let callsign = persisted
            .nickname
            .clone()
            .or(config.callsign)
            .unwrap_or_else(|| "sdk-node".into());
        let identity = identity_store.load_or_create(&callsign)?;
        let trust_store = TrustStore::load_or_create(&data_dir)?;
        let file_config = file_transfer_config(&persisted);
        let file_store = FileStore::new(&data_dir, file_config.download_dir.clone())?;
        let store = Store::open(&data_dir)?;
        let state = CoreState::from_bootstrap(CoreBootstrap {
            identity,
            identity_created,
            identity_store,
            file_store,
            store,
            config_store,
            config: persisted,
            profile: config.profile,
            trust_store,
        });

        let (events, _) = broadcast::channel(DEFAULT_EVENT_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let inner = Arc::new(KayaCoreInner {
            transport,
            events,
            state: Mutex::new(state),
            shutdown_tx,
            network_task: Mutex::new(None),
            maintenance_task: Mutex::new(None),
        });
        let core = Self { inner };
        core.start_background_tasks(shutdown_rx).await;
        {
            let mut state = core.inner.state.lock().await;
            state
                .bootstrap(core.inner.transport.as_ref(), &core.inner.events)
                .await;
        }
        Ok(core)
    }

    async fn start_background_tasks(&self, shutdown_rx: watch::Receiver<bool>) {
        let network_inner = self.inner.clone();
        let network_shutdown = shutdown_rx.clone();
        let network = tokio::spawn(async move {
            run_network_loop(network_inner, network_shutdown).await;
        });
        *self.inner.network_task.lock().await = Some(network);

        let maintenance_inner = self.inner.clone();
        let maintenance = tokio::spawn(async move {
            run_maintenance_loop(maintenance_inner, shutdown_rx).await;
        });
        *self.inner.maintenance_task.lock().await = Some(maintenance);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<KayaEvent> {
        self.inner.events.subscribe()
    }

    pub fn help_text(&self) -> String {
        CommandRegistry::default().help_text()
    }

    pub async fn execute_input(&self, input: &str) -> Result<bool> {
        let mut state = self.inner.state.lock().await;
        state
            .execute_input(self.inner.transport.as_ref(), &self.inner.events, input)
            .await
    }

    pub async fn set_callsign(&self, callsign: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .set_callsign(self.inner.transport.as_ref(), &self.inner.events, callsign)
            .await
    }

    pub async fn set_presence(&self, presence: PresenceStatus) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .execute_command(
                self.inner.transport.as_ref(),
                &self.inner.events,
                Command::Presence { status: presence },
            )
            .await
            .map(|_| ())
    }

    pub async fn join_room(&self, room: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .join_room(self.inner.transport.as_ref(), &self.inner.events, room)
            .await
    }

    pub async fn send_room_message(&self, room: &str, body: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .send_room_message(
                self.inner.transport.as_ref(),
                &self.inner.events,
                Some(room),
                body,
            )
            .await
    }

    pub async fn send_direct_message(&self, target: &str, body: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .send_direct_message(
                self.inner.transport.as_ref(),
                &self.inner.events,
                target,
                body,
            )
            .await
    }

    pub async fn send_secure_direct_message(&self, target: &str, body: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .send_secure_direct_message(
                self.inner.transport.as_ref(),
                &self.inner.events,
                target,
                body,
            )
            .await
    }

    pub async fn send_file(&self, target: &str, path: impl Into<PathBuf>) -> Result<String> {
        let mut state = self.inner.state.lock().await;
        state
            .send_file_offer(
                self.inner.transport.as_ref(),
                &self.inner.events,
                target,
                path,
            )
            .await
    }

    pub async fn request_route(&self, destination_node: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .send_route_request(
                self.inner.transport.as_ref(),
                &self.inner.events,
                destination_node,
            )
            .await;
        Ok(())
    }

    pub async fn trust_peer(&self, target: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .set_peer_trust(target, TrustStatus::Trusted, &self.inner.events)
            .await
    }

    pub async fn block_peer(&self, target: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .set_peer_trust(target, TrustStatus::Blocked, &self.inner.events)
            .await
    }

    pub async fn untrust_peer(&self, target: &str) -> Result<()> {
        let mut state = self.inner.state.lock().await;
        state
            .set_peer_trust(target, TrustStatus::Unknown, &self.inner.events)
            .await
    }

    pub async fn callsign(&self) -> String {
        self.inner.state.lock().await.callsign.clone()
    }

    pub async fn node_id(&self) -> String {
        self.inner.state.lock().await.node_id.clone()
    }

    pub async fn list_peers(&self) -> Vec<PeerSnapshot> {
        self.inner.state.lock().await.peers.snapshots()
    }

    pub async fn list_rooms(&self) -> Vec<RoomSummary> {
        self.inner.state.lock().await.rooms.summaries()
    }

    pub async fn list_routes(&self) -> Vec<RouteEntry> {
        self.inner.state.lock().await.mesh.table.entries()
    }

    pub async fn mesh_diagnostics(&self) -> MeshDiagnostics {
        self.inner.state.lock().await.mesh.diagnostics_snapshot()
    }

    pub async fn current_room(&self) -> String {
        self.inner
            .state
            .lock()
            .await
            .rooms
            .current_room()
            .to_string()
    }

    pub async fn trust_list(&self) -> Vec<kaya_security::TrustedPeer> {
        self.inner.state.lock().await.trust_store.list()
    }

    pub async fn secure_sessions(&self) -> Vec<SecureSessionView> {
        self.inner.state.lock().await.sessions.views()
    }

    pub async fn file_transfers(&self) -> Vec<TransferSession> {
        self.inner.state.lock().await.files.sessions()
    }

    pub async fn stop(&self) -> Result<()> {
        let _ = self.inner.shutdown_tx.send(true);
        {
            let mut state = self.inner.state.lock().await;
            state
                .shutdown(self.inner.transport.as_ref(), &self.inner.events)
                .await?;
        }
        if let Some(task) = self.inner.network_task.lock().await.take() {
            let _ = task.await;
        }
        if let Some(task) = self.inner.maintenance_task.lock().await.take() {
            let _ = task.await;
        }
        Ok(())
    }
}

async fn run_network_loop(inner: Arc<KayaCoreInner>, mut shutdown_rx: watch::Receiver<bool>) {
    let recv_timeout = Duration::from_millis(250);
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    break;
                }
            }
            received = time::timeout(recv_timeout, inner.transport.recv_packet()) => {
                match received {
                    Ok(Ok((packet, source, bytes))) => {
                        let mut state = inner.state.lock().await;
                        state.handle_packet_received(inner.transport.as_ref(), &inner.events, packet, source, bytes).await;
                    }
                    Ok(Err(err)) => publish(&inner.events, KayaEvent::ErrorOccurred {
                        scope: "transport.rx".into(),
                        message: err.to_string(),
                    }),
                    Err(_) => {}
                }
            }
        }
    }
}

async fn run_maintenance_loop(inner: Arc<KayaCoreInner>, mut shutdown_rx: watch::Receiver<bool>) {
    let heartbeat_every = {
        let state = inner.state.lock().await;
        Duration::from_secs(state.config.heartbeat_interval_secs)
    };
    let mut heartbeat = time::interval(heartbeat_every);
    let mut prune = time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    break;
                }
            }
            _ = heartbeat.tick() => {
                let mut state = inner.state.lock().await;
                let heartbeat_packet = state.heartbeat_packet();
                let route_packet = state.route_announce_packet();
                state.send_packet(inner.transport.as_ref(), &inner.events, heartbeat_packet).await;
                state.send_packet(inner.transport.as_ref(), &inner.events, route_packet).await;
            }
            _ = prune.tick() => {
                let mut state = inner.state.lock().await;
                state.expire_idle_state(&inner.events);
            }
        }
    }
}

fn file_transfer_config(config: &PersistedConfig) -> FileTransferConfig {
    FileTransferConfig {
        enabled: config.file_transfer.enabled,
        max_file_size_bytes: config.file_transfer.max_file_size_mb * 1024 * 1024,
        chunk_size: (config.file_transfer.chunk_size_kb * 1024) as usize,
        accept_from_unknown: config.file_transfer.accept_from_unknown,
        download_dir: config.file_transfer.download_dir.clone(),
    }
}

fn mesh_policy(config: &PersistedConfig) -> MeshPolicy {
    MeshPolicy {
        enabled: config.mesh.enabled,
        max_ttl: config.mesh.max_ttl,
        allow_relay_for_unknown: config.mesh.allow_relay_for_unknown,
        allow_relay_for_blocked: config.mesh.allow_relay_for_blocked,
        relay_encrypted_only: config.mesh.relay_encrypted_only,
        route_expiry_seconds: config.mesh.route_expiry_seconds,
        max_seen_packets: config.mesh.max_seen_packets,
    }
}

fn transport_config(config: &PersistedConfig) -> Result<TransportConfig> {
    let multicast_ip: Ipv4Addr = config
        .multicast_address
        .parse()
        .map_err(|err| KayaError::Config(format!("invalid multicast address: {err}")))?;
    Ok(TransportConfig {
        multicast_ip,
        port: config.multicast_port,
        loopback: true,
        max_packet_bytes: config.packet_max_bytes,
    })
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

fn file_chunk_from_packet(packet: &Packet) -> Result<FileChunk> {
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

fn publish(events: &broadcast::Sender<KayaEvent>, event: KayaEvent) {
    let _ = events.send(event);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn creates_core_with_mock_transport() {
        let (transport, _) = MockTransport::pair();
        let temp = tempdir().unwrap();
        let config = CoreConfig {
            data_dir: Some(temp.path().to_path_buf()),
            ..CoreConfig::default()
        };

        let core = KayaCore::with_transport(config, transport).await.unwrap();
        assert!(!core.node_id().await.is_empty());
        core.stop().await.unwrap();
    }
}
