use crate::errors::{DirectError, DirectResult};
use kaya_protocol::{Packet, PacketType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPeerHello {
    pub node_id: String,
    pub callsign: String,
    pub capabilities: Vec<String>,
    pub fingerprint: Option<String>,
}

pub fn validate_hello(packet: &Packet) -> DirectResult<DirectPeerHello> {
    if packet.packet_type != PacketType::Hello {
        return Err(DirectError::InvalidHandshake(format!(
            "expected HELLO, got {:?}",
            packet.packet_type
        )));
    }

    let capabilities = packet
        .payload
        .get("capabilities")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    let fingerprint = packet
        .payload
        .get("fingerprint")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    Ok(DirectPeerHello {
        node_id: packet.node_id.clone(),
        callsign: packet.callsign.clone(),
        capabilities,
        fingerprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaya_protocol::Packet;
    use serde_json::json;

    #[test]
    fn accepts_hello_handshake() {
        let mut packet = Packet::hello("KY-71AF92", "Ana", "geral");
        packet.payload = json!({
            "capabilities": ["direct_tcp", "encrypted_dm"],
            "fingerprint": "KAYA-FP: 8A19-FC90-B2D1"
        });

        let hello = validate_hello(&packet).unwrap();

        assert_eq!(hello.node_id, "KY-71AF92");
        assert!(hello.capabilities.contains(&"direct_tcp".to_string()));
    }

    #[test]
    fn rejects_non_hello_handshake() {
        let packet = Packet::ping("KY-71AF92", "Ana", "KY-A91C0D");
        let err = validate_hello(&packet).unwrap_err();

        assert!(matches!(err, DirectError::InvalidHandshake(_)));
    }
}
