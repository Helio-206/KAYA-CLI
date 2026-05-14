use kaya_files::{
    chunk_bytes, FileMetadata, FileTransferConfig, FileTransferManager, TransferSecurity,
};
use std::time::Instant;

fn main() {
    let size_mb = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(8);
    let bytes = deterministic_bytes(size_mb * 1024 * 1024);
    let config = FileTransferConfig::default();
    let metadata =
        FileMetadata::from_bytes("bench.bin", &bytes, None, "KY-71AF92", "Helio", &config)
            .expect("metadata");
    let chunks = chunk_bytes(&metadata.file_id, &bytes, metadata.chunk_size).expect("chunks");
    let mut manager = FileTransferManager::new();
    manager.receive_offer(
        metadata.clone(),
        "KY-71AF92",
        "Helio",
        TransferSecurity::Unencrypted,
        true,
        true,
    );
    manager.accept(&metadata.file_id).expect("accepted");

    let started = Instant::now();
    let mut completed = None;
    for chunk in chunks {
        completed = manager.receive_chunk(chunk).expect("chunk accepted");
    }
    let elapsed = started.elapsed();
    let completed = completed.expect("completed");
    let mb = completed.len() as f64 / (1024.0 * 1024.0);
    let seconds = elapsed.as_secs_f64().max(0.001);

    println!(
        "file_transfer_simulated: size_mb={} chunks={} elapsed_ms={} mb_per_sec={:.2}",
        size_mb,
        metadata.total_chunks,
        elapsed.as_millis(),
        mb / seconds
    );
}

fn deterministic_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|index| (index % 251) as u8).collect()
}
