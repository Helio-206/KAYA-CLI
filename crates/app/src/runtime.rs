mod events;
mod input;
mod network;
mod presentation;

use crate::diagnostics::RuntimeDiagnostics;
use kaya_commands::CommandRegistry;
use kaya_events::{EventBus, KayaEvent};
use kaya_peer::PeerRegistry;
use kaya_persistence::{ConfigStore, KayaConfig, Store};
use kaya_shared::Result;
use kaya_transport::{MulticastTransport, PacketDeduplicator};
use kaya_ui::{TerminalUi, UiState};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::info;

pub struct Runtime {
    node_id: String,
    callsign: String,
    transport: MulticastTransport,
    bus: EventBus,
    event_rx: broadcast::Receiver<KayaEvent>,
    peers: PeerRegistry,
    rooms: kaya_rooms::RoomStore,
    store: Store,
    config_store: ConfigStore,
    config: KayaConfig,
    commands: CommandRegistry,
    ui_state: UiState,
    diagnostics: RuntimeDiagnostics,
    dedup: PacketDeduplicator,
    network_task: Option<JoinHandle<()>>,
}

impl Runtime {
    pub fn new(
        node_id: String,
        callsign: String,
        transport: MulticastTransport,
        store: Store,
        config_store: ConfigStore,
        config: KayaConfig,
        bus: EventBus,
    ) -> Self {
        let mut rooms = kaya_rooms::RoomStore::new(&node_id, &callsign);
        rooms.join(config.active_room());

        let diagnostics = RuntimeDiagnostics::new(
            config.heartbeat_interval_secs,
            config.peer_timeout_secs,
            config.packet_max_bytes,
        );
        let mut ui_state = UiState::new(&node_id, &callsign, rooms.current_room());
        ui_state.diagnostics = diagnostics.to_ui();

        Self {
            peers: PeerRegistry::with_timeout(
                &node_id,
                Duration::from_secs(config.peer_timeout_secs),
            ),
            node_id,
            callsign,
            transport,
            event_rx: bus.subscribe(),
            bus,
            rooms,
            store,
            config_store,
            config,
            commands: CommandRegistry::default(),
            ui_state,
            diagnostics,
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
                }
                _ = prune.tick() => {
                    for event in self.peers.prune() {
                        self.publish_peer_event(event);
                    }
                    self.sync_peers_to_ui();
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
