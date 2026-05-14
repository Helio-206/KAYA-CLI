use kaya_protocol::Packet;
use kaya_rooms::{RoomStore, RouteOutcome};

#[test]
fn simultaneous_joins_share_room_membership() {
    let mut store = RoomStore::new("KY-000001", "Helio");
    let ana = Packet::join_room("KY-71AF92", "Ana", "semana-info");
    let bruno = Packet::join_room("KY-BB0022", "Bruno", "semana-info");

    assert!(matches!(
        store.route_packet(&ana),
        RouteOutcome::Joined { room, .. } if room == "semana-info"
    ));
    assert!(matches!(
        store.route_packet(&bruno),
        RouteOutcome::Joined { room, .. } if room == "semana-info"
    ));

    assert_eq!(store.room_names(), vec!["geral", "semana-info"]);
}

#[test]
fn room_messages_do_not_switch_local_current_room() {
    let mut store = RoomStore::new("KY-000001", "Helio");
    let packet = Packet::room_message("KY-71AF92", "Ana", "ops", "ping");

    assert_eq!(store.route_packet(&packet), RouteOutcome::Ignored);
    assert_eq!(store.current_room(), "geral");
}
