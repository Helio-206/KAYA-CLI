use kaya_shared::{KayaError, Result, DEFAULT_ROOM};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

const HISTORY_PREFIX: &str = "history:";
const PEER_PREFIX: &str = "peer:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KayaConfig {
    pub nickname: Option<String>,
    pub multicast_address: String,
    pub multicast_port: u16,
    pub heartbeat_interval_secs: u64,
    pub peer_timeout_secs: u64,
    pub theme: String,
    pub packet_max_bytes: usize,
    pub default_room: String,
    pub last_room: Option<String>,
    pub log_level: String,
    #[serde(default)]
    pub file_transfer: FileTransferSettings,
    #[serde(default)]
    pub mesh: MeshSettings,
    #[serde(default)]
    pub timeouts: TimeoutSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigProfile {
    Default,
    Demo,
    Lab,
    Paranoid,
}

impl ConfigProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Demo => "demo",
            Self::Lab => "lab",
            Self::Paranoid => "paranoid",
        }
    }

    pub fn default_data_dir(self) -> PathBuf {
        match self {
            Self::Demo => default_named_data_dir(".kaya-demo"),
            Self::Lab => default_named_data_dir(".kaya-lab"),
            Self::Paranoid => default_named_data_dir(".kaya-paranoid"),
            Self::Default => default_named_data_dir(".kaya"),
        }
    }
}

impl FromStr for ConfigProfile {
    type Err = KayaError;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "default" => Ok(Self::Default),
            "demo" => Ok(Self::Demo),
            "lab" => Ok(Self::Lab),
            "paranoid" => Ok(Self::Paranoid),
            other => Err(KayaError::Config(format!(
                "unknown profile {other}; expected default, demo, lab or paranoid"
            ))),
        }
    }
}

impl Default for KayaConfig {
    fn default() -> Self {
        Self {
            nickname: None,
            multicast_address: kaya_shared::MULTICAST_IPV4.into(),
            multicast_port: kaya_shared::MULTICAST_PORT,
            heartbeat_interval_secs: kaya_shared::HEARTBEAT_INTERVAL_SECS,
            peer_timeout_secs: kaya_shared::PEER_TIMEOUT_SECS,
            theme: "kaya-dark".into(),
            packet_max_bytes: kaya_shared::MAX_PACKET_BYTES,
            default_room: DEFAULT_ROOM.into(),
            last_room: None,
            log_level: "kaya=info".into(),
            file_transfer: FileTransferSettings::default(),
            mesh: MeshSettings::default(),
            timeouts: TimeoutSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTransferSettings {
    pub enabled: bool,
    pub max_file_size_mb: u64,
    pub chunk_size_kb: u64,
    pub accept_from_unknown: bool,
    pub download_dir: Option<String>,
}

impl Default for FileTransferSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size_mb: 50,
            chunk_size_kb: 64,
            accept_from_unknown: true,
            download_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshSettings {
    pub enabled: bool,
    pub max_ttl: u8,
    pub allow_relay_for_unknown: bool,
    pub allow_relay_for_blocked: bool,
    pub relay_encrypted_only: bool,
    pub route_expiry_seconds: u64,
    pub max_seen_packets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutSettings {
    pub packet_send_ms: u64,
    pub secure_session_ms: u64,
    pub file_transfer_idle_ms: u64,
    pub route_discovery_ms: u64,
    pub shutdown_ms: u64,
    pub network_recv_ms: u64,
}

impl Default for TimeoutSettings {
    fn default() -> Self {
        Self {
            packet_send_ms: 1_500,
            secure_session_ms: 10_000,
            file_transfer_idle_ms: 30_000,
            route_discovery_ms: 5_000,
            shutdown_ms: 3_000,
            network_recv_ms: 1_000,
        }
    }
}

impl Default for MeshSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_ttl: 5,
            allow_relay_for_unknown: true,
            allow_relay_for_blocked: false,
            relay_encrypted_only: false,
            route_expiry_seconds: 120,
            max_seen_packets: 5000,
        }
    }
}

impl KayaConfig {
    pub fn active_room(&self) -> &str {
        self.last_room
            .as_deref()
            .filter(|room| !room.trim().is_empty())
            .unwrap_or(&self.default_room)
    }

    pub fn validate(&self) -> Result<()> {
        if self
            .multicast_address
            .parse::<std::net::Ipv4Addr>()
            .is_err()
        {
            return Err(KayaError::Config(format!(
                "invalid multicast_address {}",
                self.multicast_address
            )));
        }
        if self.multicast_port == 0 {
            return Err(KayaError::Config("multicast_port cannot be 0".into()));
        }
        if self.heartbeat_interval_secs == 0 {
            return Err(KayaError::Config(
                "heartbeat_interval_secs must be greater than 0".into(),
            ));
        }
        if self.peer_timeout_secs <= self.heartbeat_interval_secs {
            return Err(KayaError::Config(
                "peer_timeout_secs must be greater than heartbeat_interval_secs".into(),
            ));
        }
        if self.packet_max_bytes < kaya_shared::MIN_PACKET_BYTES {
            return Err(KayaError::Config(format!(
                "packet_max_bytes must be at least {}",
                kaya_shared::MIN_PACKET_BYTES
            )));
        }
        if self.default_room.trim().is_empty() {
            return Err(KayaError::Config("default_room cannot be empty".into()));
        }
        if self.file_transfer.enabled && self.file_transfer.max_file_size_mb == 0 {
            return Err(KayaError::Config(
                "file_transfer.max_file_size_mb must be greater than 0".into(),
            ));
        }
        if self.file_transfer.enabled && self.file_transfer.chunk_size_kb == 0 {
            return Err(KayaError::Config(
                "file_transfer.chunk_size_kb must be greater than 0".into(),
            ));
        }
        if self.mesh.enabled && self.mesh.max_ttl == 0 {
            return Err(KayaError::Config(
                "mesh.max_ttl must be greater than 0".into(),
            ));
        }
        if self.mesh.enabled && self.mesh.route_expiry_seconds == 0 {
            return Err(KayaError::Config(
                "mesh.route_expiry_seconds must be greater than 0".into(),
            ));
        }
        if self.mesh.enabled && self.mesh.max_seen_packets == 0 {
            return Err(KayaError::Config(
                "mesh.max_seen_packets must be greater than 0".into(),
            ));
        }
        if self.timeouts.packet_send_ms == 0
            || self.timeouts.secure_session_ms == 0
            || self.timeouts.file_transfer_idle_ms == 0
            || self.timeouts.route_discovery_ms == 0
            || self.timeouts.shutdown_ms == 0
            || self.timeouts.network_recv_ms == 0
        {
            return Err(KayaError::Config(
                "timeout values must be greater than 0".into(),
            ));
        }
        Ok(())
    }

    pub fn apply_profile(&mut self, profile: ConfigProfile) {
        match profile {
            ConfigProfile::Default => {}
            ConfigProfile::Demo => {
                self.log_level = "kaya=info".into();
                self.default_room = "semana-info".into();
                self.file_transfer.accept_from_unknown = true;
                self.mesh.enabled = true;
                self.mesh.allow_relay_for_unknown = true;
                self.mesh.relay_encrypted_only = false;
                self.timeouts.route_discovery_ms = 2_500;
            }
            ConfigProfile::Lab => {
                self.log_level = "kaya=debug".into();
                self.peer_timeout_secs = self.peer_timeout_secs.max(15);
                self.mesh.route_expiry_seconds = self.mesh.route_expiry_seconds.max(180);
                self.file_transfer.max_file_size_mb = self.file_transfer.max_file_size_mb.max(100);
                self.timeouts.file_transfer_idle_ms =
                    self.timeouts.file_transfer_idle_ms.max(45_000);
            }
            ConfigProfile::Paranoid => {
                self.log_level = "kaya=info,kaya_security=debug".into();
                self.file_transfer.accept_from_unknown = false;
                self.mesh.allow_relay_for_unknown = false;
                self.mesh.allow_relay_for_blocked = false;
                self.mesh.relay_encrypted_only = true;
                self.timeouts.secure_session_ms = self.timeouts.secure_session_ms.min(8_000);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    data_dir: PathBuf,
}

impl ConfigStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }

    pub fn load_or_create(&self) -> Result<KayaConfig> {
        let path = self.path();
        if !path.exists() {
            let config = KayaConfig::default();
            self.save(&config)?;
            return Ok(config);
        }

        let text = fs::read_to_string(&path)?;
        let config: KayaConfig =
            toml::from_str(&text).map_err(|err| KayaError::Config(err.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, config: &KayaConfig) -> Result<()> {
        config.validate()?;
        fs::create_dir_all(&self.data_dir)?;
        let text =
            toml::to_string_pretty(config).map_err(|err| KayaError::Config(err.to_string()))?;
        fs::write(self.path(), text)?;
        Ok(())
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new(default_data_dir())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub timestamp: String,
    pub room: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    pub from: String,
    pub body: String,
    pub direct: bool,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default)]
    pub event: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownPeer {
    pub node_id: String,
    pub callsign: String,
    #[serde(default)]
    pub fingerprint: Option<String>,
    pub last_seen: String,
}

#[derive(Debug)]
pub struct Store {
    db: sled::Db,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path).map_err(|err| KayaError::Storage(err.to_string()))?;
        Ok(Self { db })
    }

    pub fn open_default() -> Result<Self> {
        Self::open(default_data_dir())
    }

    pub fn append_history(&self, record: &HistoryRecord) -> Result<()> {
        let key = format!("{HISTORY_PREFIX}{}:{}", record.timestamp, Uuid::new_v4());
        let bytes = serde_json::to_vec(record)?;
        self.db
            .insert(key.as_bytes(), bytes)
            .map_err(|err| KayaError::Storage(err.to_string()))?;
        Ok(())
    }

    pub fn list_history(&self, limit: usize) -> Result<Vec<HistoryRecord>> {
        let mut records: Vec<HistoryRecord> = Vec::new();
        for item in self.db.scan_prefix(HISTORY_PREFIX.as_bytes()) {
            let (_, value) = item.map_err(|err| KayaError::Storage(err.to_string()))?;
            records.push(serde_json::from_slice(&value)?);
        }
        if records.len() > limit {
            records = records.split_off(records.len() - limit);
        }
        Ok(records)
    }

    pub fn list_room_history(&self, room: &str, limit: usize) -> Result<Vec<HistoryRecord>> {
        let room = kaya_shared::validate_room_name(room)?;
        let mut records: Vec<_> = self
            .list_history(usize::MAX)?
            .into_iter()
            .filter(|record| record.room.as_deref() == Some(room.as_str()) && !record.direct)
            .collect();
        if records.len() > limit {
            records = records.split_off(records.len() - limit);
        }
        Ok(records)
    }

    pub fn list_dm_history(&self, peer: &str, limit: usize) -> Result<Vec<HistoryRecord>> {
        let peer = peer.to_ascii_lowercase();
        let mut records: Vec<_> = self
            .list_history(usize::MAX)?
            .into_iter()
            .filter(|record| {
                record.direct
                    && (record.from.eq_ignore_ascii_case(&peer)
                        || record
                            .target
                            .as_deref()
                            .map(|target| target.eq_ignore_ascii_case(&peer))
                            .unwrap_or(false))
            })
            .collect();
        if records.len() > limit {
            records = records.split_off(records.len() - limit);
        }
        Ok(records)
    }

    pub fn remember_peer(&self, peer: &KnownPeer) -> Result<()> {
        let key = format!("{PEER_PREFIX}{}", peer.node_id);
        let bytes = serde_json::to_vec(peer)?;
        self.db
            .insert(key.as_bytes(), bytes)
            .map_err(|err| KayaError::Storage(err.to_string()))?;
        Ok(())
    }

    pub fn list_known_peers(&self) -> Result<Vec<KnownPeer>> {
        let mut peers: Vec<KnownPeer> = Vec::new();
        for item in self.db.scan_prefix(PEER_PREFIX.as_bytes()) {
            let (_, value) = item.map_err(|err| KayaError::Storage(err.to_string()))?;
            peers.push(serde_json::from_slice(&value)?);
        }
        peers.sort_by(|left, right| left.callsign.cmp(&right.callsign));
        Ok(peers)
    }

    pub fn flush(&self) -> Result<()> {
        self.db
            .flush()
            .map(|_| ())
            .map_err(|err| KayaError::Storage(err.to_string()))
    }
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("KAYA_HOME") {
        return PathBuf::from(path);
    }

    default_named_data_dir(".kaya")
}

pub fn profile_data_dir(profile: ConfigProfile) -> PathBuf {
    if let Ok(path) = std::env::var("KAYA_HOME") {
        return PathBuf::from(path);
    }

    profile.default_data_dir()
}

fn default_named_data_dir(dir_name: &str) -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(dir_name);
    }

    PathBuf::from(dir_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaya_shared::now_millis;
    use std::fs;

    fn test_store() -> (Store, PathBuf) {
        let path = std::env::temp_dir().join(format!("kaya-persistence-{}", Uuid::new_v4()));
        (Store::open(&path).unwrap(), path)
    }

    #[test]
    fn saves_and_loads_toml_config() {
        let path = std::env::temp_dir().join(format!("kaya-config-{}", Uuid::new_v4()));
        let config_store = ConfigStore::new(&path);
        let config = KayaConfig {
            nickname: Some("Helio".into()),
            multicast_address: "239.71.0.1".into(),
            multicast_port: 42424,
            heartbeat_interval_secs: 3,
            peer_timeout_secs: 12,
            theme: "kaya-dark".into(),
            packet_max_bytes: kaya_shared::MAX_PACKET_BYTES,
            default_room: "geral".into(),
            last_room: Some("semana-info".into()),
            log_level: "kaya=debug".into(),
            file_transfer: FileTransferSettings::default(),
            mesh: MeshSettings::default(),
            timeouts: TimeoutSettings::default(),
        };

        config_store.save(&config).unwrap();
        assert_eq!(config_store.load_or_create().unwrap(), config);

        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn stores_history_records() {
        let (store, path) = test_store();
        let record = HistoryRecord {
            timestamp: now_millis().to_string(),
            room: Some("geral".into()),
            target: None,
            from: "Ana".into(),
            body: "recebido".into(),
            direct: false,
            encrypted: false,
            event: false,
        };

        store.append_history(&record).unwrap();
        assert_eq!(store.list_history(10).unwrap(), vec![record]);

        drop(store);
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn applies_demo_profile_defaults() {
        let mut config = KayaConfig::default();

        config.apply_profile(ConfigProfile::Demo);

        assert_eq!(config.default_room, "semana-info");
        assert!(config.file_transfer.accept_from_unknown);
        assert!(config.mesh.enabled);
    }

    #[test]
    fn applies_paranoid_profile_restrictions() {
        let mut config = KayaConfig::default();

        config.apply_profile(ConfigProfile::Paranoid);

        assert!(!config.file_transfer.accept_from_unknown);
        assert!(!config.mesh.allow_relay_for_unknown);
        assert!(config.mesh.relay_encrypted_only);
    }

    #[test]
    fn rejects_zero_timeout_settings() {
        let mut config = KayaConfig::default();
        config.timeouts.shutdown_ms = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn filters_room_and_dm_history() {
        let (store, path) = test_store();
        let room_record = HistoryRecord {
            timestamp: now_millis().to_string(),
            room: Some("geral".into()),
            target: None,
            from: "Ana".into(),
            body: "sala".into(),
            direct: false,
            encrypted: false,
            event: false,
        };
        let dm_record = HistoryRecord {
            timestamp: (now_millis() + 1).to_string(),
            room: None,
            target: Some("Helio".into()),
            from: "Bruno".into(),
            body: "dm".into(),
            direct: true,
            encrypted: true,
            event: false,
        };

        store.append_history(&room_record).unwrap();
        store.append_history(&dm_record).unwrap();

        assert_eq!(
            store.list_room_history("geral", 10).unwrap(),
            vec![room_record]
        );
        assert_eq!(store.list_dm_history("Bruno", 10).unwrap(), vec![dm_record]);

        drop(store);
        let _ = fs::remove_dir_all(path);
    }
}
