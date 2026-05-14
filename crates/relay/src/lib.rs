pub mod client;
pub mod diagnostics;
pub mod errors;
pub mod framing;
pub mod policy;
pub mod registry;
pub mod server;

pub use client::{RelayClient, RelayRegistration};
pub use diagnostics::{RelayDiagnostics, RelayDiagnosticsSnapshot};
pub use errors::{RelayError, RelayResult};
pub use policy::{RelayFileTransferPolicy, RelayPolicy, RelayRoomPolicy};
pub use registry::{RelayPeerInfo, RelayRegistry};
pub use server::{RelayServer, RelayServerHandle};

#[cfg(test)]
mod tests {
    use super::*;
    use kaya_protocol::{Packet, PacketType, RelayForwardPayload};
    use tokio::net::TcpStream;
    use tokio::time::{sleep, timeout, Duration};

    async fn spawn_server(policy: RelayPolicy) -> (RelayServer, RelayServerHandle) {
        let server = RelayServer::bind("127.0.0.1:0", policy).await.unwrap();
        let handle = server.clone().spawn();
        (server, handle)
    }

    async fn connect_client(server: &RelayServer, node_id: &str, callsign: &str) -> RelayClient {
        RelayClient::connect(
            &format!("tcp://{}", server.bind_addr()),
            RelayRegistration {
                node_id: node_id.into(),
                callsign: callsign.into(),
                fingerprint: format!("KAYA-FP: {}-00-00", &node_id[3..5]),
                capabilities: vec!["rooms".into(), "dm".into()],
            },
            RelayPolicy {
                heartbeat_interval_ms: 0,
                ..RelayPolicy::default()
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn relay_registration_returns_registered_packet() {
        let (server, handle) = spawn_server(RelayPolicy::default()).await;
        let mut client = connect_client(&server, "KY-71AF92", "Ana").await;

        let packet = timeout(Duration::from_secs(1), client.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(packet.packet_type, PacketType::RelayRegistered);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn relay_peer_list_is_broadcast_after_registration() {
        let (server, handle) = spawn_server(RelayPolicy::default()).await;
        let mut ana = connect_client(&server, "KY-71AF92", "Ana").await;
        let _ = ana.recv().await;
        let _ = ana.recv().await;
        let mut bruno = connect_client(&server, "KY-A91C0D", "Bruno").await;
        let _ = bruno.recv().await;
        let _ = bruno.recv().await;

        let packet = timeout(Duration::from_secs(1), ana.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(packet.packet_type, PacketType::RelayPeerList);
        assert_eq!(server.connected_peers().await, 2);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn relay_forwards_targeted_packets() {
        let (server, handle) = spawn_server(RelayPolicy::default()).await;
        let mut ana = connect_client(&server, "KY-71AF92", "Ana").await;
        let _ = ana.recv().await;
        let _ = ana.recv().await;
        let mut bruno = connect_client(&server, "KY-A91C0D", "Bruno").await;
        let _ = bruno.recv().await;
        let _ = bruno.recv().await;
        let _ = timeout(Duration::from_secs(1), ana.recv()).await.unwrap();

        ana.send(Packet::relay_forward(
            "KY-71AF92",
            "Ana",
            "KY-A91C0D",
            None,
            serde_json::to_value(Packet::direct_message(
                "KY-71AF92",
                "Ana",
                "KY-A91C0D",
                "hello",
            ))
            .unwrap(),
        ))
        .unwrap();

        let forwarded = timeout(Duration::from_secs(1), bruno.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(forwarded.packet_type, PacketType::RelayForward);
        let payload: RelayForwardPayload = serde_json::from_value(forwarded.payload).unwrap();
        assert_eq!(payload.destination_node, "KY-A91C0D");

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn relay_cleans_up_disconnected_peers() {
        let (server, handle) = spawn_server(RelayPolicy::default()).await;
        let mut ana = connect_client(&server, "KY-71AF92", "Ana").await;
        let _ = ana.recv().await;
        let _ = ana.recv().await;
        let bruno = connect_client(&server, "KY-A91C0D", "Bruno").await;
        drop(ana);
        drop(bruno);

        sleep(Duration::from_millis(200)).await;
        assert_eq!(server.connected_peers().await, 0);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn relay_rejects_malformed_frame() {
        let (server, handle) = spawn_server(RelayPolicy::default()).await;
        let mut stream = TcpStream::connect(server.bind_addr()).await.unwrap();
        tokio::io::AsyncWriteExt::write_u32(&mut stream, u32::MAX)
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;
        assert!(server.diagnostics().snapshot().malformed_frames >= 1);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn relay_times_out_missing_heartbeat() {
        let (server, handle) = spawn_server(RelayPolicy {
            heartbeat_interval_ms: 50,
            connection_timeout_ms: 50,
            ..RelayPolicy::default()
        })
        .await;
        let mut ana = connect_client(&server, "KY-71AF92", "Ana").await;
        let _ = ana.recv().await;
        let _ = ana.recv().await;

        sleep(Duration::from_millis(200)).await;
        assert_eq!(server.connected_peers().await, 0);
        assert!(server.diagnostics().snapshot().heartbeat_timeouts >= 1);

        handle.shutdown().await.unwrap();
    }
}
