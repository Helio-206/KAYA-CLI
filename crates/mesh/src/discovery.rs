use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDescriptor {
    pub destination_node: String,
    pub destination_callsign: Option<String>,
    pub hop_count: u8,
    pub score: i64,
    pub trusted: bool,
    pub encrypted_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAnnouncement {
    pub routes: Vec<RouteDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRequest {
    pub request_id: String,
    pub destination_node: String,
    pub ttl: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteResponse {
    pub request_id: String,
    pub destination_node: String,
    pub destination_callsign: Option<String>,
    pub next_hop: String,
    pub hop_count: u8,
    pub score: i64,
    pub route_trace: Vec<String>,
    pub trusted: bool,
    pub encrypted_capable: bool,
}
