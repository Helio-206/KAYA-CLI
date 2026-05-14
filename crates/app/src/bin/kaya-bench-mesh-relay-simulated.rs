use kaya_mesh::{MeshEnvelope, MeshPolicy};
use kaya_protocol::Packet;
use std::time::Instant;

fn main() {
    let packets = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let policy = MeshPolicy::default();
    let inner = Packet::direct_message("KY-71AF92", "Helio", "KY-A91C0D", "bench");
    let started = Instant::now();
    let mut bytes = 0usize;

    for index in 0..packets {
        let envelope = MeshEnvelope::new(
            "KY-71AF92",
            "KY-A91C0D",
            "KY-71AF92",
            Some("KY-AAAAAA".into()),
            policy.max_ttl,
            inner.clone(),
        );
        let relayed = envelope
            .relay("KY-AAAAAA", Some(format!("KY-{index:06X}")))
            .expect("relay should preserve ttl");
        bytes += serde_json::to_vec(&relayed.to_value().expect("mesh json"))
            .expect("mesh bytes")
            .len();
    }

    let elapsed = started.elapsed();
    let per_second = packets as f64 / elapsed.as_secs_f64().max(0.001);
    println!(
        "mesh_relay_simulated: packets={} envelope_bytes={} elapsed_ms={} packets_per_sec={:.0}",
        packets,
        bytes,
        elapsed.as_millis(),
        per_second
    );
}
