# KAYA CLI Architecture

KAYA is a modular Rust workspace designed around strict separation of responsibilities. The first implementation targets a local UDP multicast mesh with terminal-first operation.

## Runtime Flow

1. `app` loads local config from `persistence`.
2. The user chooses a callsign.
3. `shared` generates a temporary `KY-XXXXXX` node id.
4. `transport` binds an IPv4 multicast UDP socket.
5. `app` broadcasts `HELLO` and `JOIN_ROOM`.
6. Incoming datagrams are decoded by `protocol`.
7. `peer` updates presence and timeout state.
8. `rooms` routes room messages and DMs.
9. `ui` renders traffic, peers, logs, and input.
10. `persistence` records config, known peers, and basic history.

## Crates

### app

Application bootstrap and orchestration. Owns the Tokio runtime loop, heartbeat, pruning, command handling, and integration between crates.

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

Stores local config, known peers, and basic history in sled.

### ui

Renders the operational TUI using ratatui and crossterm.

### shared

Common constants, node id generation, errors, timestamps, and normalization helpers.

## Event Model

KAYA currently uses simple packet-driven state:

- `HELLO`: announces a node.
- `HEARTBEAT`: refreshes peer presence.
- `JOIN_ROOM`: adds a node to a room.
- `ROOM_MESSAGE`: routes public text to a room.
- `DIRECT_MESSAGE`: routes private text to a target node id or callsign.
- `LEAVE`: marks a peer offline.
- `PING`/`PONG`: reserved for latency measurement.

## Phase 1 Boundaries

Phase 1 intentionally avoids centralized identity, encryption, store-and-forward relays, and internet fallback. The design keeps these future additions behind protocol and transport boundaries.
