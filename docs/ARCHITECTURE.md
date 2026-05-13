# KAYA CLI Architecture

KAYA is a modular Rust workspace designed around strict separation of responsibilities. The first implementation targets a local UDP multicast mesh with terminal-first operation.

## Runtime Flow

1. `app` loads local config from `persistence`.
2. The user chooses a callsign.
3. `shared` generates a temporary `KY-XXXXXX` node id.
4. `transport` binds an IPv4 multicast UDP socket.
5. The network reader task emits `PacketReceived` events.
6. Incoming datagrams are decoded by `protocol`.
7. The runtime deduplicates packet ids.
8. `peer` updates presence and timeout state.
9. `rooms` routes room messages and DMs.
10. Domain events update the UI projection.
11. `persistence` records config, known peers, and basic history.

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
- `ROOM_MESSAGE`: routes public text to a room.
- `DIRECT_MESSAGE`: routes private text to a target node id or callsign.
- `LEAVE`: marks a peer offline.
- `PING`/`PONG`: reserved for latency measurement.

## Phase 1 Boundaries

Phase 1 intentionally avoids centralized identity, encryption, store-and-forward relays, and internet fallback. The design keeps these future additions behind protocol and transport boundaries.
