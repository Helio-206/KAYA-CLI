use crate::diagnostics::RelayDiagnostics;
use crate::errors::{RelayError, RelayResult};
use crate::framing::{read_packet, write_packet};
use crate::policy::RelayPolicy;
use crate::registry::RelayRegistry;
use kaya_protocol::{Packet, PacketType, RelayForwardPayload, RelayRegisterPayload};
use kaya_shared::now_millis;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tracing::{info, warn};
use uuid::Uuid;

const RELAY_NODE_ID: &str = "KY-7E1A90";
const RELAY_CALLSIGN: &str = "relay";

#[derive(Clone)]
pub struct RelayServer {
    inner: Arc<RelayServerInner>,
}

struct RelayServerInner {
    listener: TcpListener,
    bind_addr: String,
    registry: Mutex<RelayRegistry>,
    policy: RelayPolicy,
    diagnostics: RelayDiagnostics,
}

pub struct RelayServerHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<RelayResult<()>>,
}

impl RelayServer {
    pub async fn bind(bind_addr: &str, policy: RelayPolicy) -> RelayResult<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let bind_addr = listener.local_addr()?.to_string();
        Ok(Self {
            inner: Arc::new(RelayServerInner {
                listener,
                bind_addr,
                registry: Mutex::new(RelayRegistry::default()),
                policy,
                diagnostics: RelayDiagnostics::default(),
            }),
        })
    }

    pub fn bind_addr(&self) -> &str {
        &self.inner.bind_addr
    }

    pub fn diagnostics(&self) -> RelayDiagnostics {
        self.inner.diagnostics.clone()
    }

    pub async fn connected_peers(&self) -> usize {
        self.inner.registry.lock().await.len()
    }

    pub fn spawn(self) -> RelayServerHandle {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(run_server(self.inner.clone(), shutdown_rx));
        RelayServerHandle {
            shutdown_tx: Some(shutdown_tx),
            task,
        }
    }
}

impl RelayServerHandle {
    pub async fn shutdown(mut self) -> RelayResult<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        self.task
            .await
            .map_err(|err| RelayError::ChannelClosed(err.to_string()))?
    }
}

async fn run_server(
    inner: Arc<RelayServerInner>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> RelayResult<()> {
    info!(bind = %inner.bind_addr, "relay listening");
    let cleanup_inner = inner.clone();
    let cleanup_task = tokio::spawn(async move {
        let interval_ms = if cleanup_inner.policy.heartbeat_interval_ms == 0 {
            250
        } else {
            cleanup_inner.policy.heartbeat_interval_ms
        };
        let mut interval = time::interval(Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            let cutoff = now_millis().saturating_sub(cleanup_inner.policy.connection_timeout_ms);
            let stale = cleanup_inner.registry.lock().await.cleanup_stale(cutoff);
            if stale.is_empty() {
                continue;
            }
            for _ in &stale {
                cleanup_inner.diagnostics.inc_heartbeat_timeouts();
                cleanup_inner.diagnostics.inc_disconnected();
            }
            let peer_list = Packet::relay_peer_list(
                RELAY_NODE_ID,
                RELAY_CALLSIGN,
                cleanup_inner.registry.lock().await.peer_list(),
            );
            let _ = cleanup_inner
                .registry
                .lock()
                .await
                .broadcast_except("", &peer_list);
        }
    });

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                cleanup_task.abort();
                break;
            }
            accepted = inner.listener.accept() => {
                let (stream, _) = accepted?;
                let cloned = inner.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_connection(cloned, stream).await {
                        warn!(%err, "relay connection ended with error");
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_connection(inner: Arc<RelayServerInner>, stream: TcpStream) -> RelayResult<()> {
    if inner.registry.lock().await.len() >= inner.policy.max_clients {
        return Err(RelayError::Policy("max relay clients reached".into()));
    }

    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Packet>();
    let writer_task = tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            write_packet(&mut writer, &packet).await?;
        }
        Ok::<_, RelayError>(())
    });

    let register_packet = match read_packet(&mut reader).await {
        Ok(packet) => packet,
        Err(err) => {
            inner.diagnostics.inc_malformed();
            return Err(err);
        }
    };

    if register_packet.packet_type != PacketType::RelayRegister {
        let _ = tx.send(Packet::relay_error(
            RELAY_NODE_ID,
            RELAY_CALLSIGN,
            Some(register_packet.node_id.clone()),
            "invalid_register",
            "first packet must be RELAY_REGISTER",
        ));
        return Err(RelayError::Registration(
            "first packet must be RELAY_REGISTER".into(),
        ));
    }

    let payload: RelayRegisterPayload = serde_json::from_value(register_packet.payload.clone())
        .map_err(|err| RelayError::Registration(err.to_string()))?;
    let peer = inner.registry.lock().await.register(
        register_packet.node_id.clone(),
        register_packet.callsign.clone(),
        payload.fingerprint,
        payload.capabilities,
        tx.clone(),
    )?;
    inner.diagnostics.inc_accepted();
    info!(node_id = %peer.node_id, callsign = %peer.callsign, "relay peer connected");

    tx.send(Packet::relay_registered(
        RELAY_NODE_ID,
        RELAY_CALLSIGN,
        format!("RL-{}", Uuid::new_v4().simple()),
        format!("connected to relay at {}", inner.bind_addr),
    ))
    .map_err(|err| RelayError::ChannelClosed(err.to_string()))?;

    broadcast_peer_list(&inner).await?;

    loop {
        match read_packet(&mut reader).await {
            Ok(packet) => {
                inner.registry.lock().await.update_seen(&peer.node_id);
                match packet.packet_type {
                    PacketType::RelayHeartbeat => {}
                    PacketType::RelayDisconnect => break,
                    PacketType::RelayForward => {
                        forward_packet(&inner, &peer.node_id, packet).await?;
                    }
                    _ => {
                        let forward = Packet::relay_forward(
                            peer.node_id.clone(),
                            peer.callsign.clone(),
                            packet.target_node.clone().unwrap_or_else(|| "*".into()),
                            packet.room.clone(),
                            serde_json::to_value(&packet)
                                .map_err(|err| RelayError::Protocol(err.to_string()))?,
                        );
                        forward_packet(&inner, &peer.node_id, forward).await?;
                    }
                }
            }
            Err(RelayError::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(RelayError::MalformedFrame(_)) => {
                inner.diagnostics.inc_malformed();
                let _ = tx.send(Packet::relay_error(
                    RELAY_NODE_ID,
                    RELAY_CALLSIGN,
                    Some(peer.node_id.clone()),
                    "malformed_frame",
                    "relay rejected malformed frame",
                ));
                break;
            }
            Err(err) => {
                let _ = tx.send(Packet::relay_error(
                    RELAY_NODE_ID,
                    RELAY_CALLSIGN,
                    Some(peer.node_id.clone()),
                    "relay_error",
                    err.to_string(),
                ));
                break;
            }
        }
    }

    inner.registry.lock().await.unregister(&peer.node_id);
    inner.diagnostics.inc_disconnected();
    broadcast_peer_list(&inner).await?;
    writer_task.abort();
    Ok(())
}

async fn broadcast_peer_list(inner: &Arc<RelayServerInner>) -> RelayResult<()> {
    let peer_list = {
        let registry = inner.registry.lock().await;
        Packet::relay_peer_list(RELAY_NODE_ID, RELAY_CALLSIGN, registry.peer_list())
    };
    let _ = inner
        .registry
        .lock()
        .await
        .broadcast_except("__none__", &peer_list)?;
    Ok(())
}

async fn forward_packet(
    inner: &Arc<RelayServerInner>,
    source_node_id: &str,
    packet: Packet,
) -> RelayResult<()> {
    let payload: RelayForwardPayload = serde_json::from_value(packet.payload.clone())
        .map_err(|err| RelayError::Protocol(err.to_string()))?;

    if payload.destination_node != "*" {
        inner
            .registry
            .lock()
            .await
            .send_to(&payload.destination_node, packet.clone())?;
        inner.diagnostics.inc_forwarded();
        inner.registry.lock().await.send_to(
            source_node_id,
            Packet::relay_delivered(
                RELAY_NODE_ID,
                RELAY_CALLSIGN,
                source_node_id.to_string(),
                payload.destination_node,
                packet.packet_id.to_string(),
            ),
        )?;
        return Ok(());
    }

    if !inner.policy.rooms.enabled || !inner.policy.rooms.broadcast {
        return Err(RelayError::Policy("relay room broadcast disabled".into()));
    }
    let _ = inner
        .registry
        .lock()
        .await
        .broadcast_except(source_node_id, &packet)?;
    inner.diagnostics.inc_broadcast();
    Ok(())
}
