use kaya_events::EventCounters;
use kaya_ui::UiDiagnostics;
use std::fs;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct RuntimeDiagnostics {
    started_at: SystemTime,
    pub heartbeat_interval_secs: u64,
    pub peer_timeout_secs: u64,
    pub packet_max_bytes: usize,
    pub duplicate_packets: u64,
    pub malformed_packets: u64,
    pub render_time_ms: u64,
    pub counters: EventCounters,
}

impl RuntimeDiagnostics {
    pub fn new(
        heartbeat_interval_secs: u64,
        peer_timeout_secs: u64,
        packet_max_bytes: usize,
    ) -> Self {
        Self {
            started_at: SystemTime::now(),
            heartbeat_interval_secs,
            peer_timeout_secs,
            packet_max_bytes,
            duplicate_packets: 0,
            malformed_packets: 0,
            render_time_ms: 0,
            counters: EventCounters::default(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        kaya_shared::monotonic_uptime_secs(self.started_at)
    }

    pub fn to_ui(&self) -> UiDiagnostics {
        let uptime_secs = self.uptime_secs();
        let events_total = self.counters.total();
        UiDiagnostics {
            uptime_secs,
            heartbeat_interval_secs: self.heartbeat_interval_secs,
            peer_timeout_secs: self.peer_timeout_secs,
            packet_max_bytes: self.packet_max_bytes,
            events_total,
            events_per_sec: events_total / uptime_secs.max(1),
            event_counters: self.counters.snapshot(),
            duplicate_packets: self.duplicate_packets,
            malformed_packets: self.malformed_packets,
            render_time_ms: self.render_time_ms,
            memory_kb: current_memory_kb(),
        }
    }
}

fn current_memory_kb() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmRSS:")?;
        value
            .split_whitespace()
            .next()
            .and_then(|number| number.parse::<u64>().ok())
    })
}
