use kaya_commands::{help_text, parse_input, Command, ParsedInput};
use kaya_peer::{PeerEvent, PeerRegistry};
use kaya_persistence::{HistoryRecord, KnownPeer, LocalConfig, Store};
use kaya_protocol::{Packet, PacketType};
use kaya_rooms::{ChatMessage, RoomStore, RouteOutcome};
use kaya_shared::{normalize_callsign, now_millis, NodeId, Result, HEARTBEAT_INTERVAL_SECS};
use kaya_transport::MulticastTransport;
use kaya_ui::{TerminalUi, UiMessage, UiPeer, UiState};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

pub async fn run() -> Result<()> {
    init_tracing();

    let store = Store::open_default()?;
    let mut config = store.load_config()?;
    let callsign = prompt_callsign(config.callsign.as_deref())?;
    config.callsign = Some(callsign.clone());
    store.save_config(&config)?;

    let node_id = NodeId::generate().to_string();
    let transport = MulticastTransport::bind_default().await?;
    let mut runtime = Runtime::new(node_id, callsign, transport, store, config);

    runtime.bootstrap().await?;
    runtime.run_loop().await?;
    runtime.shutdown().await
}

struct Runtime {
    node_id: String,
    callsign: String,
    transport: MulticastTransport,
    peers: PeerRegistry,
    rooms: RoomStore,
    store: Store,
    config: LocalConfig,
    ui_state: UiState,
}

impl Runtime {
    fn new(
        node_id: String,
        callsign: String,
        transport: MulticastTransport,
        store: Store,
        config: LocalConfig,
    ) -> Self {
        let mut rooms = RoomStore::new(&node_id, &callsign);
        if !config.last_room.trim().is_empty() {
            rooms.join(&config.last_room);
        }

        let ui_state = UiState::new(&node_id, &callsign, rooms.current_room());
        Self {
            peers: PeerRegistry::new(&node_id),
            node_id,
            callsign,
            transport,
            rooms,
            store,
            config,
            ui_state,
        }
    }

    async fn bootstrap(&mut self) -> Result<()> {
        self.ui_state.push_log(format!(
            "node {} initialized as {}",
            self.node_id, self.callsign
        ));
        self.ui_state
            .push_log(format!("joined room #{}", self.rooms.current_room()));
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
        Ok(())
    }

    async fn run_loop(&mut self) -> Result<()> {
        let mut ui = TerminalUi::enter()?;
        let mut heartbeat = time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        let mut render = time::interval(Duration::from_millis(33));
        let mut prune = time::interval(Duration::from_secs(1));
        let rx = self.transport.clone();

        loop {
            tokio::select! {
                received = rx.recv_packet() => {
                    match received {
                        Ok((packet, _addr)) => self.handle_packet(packet).await,
                        Err(err) => {
                            error!(%err, "packet receive failed");
                            self.ui_state.push_log(format!("rx error: {err}"));
                        }
                    }
                }
                _ = heartbeat.tick() => {
                    let packet = Packet::heartbeat(
                        self.node_id.clone(),
                        self.callsign.clone(),
                        self.rooms.current_room().to_string(),
                    );
                    self.send_packet(packet).await;
                }
                _ = prune.tick() => {
                    for event in self.peers.prune() {
                        self.handle_peer_event(event);
                    }
                    self.sync_peers_to_ui();
                }
                _ = render.tick() => {
                    ui.draw(&self.ui_state)?;
                    if let Some(input) = ui.poll_input(&mut self.ui_state, Duration::from_millis(0))? {
                        if self.handle_input(input).await? {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.config.last_room = self.rooms.current_room().to_string();
        self.store.save_config(&self.config)?;
        self.send_packet(Packet::leave(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
        info!("kaya node shutdown");
        Ok(())
    }

    async fn handle_input(&mut self, input: String) -> Result<bool> {
        match parse_input(&input) {
            Ok(ParsedInput::Empty) => Ok(false),
            Ok(ParsedInput::Message(body)) => {
                self.send_room_message(body).await;
                Ok(false)
            }
            Ok(ParsedInput::Command(command)) => self.handle_command(command).await,
            Err(err) => {
                self.system_message(format!("{err}"));
                self.ui_state.push_log(format!("command error: {err}"));
                Ok(false)
            }
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Help => self.system_message(help_text()),
            Command::Who => self.show_who(),
            Command::Rooms => {
                self.system_message(format!("rooms: {}", self.rooms.room_names().join(", ")))
            }
            Command::Join { room } | Command::Room { room: Some(room) } => {
                self.join_room(&room).await;
            }
            Command::Room { room: None } => {
                self.system_message(format!("current room: #{}", self.rooms.current_room()));
            }
            Command::Msg { target, body } => self.send_direct_message(target, body).await,
            Command::Status => self.show_status(),
            Command::Logs => {
                self.ui_state.show_logs = !self.ui_state.show_logs;
                self.ui_state
                    .push_log(format!("logs visible: {}", self.ui_state.show_logs));
            }
            Command::Clear => {
                self.ui_state.clear_messages();
                self.ui_state.push_log("traffic panel cleared");
            }
            Command::Exit => return Ok(true),
        }

        Ok(false)
    }

    async fn join_room(&mut self, room: &str) {
        self.rooms.join(room);
        self.config.last_room = self.rooms.current_room().to_string();
        self.ui_state.current_room = self.rooms.current_room().to_string();
        self.ui_state.space = self.rooms.current_room().to_string();
        self.ui_state
            .push_log(format!("joined room #{}", self.rooms.current_room()));
        self.system_message(format!("joined #{}", self.rooms.current_room()));
        self.send_packet(Packet::join_room(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
        ))
        .await;
    }

    async fn send_room_message(&mut self, body: String) {
        let message = self.rooms.add_local_room_message(body.clone());
        self.push_chat_message(&message, true);
        self.persist_chat_message(&message);
        self.send_packet(Packet::room_message(
            self.node_id.clone(),
            self.callsign.clone(),
            self.rooms.current_room().to_string(),
            body,
        ))
        .await;
    }

    async fn send_direct_message(&mut self, target: String, body: String) {
        let target_node = self
            .peers
            .resolve_target(&target)
            .map(|peer| peer.node_id.clone())
            .unwrap_or_else(|| target.clone());

        let message = self
            .rooms
            .add_local_direct_message(target.clone(), body.clone());
        self.push_chat_message(&message, true);
        self.persist_chat_message(&message);
        self.send_packet(Packet::direct_message(
            self.node_id.clone(),
            self.callsign.clone(),
            target_node,
            body,
        ))
        .await;
    }

    async fn handle_packet(&mut self, packet: Packet) {
        if packet.node_id == self.node_id {
            return;
        }

        self.ui_state.packets_rx += 1;
        debug!(packet_type = ?packet.packet_type, node_id = %packet.node_id, "handling packet");

        if let Some(event) = self.peers.observe_packet(&packet) {
            self.handle_peer_event(event);
        }
        self.remember_peer(&packet);
        self.sync_peers_to_ui();

        match self.rooms.route_packet(&packet) {
            RouteOutcome::RoomMessage(message) => {
                if message.room.as_deref() == Some(self.rooms.current_room()) {
                    self.push_chat_message(&message, false);
                }
                self.persist_chat_message(&message);
            }
            RouteOutcome::DirectMessage(message) => {
                self.push_chat_message(&message, false);
                self.persist_chat_message(&message);
            }
            RouteOutcome::Joined { node_id, room } => {
                if matches!(packet.packet_type, PacketType::Hello | PacketType::JoinRoom) {
                    self.ui_state
                        .push_log(format!("peer {node_id} present in #{room}"));
                }
            }
            RouteOutcome::Left { node_id } => {
                self.ui_state.push_log(format!("peer {node_id} left"));
            }
            RouteOutcome::Ignored => {}
        }

        if packet.packet_type == PacketType::Hello {
            let pong = Packet::pong(
                self.node_id.clone(),
                self.callsign.clone(),
                packet.node_id.clone(),
                packet.packet_id,
            );
            self.send_packet(pong).await;
        }
    }

    async fn send_packet(&mut self, packet: Packet) {
        match self.transport.send_packet(&packet).await {
            Ok(_) => {
                self.ui_state.packets_tx += 1;
                self.ui_state
                    .push_log(format!("tx {:?} {}", packet.packet_type, packet.packet_id));
            }
            Err(err) => {
                error!(%err, "packet send failed");
                self.ui_state.push_log(format!("tx error: {err}"));
            }
        }
    }

    fn handle_peer_event(&mut self, event: PeerEvent) {
        match event {
            PeerEvent::Discovered(node_id) => {
                self.ui_state.push_log(format!("peer discovered {node_id}"))
            }
            PeerEvent::Left(node_id) => self.ui_state.push_log(format!("peer offline {node_id}")),
            PeerEvent::TimedOut(node_id) => {
                self.ui_state.push_log(format!("peer timeout {node_id}"))
            }
            PeerEvent::Updated(_) => {}
        }
    }

    fn show_who(&mut self) {
        let peers = self.peers.snapshots();
        if peers.is_empty() {
            self.system_message("no peers discovered");
            return;
        }

        let summary = peers
            .into_iter()
            .map(|peer| {
                let status = if peer.online { "online" } else { "offline" };
                format!("{} {} {}", peer.callsign, peer.node_id, status)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(summary);
    }

    fn show_status(&mut self) {
        self.system_message(format!(
            "node={} room=#{} peers={} packets_tx={} packets_rx={}",
            self.node_id,
            self.rooms.current_room(),
            self.peers.online_count(),
            self.ui_state.packets_tx,
            self.ui_state.packets_rx
        ));
    }

    fn system_message(&mut self, body: impl Into<String>) {
        self.ui_state.push_message(UiMessage {
            room: Some(self.rooms.current_room().to_string()),
            from: "system".into(),
            target: None,
            body: body.into(),
            direct: false,
            local: false,
        });
    }

    fn push_chat_message(&mut self, message: &ChatMessage, local: bool) {
        self.ui_state.push_message(UiMessage {
            room: message.room.clone(),
            from: message.from_callsign.clone(),
            target: message.target_node.clone(),
            body: message.body.clone(),
            direct: message.direct,
            local,
        });
    }

    fn persist_chat_message(&mut self, message: &ChatMessage) {
        let record = HistoryRecord {
            timestamp: now_millis().to_string(),
            room: message.room.clone(),
            from: message.from_callsign.clone(),
            body: message.body.clone(),
            direct: message.direct,
        };
        if let Err(err) = self.store.append_history(&record) {
            self.ui_state.push_log(format!("history error: {err}"));
        }
    }

    fn remember_peer(&mut self, packet: &Packet) {
        let peer = KnownPeer {
            node_id: packet.node_id.clone(),
            callsign: packet.callsign.clone(),
            last_seen: packet.timestamp.clone(),
        };
        if let Err(err) = self.store.remember_peer(&peer) {
            self.ui_state.push_log(format!("peer cache error: {err}"));
        }
    }

    fn sync_peers_to_ui(&mut self) {
        self.ui_state.peers = self
            .peers
            .snapshots()
            .into_iter()
            .map(|peer| UiPeer {
                node_id: peer.node_id,
                callsign: peer.callsign,
                online: peer.online,
                latency_ms: peer.latency_ms,
            })
            .collect();
    }
}

fn prompt_callsign(default: Option<&str>) -> Result<String> {
    let mut stdout = io::stdout();
    match default {
        Some(value) if !value.trim().is_empty() => {
            write!(stdout, "KAYA callsign [{value}]: ")?;
        }
        _ => {
            write!(stdout, "KAYA callsign: ")?;
        }
    }
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let callsign = normalize_callsign(&input);
    if !callsign.is_empty() {
        return Ok(callsign);
    }

    if let Some(default) = default {
        let default = normalize_callsign(default);
        if !default.is_empty() {
            return Ok(default);
        }
    }

    Ok("operator".into())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kaya=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
