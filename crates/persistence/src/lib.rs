use kaya_shared::{KayaError, Result, DEFAULT_ROOM};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const CONFIG_KEY: &[u8] = b"config";
const HISTORY_PREFIX: &str = "history:";
const PEER_PREFIX: &str = "peer:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalConfig {
    pub callsign: Option<String>,
    pub theme: String,
    pub last_room: String,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            callsign: None,
            theme: "kaya-dark".into(),
            last_room: DEFAULT_ROOM.into(),
        }
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

    pub fn load_config(&self) -> Result<LocalConfig> {
        let Some(bytes) = self
            .db
            .get(CONFIG_KEY)
            .map_err(|err| KayaError::Storage(err.to_string()))?
        else {
            return Ok(LocalConfig::default());
        };
        serde_json::from_slice(&bytes).map_err(Into::into)
    }

    pub fn save_config(&self, config: &LocalConfig) -> Result<()> {
        let bytes = serde_json::to_vec(config)?;
        self.db
            .insert(CONFIG_KEY, bytes)
            .map_err(|err| KayaError::Storage(err.to_string()))?;
        self.db
            .flush()
            .map_err(|err| KayaError::Storage(err.to_string()))?;
        Ok(())
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
    fn saves_and_loads_config() {
        let (store, path) = test_store();
        let config = LocalConfig {
            callsign: Some("Helio".into()),
            theme: "kaya-dark".into(),
            last_room: "semana-info".into(),
        };

        store.save_config(&config).unwrap();
        assert_eq!(store.load_config().unwrap(), config);

        drop(store);
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
