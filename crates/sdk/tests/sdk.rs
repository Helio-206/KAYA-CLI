use kaya_protocol::{Packet, PacketType};
use kaya_sdk::{KayaClient, KayaConfig, KayaEvent, MockTransport, PeerSnapshot};
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::timeout;

async fn test_client() -> (KayaClient, kaya_sdk::MockTransportHandle, tempfile::TempDir) {
    let temp = tempdir().unwrap();
    let (transport, handle) = MockTransport::pair();
    let client = KayaClient::with_transport(
        KayaConfig {
            data_dir: Some(temp.path().to_path_buf()),
            ..KayaConfig::default()
        },
        transport,
    )
    .await
    .unwrap();
    (client, handle, temp)
}

async fn drain_sent(handle: &kaya_sdk::MockTransportHandle) {
    while timeout(Duration::from_millis(10), handle.next_sent())
        .await
        .ok()
        .flatten()
        .is_some()
    {}
}

async fn wait_for_peer(client: &KayaClient, expected_callsign: &str) -> PeerSnapshot {
    for _ in 0..40 {
        let peers = client.list_peers().await;
        if let Some(peer) = peers
            .into_iter()
            .find(|peer| peer.callsign == expected_callsign)
        {
            return peer;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("peer {expected_callsign} not observed in time");
}

#[tokio::test]
async fn sdk_client_creation_and_shutdown_work() {
    let (client, _handle, _temp) = test_client().await;

    assert!(!client.node_id().await.is_empty());
    client.stop().await.unwrap();
}

#[tokio::test]
async fn event_subscription_receives_peer_discovery() {
    let (client, handle, _temp) = test_client().await;
    let mut events = client.subscribe_events();

    handle
        .inject(Packet::hello("KY-71AF92", "Ana", "geral"))
        .unwrap();

    let event = timeout(Duration::from_secs(1), async {
        loop {
            let event = events.recv().await.unwrap();
            if let KayaEvent::PeerDiscovered { callsign, .. } = &event {
                if callsign == "Ana" {
                    break event;
                }
            }
        }
    })
    .await
    .unwrap();

    assert!(matches!(event, KayaEvent::PeerDiscovered { .. }));
    client.stop().await.unwrap();
}

#[tokio::test]
async fn join_room_through_sdk_updates_state() {
    let (client, handle, _temp) = test_client().await;
    drain_sent(&handle).await;

    client.join_room("dev").await.unwrap();

    assert_eq!(client.current_room().await, "dev");
    assert!(client
        .list_rooms()
        .await
        .iter()
        .any(|room| room.name == "dev" && room.local_joined));
    let packet = timeout(Duration::from_secs(1), handle.next_sent())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(packet.packet_type, PacketType::RoomJoin);

    client.stop().await.unwrap();
}

#[tokio::test]
async fn send_room_message_through_sdk_emits_packet() {
    let (client, handle, _temp) = test_client().await;
    drain_sent(&handle).await;

    client
        .send_room_message("geral", "hello offline")
        .await
        .unwrap();

    let packet = timeout(Duration::from_secs(1), handle.next_sent())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(packet.packet_type, PacketType::RoomMessage);
    assert_eq!(packet.body(), Some("hello offline"));

    client.stop().await.unwrap();
}

#[tokio::test]
async fn peer_listing_updates_from_mock_transport() {
    let (client, handle, _temp) = test_client().await;
    handle
        .inject(Packet::hello("KY-71AF92", "Ana", "geral"))
        .unwrap();

    let peer = wait_for_peer(&client, "Ana").await;

    assert_eq!(peer.node_id, "KY-71AF92");
    assert!(peer.online);
    client.stop().await.unwrap();
}

#[tokio::test]
async fn mock_transport_captures_startup_packets() {
    let (client, handle, _temp) = test_client().await;

    let packet = timeout(Duration::from_secs(1), handle.next_sent())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(packet.packet_type, PacketType::Hello);

    client.stop().await.unwrap();
}

#[tokio::test]
async fn stop_node_is_clean_after_activity() {
    let (client, handle, _temp) = test_client().await;
    handle
        .inject(Packet::hello("KY-71AF92", "Ana", "geral"))
        .unwrap();
    let _ = wait_for_peer(&client, "Ana").await;
    client.send_room_message("geral", "bye").await.unwrap();

    client.stop().await.unwrap();
}
