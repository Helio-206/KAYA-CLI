use kaya_files::{
    chunk_bytes, reassemble_chunks, safe_file_name, FileMetadata, FileStore, FileTransferConfig,
    FileTransferManager, OutgoingFileRequest, TransferSecurity, TransferStatus,
};
use std::fs;
use uuid::Uuid;

fn config() -> FileTransferConfig {
    FileTransferConfig {
        enabled: true,
        max_file_size_bytes: 1024 * 1024,
        chunk_size: 8,
        accept_from_unknown: true,
        download_dir: None,
    }
}

fn metadata(bytes: &[u8]) -> FileMetadata {
    FileMetadata::from_bytes("report.pdf", bytes, None, "KY-71AF92", "Ana", &config()).unwrap()
}

#[test]
fn metadata_validation_and_safe_filename_work() {
    let meta = metadata(b"hello world");

    assert!(meta.file_id.starts_with("KF-"));
    assert_eq!(meta.file_name, "report.pdf");
    assert_eq!(meta.total_chunks, 2);
    assert!(safe_file_name("report.pdf").is_ok());
    assert!(safe_file_name("../secret.txt").is_err());
    assert!(safe_file_name("/tmp/secret.txt").is_err());
    assert!(safe_file_name("bad/name.txt").is_err());
}

#[test]
fn dangerous_extensions_warn_without_blocking() {
    let meta = FileMetadata::from_bytes(
        "install.sh",
        b"echo hi",
        None,
        "KY-71AF92",
        "Ana",
        &config(),
    )
    .unwrap();

    assert!(meta.dangerous_extension);
}

#[test]
fn chunk_split_reassembly_and_hash_validation_work() {
    let bytes = b"abcdefghijklmnopqrstuvwxyz";
    let meta = metadata(bytes);
    let chunks = chunk_bytes(&meta.file_id, bytes, meta.chunk_size).unwrap();

    assert_eq!(chunks.len() as u32, meta.total_chunks);
    assert!(chunks.iter().all(|chunk| chunk.validate().is_ok()));

    let reassembled = reassemble_chunks(&meta, &chunks).unwrap();
    assert_eq!(reassembled, bytes);
}

#[test]
fn corrupted_chunk_is_rejected() {
    let bytes = b"abcdefghijklmnopqrstuvwxyz";
    let meta = metadata(bytes);
    let mut chunks = chunk_bytes(&meta.file_id, bytes, meta.chunk_size).unwrap();
    chunks[0].payload[0] ^= 0x01;

    assert!(reassemble_chunks(&meta, &chunks).is_err());
}

#[test]
fn transfer_state_flow_accept_reject_cancel() {
    let bytes = b"hello transfer";
    let path = std::env::temp_dir().join(format!("kaya-file-{}.txt", Uuid::new_v4()));
    fs::write(&path, bytes).unwrap();

    let mut manager = FileTransferManager::new();
    let session = manager
        .prepare_outgoing(
            OutgoingFileRequest {
                path: path.clone(),
                sender_node_id: "KY-71AF92".into(),
                sender_callsign: "Helio".into(),
                peer_node_id: "KY-AAAAAA".into(),
                peer_callsign: "Ana".into(),
                security: TransferSecurity::Unencrypted,
            },
            &config(),
        )
        .unwrap()
        .clone();

    assert_eq!(session.status, TransferStatus::Offered);
    manager.mark_outgoing_accepted(&session.file_id).unwrap();
    assert_eq!(
        manager.session(&session.file_id).unwrap().status,
        TransferStatus::Accepted
    );
    manager.cancel(&session.file_id).unwrap();
    assert_eq!(
        manager.session(&session.file_id).unwrap().status,
        TransferStatus::Cancelled
    );

    let _ = fs::remove_file(path);
}

#[test]
fn incoming_transfer_completes_after_all_chunks() {
    let bytes = b"hello transfer";
    let meta = metadata(bytes);
    let chunks = chunk_bytes(&meta.file_id, bytes, meta.chunk_size).unwrap();

    let mut manager = FileTransferManager::new();
    manager.receive_offer(
        meta.clone(),
        "KY-71AF92",
        "Helio",
        TransferSecurity::Unencrypted,
        true,
        true,
    );
    manager.accept(&meta.file_id).unwrap();

    let mut completed = None;
    for chunk in chunks {
        completed = manager.receive_chunk(chunk).unwrap();
    }

    assert_eq!(completed.unwrap(), bytes);
    assert_eq!(
        manager.session(&meta.file_id).unwrap().status,
        TransferStatus::Completed
    );
}

#[test]
fn transfer_metadata_persists() {
    let path = std::env::temp_dir().join(format!("kaya-files-store-{}", Uuid::new_v4()));
    let store = FileStore::new(&path, None).unwrap();
    let meta = metadata(b"hello");
    let mut manager = FileTransferManager::new();
    let session = manager
        .receive_offer(
            meta.clone(),
            "KY-71AF92",
            "Ana",
            TransferSecurity::Encrypted,
            true,
            false,
        )
        .clone();

    store.save_record(&session).unwrap();
    let records = store.list_records().unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].session.file_id, session.file_id);

    manager.load_record(session);
    assert!(manager.session(&meta.file_id).is_ok());

    let _ = fs::remove_dir_all(path);
}
