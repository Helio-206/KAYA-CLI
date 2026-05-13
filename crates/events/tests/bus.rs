use kaya_events::{EventBus, EventCounters, EventKind, KayaEvent};

#[tokio::test]
async fn multiple_subscribers_receive_same_event() {
    let bus = EventBus::new(8);
    let mut left = bus.subscribe();
    let mut right = bus.subscribe();

    bus.publish(KayaEvent::ShutdownInitiated {
        reason: "test".into(),
    })
    .unwrap();

    assert_eq!(
        left.recv().await.unwrap().kind(),
        EventKind::ShutdownInitiated
    );
    assert_eq!(
        right.recv().await.unwrap().kind(),
        EventKind::ShutdownInitiated
    );
}

#[test]
fn event_counter_snapshot_is_stable() {
    let mut counters = EventCounters::default();
    counters.increment(EventKind::PacketSent);
    counters.increment(EventKind::PacketReceived);
    counters.increment(EventKind::PacketReceived);

    let snapshot = counters.snapshot();
    assert!(snapshot.contains(&("packet.sent".into(), 1)));
    assert!(snapshot.contains(&("packet.received".into(), 2)));
}
