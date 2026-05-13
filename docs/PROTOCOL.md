# KAYA Protocol

KAYA protocol version: `1`

Transport: JSON packets over IPv4 UDP multicast.

Default multicast group: `239.71.0.1:42424`

## Packet Shape

```json
{
  "protocol_version": 1,
  "packet_id": "4b8c7d67-1cd2-4f66-b6f5-b58fd2528a58",
  "type": "ROOM_MESSAGE",
  "node_id": "KY-71AF92",
  "callsign": "Helio",
  "timestamp": "1778673210123",
  "room": "semana-info",
  "target_node": null,
  "payload": {
    "body": "alguem recebe?"
  }
}
```

`timestamp` is a unix millisecond string in Phase 1.

## Packet Types

| Type | Required fields | Purpose |
| --- | --- | --- |
| `HELLO` | `room` | Announces node presence and capabilities. |
| `HEARTBEAT` | `room` | Keeps the peer online. |
| `LEAVE` | `room` | Announces local shutdown. |
| `JOIN_ROOM` | `room` | Announces current room membership. |
| `ROOM_MESSAGE` | `room`, `payload.body` | Sends public room text. |
| `DIRECT_MESSAGE` | `target_node`, `payload.body` | Sends a private message. |
| `PING` | `target_node` | Reserved for latency measurement. |
| `PONG` | `target_node` | Reply to ping/hello. |
| `SYSTEM` | `payload.message` | Reserved for local/system notifications. |
| `ERROR` | `payload.message` | Reserved for protocol errors. |

## Node ID

Each runtime generates a temporary node id:

```text
KY-XXXXXX
```

Example:

```text
KY-71AF92
```

The suffix uses six uppercase hexadecimal characters derived from a UUID.

## Validation

The protocol crate rejects:

- unsupported protocol versions;
- nil packet ids;
- malformed node ids;
- empty callsigns;
- missing, zero, or far-future timestamps;
- non-object payloads;
- packets above the configured byte limit;
- malformed JSON;
- unknown packet types;
- room packets without `room`;
- direct packets without `target_node`;
- message packets without `payload.body`.

## Versioning

Phase 1 uses `protocol_version = 1`. Future versions should add capabilities in `payload` first and only increment the protocol version when old clients cannot safely ignore the new fields.
