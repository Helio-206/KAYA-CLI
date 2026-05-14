use kaya_protocol::Packet;
use kaya_security::{
    sign_packet, verify_packet_signature, IdentityStore, LocalIdentity, SecureSessionManager,
    SignatureStatus, TrustObservation, TrustStatus, TrustStore,
};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{label}-{}", Uuid::new_v4()))
}

#[test]
fn identity_generation_and_persistence_are_stable() {
    let path = temp_dir("kaya-identity");
    let store = IdentityStore::new(&path);

    let identity = store.load_or_create("Helio").unwrap();
    let reloaded = store.load_or_create("Helio").unwrap();

    assert_eq!(identity.node_id, reloaded.node_id);
    assert_eq!(identity.fingerprint, reloaded.fingerprint);
    assert!(identity.fingerprint.starts_with("KAYA-FP: "));
    assert_eq!(identity.ed25519_public_key_hex().len(), 64);
    assert_eq!(identity.x25519_public_key_hex().len(), 64);

    let _ = fs::remove_dir_all(path);
}

#[test]
fn packet_signing_and_validation_work() {
    let identity = LocalIdentity::generate("Ana");
    let mut packet = Packet::hello(identity.node_id.clone(), "Ana", "geral");

    sign_packet(&mut packet, &identity).unwrap();

    assert!(matches!(
        verify_packet_signature(&packet),
        SignatureStatus::Valid { .. }
    ));
}

#[test]
fn invalid_signature_is_detected_after_tamper() {
    let identity = LocalIdentity::generate("Ana");
    let mut packet = Packet::direct_message(identity.node_id.clone(), "Ana", "KY-AAAAAA", "one");

    sign_packet(&mut packet, &identity).unwrap();
    packet.payload["body"] = serde_json::Value::String("two".into());

    assert!(matches!(
        verify_packet_signature(&packet),
        SignatureStatus::Invalid { .. }
    ));
}

#[test]
fn trust_store_tracks_status_and_blocking() {
    let path = temp_dir("kaya-trust");
    let mut store = TrustStore::load_or_create(&path).unwrap();

    let observation = store
        .record_seen("KY-71AF92", "Ana", "KAYA-FP: 8A19-FC90-B2D1")
        .unwrap();
    assert_eq!(observation, TrustObservation::New);
    assert_eq!(store.status("KY-71AF92"), TrustStatus::Unknown);

    store.set_status("KY-71AF92", TrustStatus::Trusted).unwrap();
    assert_eq!(store.status("KY-71AF92"), TrustStatus::Trusted);

    store.set_status("KY-71AF92", TrustStatus::Blocked).unwrap();
    assert!(store.is_blocked("KY-71AF92"));

    let _ = fs::remove_dir_all(path);
}

#[test]
fn encrypted_dm_roundtrip_proves_shared_secret_equality() {
    let alice = LocalIdentity::generate("Ana");
    let bob = LocalIdentity::generate("Bruno");
    let mut alice_sessions = SecureSessionManager::new(alice.clone());
    let mut bob_sessions = SecureSessionManager::new(bob.clone());

    let request = alice_sessions.start_request(&bob.node_id);
    let accept = bob_sessions
        .accept_request(
            &alice.node_id,
            &request.session_id,
            &request.x25519_public_key,
            &request.fingerprint,
        )
        .unwrap();
    alice_sessions
        .complete_accept(
            &bob.node_id,
            &accept.session_id,
            &accept.x25519_public_key,
            &accept.fingerprint,
        )
        .unwrap();

    let encrypted = alice_sessions.encrypt(&bob.node_id, "segredo").unwrap();
    let decrypted = bob_sessions.decrypt(&alice.node_id, &encrypted).unwrap();

    assert_eq!(decrypted, "segredo");
    assert_eq!(alice_sessions.active_count(), 1);
    assert_eq!(bob_sessions.active_count(), 1);
}

#[test]
fn encrypted_dm_tamper_fails() {
    let alice = LocalIdentity::generate("Ana");
    let bob = LocalIdentity::generate("Bruno");
    let mut alice_sessions = SecureSessionManager::new(alice.clone());
    let mut bob_sessions = SecureSessionManager::new(bob.clone());

    let request = alice_sessions.start_request(&bob.node_id);
    let accept = bob_sessions
        .accept_request(
            &alice.node_id,
            &request.session_id,
            &request.x25519_public_key,
            &request.fingerprint,
        )
        .unwrap();
    alice_sessions
        .complete_accept(
            &bob.node_id,
            &accept.session_id,
            &accept.x25519_public_key,
            &accept.fingerprint,
        )
        .unwrap();

    let mut encrypted = alice_sessions.encrypt(&bob.node_id, "segredo").unwrap();
    encrypted.ciphertext.replace_range(0..2, "00");

    assert!(bob_sessions.decrypt(&alice.node_id, &encrypted).is_err());
}

#[test]
fn encrypted_file_chunk_roundtrip_and_tamper_failure() {
    let alice = LocalIdentity::generate("Ana");
    let bob = LocalIdentity::generate("Bruno");
    let mut alice_sessions = SecureSessionManager::new(alice.clone());
    let mut bob_sessions = SecureSessionManager::new(bob.clone());

    let request = alice_sessions.start_request(&bob.node_id);
    let accept = bob_sessions
        .accept_request(
            &alice.node_id,
            &request.session_id,
            &request.x25519_public_key,
            &request.fingerprint,
        )
        .unwrap();
    alice_sessions
        .complete_accept(
            &bob.node_id,
            &accept.session_id,
            &accept.x25519_public_key,
            &accept.fingerprint,
        )
        .unwrap();

    let encrypted = alice_sessions
        .encrypt_file_chunk(&bob.node_id, b"chunk bytes")
        .unwrap();
    assert_eq!(
        bob_sessions
            .decrypt_file_chunk(&alice.node_id, &encrypted)
            .unwrap(),
        b"chunk bytes"
    );

    let mut tampered = encrypted;
    tampered.ciphertext.replace_range(0..2, "00");
    assert!(bob_sessions
        .decrypt_file_chunk(&alice.node_id, &tampered)
        .is_err());
}

#[test]
fn session_lifecycle_can_close_active_session() {
    let alice = LocalIdentity::generate("Ana");
    let bob = LocalIdentity::generate("Bruno");
    let mut sessions = SecureSessionManager::new(alice.clone());
    let request = sessions.start_request(&bob.node_id);

    sessions
        .complete_accept(
            &bob.node_id,
            &request.session_id,
            &bob.x25519_public_key_hex(),
            &bob.fingerprint,
        )
        .unwrap();

    assert!(sessions.has_active(&bob.node_id));
    assert!(sessions.close(&bob.node_id));
    assert!(!sessions.has_active(&bob.node_id));
}
