use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use kaya_protocol::{Packet, PacketType};
use kaya_shared::{now_millis, KayaError, NodeId, Result};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

pub const IDENTITY_FILE: &str = "identity.toml";
pub const TRUST_FILE: &str = "trust.toml";
pub const FINGERPRINT_PREFIX: &str = "KAYA-FP: ";

const ED25519_PUBLIC_KEY_BYTES: usize = 32;
const ED25519_SECRET_KEY_BYTES: usize = 32;
const ED25519_SIGNATURE_BYTES: usize = 64;
const X25519_KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;

#[derive(Clone)]
pub struct LocalIdentity {
    pub node_id: String,
    pub callsign: String,
    ed25519_secret_key: [u8; ED25519_SECRET_KEY_BYTES],
    ed25519_public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
    x25519_secret_key: [u8; X25519_KEY_BYTES],
    x25519_public_key: [u8; X25519_KEY_BYTES],
    pub fingerprint: String,
    pub created_at: String,
}

impl LocalIdentity {
    pub fn generate(callsign: impl Into<String>) -> Self {
        let callsign = callsign.into();
        let mut ed_secret = [0_u8; ED25519_SECRET_KEY_BYTES];
        let mut x_secret = [0_u8; X25519_KEY_BYTES];
        OsRng.fill_bytes(&mut ed_secret);
        OsRng.fill_bytes(&mut x_secret);

        let signing_key = SigningKey::from_bytes(&ed_secret);
        let ed_public = signing_key.verifying_key().to_bytes();
        let x_static = StaticSecret::from(x_secret);
        let x_public = X25519PublicKey::from(&x_static).to_bytes();
        let fingerprint = fingerprint_from_ed25519_public_key(&ed_public);

        Self {
            node_id: NodeId::generate().to_string(),
            callsign,
            ed25519_secret_key: ed_secret,
            ed25519_public_key: ed_public,
            x25519_secret_key: x_secret,
            x25519_public_key: x_public,
            fingerprint,
            created_at: now_millis().to_string(),
        }
    }

    pub fn ed25519_public_key_hex(&self) -> String {
        encode_hex(&self.ed25519_public_key)
    }

    pub fn x25519_public_key_hex(&self) -> String {
        encode_hex(&self.x25519_public_key)
    }

    pub fn short_fingerprint(&self) -> String {
        self.fingerprint
            .strip_prefix(FINGERPRINT_PREFIX)
            .unwrap_or(&self.fingerprint)
            .to_string()
    }

    fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.ed25519_secret_key)
    }

    fn x25519_secret(&self) -> StaticSecret {
        StaticSecret::from(self.x25519_secret_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredIdentity {
    node_id: String,
    callsign: String,
    ed25519_private_key: String,
    ed25519_public_key: String,
    x25519_private_key: String,
    x25519_public_key: String,
    fingerprint: String,
    created_at: String,
}

impl TryFrom<StoredIdentity> for LocalIdentity {
    type Error = KayaError;

    fn try_from(value: StoredIdentity) -> Result<Self> {
        let ed_secret = decode_fixed::<ED25519_SECRET_KEY_BYTES>(&value.ed25519_private_key)?;
        let x_secret = decode_fixed::<X25519_KEY_BYTES>(&value.x25519_private_key)?;
        let signing_key = SigningKey::from_bytes(&ed_secret);
        let ed_public = signing_key.verifying_key().to_bytes();
        let x_static = StaticSecret::from(x_secret);
        let x_public = X25519PublicKey::from(&x_static).to_bytes();
        let fingerprint = fingerprint_from_ed25519_public_key(&ed_public);

        Ok(Self {
            node_id: NodeId::parse(value.node_id)?.to_string(),
            callsign: value.callsign,
            ed25519_secret_key: ed_secret,
            ed25519_public_key: ed_public,
            x25519_secret_key: x_secret,
            x25519_public_key: x_public,
            fingerprint,
            created_at: value.created_at,
        })
    }
}

impl From<&LocalIdentity> for StoredIdentity {
    fn from(identity: &LocalIdentity) -> Self {
        Self {
            node_id: identity.node_id.clone(),
            callsign: identity.callsign.clone(),
            ed25519_private_key: encode_hex(&identity.ed25519_secret_key),
            ed25519_public_key: identity.ed25519_public_key_hex(),
            x25519_private_key: encode_hex(&identity.x25519_secret_key),
            x25519_public_key: identity.x25519_public_key_hex(),
            fingerprint: identity.fingerprint.clone(),
            created_at: identity.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IdentityStore {
    data_dir: PathBuf,
}

impl IdentityStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.data_dir.join(IDENTITY_FILE)
    }

    pub fn load_or_create(&self, callsign: &str) -> Result<LocalIdentity> {
        if !self.path().exists() {
            let identity = LocalIdentity::generate(callsign);
            self.save(&identity)?;
            return Ok(identity);
        }

        let mut identity = self.load()?;
        let normalized = kaya_shared::normalize_callsign(callsign);
        if identity.callsign != normalized {
            identity.callsign = normalized;
            self.save(&identity)?;
        }
        Ok(identity)
    }

    pub fn load(&self) -> Result<LocalIdentity> {
        let text = fs::read_to_string(self.path())?;
        let stored: StoredIdentity =
            toml::from_str(&text).map_err(|err| KayaError::Security(err.to_string()))?;
        stored.try_into()
    }

    pub fn save(&self, identity: &LocalIdentity) -> Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        let stored = StoredIdentity::from(identity);
        let text =
            toml::to_string_pretty(&stored).map_err(|err| KayaError::Security(err.to_string()))?;
        fs::write(self.path(), text)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    Missing,
    Valid { fingerprint: String },
    Invalid { reason: String },
}

pub fn sign_packet(packet: &mut Packet, identity: &LocalIdentity) -> Result<()> {
    packet.public_key = Some(identity.ed25519_public_key_hex());
    packet.signature = None;
    let canonical = canonical_packet_bytes(packet)?;
    let signature = identity.signing_key().sign(&canonical);
    packet.signature = Some(encode_hex(&signature.to_bytes()));
    Ok(())
}

pub fn verify_packet_signature(packet: &Packet) -> SignatureStatus {
    let (Some(public_key), Some(signature)) = (&packet.public_key, &packet.signature) else {
        if packet.public_key.is_none() && packet.signature.is_none() {
            return SignatureStatus::Missing;
        }
        return SignatureStatus::Invalid {
            reason: "partial signature envelope".into(),
        };
    };

    let public_key = match decode_fixed::<ED25519_PUBLIC_KEY_BYTES>(public_key) {
        Ok(value) => value,
        Err(err) => {
            return SignatureStatus::Invalid {
                reason: err.to_string(),
            }
        }
    };
    let signature = match decode_fixed::<ED25519_SIGNATURE_BYTES>(signature) {
        Ok(value) => Signature::from_bytes(&value),
        Err(err) => {
            return SignatureStatus::Invalid {
                reason: err.to_string(),
            }
        }
    };
    let verifying_key = match VerifyingKey::from_bytes(&public_key) {
        Ok(value) => value,
        Err(err) => {
            return SignatureStatus::Invalid {
                reason: err.to_string(),
            }
        }
    };
    let canonical = match canonical_packet_bytes(packet) {
        Ok(value) => value,
        Err(err) => {
            return SignatureStatus::Invalid {
                reason: err.to_string(),
            }
        }
    };

    match verifying_key.verify(&canonical, &signature) {
        Ok(()) => SignatureStatus::Valid {
            fingerprint: fingerprint_from_ed25519_public_key(&public_key),
        },
        Err(err) => SignatureStatus::Invalid {
            reason: err.to_string(),
        },
    }
}

pub fn packet_requires_signature_validation(packet_type: PacketType) -> bool {
    matches!(
        packet_type,
        PacketType::Hello
            | PacketType::Heartbeat
            | PacketType::RoomJoin
            | PacketType::RoomLeave
            | PacketType::PresenceUpdate
            | PacketType::DirectMessage
            | PacketType::DmSessionRequest
            | PacketType::DmSessionAccept
            | PacketType::DirectMessageEncrypted
    )
}

fn canonical_packet_bytes(packet: &Packet) -> Result<Vec<u8>> {
    let mut canonical = packet.clone();
    canonical.signature = None;
    serde_json::to_vec(&canonical).map_err(KayaError::from)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustStatus {
    Unknown,
    Trusted,
    Blocked,
}

impl TrustStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TrustStatus::Unknown => "unknown",
            TrustStatus::Trusted => "trusted",
            TrustStatus::Blocked => "blocked",
        }
    }
}

impl std::fmt::Display for TrustStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub node_id: String,
    pub callsign: String,
    pub fingerprint: String,
    pub first_seen: String,
    pub last_seen: String,
    pub trust_status: TrustStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustObservation {
    New,
    Updated,
    FingerprintChanged { previous: String, current: String },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TrustFile {
    peers: Vec<TrustedPeer>,
}

#[derive(Debug, Clone)]
pub struct TrustStore {
    path: PathBuf,
    peers: HashMap<String, TrustedPeer>,
}

impl TrustStore {
    pub fn load_or_create(data_dir: impl AsRef<Path>) -> Result<Self> {
        let path = data_dir.as_ref().join(TRUST_FILE);
        if !path.exists() {
            let store = Self {
                path,
                peers: HashMap::new(),
            };
            store.save()?;
            return Ok(store);
        }

        let text = fs::read_to_string(&path)?;
        let file: TrustFile =
            toml::from_str(&text).map_err(|err| KayaError::Security(err.to_string()))?;
        let peers = file
            .peers
            .into_iter()
            .map(|peer| (peer.node_id.clone(), peer))
            .collect();
        Ok(Self { path, peers })
    }

    pub fn record_seen(
        &mut self,
        node_id: &str,
        callsign: &str,
        fingerprint: &str,
    ) -> Result<TrustObservation> {
        let now = now_millis().to_string();
        let observation = match self.peers.get_mut(node_id) {
            Some(peer) if peer.fingerprint != fingerprint => {
                let previous = peer.fingerprint.clone();
                peer.callsign = callsign.to_string();
                peer.fingerprint = fingerprint.to_string();
                peer.last_seen = now;
                TrustObservation::FingerprintChanged {
                    previous,
                    current: fingerprint.to_string(),
                }
            }
            Some(peer) => {
                peer.callsign = callsign.to_string();
                peer.last_seen = now;
                TrustObservation::Updated
            }
            None => {
                self.peers.insert(
                    node_id.to_string(),
                    TrustedPeer {
                        node_id: node_id.to_string(),
                        callsign: callsign.to_string(),
                        fingerprint: fingerprint.to_string(),
                        first_seen: now.clone(),
                        last_seen: now,
                        trust_status: TrustStatus::Unknown,
                    },
                );
                TrustObservation::New
            }
        };
        self.save()?;
        Ok(observation)
    }

    pub fn set_status(&mut self, node_id: &str, status: TrustStatus) -> Result<()> {
        let Some(peer) = self.peers.get_mut(node_id) else {
            return Err(KayaError::Security(format!(
                "peer not in trust store: {node_id}"
            )));
        };
        peer.trust_status = status;
        peer.last_seen = now_millis().to_string();
        self.save()
    }

    pub fn status(&self, node_id: &str) -> TrustStatus {
        self.peers
            .get(node_id)
            .map(|peer| peer.trust_status)
            .unwrap_or(TrustStatus::Unknown)
    }

    pub fn is_blocked(&self, node_id: &str) -> bool {
        self.status(node_id) == TrustStatus::Blocked
    }

    pub fn get(&self, node_id: &str) -> Option<&TrustedPeer> {
        self.peers.get(node_id)
    }

    pub fn find(&self, target: &str) -> Option<&TrustedPeer> {
        self.peers.get(target).or_else(|| {
            self.peers.values().find(|peer| {
                peer.callsign.eq_ignore_ascii_case(target)
                    || peer.fingerprint.eq_ignore_ascii_case(target)
            })
        })
    }

    pub fn list(&self) -> Vec<TrustedPeer> {
        let mut peers: Vec<_> = self.peers.values().cloned().collect();
        peers.sort_by(|left, right| left.callsign.cmp(&right.callsign));
        peers
    }

    pub fn trusted_count(&self) -> usize {
        self.peers
            .values()
            .filter(|peer| peer.trust_status == TrustStatus::Trusted)
            .count()
    }

    pub fn blocked_count(&self) -> usize {
        self.peers
            .values()
            .filter(|peer| peer.trust_status == TrustStatus::Blocked)
            .count()
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut peers: Vec<_> = self.peers.values().cloned().collect();
        peers.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        let text = toml::to_string_pretty(&TrustFile { peers })
            .map_err(|err| KayaError::Security(err.to_string()))?;
        fs::write(&self.path, text)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecureSessionStatus {
    Pending,
    Active,
    Closed,
}

impl std::fmt::Display for SecureSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecureSessionStatus::Pending => f.write_str("pending"),
            SecureSessionStatus::Active => f.write_str("active"),
            SecureSessionStatus::Closed => f.write_str("closed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecureSessionView {
    pub peer_node_id: String,
    pub session_id: String,
    pub created_at: String,
    pub last_used_at: String,
    pub message_counter: u64,
    pub status: SecureSessionStatus,
    pub peer_fingerprint: Option<String>,
}

#[derive(Clone)]
struct SecureSession {
    peer_node_id: String,
    session_id: String,
    created_at: String,
    last_used_at: String,
    message_counter: u64,
    status: SecureSessionStatus,
    key: Option<[u8; 32]>,
    peer_fingerprint: Option<String>,
}

impl SecureSession {
    fn view(&self) -> SecureSessionView {
        SecureSessionView {
            peer_node_id: self.peer_node_id.clone(),
            session_id: self.session_id.clone(),
            created_at: self.created_at.clone(),
            last_used_at: self.last_used_at.clone(),
            message_counter: self.message_counter,
            status: self.status,
            peer_fingerprint: self.peer_fingerprint.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionHandshake {
    pub peer_node_id: String,
    pub session_id: String,
    pub x25519_public_key: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub session_id: String,
    pub nonce: String,
    pub ciphertext: String,
    pub sender_fingerprint: String,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct SecureSessionManager {
    identity: LocalIdentity,
    sessions: HashMap<String, SecureSession>,
}

impl SecureSessionManager {
    pub fn new(identity: LocalIdentity) -> Self {
        Self {
            identity,
            sessions: HashMap::new(),
        }
    }

    pub fn start_request(&mut self, peer_node_id: &str) -> SessionHandshake {
        let session_id = Uuid::new_v4().to_string();
        let now = now_millis().to_string();
        self.sessions.insert(
            peer_node_id.to_string(),
            SecureSession {
                peer_node_id: peer_node_id.to_string(),
                session_id: session_id.clone(),
                created_at: now.clone(),
                last_used_at: now,
                message_counter: 0,
                status: SecureSessionStatus::Pending,
                key: None,
                peer_fingerprint: None,
            },
        );
        SessionHandshake {
            peer_node_id: peer_node_id.to_string(),
            session_id,
            x25519_public_key: self.identity.x25519_public_key_hex(),
            fingerprint: self.identity.fingerprint.clone(),
        }
    }

    pub fn accept_request(
        &mut self,
        peer_node_id: &str,
        session_id: &str,
        peer_x25519_public_key: &str,
        peer_fingerprint: &str,
    ) -> Result<SessionHandshake> {
        let key = self.derive_session_key(session_id, peer_x25519_public_key)?;
        let now = now_millis().to_string();
        self.sessions.insert(
            peer_node_id.to_string(),
            SecureSession {
                peer_node_id: peer_node_id.to_string(),
                session_id: session_id.to_string(),
                created_at: now.clone(),
                last_used_at: now,
                message_counter: 0,
                status: SecureSessionStatus::Active,
                key: Some(key),
                peer_fingerprint: Some(peer_fingerprint.to_string()),
            },
        );
        Ok(SessionHandshake {
            peer_node_id: peer_node_id.to_string(),
            session_id: session_id.to_string(),
            x25519_public_key: self.identity.x25519_public_key_hex(),
            fingerprint: self.identity.fingerprint.clone(),
        })
    }

    pub fn complete_accept(
        &mut self,
        peer_node_id: &str,
        session_id: &str,
        peer_x25519_public_key: &str,
        peer_fingerprint: &str,
    ) -> Result<()> {
        let key = self.derive_session_key(session_id, peer_x25519_public_key)?;
        let now = now_millis().to_string();
        let session = self
            .sessions
            .entry(peer_node_id.to_string())
            .or_insert_with(|| SecureSession {
                peer_node_id: peer_node_id.to_string(),
                session_id: session_id.to_string(),
                created_at: now.clone(),
                last_used_at: now.clone(),
                message_counter: 0,
                status: SecureSessionStatus::Pending,
                key: None,
                peer_fingerprint: None,
            });
        session.session_id = session_id.to_string();
        session.last_used_at = now;
        session.status = SecureSessionStatus::Active;
        session.key = Some(key);
        session.peer_fingerprint = Some(peer_fingerprint.to_string());
        Ok(())
    }

    pub fn has_active(&self, peer_node_id: &str) -> bool {
        self.sessions
            .get(peer_node_id)
            .map(|session| session.status == SecureSessionStatus::Active && session.key.is_some())
            .unwrap_or(false)
    }

    pub fn close(&mut self, peer_node_id: &str) -> bool {
        let Some(session) = self.sessions.get_mut(peer_node_id) else {
            return false;
        };
        session.status = SecureSessionStatus::Closed;
        session.key = None;
        session.last_used_at = now_millis().to_string();
        true
    }

    pub fn encrypt(&mut self, peer_node_id: &str, body: &str) -> Result<EncryptedPayload> {
        let session = self.active_session_mut(peer_node_id)?;
        let key = session
            .key
            .ok_or_else(|| KayaError::Security("secure session has no key".into()))?;
        let mut nonce = [0_u8; NONCE_BYTES];
        OsRng.fill_bytes(&mut nonce);
        let cipher = ChaCha20Poly1305::new((&key).into());
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), body.as_bytes())
            .map_err(|err| KayaError::Security(format!("dm encrypt failed: {err}")))?;

        session.message_counter += 1;
        session.last_used_at = now_millis().to_string();
        Ok(EncryptedPayload {
            session_id: session.session_id.clone(),
            nonce: encode_hex(&nonce),
            ciphertext: encode_hex(&ciphertext),
            sender_fingerprint: self.identity.fingerprint.clone(),
            timestamp: now_millis().to_string(),
        })
    }

    pub fn decrypt(&mut self, peer_node_id: &str, payload: &EncryptedPayload) -> Result<String> {
        let session = self.active_session_mut(peer_node_id)?;
        if session.session_id != payload.session_id {
            return Err(KayaError::Security("encrypted dm session mismatch".into()));
        }
        let key = session
            .key
            .ok_or_else(|| KayaError::Security("secure session has no key".into()))?;
        let nonce = decode_fixed::<NONCE_BYTES>(&payload.nonce)?;
        let ciphertext = decode_hex(&payload.ciphertext)?;
        let cipher = ChaCha20Poly1305::new((&key).into());
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|err| KayaError::Security(format!("dm decrypt failed: {err}")))?;

        session.message_counter += 1;
        session.last_used_at = now_millis().to_string();
        String::from_utf8(plaintext).map_err(|err| KayaError::Security(err.to_string()))
    }

    pub fn views(&self) -> Vec<SecureSessionView> {
        let mut sessions: Vec<_> = self.sessions.values().map(SecureSession::view).collect();
        sessions.sort_by(|left, right| left.peer_node_id.cmp(&right.peer_node_id));
        sessions
    }

    pub fn active_count(&self) -> usize {
        self.sessions
            .values()
            .filter(|session| session.status == SecureSessionStatus::Active)
            .count()
    }

    fn active_session_mut(&mut self, peer_node_id: &str) -> Result<&mut SecureSession> {
        let session = self
            .sessions
            .get_mut(peer_node_id)
            .ok_or_else(|| KayaError::Security(format!("no secure session for {peer_node_id}")))?;
        if session.status != SecureSessionStatus::Active {
            return Err(KayaError::Security(format!(
                "secure session for {peer_node_id} is {}",
                session.status
            )));
        }
        Ok(session)
    }

    fn derive_session_key(
        &self,
        session_id: &str,
        peer_x25519_public_key: &str,
    ) -> Result<[u8; 32]> {
        let peer_public = decode_fixed::<X25519_KEY_BYTES>(peer_x25519_public_key)?;
        let peer_public = X25519PublicKey::from(peer_public);
        let shared = self.identity.x25519_secret().diffie_hellman(&peer_public);
        let hkdf = Hkdf::<Sha256>::new(None, shared.as_bytes());
        let mut key = [0_u8; 32];
        hkdf.expand(format!("kaya-dm-v1:{session_id}").as_bytes(), &mut key)
            .map_err(|err| KayaError::Security(format!("session key derivation failed: {err}")))?;
        Ok(key)
    }
}

pub fn session_request_from_packet(packet: &Packet) -> Result<SessionHandshake> {
    let session_id = payload_str(packet, "session_id")?.to_string();
    let x25519_public_key = payload_str(packet, "x25519_public_key")?.to_string();
    let fingerprint = payload_str(packet, "fingerprint")?.to_string();
    Ok(SessionHandshake {
        peer_node_id: packet.node_id.clone(),
        session_id,
        x25519_public_key,
        fingerprint,
    })
}

pub fn session_accept_from_packet(packet: &Packet) -> Result<SessionHandshake> {
    session_request_from_packet(packet)
}

pub fn encrypted_payload_from_packet(packet: &Packet) -> Result<EncryptedPayload> {
    serde_json::from_value(packet.payload.clone()).map_err(KayaError::from)
}

fn payload_str<'a>(packet: &'a Packet, field: &'static str) -> Result<&'a str> {
    packet
        .payload
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| KayaError::Security(format!("packet payload missing {field}")))
}

pub fn fingerprint_from_ed25519_public_key(public_key: &[u8; ED25519_PUBLIC_KEY_BYTES]) -> String {
    fingerprint_from_bytes(public_key)
}

pub fn fingerprint_from_keys(
    ed25519_public_key: &[u8; ED25519_PUBLIC_KEY_BYTES],
    x25519_public_key: &[u8; X25519_KEY_BYTES],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ed25519_public_key);
    hasher.update(x25519_public_key);
    fingerprint_from_digest(&hasher.finalize())
}

fn fingerprint_from_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    fingerprint_from_digest(&digest)
}

fn fingerprint_from_digest(digest: &[u8]) -> String {
    format!(
        "{FINGERPRINT_PREFIX}{}-{}-{}",
        encode_hex_upper(&digest[0..2]),
        encode_hex_upper(&digest[2..4]),
        encode_hex_upper(&digest[4..6])
    )
}

pub fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(nibble_to_hex(byte >> 4, false));
        out.push(nibble_to_hex(byte & 0x0f, false));
    }
    out
}

pub fn decode_hex(input: &str) -> Result<Vec<u8>> {
    let input = input.trim();
    if !input.len().is_multiple_of(2) {
        return Err(KayaError::Security("hex value has odd length".into()));
    }
    let mut bytes = Vec::with_capacity(input.len() / 2);
    for chunk in input.as_bytes().chunks(2) {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

pub fn decode_fixed<const N: usize>(input: &str) -> Result<[u8; N]> {
    let bytes = decode_hex(input)?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        KayaError::Security(format!("expected {N} bytes, got {}", bytes.len()))
    })
}

fn encode_hex_upper(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(nibble_to_hex(byte >> 4, true));
        out.push(nibble_to_hex(byte & 0x0f, true));
    }
    out
}

fn nibble_to_hex(nibble: u8, upper: bool) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 if upper => (b'A' + (nibble - 10)) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => '0',
    }
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(KayaError::Security("invalid hex value".into())),
    }
}
