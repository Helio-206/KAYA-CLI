use super::Runtime;
use kaya_direct::{
    read_packet_frame, validate_hello, write_packet, ConnectionState, DirectConnectionView,
    DirectError, TransportType,
};
use kaya_events::KayaEvent;
use kaya_protocol::{Packet, PacketType};
use kaya_security::{sign_packet, LocalIdentity};
use kaya_shared::{PresenceStatus, PROTOCOL_VERSION};
use serde_json::json;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tracing::{debug, warn};

const DIRECT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct DirectConnectionHandle {
    pub view: DirectConnectionView,
    sender: mpsc::UnboundedSender<Packet>,
    reader_task: JoinHandle<()>,
    writer_task: JoinHandle<()>,
}

pub(super) enum DirectRuntimeEvent {
    ListenerStopped {
        addr: Option<String>,
        reason: Option<String>,
    },
    ConnectFailed {
        address: String,
        reason: String,
    },
    PeerConnected {
        view: DirectConnectionView,
        sender: mpsc::UnboundedSender<Packet>,
        reader_task: JoinHandle<()>,
        writer_task: JoinHandle<()>,
        hello_packet: Packet,
        source: String,
        bytes: usize,
    },
    PeerDisconnected {
        node_id: String,
        reason: Option<String>,
    },
    PacketReceived {
        node_id: String,
        packet: Packet,
        source: String,
        bytes: usize,
    },
}

impl Runtime {
    pub(super) async fn start_direct_listener(&mut self, port: u16) {
        if self.direct_listener_task.is_some() {
            self.system_message(format!(
                "direct listener already active on {}",
                self.direct_listener_addr.as_deref().unwrap_or("--")
            ));
            return;
        }

        let bind_addr = format!("0.0.0.0:{port}");
        let listener = match kaya_direct::bind(&bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                self.publish(KayaEvent::DirectConnectFailed {
                    address: bind_addr,
                    reason: err.to_string(),
                });
                return;
            }
        };
        let addr = listener
            .local_addr()
            .map(|addr| addr.to_string())
            .unwrap_or(bind_addr);
        let direct_tx = self.direct_tx.clone();
        let node_id = self.node_id.clone();
        let callsign = self.callsign.clone();
        let room = self.rooms.current_room().to_string();
        let identity = self.identity.clone();
        let fingerprint = self.identity.fingerprint.clone();
        let listener_addr = addr.clone();

        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_addr)) => {
                        let tx = direct_tx.clone();
                        let node_id = node_id.clone();
                        let callsign = callsign.clone();
                        let room = room.clone();
                        let identity = identity.clone();
                        let fingerprint = fingerprint.clone();
                        tokio::spawn(async move {
                            if let Err(err) = inbound_session(
                                stream,
                                node_id,
                                callsign,
                                room,
                                identity,
                                fingerprint,
                                tx.clone(),
                            )
                            .await
                            {
                                let _ = tx.send(DirectRuntimeEvent::ConnectFailed {
                                    address: remote_addr.to_string(),
                                    reason: err.to_string(),
                                });
                            }
                        });
                    }
                    Err(err) => {
                        let _ = direct_tx.send(DirectRuntimeEvent::ListenerStopped {
                            addr: Some(listener_addr.clone()),
                            reason: Some(err.to_string()),
                        });
                        break;
                    }
                }
            }
        });

        self.direct_listener_addr = Some(addr.clone());
        self.direct_listener_task = Some(task);
        self.publish(KayaEvent::DirectListenerStarted { addr: addr.clone() });
        self.system_message(format!("direct listener active on {addr}"));
        self.sync_direct_to_ui();
    }

    pub(super) fn stop_direct_listener(&mut self) {
        let addr = self.direct_listener_addr.take();
        if let Some(task) = self.direct_listener_task.take() {
            task.abort();
            self.publish(KayaEvent::DirectListenerStopped { addr: addr.clone() });
            self.sync_direct_to_ui();
        }
    }

    pub(super) async fn connect_direct(&mut self, address: &str) {
        let address = address.to_string();
        let direct_tx = self.direct_tx.clone();
        let node_id = self.node_id.clone();
        let callsign = self.callsign.clone();
        let room = self.rooms.current_room().to_string();
        let identity = self.identity.clone();
        let fingerprint = self.identity.fingerprint.clone();

        self.system_message(format!("direct dialing {address}"));
        tokio::spawn(async move {
            if let Err(err) = outbound_session(
                address.clone(),
                node_id,
                callsign,
                room,
                identity,
                fingerprint,
                direct_tx.clone(),
            )
            .await
            {
                let _ = direct_tx.send(DirectRuntimeEvent::ConnectFailed {
                    address,
                    reason: err.to_string(),
                });
            }
        });
    }

    pub(super) fn disconnect_direct(&mut self, target: &str) {
        let Some(node_id) = self.resolve_direct_connection_target(target) else {
            self.system_message(format!("direct connection not found: {target}"));
            return;
        };
        self.disconnect_direct_node(&node_id, Some("operator disconnect".into()));
    }

    pub(super) fn show_direct_connections(&mut self) {
        if self.direct_connections.is_empty() {
            self.system_message("direct connections: none");
            return;
        }
        let summary = self
            .direct_connections
            .values()
            .take(8)
            .map(|connection| {
                format!(
                    "{} {} {} {}",
                    connection.view.peer_callsign,
                    connection.view.peer_node_id,
                    connection.view.transport_type,
                    connection.view.connection_state
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.system_message(format!("direct connections: {summary}"));
    }

    pub(super) fn show_direct_listener_status(&mut self) {
        self.system_message(format!(
            "direct listener active={} addr={} connections={}",
            self.direct_listener_task.is_some(),
            self.direct_listener_addr.as_deref().unwrap_or("--"),
            self.direct_connections.len()
        ));
    }

    pub(super) async fn handle_direct_runtime_event(&mut self, event: DirectRuntimeEvent) {
        match event {
            DirectRuntimeEvent::ListenerStopped { addr, reason } => {
                if self.direct_listener_addr == addr {
                    self.direct_listener_addr = None;
                    self.direct_listener_task = None;
                }
                self.publish(KayaEvent::DirectListenerStopped { addr });
                if let Some(reason) = reason {
                    self.publish(KayaEvent::ErrorOccurred {
                        scope: "direct.listener".into(),
                        message: reason,
                    });
                }
                self.sync_direct_to_ui();
            }
            DirectRuntimeEvent::ConnectFailed { address, reason } => {
                self.publish(KayaEvent::DirectConnectFailed { address, reason });
            }
            DirectRuntimeEvent::PeerConnected {
                view,
                sender,
                reader_task,
                writer_task,
                hello_packet,
                source,
                bytes,
            } => {
                if view.peer_node_id == self.node_id {
                    reader_task.abort();
                    writer_task.abort();
                    self.publish(KayaEvent::DirectConnectFailed {
                        address: view.remote_addr,
                        reason: "loopback direct connection rejected".into(),
                    });
                    return;
                }
                if self.trust_store.is_blocked(&view.peer_node_id) {
                    reader_task.abort();
                    writer_task.abort();
                    self.publish(KayaEvent::DirectConnectFailed {
                        address: view.remote_addr,
                        reason: "blocked peer rejected".into(),
                    });
                    return;
                }
                if self.direct_connections.contains_key(&view.peer_node_id) {
                    reader_task.abort();
                    writer_task.abort();
                    self.publish(KayaEvent::DirectConnectFailed {
                        address: view.remote_addr,
                        reason: format!("duplicate connection for {}", view.peer_node_id),
                    });
                    return;
                }

                self.direct_connections.insert(
                    view.peer_node_id.clone(),
                    DirectConnectionHandle {
                        view: view.clone(),
                        sender,
                        reader_task,
                        writer_task,
                    },
                );
                self.publish(KayaEvent::DirectPeerConnected {
                    node_id: view.peer_node_id.clone(),
                    callsign: view.peer_callsign.clone(),
                    addr: view.remote_addr.clone(),
                });
                self.sync_direct_to_ui();
                self.handle_packet_received(hello_packet, source, bytes)
                    .await;
            }
            DirectRuntimeEvent::PeerDisconnected { node_id, reason } => {
                self.disconnect_direct_node(&node_id, reason);
            }
            DirectRuntimeEvent::PacketReceived {
                node_id,
                packet,
                source,
                bytes,
            } => {
                self.publish(KayaEvent::DirectPacketReceived {
                    node_id,
                    packet_id: packet.packet_id,
                    packet_type: packet.packet_type,
                    source: source.clone(),
                    bytes,
                });
                self.handle_packet_received(packet, source, bytes).await;
            }
        }
    }

    pub(super) fn send_packet_via_direct_tcp(
        &mut self,
        destination_node: &str,
        packet: &mut Packet,
    ) -> bool {
        let Some(sender) = self
            .direct_connections
            .get(destination_node)
            .map(|connection| connection.sender.clone())
        else {
            return false;
        };

        if packet.signature.is_none() {
            if let Err(err) = sign_packet(packet, &self.identity) {
                self.publish(KayaEvent::SecurityWarning {
                    node_id: Some(self.node_id.clone()),
                    message: format!("direct packet signing failed: {err}"),
                });
                return false;
            }
        }

        let bytes = encoded_packet_len(packet);
        match sender.send(packet.clone()) {
            Ok(()) => {
                self.publish(KayaEvent::DirectPacketSent {
                    node_id: destination_node.to_string(),
                    packet_id: packet.packet_id,
                    packet_type: packet.packet_type,
                    bytes,
                });
                true
            }
            Err(err) => {
                self.disconnect_direct_node(
                    destination_node,
                    Some(format!("direct send failed: {err}")),
                );
                false
            }
        }
    }

    pub(super) fn mirror_packet_to_direct(&mut self, packet: &Packet) {
        if !mirrors_over_direct(packet.packet_type) {
            return;
        }

        let peers = self
            .direct_connections
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        for peer in peers {
            let mut packet = packet.clone();
            let _ = self.send_packet_via_direct_tcp(&peer, &mut packet);
        }
    }

    pub(super) fn direct_peer_connected(&self, node_id: &str) -> bool {
        self.direct_connections
            .get(node_id)
            .map(|connection| connection.view.connection_state == ConnectionState::Connected)
            .unwrap_or(false)
    }

    pub(super) fn close_all_direct_connections(&mut self, reason: &str) {
        let nodes = self
            .direct_connections
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        for node_id in nodes {
            self.disconnect_direct_node(&node_id, Some(reason.to_string()));
        }
    }

    fn disconnect_direct_node(&mut self, node_id: &str, reason: Option<String>) {
        let Some(connection) = self.direct_connections.remove(node_id) else {
            return;
        };
        connection.reader_task.abort();
        connection.writer_task.abort();
        self.publish(KayaEvent::DirectPeerDisconnected {
            node_id: node_id.to_string(),
            reason,
        });
        self.sync_direct_to_ui();
        self.sync_peers_to_ui();
    }

    fn resolve_direct_connection_target(&mut self, target: &str) -> Option<String> {
        if self.direct_connections.contains_key(target) {
            return Some(target.to_string());
        }

        let matches = self
            .direct_connections
            .values()
            .filter(|connection| connection.view.peer_callsign.eq_ignore_ascii_case(target))
            .map(|connection| connection.view.peer_node_id.clone())
            .collect::<Vec<_>>();

        match matches.len() {
            0 => None,
            1 => matches.into_iter().next(),
            _ => {
                self.system_message(format!(
                    "callsign {target} is ambiguous: {}",
                    matches.join(", ")
                ));
                None
            }
        }
    }

    pub(super) fn sync_direct_to_ui(&mut self) {
        self.ui_state.direct_listener = self.direct_listener_addr.clone();
        self.ui_state.connections = self
            .direct_connections
            .values()
            .map(|connection| kaya_ui::UiConnection {
                peer_node_id: connection.view.peer_node_id.clone(),
                peer_callsign: connection.view.peer_callsign.clone(),
                transport_type: connection.view.transport_type.to_string(),
                remote_addr: connection.view.remote_addr.clone(),
                state: connection.view.connection_state.to_string(),
                latency_ms: connection.view.latency_ms,
                encrypted_capable: connection.view.encrypted_capable,
            })
            .collect();
    }
}

async fn outbound_session(
    address: String,
    node_id: String,
    callsign: String,
    room: String,
    identity: LocalIdentity,
    fingerprint: String,
    direct_tx: mpsc::UnboundedSender<DirectRuntimeEvent>,
) -> Result<(), DirectError> {
    let stream = time::timeout(DIRECT_HANDSHAKE_TIMEOUT, kaya_direct::connect(&address))
        .await
        .map_err(|_| DirectError::InvalidHandshake("connect timed out".into()))??;
    stream.set_nodelay(true)?;
    let local_addr = stream.local_addr()?.to_string();
    let remote_addr = stream.peer_addr()?.to_string();
    let (mut reader, mut writer) = stream.into_split();

    let mut hello = direct_hello_packet(&node_id, &callsign, &room, &fingerprint);
    sign_packet(&mut hello, &identity).map_err(|err| DirectError::Protocol(err.to_string()))?;
    write_packet(&mut writer, &hello).await?;

    let (peer_hello, bytes) =
        time::timeout(DIRECT_HANDSHAKE_TIMEOUT, read_packet_frame(&mut reader))
            .await
            .map_err(|_| DirectError::InvalidHandshake("hello timed out".into()))??;
    let peer = validate_hello(&peer_hello)?;
    let encrypted_capable = peer
        .capabilities
        .iter()
        .any(|capability| capability == "encrypted_dm");
    start_session_tasks(
        reader,
        writer,
        peer.node_id,
        peer.callsign,
        local_addr,
        remote_addr,
        encrypted_capable,
        peer_hello,
        format!("direct_tcp:{address}"),
        bytes,
        direct_tx,
    )
}

async fn inbound_session(
    stream: TcpStream,
    node_id: String,
    callsign: String,
    room: String,
    identity: LocalIdentity,
    fingerprint: String,
    direct_tx: mpsc::UnboundedSender<DirectRuntimeEvent>,
) -> Result<(), DirectError> {
    stream.set_nodelay(true)?;
    let local_addr = stream.local_addr()?.to_string();
    let remote_addr = stream.peer_addr()?.to_string();
    let (mut reader, mut writer) = stream.into_split();

    let (peer_hello, bytes) =
        time::timeout(DIRECT_HANDSHAKE_TIMEOUT, read_packet_frame(&mut reader))
            .await
            .map_err(|_| DirectError::InvalidHandshake("hello timed out".into()))??;
    let peer = validate_hello(&peer_hello)?;
    let mut hello = direct_hello_packet(&node_id, &callsign, &room, &fingerprint);
    sign_packet(&mut hello, &identity).map_err(|err| DirectError::Protocol(err.to_string()))?;
    write_packet(&mut writer, &hello).await?;

    let encrypted_capable = peer
        .capabilities
        .iter()
        .any(|capability| capability == "encrypted_dm");
    start_session_tasks(
        reader,
        writer,
        peer.node_id,
        peer.callsign,
        local_addr,
        remote_addr.clone(),
        encrypted_capable,
        peer_hello,
        format!("direct_tcp:{remote_addr}"),
        bytes,
        direct_tx,
    )
}

#[allow(clippy::too_many_arguments)]
fn start_session_tasks(
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    peer_node_id: String,
    peer_callsign: String,
    local_addr: String,
    remote_addr: String,
    encrypted_capable: bool,
    hello_packet: Packet,
    source: String,
    bytes: usize,
    direct_tx: mpsc::UnboundedSender<DirectRuntimeEvent>,
) -> Result<(), DirectError> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let reader_task = tokio::spawn(reader_loop(
        reader,
        peer_node_id.clone(),
        source.clone(),
        direct_tx.clone(),
    ));
    let writer_task = tokio::spawn(writer_loop(
        writer,
        peer_node_id.clone(),
        remote_addr.clone(),
        receiver,
        direct_tx.clone(),
    ));
    let view = DirectConnectionView {
        peer_node_id: peer_node_id.clone(),
        peer_callsign,
        local_addr,
        remote_addr,
        connected_at_ms: kaya_shared::now_millis(),
        transport_type: TransportType::DirectTcp,
        latency_ms: None,
        encrypted_capable,
        connection_state: ConnectionState::Connected,
    };

    direct_tx
        .send(DirectRuntimeEvent::PeerConnected {
            view,
            sender,
            reader_task,
            writer_task,
            hello_packet,
            source,
            bytes,
        })
        .map_err(|err| DirectError::ChannelClosed(err.to_string()))
}

async fn reader_loop(
    mut reader: OwnedReadHalf,
    node_id: String,
    source: String,
    direct_tx: mpsc::UnboundedSender<DirectRuntimeEvent>,
) {
    loop {
        match read_packet_frame(&mut reader).await {
            Ok((packet, bytes)) => {
                let _ = direct_tx.send(DirectRuntimeEvent::PacketReceived {
                    node_id: node_id.clone(),
                    packet,
                    source: source.clone(),
                    bytes,
                });
            }
            Err(err) => {
                let _ = direct_tx.send(DirectRuntimeEvent::PeerDisconnected {
                    node_id,
                    reason: Some(err.to_string()),
                });
                break;
            }
        }
    }
}

async fn writer_loop(
    mut writer: OwnedWriteHalf,
    node_id: String,
    remote_addr: String,
    mut receiver: mpsc::UnboundedReceiver<Packet>,
    direct_tx: mpsc::UnboundedSender<DirectRuntimeEvent>,
) {
    while let Some(packet) = receiver.recv().await {
        match write_packet(&mut writer, &packet).await {
            Ok(bytes) => {
                debug!(
                    node_id = %node_id,
                    remote_addr = %remote_addr,
                    packet_id = %packet.packet_id,
                    packet_type = ?packet.packet_type,
                    bytes,
                    "DIRECT_PACKET_SENT"
                );
            }
            Err(err) => {
                warn!(%node_id, %remote_addr, %err, "DIRECT_PACKET_SEND_FAILED");
                let _ = direct_tx.send(DirectRuntimeEvent::PeerDisconnected {
                    node_id,
                    reason: Some(err.to_string()),
                });
                break;
            }
        }
    }
}

fn direct_hello_packet(node_id: &str, callsign: &str, room: &str, fingerprint: &str) -> Packet {
    let mut packet = Packet::hello(node_id, callsign, room);
    packet.payload = json!({
        "agent": "kaya-cli",
        "protocol_version": PROTOCOL_VERSION,
        "fingerprint": fingerprint,
        "presence": PresenceStatus::Online.as_str(),
        "capabilities": [
            "direct_tcp",
            "encrypted_dm",
            "file_transfer",
            "mesh",
            "relay",
            "rooms",
            "presence"
        ],
    });
    packet
}

fn mirrors_over_direct(packet_type: PacketType) -> bool {
    matches!(
        packet_type,
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
            | PacketType::RouteAnnounce
            | PacketType::RouteRequest
    )
}

fn encoded_packet_len(packet: &Packet) -> usize {
    serde_json::to_vec(packet)
        .map(|bytes| bytes.len() + 4)
        .unwrap_or_default()
}
