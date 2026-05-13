use kaya_peer::PeerRegistry;
use kaya_protocol::Packet;
use std::time::Instant;

fn main() {
    let peers = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let mut registry = PeerRegistry::new("KY-FFFFFF");
    let started = Instant::now();

    for index in 0..peers {
        let node_id = format!("KY-{index:06X}");
        let callsign = format!("peer-{index}");
        registry.observe_packet(&Packet::hello(node_id, callsign, "geral"));
    }

    let elapsed = started.elapsed();
    println!(
        "peers: {peers} online: {} elapsed_ms: {}",
        registry.online_count(),
        elapsed.as_millis()
    );
}
