# KAYA CLI Architecture

KAYA is a modular Rust workspace designed around strict separation of responsibilities. The first implementation targets a local UDP multicast mesh with terminal-first operation.

## Runtime Flow

1. `app` loads local config from `persistence`.
2. The user chooses a callsign.
3. `security` loads or creates `~/.kaya/identity.toml`.
4. `transport` binds an IPv4 multicast UDP socket.
5. The network reader task emits `PacketReceived` events.
6. Incoming datagrams are decoded by `protocol`.
7. The runtime deduplicates packet ids.
8. `security` verifies signed packet envelopes and applies block rules.
9. `peer` updates presence and timeout state.
10. `rooms` routes room messages and plaintext DMs.
11. Secure DM packets are routed through `security` session state.
12. Domain events update the UI projection.
13. `persistence` records config, known peers, and basic history.

## Crates

### app

Application bootstrap and orchestration. Owns the Tokio runtime loop, heartbeat, pruning, command handling, event consumption, and integration between crates.

### events

Internal event bus built on Tokio broadcast channels. It carries packet, peer, room, error, network, and shutdown events.

### protocol

Defines the JSON packet contract, packet types, constructors, encode/decode helpers, and validation.

### transport

Owns UDP multicast socket setup. Uses `socket2` so multiple KAYA processes can bind the same multicast port during local labs.

### peer

Tracks nearby nodes, callsigns, rooms, online/offline state, and timeouts.

### rooms

Tracks room membership, current room, public message history, and direct messages accepted for the local node.

### security

Owns persistent local cryptographic identity, Ed25519 packet signing, peer fingerprints, trust store state, X25519 secure DM session setup, HKDF key derivation, and ChaCha20-Poly1305 encrypted DM payload handling.

### commands

Parses terminal input into typed commands or room messages.

### persistence

Stores `~/.kaya/config.toml`, known peers, and basic history. Config is TOML; history/cache use sled.

### ui

Renders the operational TUI using ratatui and crossterm.

### shared

Common constants, node id generation, errors, timestamps, and normalization helpers.

## Event Model

KAYA uses packet-driven state behind an internal event bus:

```text
transport -> PacketReceived -> runtime -> peer/rooms -> domain events -> UI projection
```

- `HELLO`: announces a node.
- `HEARTBEAT`: refreshes peer presence.
- `JOIN_ROOM`: adds a node to a room.
- `ROOM_ANNOUNCE`: advertises known rooms.
- `ROOM_JOIN` / `ROOM_LEAVE`: synchronizes lightweight room membership.
- `ROOM_MEMBERS_REQUEST` / `ROOM_MEMBERS_RESPONSE`: reconciles a light member snapshot.
- `ROOM_MESSAGE`: routes public text to a room.
- `DIRECT_MESSAGE`: routes private text to a target node id or callsign.
- `DM_SESSION_REQUEST` / `DM_SESSION_ACCEPT`: establishes encrypted DM session state.
- `DIRECT_MESSAGE_ENCRYPTED`: routes encrypted private text after local decryption.
- `LEAVE`: marks a peer offline.
- `PRESENCE_UPDATE`: updates peer presence.
- `PING`/`PONG`: reserved for latency measurement.

## Phase 1 Boundaries

Phase 1 intentionally avoids centralized identity, encryption, store-and-forward relays, and internet fallback. The design keeps these future additions behind protocol and transport boundaries.

## Phase 2 Social Sync

Phase 2 adds a light social synchronization layer:

- rooms can be created, announced, joined, and left;
- peers exchange room membership snapshots on join/hello;
- DMs resolve callsigns to node ids and reject ambiguous callsigns;
- presence is tracked as `online`, `away`, `busy`, `invisible`, or derived `offline`;
- local history stores room messages and DMs through the persistence crate.

## Phase 3 Security Layer

Phase 3 keeps public rooms compatible and adds security around identity and DMs:

- every current KAYA node signs outgoing packets with its local Ed25519 identity;
- peers are fingerprinted from their signing public key and recorded in the trust store;
- blocked peers are rejected before peer, room, or DM routing;
- `/secure-msg` negotiates an X25519 session and queues the first message until `DM_SESSION_ACCEPT`;
- encrypted DMs are decrypted in runtime before being projected as `[SECURE]` chat events.
