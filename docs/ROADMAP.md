# KAYA Roadmap

## Phase 1: Local Ephemeral Mesh

- Rust workspace.
- JSON protocol v1.
- UDP multicast discovery.
- HELLO, HEARTBEAT, JOIN_ROOM, ROOM_MESSAGE, DIRECT_MESSAGE, LEAVE.
- Peer timeout.
- Internal event bus.
- Packet deduplication.
- Config TOML.
- Runtime diagnostics panel.
- Default room and room switching.
- Basic DMs by callsign or node id.
- ratatui/crossterm TUI.
- sled config, peer cache, and history.
- Unit tests for protocol, commands, peers, rooms, persistence, and decoding.

## Phase 2: Operational Hardening

- Latency measurement with PING/PONG.
- Message id retention tuning.
- Better reconnect behavior.
- Scrollback navigation.
- Command autocomplete.
- Configurable multicast group and port.
- Export/import local state.

## Phase 3: Trust and Privacy

- Local keypair identity.
- Signed packets.
- Optional encrypted DMs.
- Trust-on-first-use peer fingerprints.
- Spoofing and replay protection.

## Phase 4: Mesh Extensions

- Multi-interface discovery.
- Optional relay mode.
- Store-and-forward for intermittent peers.
- File capsules for small offline artifacts.
- Protocol compatibility matrix.

## Phase 5: Field Tooling

- Diagnostics dashboard.
- Packet inspector.
- Lab simulator.
- Scripted scenarios.
- Distribution packages.
