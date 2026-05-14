use kaya_files::{chunk_bytes, sha256_hex, FileMetadata, FileTransferConfig};
use std::time::Instant;

fn main() {
    let size_mb = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(8);
    let bytes = deterministic_bytes(size_mb * 1024 * 1024);
    let config = FileTransferConfig::default();
    let metadata =
        FileMetadata::from_bytes("bench.bin", &bytes, None, "KY-71AF92", "bench", &config)
            .expect("metadata");

    let started = Instant::now();
    let chunks = chunk_bytes(&metadata.file_id, &bytes, metadata.chunk_size).expect("chunks");
    let elapsed = started.elapsed();
    let mb = bytes.len() as f64 / (1024.0 * 1024.0);
    let seconds = elapsed.as_secs_f64().max(0.001);

    println!(
        "file_chunking: size_mb={} chunks={} elapsed_ms={} mb_per_sec={:.2} sha256={}",
        size_mb,
        chunks.len(),
        elapsed.as_millis(),
        mb / seconds,
        sha256_hex(&bytes)
    );
}

fn deterministic_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|index| (index % 251) as u8).collect()
}
