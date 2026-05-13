use kaya_protocol::{decode, encode, Packet};
use std::time::Instant;

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(50_000);
    let packet = Packet::room_message("KY-71AF92", "Ana", "geral", "benchmark message");
    let started = Instant::now();

    for _ in 0..iterations {
        let bytes = encode(&packet).expect("benchmark packet should encode");
        let _decoded = decode(&bytes).expect("benchmark packet should decode");
    }

    let elapsed = started.elapsed();
    let per_second = iterations as f64 / elapsed.as_secs_f64();
    println!(
        "messages: {iterations} elapsed_ms: {} throughput_per_sec: {:.0}",
        elapsed.as_millis(),
        per_second
    );
}
