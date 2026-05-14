mod events;
mod files;
mod input;
mod mesh;
mod network;
mod presentation;

use crate::diagnostics::RuntimeDiagnostics;
use kaya_commands::CommandRegistry;
use kaya_events::{EventBus, KayaEvent};
use kaya_files::{FileStore, FileTransferConfig, FileTransferManager};
use kaya_mesh::{MeshPolicy, MeshState};
use kaya_peer::PeerRegistry;
use kaya_persistence::{ConfigStore, KayaConfig, Store};
use kaya_security::{LocalIdentity, SecureSessionManager, TrustStore};
use kaya_shared::{PresenceStatus, Result};
use kaya_transport::{MulticastTransport, PacketDeduplicator};
use kaya_ui::{TerminalUi, UiState};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::info;

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
    pending_secure_messages: HashMap<String, Vec<String>>,
    config_store: ConfigStore,
    config: KayaConfig,
    commands: CommandRegistry,
    ui_state: UiState,
    diagnostics: RuntimeDiagnostics,
    presence: PresenceStatus,
    dedup: PacketDeduplicator,
    network_task: Option<JoinHandle<()>>,
}

pub struct RuntimeInit {
    pub identity: LocalIdentity,
    pub identity_created: bool,
    pub transport: MulticastTransport,
    pub store: Store,
    pub config_store: ConfigStore,
    pub config: KayaConfig,
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
        let mut ui_state = UiState::new(&node_id, &callsign, rooms.current_room());
        ui_state.diagnostics = diagnostics.to_ui();
        ui_state.identity_fingerprint = identity.short_fingerprint();
        let mut files = FileTransferManager::new();
        if let Ok(records) = file_store.list_records() {
            for record in records {
                files.load_record(record.session);
            }
        }

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
            config_store,
            config,
            commands: CommandRegistry::default(),
            ui_state,
            diagnostics,
            presence: PresenceStatus::Online,
            dedup: PacketDeduplicator::new(4096),
            network_task: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.network_task = Some(self.spawn_network_reader());
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
                _ = heartbeat.tick() => {
                    self.send_packet(self.heartbeat_packet()).await;
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
                    self.sync_peers_to_ui();
                    self.sync_mesh_to_ui();
                }
                _ = render.tick() => {
                    self.ui_state.diagnostics = self.diagnostics.to_ui();
                    let started = Instant::now();
                    ui.draw(&self.ui_state)?;
                    self.diagnostics.render_time_ms = started.elapsed().as_millis() as u64;
                    if let Some(input) = ui.poll_input(&mut self.ui_state, Duration::from_millis(0))? {
                        if self.handle_input(input).await? {
                            self.publish(KayaEvent::ShutdownInitiated {
                                reason: "operator requested exit".into(),
                            });
                            break;
                        }
                    }
                }
            }
        }

        self.shutdown().await
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.send_packet(self.leave_packet()).await;
        self.config.last_room = Some(self.rooms.current_room().to_string());
        self.config_store.save(&self.config)?;

        if let Some(task) = self.network_task.take() {
            task.abort();
        }

        info!(node_id = %self.node_id, "kaya node shutdown");
        Ok(())
    }
}
