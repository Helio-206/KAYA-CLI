use crate::errors::{FileTransferError, FileTransferResult};
use crate::metadata::safe_file_name;
use crate::session::TransferSession;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredTransferRecord {
    pub session: TransferSession,
}

#[derive(Debug, Clone)]
pub struct FileStore {
    root: PathBuf,
    completed_dir_override: Option<PathBuf>,
}

impl FileStore {
    pub fn new(
        data_dir: impl AsRef<Path>,
        download_dir: Option<String>,
    ) -> FileTransferResult<Self> {
        let store = Self {
            root: data_dir.as_ref().join("files"),
            completed_dir_override: download_dir.map(expand_home),
        };
        store.ensure_dirs()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn incoming_dir(&self) -> PathBuf {
        self.root.join("incoming")
    }

    pub fn completed_dir(&self) -> PathBuf {
        self.completed_dir_override
            .clone()
            .unwrap_or_else(|| self.root.join("completed"))
    }

    pub fn temp_dir(&self) -> PathBuf {
        self.root.join("temp")
    }

    pub fn metadata_dir(&self) -> PathBuf {
        self.root.join("metadata")
    }

    pub fn ensure_dirs(&self) -> FileTransferResult<()> {
        fs::create_dir_all(self.incoming_dir())?;
        fs::create_dir_all(self.completed_dir())?;
        fs::create_dir_all(self.temp_dir())?;
        fs::create_dir_all(self.metadata_dir())?;
        Ok(())
    }

    pub fn save_record(&self, session: &TransferSession) -> FileTransferResult<()> {
        self.ensure_dirs()?;
        let path = self.metadata_path(&session.file_id);
        let bytes = serde_json::to_vec_pretty(&StoredTransferRecord {
            session: session.clone(),
        })?;
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn list_records(&self) -> FileTransferResult<Vec<StoredTransferRecord>> {
        self.ensure_dirs()?;
        let mut records = Vec::new();
        for entry in fs::read_dir(self.metadata_dir())? {
            let entry = entry?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let bytes = fs::read(entry.path())?;
            records.push(serde_json::from_slice(&bytes)?);
        }
        records.sort_by(
            |left: &StoredTransferRecord, right: &StoredTransferRecord| {
                right.session.updated_at.cmp(&left.session.updated_at)
            },
        );
        Ok(records)
    }

    pub fn save_completed(
        &self,
        session: &TransferSession,
        bytes: &[u8],
    ) -> FileTransferResult<PathBuf> {
        self.ensure_dirs()?;
        let name = safe_file_name(&session.metadata.file_name)?;
        let path = self
            .completed_dir()
            .join(format!("{}-{name}", session.metadata.file_id));
        fs::write(&path, bytes)?;
        Ok(path)
    }

    fn metadata_path(&self, file_id: &str) -> PathBuf {
        self.metadata_dir().join(format!("{file_id}.json"))
    }
}

fn expand_home(path: String) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

impl From<FileTransferError> for kaya_shared::KayaError {
    fn from(value: FileTransferError) -> Self {
        kaya_shared::KayaError::Storage(value.to_string())
    }
}
