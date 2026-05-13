use kaya_transport::PacketDeduplicator;
use uuid::Uuid;

#[test]
fn dedup_cache_keeps_recent_packet_ids_only() {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let third = Uuid::new_v4();
    let mut dedup = PacketDeduplicator::new(2);

    assert!(dedup.observe(first));
    assert!(dedup.observe(second));
    assert!(!dedup.observe(first));
    assert!(dedup.observe(third));
    assert!(dedup.observe(first));
}
