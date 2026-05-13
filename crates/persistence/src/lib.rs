use kaya_shared::{KayaError, Result, DEFAULT_ROOM};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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
        Ok(())
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
    pub from: String,
    pub body: String,
    pub direct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownPeer {
    pub node_id: String,
    pub callsign: String,
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
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("KAYA_HOME") {
        return PathBuf::from(path);
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".kaya");
    }

    PathBuf::from(".kaya")
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
            from: "Ana".into(),
            body: "recebido".into(),
            direct: false,
        };

        store.append_history(&record).unwrap();
        assert_eq!(store.list_history(10).unwrap(), vec![record]);

        drop(store);
        let _ = fs::remove_dir_all(path);
    }
}
