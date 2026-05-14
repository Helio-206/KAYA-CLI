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

## Phase 3: Secure Identity and Encrypted DMs

- Local keypair identity.
- Signed packets.
- Encrypted DMs with X25519, HKDF-SHA256, and ChaCha20-Poly1305.
- Trust store with unknown, trusted, and blocked peer states.
- Peer fingerprints in commands and UI.
- Security event counters and warnings.
- Remaining future work: stronger replay windows, passphrase-protected identity files, and encrypted room modes.

## Phase 4: Secure File Transfer

- Explicit file offers and accept/reject commands.
- Safe filename validation and path traversal protection.
- SHA-256 chunk and final-file verification.
- Optional encrypted chunks over active secure sessions.
- Transfer progress and metadata persistence.
- Remaining future work: retransmission windows, resumable transfers, and mesh routing.

## Phase 5: Mesh Extensions

- Multi-interface discovery.
- Optional relay mode.
- Store-and-forward for intermittent peers.
- File capsules for small offline artifacts.
- Protocol compatibility matrix.

## Phase 6: Field Tooling

- Diagnostics dashboard.
- Packet inspector.
- Lab simulator.
- Scripted scenarios.
- Distribution packages.

## Phase 2A: Social Synchronization

- Room creation, join, leave, membership snapshots.
- Room announcements and light state reconciliation.
- Robust DM target resolution with duplicate callsign detection.
- Presence states: online, away, busy, invisible, derived offline.
- Local room and DM history commands.
- TUI rooms/members/peers/DM panels.
