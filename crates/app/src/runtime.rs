mod demo;
mod direct;
mod events;
mod files;
mod input;
mod mesh;
mod network;
mod presentation;
mod voice;
mod voice_media;

use crate::diagnostics::RuntimeDiagnostics;
use kaya_commands::CommandRegistry;
use kaya_events::{EventBus, KayaEvent};
use kaya_files::{FileStore, FileTransferConfig, FileTransferManager};
use kaya_mesh::{MeshPolicy, MeshState};
use kaya_peer::PeerRegistry;
use kaya_persistence::{ConfigProfile, ConfigStore, KayaConfig, Store, TimeoutSettings};
use kaya_protocol::RelayPeerDescriptor;
use kaya_security::{LocalIdentity, SecureSessionManager, TrustStore};
use kaya_shared::{PresenceStatus, Result};
use kaya_transport::{MulticastTransport, PacketDeduplicator};
use kaya_ui::{TerminalAction, TerminalUi, UiState};
use kaya_voice::VoiceState;

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::info;
use voice_media::VoiceMediaRuntime;

#[derive(Debug, Clone)]
pub(super) struct PendingSecureQueue {
    pub queued_at_ms: u64,
    pub messages: Vec<String>,
}

pub struct Runtime {
    node_id: String,
    callsign: String,
    identity: LocalIdentity,
    identity_created: bool,
    transport: MulticastTransport,
    bus: EventBus,
    event_rx: broadcast::Receiver<KayaEvent>,
    peers: PeerRegistry,
    rooms: kaya_rooms::RoomStore,
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
    config: KayaConfig,
    profile: ConfigProfile,
    demo_mode: bool,
    timeouts: TimeoutSettings,
    commands: CommandRegistry,
    ui_state: UiState,
    voice: VoiceState,
    diagnostics: RuntimeDiagnostics,
    presence: PresenceStatus,
    dedup: PacketDeduplicator,
    network_task: Option<JoinHandle<()>>,
    relay_tx: Option<mpsc::UnboundedSender<kaya_protocol::Packet>>,
    relay_task: Option<JoinHandle<()>>,
    relay_connected: bool,
    relay_peers: HashMap<String, RelayPeerDescriptor>,
    direct_tx: mpsc::UnboundedSender<direct::DirectRuntimeEvent>,
    direct_rx: mpsc::UnboundedReceiver<direct::DirectRuntimeEvent>,
    direct_listener_task: Option<JoinHandle<()>>,
    direct_listener_addr: Option<String>,
    direct_connections: HashMap<String, direct::DirectConnectionHandle>,
    voice_media: Option<VoiceMediaRuntime>,
    network_shutdown_tx: Option<watch::Sender<bool>>,
}

pub struct RuntimeInit {
    pub identity: LocalIdentity,
    pub identity_created: bool,
    pub transport: MulticastTransport,
    pub store: Store,
    pub config_store: ConfigStore,
    pub config: KayaConfig,
    pub profile: ConfigProfile,
    pub demo_mode: bool,
    pub bus: EventBus,
    pub trust_store: TrustStore,
    pub file_store: FileStore,
    pub file_config: FileTransferConfig,
    pub mesh_policy: MeshPolicy,
}

impl Runtime {
    pub fn new(init: RuntimeInit) -> Self {
        let RuntimeInit {
            identity,
            identity_created,
            transport,
            store,
            config_store,
            config,
            profile,
            demo_mode,
            bus,
            trust_store,
            file_store,
            file_config,
            mesh_policy,
        } = init;
        let node_id = identity.node_id.clone();
        let callsign = identity.callsign.clone();
        let mut rooms = kaya_rooms::RoomStore::new(&node_id, &callsign);
        if rooms.join(config.active_room()).is_err() {
            let _ = rooms.join(kaya_shared::DEFAULT_ROOM);
        }

        let diagnostics = RuntimeDiagnostics::new(
            config.heartbeat_interval_secs,
            config.peer_timeout_secs,
            config.packet_max_bytes,
        );
        let timeouts = config.timeouts.clone();
        let mut ui_state = UiState::new(&node_id, &callsign, rooms.current_room());
        if demo_mode {
            ui_state.status = "DEMO".into();
        }
        ui_state.diagnostics = diagnostics.to_ui();
        ui_state.identity_fingerprint = identity.short_fingerprint();
        let mut files = FileTransferManager::new();
        if let Ok(records) = file_store.list_records() {
            for record in records {
                files.load_record(record.session);
            }
        }
        let (direct_tx, direct_rx) = mpsc::unbounded_channel();
        let voice = VoiceState::new(&config.voice);
        ui_state.voice.enabled = voice.enabled;

        Self {
            peers: PeerRegistry::with_timeout(
                &node_id,
                Duration::from_secs(config.peer_timeout_secs),
            ),
            identity: identity.clone(),
            identity_created,
            node_id: node_id.clone(),
            callsign,
            transport,
            event_rx: bus.subscribe(),
            bus,
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
            config,
            profile,
            demo_mode,
            timeouts,
            commands: CommandRegistry::default(),
            ui_state,
            voice,
            diagnostics,
            presence: PresenceStatus::Online,
            dedup: PacketDeduplicator::new(4096),
            network_task: None,
            relay_tx: None,
            relay_task: None,
            relay_connected: false,
            relay_peers: HashMap::new(),
            direct_tx,
            direct_rx,
            direct_listener_task: None,
            direct_listener_addr: None,
            direct_connections: HashMap::new(),
            voice_media: None,
            network_shutdown_tx: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.network_shutdown_tx = Some(shutdown_tx);
        self.network_task =
            Some(self.spawn_network_reader(shutdown_rx.clone(), self.timeouts.network_recv_ms));
        self.connect_relay(shutdown_rx).await;
        self.bootstrap().await;

        let mut ui = TerminalUi::enter()?;
        let mut heartbeat =
            time::interval(Duration::from_secs(self.config.heartbeat_interval_secs));
        let mut render = time::interval(Duration::from_millis(33));
        let mut prune = time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                received = self.event_rx.recv() => {
                    match received {
                        Ok(event) => self.handle_event(event).await,
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            self.ui_state.push_log(format!("event bus lagged by {skipped} events"));
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                direct_event = self.direct_rx.recv() => {
                    if let Some(event) = direct_event {
                        self.handle_direct_runtime_event(event).await;
                    }
                }
                voice_event = async {
                    match self.voice_media.as_mut() {
                        Some(runtime) => runtime.event_rx.recv().await,
                        None => std::future::pending::<Option<voice_media::VoiceRuntimeEvent>>().await,
                    }
                } => {
                    if let Some(event) = voice_event {
                        self.handle_voice_runtime_event(event).await;
                    }
                }
                _ = heartbeat.tick() => {
                    self.send_packet(self.heartbeat_packet()).await;
                    if let Some(packet) = self.voice_heartbeat_packet() {
                        self.send_packet(packet).await;
                    }
                    self.send_packet(self.route_announce_packet()).await;
                }
                _ = prune.tick() => {
                    for event in self.peers.prune() {
                        self.publish_peer_event(event);
                    }
                    for route in self.mesh.expire_routes() {
                        self.publish(KayaEvent::RouteExpired {
                            destination_node: route.destination_node,
                        });
                    }
                    self.maintain_timeouts();
                    self.sync_peers_to_ui();
                    self.sync_mesh_to_ui();
                }
                _ = render.tick() => {
                    if let Some(action) = ui.poll_input(&mut self.ui_state, Duration::from_millis(0))? {
                        match action {
                            TerminalAction::Submit(input) => {
                                if self.handle_input(input).await? {
                                    self.publish(KayaEvent::ShutdownInitiated {
                                        reason: "operator requested exit".into(),
                                    });
                                    break;
                                }
                            }
                            TerminalAction::VoicePttStart => self.set_voice_ptt_holding(true).await,
                            TerminalAction::VoicePttStop => self.set_voice_ptt_holding(false).await,
                        }
                    }
                    self.ui_state.diagnostics = self.diagnostics.to_ui();
                    let started = Instant::now();
                    ui.draw(&self.ui_state)?;
                    self.diagnostics.render_time_ms = started.elapsed().as_millis() as u64;
                }
            }
        }

        self.shutdown().await
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.stop_voice_media();
        self.send_packet(self.leave_packet()).await;
        self.config.last_room = Some(self.rooms.current_room().to_string());
        self.config_store.save(&self.config)?;
        self.store.flush()?;

        if let Some(tx) = self.network_shutdown_tx.take() {
            let _ = tx.send(true);
        }

        if let Some(task) = self.network_task.take() {
            match time::timeout(Duration::from_millis(self.timeouts.shutdown_ms), task).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) if err.is_cancelled() => {}
                Ok(Err(err)) => {
                    self.ui_state
                        .push_log(format!("network task join failed: {err}"));
                }
                Err(_) => {
                    self.ui_state.push_log("network shutdown timed out");
                }
            }
        }

        if let Some(relay_tx) = &self.relay_tx {
            let _ = relay_tx.send(kaya_protocol::Packet::relay_disconnect(
                self.node_id.clone(),
                self.callsign.clone(),
                "shutdown",
            ));
        }
        self.relay_tx = None;
        self.relay_connected = false;

        if let Some(task) = self.relay_task.take() {
            match time::timeout(Duration::from_millis(self.timeouts.shutdown_ms), task).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) if err.is_cancelled() => {}
                Ok(Err(err)) => {
                    self.ui_state
                        .push_log(format!("relay task join failed: {err}"));
                }
                Err(_) => {
                    self.ui_state.push_log("relay shutdown timed out");
                }
            }
        }

        self.stop_direct_listener();
        self.close_all_direct_connections("shutdown");

        info!(node_id = %self.node_id, "kaya node shutdown");
        Ok(())
    }

    fn maintain_timeouts(&mut self) {
        let now = kaya_shared::now_millis();
        let route_deadline = now.saturating_sub(self.timeouts.route_discovery_ms);
        let expired_routes: Vec<_> = self
            .pending_route_requests
            .iter()
            .filter(|(_, requested_at)| **requested_at < route_deadline)
            .map(|(destination, _)| destination.clone())
            .collect();
        for destination in expired_routes {
            self.pending_route_requests.remove(&destination);
            self.publish(KayaEvent::ErrorOccurred {
                scope: "mesh.route".into(),
                message: format!("route discovery timed out for {destination}"),
            });
        }

        let session_deadline = now.saturating_sub(self.timeouts.secure_session_ms);
        let expired_pending: Vec<_> = self
            .pending_secure_messages
            .iter()
            .filter(|(_, queue)| queue.queued_at_ms < session_deadline)
            .map(|(peer, _)| peer.clone())
            .collect();
        for peer in expired_pending {
            self.pending_secure_messages.remove(&peer);
            let _ = self.sessions.close(&peer);
            self.publish(KayaEvent::SecurityWarning {
                node_id: Some(peer.clone()),
                message: format!("secure session timed out for {peer}"),
            });
        }

        for session in self.sessions.expire_before(session_deadline) {
            self.publish(KayaEvent::SecurityWarning {
                node_id: Some(session.peer_node_id.clone()),
                message: format!("secure session expired for {}", session.peer_node_id),
            });
        }

        self.expire_stale_file_transfers(now.saturating_sub(self.timeouts.file_transfer_idle_ms));
        self.sync_security_to_ui();
    }
}
