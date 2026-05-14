const MAX_MESSAGES: usize = 200;
const MAX_LOGS: usize = 200;
const MAX_COMMAND_HISTORY: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiModal {
    FileOffer {
        file_id: String,
        file_name: String,
        from_callsign: String,
        encrypted: bool,
    },
    TrustWarning {
        node_id: Option<String>,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMessage {
    pub timestamp: String,
    pub room: Option<String>,
    pub from: String,
    pub target: Option<String>,
    pub body: String,
    pub direct: bool,
    pub encrypted: bool,
    pub local: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPeer {
    pub node_id: String,
    pub callsign: String,
    pub presence: kaya_shared::PresenceStatus,
    pub fingerprint: Option<String>,
    pub trust_status: String,
    pub online: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRoom {
    pub name: String,
    pub member_count: usize,
    pub joined: bool,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiFileTransfer {
    pub file_id: String,
    pub file_name: String,
    pub peer: String,
    pub percent: f64,
    pub status: String,
    pub security: String,
    pub trusted: bool,
    pub signed: bool,
    pub hash_ok: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiConnection {
    pub peer_node_id: String,
    pub peer_callsign: String,
    pub transport_type: String,
    pub remote_addr: String,
    pub state: String,
    pub latency_ms: Option<u64>,
    pub encrypted_capable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiMeshDiagnostics {
    pub enabled: bool,
    pub routes: u64,
    pub relayed_packets: u64,
    pub delivered_packets: u64,
    pub dropped_packets: u64,
    pub avg_hop_count: u64,
    pub last_route_discovered: Option<String>,
    pub current_route_trace: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UiDiagnostics {
    pub uptime_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub peer_timeout_secs: u64,
    pub packet_max_bytes: usize,
    pub events_total: u64,
    pub events_per_sec: u64,
    pub event_counters: Vec<(String, u64)>,
    pub duplicate_packets: u64,
    pub malformed_packets: u64,
    pub render_time_ms: u64,
    pub memory_kb: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub space: String,
    pub node_id: String,
    pub callsign: String,
    pub status: String,
    pub current_room: String,
    pub presence: kaya_shared::PresenceStatus,
    pub input: String,
    pub messages: Vec<UiMessage>,
    pub direct_messages: Vec<UiMessage>,
    pub rooms: Vec<UiRoom>,
    pub current_members: Vec<String>,
    pub peers: Vec<UiPeer>,
    pub files: Vec<UiFileTransfer>,
    pub connections: Vec<UiConnection>,
    pub direct_listener: Option<String>,
    pub mesh: UiMeshDiagnostics,
    pub identity_fingerprint: String,
    pub trusted_peers: usize,
    pub blocked_peers: usize,
    pub secure_sessions: usize,
    pub security_warnings: u64,
    pub logs: Vec<String>,
    pub show_logs: bool,
    pub show_splash: bool,
    pub modal: Option<UiModal>,
    pub packets_tx: u64,
    pub packets_rx: u64,
    pub bytes_tx: u64,
    pub bytes_rx: u64,
    pub message_scroll: usize,
    pub command_history: Vec<String>,
    pub history_cursor: Option<usize>,
    pub diagnostics: UiDiagnostics,
}

impl UiState {
    pub fn new(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        let room = room.into();
        Self {
            space: room.clone(),
            node_id: node_id.into(),
            callsign: callsign.into(),
            status: "CONNECTED".into(),
            current_room: room,
            presence: kaya_shared::PresenceStatus::Online,
            input: String::new(),
            messages: Vec::new(),
            direct_messages: Vec::new(),
            rooms: Vec::new(),
            current_members: Vec::new(),
            peers: Vec::new(),
            files: Vec::new(),
            connections: Vec::new(),
            direct_listener: None,
            mesh: UiMeshDiagnostics::default(),
            identity_fingerprint: "--".into(),
            trusted_peers: 0,
            blocked_peers: 0,
            secure_sessions: 0,
            security_warnings: 0,
            logs: Vec::new(),
            show_logs: true,
            show_splash: true,
            modal: None,
            packets_tx: 0,
            packets_rx: 0,
            bytes_tx: 0,
            bytes_rx: 0,
            message_scroll: 0,
            command_history: Vec::new(),
            history_cursor: None,
            diagnostics: UiDiagnostics::default(),
        }
    }

    pub fn push_message(&mut self, message: UiMessage) {
        if message.direct {
            self.direct_messages.push(message.clone());
            if self.direct_messages.len() > MAX_MESSAGES {
                let overflow = self.direct_messages.len() - MAX_MESSAGES;
                self.direct_messages.drain(0..overflow);
            }
        } else {
            self.messages.push(message);
            if self.messages.len() > MAX_MESSAGES {
                let overflow = self.messages.len() - MAX_MESSAGES;
                self.messages.drain(0..overflow);
            }
        }
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        self.logs.push(line.into());
        if self.logs.len() > MAX_LOGS {
            let overflow = self.logs.len() - MAX_LOGS;
            self.logs.drain(0..overflow);
        }
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.message_scroll = 0;
    }

    pub fn dismiss_overlays(&mut self) {
        self.show_splash = false;
        self.modal = None;
    }

    pub fn show_file_offer_modal(
        &mut self,
        file_id: impl Into<String>,
        file_name: impl Into<String>,
        from_callsign: impl Into<String>,
        encrypted: bool,
    ) {
        self.modal = Some(UiModal::FileOffer {
            file_id: file_id.into(),
            file_name: file_name.into(),
            from_callsign: from_callsign.into(),
            encrypted,
        });
    }

    pub fn show_trust_warning(&mut self, node_id: Option<String>, message: impl Into<String>) {
        self.modal = Some(UiModal::TrustWarning {
            node_id,
            message: message.into(),
        });
    }

    pub fn record_submitted_input(&mut self, input: &str) {
        if input.trim().is_empty() {
            return;
        }
        if self
            .command_history
            .last()
            .map(|last| last != input)
            .unwrap_or(true)
        {
            self.command_history.push(input.to_string());
            if self.command_history.len() > MAX_COMMAND_HISTORY {
                let overflow = self.command_history.len() - MAX_COMMAND_HISTORY;
                self.command_history.drain(0..overflow);
            }
        }
        self.history_cursor = None;
    }

    pub(crate) fn history_previous(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let next = self
            .history_cursor
            .map(|index| index.saturating_sub(1))
            .unwrap_or_else(|| self.command_history.len().saturating_sub(1));
        self.history_cursor = Some(next);
        self.input = self.command_history[next].clone();
    }

    pub(crate) fn history_next(&mut self) {
        let Some(cursor) = self.history_cursor else {
            return;
        };
        let next = cursor + 1;
        if next >= self.command_history.len() {
            self.history_cursor = None;
            self.input.clear();
        } else {
            self.history_cursor = Some(next);
            self.input = self.command_history[next].clone();
        }
    }

    pub(crate) fn scroll_messages_up(&mut self) {
        self.message_scroll = self.message_scroll.saturating_add(3);
    }

    pub(crate) fn scroll_messages_down(&mut self) {
        self.message_scroll = self.message_scroll.saturating_sub(3);
    }
}
