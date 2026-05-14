use kaya_peer::{PeerEvent, PeerRegistry};
use kaya_protocol::Packet;
use kaya_shared::PresenceStatus;
use std::time::{Duration, Instant};

#[test]
fn duplicate_peer_updates_single_record() {
    let mut registry = PeerRegistry::new("KY-000001");

    registry.observe_packet(&Packet::hello("KY-71AF92", "Ana", "geral"));
    registry.observe_packet(&Packet::heartbeat(
        "KY-71AF92",
        "Ana",
        "geral",
        PresenceStatus::Online,
    ));

    assert_eq!(registry.snapshots().len(), 1);
    assert_eq!(registry.online_count(), 1);
}

#[test]
fn timeout_event_only_emits_once_for_offline_peer() {
    let start = Instant::now();
    let mut registry = PeerRegistry::with_timeout("KY-000001", Duration::from_secs(1));

    registry.observe_packet_at(&Packet::hello("KY-71AF92", "Ana", "geral"), start);
    assert_eq!(
        registry.prune_at(start + Duration::from_secs(2)),
        vec![PeerEvent::TimedOut("KY-71AF92".into())]
    );
    assert!(registry.prune_at(start + Duration::from_secs(3)).is_empty());
}
