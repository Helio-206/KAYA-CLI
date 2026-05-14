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
  },
  "public_key": "ed25519-public-key-hex",
  "signature": "ed25519-signature-hex"
}
```

`timestamp` is a unix millisecond string. `public_key` and `signature` are optional so public room traffic remains compatible with earlier Phase 1/2 nodes.

Signed packets are verified over a canonical JSON representation of the packet with `signature = null`/absent. Private keys are local-only and never transmitted.

## Packet Types

| Type | Required fields | Purpose |
| --- | --- | --- |
| `HELLO` | `room` | Announces node presence and capabilities. |
| `HEARTBEAT` | `room` | Keeps the peer online. |
| `LEAVE` | `room` | Announces local shutdown. |
| `JOIN_ROOM` | `room` | Legacy alias for room membership. |
| `ROOM_ANNOUNCE` | `room` | Announces a known room without requiring peers to join it. |
| `ROOM_JOIN` | `room` | Announces current room membership. |
| `ROOM_LEAVE` | `room` | Announces that a node left a room. |
| `ROOM_MEMBERS_REQUEST` | `room` | Requests a light member snapshot for a room. |
| `ROOM_MEMBERS_RESPONSE` | `room`, `payload.members` | Sends a light room member snapshot. |
| `ROOM_MESSAGE` | `room`, `payload.body` | Sends public room text. |
| `DIRECT_MESSAGE` | `target_node`, `payload.body` | Sends a private message. |
| `DM_ACK` | `target_node`, `payload.ack` | Optional acknowledgement for a received DM. |
| `DM_SESSION_REQUEST` | `target_node`, `payload.session_id`, `payload.x25519_public_key`, `payload.fingerprint` | Starts an encrypted DM session. |
| `DM_SESSION_ACCEPT` | `target_node`, `payload.session_id`, `payload.x25519_public_key`, `payload.fingerprint` | Accepts an encrypted DM session. |
| `DIRECT_MESSAGE_ENCRYPTED` | `target_node`, encrypted payload fields | Sends a ChaCha20-Poly1305 encrypted DM. |
| `PRESENCE_UPDATE` | `room`, `payload.presence` | Announces `online`, `away`, `busy`, or `invisible`. |
| `PING` | `target_node` | Reserved for latency measurement. |
| `PONG` | `target_node` | Reply to ping/hello. |
| `SYSTEM` | `payload.message` | Reserved for local/system notifications. |
| `ERROR` | `payload.message` | Reserved for protocol errors. |

## Node ID

Phase 3 persists node identity in `~/.kaya/identity.toml`. The node id keeps the same format:

```text
KY-XXXXXX
```

Example:

```text
KY-71AF92
```

The suffix uses six uppercase hexadecimal characters derived from a UUID when the identity is first created.

## Signed Packet Envelope

These packet types are signature-checked when a signature envelope is present:

- `HELLO`
- `HEARTBEAT`
- `ROOM_JOIN`
- `ROOM_LEAVE`
- `PRESENCE_UPDATE`
- `DIRECT_MESSAGE`
- `DM_SESSION_REQUEST`
- `DM_SESSION_ACCEPT`
- `DIRECT_MESSAGE_ENCRYPTED`

If `public_key`/`signature` is malformed or does not verify, the packet is rejected and a security warning is emitted. Unsigned packets are still accepted for protocol compatibility, but they cannot populate the trust store with a fingerprint.

## Encrypted DM Payload

`DIRECT_MESSAGE_ENCRYPTED` uses this payload:

```json
{
  "session_id": "c44d7c17-2f41-4f2e-b2e8-806e4f0df76e",
  "nonce": "12-byte-hex",
  "ciphertext": "ciphertext-hex",
  "sender_fingerprint": "KAYA-FP: 8A19-FC90-B2D1",
  "timestamp": "1778673210123"
}
```

Session setup uses X25519 public keys exchanged in `DM_SESSION_REQUEST` and `DM_SESSION_ACCEPT`. Both sides derive a symmetric key with HKDF-SHA256 and encrypt DMs with ChaCha20-Poly1305.

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
- invalid room names;
- room packets without `room`;
- direct packets without `target_node`;
- message packets without `payload.body`.
- malformed signature envelopes;
- malformed encrypted DM/session payloads.

## Room Names

Room names are normalized to lowercase without a leading `#`.

Allowed characters:

```text
a-z 0-9 - _ .
```

Maximum length:

```text
48 characters
```

## Presence

Valid presence values:

- `online`
- `away`
- `busy`
- `invisible`
- `offline`

Clients may emit `online`, `away`, `busy`, and `invisible`. `offline` is derived from graceful leave or peer timeout.
## Versioning

Phase 1 through Phase 3 use `protocol_version = 1`. New packet types and optional envelope fields are additive; clients should ignore packet types they do not understand only at the transport/runtime boundary, never by silently treating them as known commands.
