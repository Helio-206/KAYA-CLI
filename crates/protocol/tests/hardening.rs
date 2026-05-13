use kaya_protocol::{decode, decode_with_limit, encode, Packet, ProtocolError};
use serde_json::{json, Value};

#[test]
fn rejects_malformed_json_packets() {
    let err = decode(b"{not-json").expect_err("malformed json rejected");
    assert!(matches!(err, ProtocolError::Decode(_)));
}

#[test]
fn rejects_missing_required_schema_fields() {
    let packet = json!({
        "protocol_version": 1,
        "packet_id": "4b8c7d67-1cd2-4f66-b6f5-b58fd2528a58",
        "type": "ROOM_MESSAGE",
        "node_id": "KY-71AF92",
        "callsign": "Ana",
        "timestamp": kaya_shared::now_millis().to_string(),
        "room": null,
        "target_node": null,
        "payload": { "body": "teste" }
    });

    let err = decode(&serde_json::to_vec(&packet).unwrap()).expect_err("missing room rejected");
    assert!(matches!(
        err,
        ProtocolError::MissingField { field: "room", .. }
    ));
}

#[test]
fn rejects_future_timestamps() {
    let mut packet = Packet::hello("KY-71AF92", "Ana", "geral");
    packet.timestamp = (kaya_shared::now_millis() + 10 * 60 * 1000).to_string();

    assert!(matches!(
        packet.validate(),
        Err(ProtocolError::FutureTimestamp)
    ));
}

#[test]
fn fuzz_like_invalid_inputs_do_not_panic() {
    let samples: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0],
        vec![255; 128],
        br#"{"type":"HELLO"}"#.to_vec(),
        br#"[]"#.to_vec(),
        br#"null"#.to_vec(),
    ];

    for sample in samples {
        assert!(decode_with_limit(&sample, 512).is_err());
    }
}

#[test]
fn encoded_packets_stay_under_configured_limit() {
    let packet = Packet::room_message("KY-71AF92", "Ana", "geral", "recebido");
    let bytes = encode(&packet).expect("packet encoded");

    assert!(bytes.len() < 1024);
    assert!(decode_with_limit(&bytes, 1024).is_ok());
    assert!(matches!(
        decode_with_limit(&bytes, 16),
        Err(ProtocolError::PacketTooLarge { .. })
    ));
}

#[test]
fn rejects_unknown_packet_type_without_fallback() {
    let mut value = serde_json::to_value(Packet::hello("KY-71AF92", "Ana", "geral")).unwrap();
    value["type"] = Value::String("BOGUS".into());

    assert!(matches!(
        decode(&serde_json::to_vec(&value).unwrap()),
        Err(ProtocolError::Decode(_))
    ));
}
