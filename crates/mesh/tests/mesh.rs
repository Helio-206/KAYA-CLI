use kaya_mesh::{
    decide_relay, score_route, MeshEnvelope, MeshPolicy, MeshState, RelayDecision, RelayDropReason,
    RouteEntry, RouteEntrySpec, RouteSource, RoutingTable, SeenMeshPackets,
};
use kaya_protocol::Packet;

fn packet() -> Packet {
    Packet::direct_message("KY-AAAAAA", "Ana", "KY-BBBBBB", "opaque")
}

#[test]
fn ttl_hop_count_and_trace_update_on_relay() {
    let envelope = MeshEnvelope::new(
        "KY-AAAAAA",
        "KY-BBBBBB",
        "KY-AAAAAA",
        Some("KY-111111".into()),
        5,
        packet(),
    );

    let relayed = envelope
        .relay("KY-111111", Some("KY-222222".into()))
        .unwrap();

    assert_eq!(relayed.ttl, 4);
    assert_eq!(relayed.hop_count, 1);
    assert_eq!(relayed.route_trace, vec!["KY-AAAAAA", "KY-111111"]);
}

#[test]
fn duplicate_mesh_packets_are_rejected() {
    let mut seen = SeenMeshPackets::new(2);

    assert!(seen.observe("one"));
    assert!(!seen.observe("one"));
    assert!(seen.observe("two"));
    assert!(seen.observe("three"));
    assert!(seen.observe("one"));
}

#[test]
fn blocked_peer_relay_is_denied_by_policy() {
    let envelope = MeshEnvelope::new(
        "KY-AAAAAA",
        "KY-BBBBBB",
        "KY-AAAAAA",
        Some("KY-111111".into()),
        5,
        packet(),
    );
    let policy = MeshPolicy {
        allow_relay_for_blocked: false,
        ..MeshPolicy::default()
    };

    assert_eq!(
        decide_relay(&envelope, "KY-111111", &policy, true, Some("KY-222222")),
        RelayDecision::Drop(RelayDropReason::BlockedPeer)
    );
}

#[test]
fn no_loop_relay_to_source() {
    let envelope = MeshEnvelope::new(
        "KY-111111",
        "KY-BBBBBB",
        "KY-111111",
        Some("KY-111111".into()),
        5,
        packet(),
    );

    assert_eq!(
        decide_relay(
            &envelope,
            "KY-111111",
            &MeshPolicy::default(),
            false,
            Some("KY-222222")
        ),
        RelayDecision::Drop(RelayDropReason::LoopDetected)
    );
}

#[test]
fn route_scoring_prefers_short_trusted_recent_routes() {
    let trusted = score_route(2, true, true, Some(20), 0, 0);
    let unknown = score_route(4, false, false, Some(200), 2, 60_000);

    assert!(trusted > unknown);
}

#[test]
fn routing_table_expires_and_clears_routes() {
    let mut table = RoutingTable::new(1);
    table.upsert(RouteEntry::from_spec(RouteEntrySpec {
        destination_node: "KY-BBBBBB".into(),
        destination_callsign: Some("Bruno".into()),
        next_hop: "KY-AAAAAA".into(),
        hop_count: 2,
        trusted: true,
        encrypted_capable: true,
        source: RouteSource::Response,
        latency_ms: Some(10),
    }));

    assert!(table.best_route("Bruno").is_some());
    assert_eq!(table.expire(kaya_shared::now_millis() + 2).len(), 1);
    assert!(table.is_empty());

    table.upsert(RouteEntry::from_spec(RouteEntrySpec {
        destination_node: "KY-CCCCCC".into(),
        destination_callsign: None,
        next_hop: "KY-AAAAAA".into(),
        hop_count: 1,
        trusted: false,
        encrypted_capable: false,
        source: RouteSource::Direct,
        latency_ms: None,
    }));
    table.clear();
    assert!(table.is_empty());
}

#[test]
fn mesh_state_tracks_route_discovery_and_diagnostics() {
    let mut mesh = MeshState::new("KY-AAAAAA", MeshPolicy::default());
    mesh.observe_route(RouteEntry::from_spec(RouteEntrySpec {
        destination_node: "KY-BBBBBB".into(),
        destination_callsign: Some("Bruno".into()),
        next_hop: "KY-111111".into(),
        hop_count: 2,
        trusted: true,
        encrypted_capable: true,
        source: RouteSource::Response,
        latency_ms: Some(15),
    }));

    assert!(mesh.best_route("Bruno").is_some());
    let diagnostics = mesh.diagnostics_snapshot();
    assert_eq!(diagnostics.routes, 1);
    assert_eq!(diagnostics.routes_discovered, 1);
}

#[test]
fn encrypted_dm_payload_remains_opaque_to_relay() {
    let inner = Packet::direct_message_encrypted(
        "KY-AAAAAA",
        "Ana",
        "KY-BBBBBB",
        kaya_protocol::EncryptedDirectMessagePayload {
            session_id: "session".into(),
            nonce: "nonce".into(),
            ciphertext: "ciphertext-only".into(),
            sender_fingerprint: "KAYA-FP: 8A19-FC90-B2D1".into(),
            timestamp: "123".into(),
        },
    );
    let envelope = MeshEnvelope::new(
        "KY-AAAAAA",
        "KY-BBBBBB",
        "KY-AAAAAA",
        Some("KY-111111".into()),
        5,
        inner,
    );

    assert_eq!(
        envelope.inner_packet.payload["ciphertext"],
        serde_json::Value::String("ciphertext-only".into())
    );
    assert!(envelope.inner_packet.body().is_none());
}
