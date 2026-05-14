#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayRoomPolicy {
    pub enabled: bool,
    pub broadcast: bool,
}

impl Default for RelayRoomPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            broadcast: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayFileTransferPolicy {
    pub enabled: bool,
    pub allow_chunks: bool,
    pub max_file_size_mb: u64,
}

impl Default for RelayFileTransferPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_chunks: false,
            max_file_size_mb: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayPolicy {
    pub allow_unknown: bool,
    pub max_clients: usize,
    pub heartbeat_interval_ms: u64,
    pub connection_timeout_ms: u64,
    pub rooms: RelayRoomPolicy,
    pub file_transfer: RelayFileTransferPolicy,
}

impl Default for RelayPolicy {
    fn default() -> Self {
        Self {
            allow_unknown: true,
            max_clients: 100,
            heartbeat_interval_ms: 5_000,
            connection_timeout_ms: 15_000,
            rooms: RelayRoomPolicy::default(),
            file_transfer: RelayFileTransferPolicy::default(),
        }
    }
}
