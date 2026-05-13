const MAX_MESSAGES: usize = 200;
const MAX_LOGS: usize = 200;
const MAX_COMMAND_HISTORY: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMessage {
    pub room: Option<String>,
    pub from: String,
    pub target: Option<String>,
    pub body: String,
    pub direct: bool,
    pub local: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPeer {
    pub node_id: String,
    pub callsign: String,
    pub online: bool,
    pub latency_ms: Option<u64>,
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
    pub input: String,
    pub messages: Vec<UiMessage>,
    pub peers: Vec<UiPeer>,
    pub logs: Vec<String>,
    pub show_logs: bool,
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
            input: String::new(),
            messages: Vec::new(),
            peers: Vec::new(),
            logs: Vec::new(),
            show_logs: true,
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
        self.messages.push(message);
        if self.messages.len() > MAX_MESSAGES {
            let overflow = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..overflow);
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
