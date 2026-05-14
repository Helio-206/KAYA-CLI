use kaya_mesh::SeenMeshPackets;
use std::time::Instant;

fn main() {
    let packets = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let mut dedup = SeenMeshPackets::new(packets.max(1));
    let started = Instant::now();
    let mut accepted = 0usize;
    let mut duplicates = 0usize;

    for index in 0..packets {
        let packet_id = format!("mesh-{index}");
        if dedup.observe(&packet_id) {
            accepted += 1;
        }
        if !dedup.observe(&packet_id) {
            duplicates += 1;
        }
    }

    let elapsed = started.elapsed();
    let per_second = (packets * 2) as f64 / elapsed.as_secs_f64().max(0.001);
    println!(
        "mesh_dedup: packets={} accepted={} duplicates={} elapsed_ms={} checks_per_sec={:.0}",
        packets,
        accepted,
        duplicates,
        elapsed.as_millis(),
        per_second
    );
}
