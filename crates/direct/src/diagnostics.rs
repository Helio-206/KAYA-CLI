#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectDiagnostics {
    pub listener_active: bool,
    pub connections: usize,
    pub packets_tx: u64,
    pub packets_rx: u64,
    pub failed_connects: u64,
}
