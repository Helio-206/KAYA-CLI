# KAYA CLI

KAYA CLI is an offline-first, decentralized, terminal-based communication system for local networks. It is not just an "offline chat"; it is ephemeral social infrastructure built from physical proximity.

When multiple devices enter the same LAN, KAYA creates a temporary digital space where operators can discover nearby peers, join rooms, exchange public messages, send DMs, inspect presence, verify fingerprints, establish encrypted DM sessions, exchange files, relay encrypted DMs through an experimental local mesh, and keep working without internet, cloud, or a central server.

```text
+-------------------- KAYA --------------------+
| SPACE: semana-info      ROOM: #semana-info   |
| NODE: KY-71AF92        CALLSIGN: Helio       |
| STATUS: CONNECTED                            |
+------------------- TRAFFIC ------------------+
| [#geral] Ana: alguem recebe?                 |
| [#geral] Helio: recebido                     |
| [SECURE] Bruno -> Helio: teste privado       |
+------------------- NETWORK ------------------+
| peers: 12    latency avg: 11ms               |
| packets tx/rx: 221 / 204                     |
+-------------------- INPUT -------------------+
| >                                            |
+----------------------------------------------+
```

## Principles

- Offline-first: LAN communication works without internet.
- Decentralized: there is no central server.
- Local presence: the human cluster creates the digital space.
- Ephemeral infrastructure: when peers leave, the network fades.
- Terminal-first: the primary interface is a serious TUI.
- Clean architecture: UI, protocol, transport, state, persistence, and commands are separate crates.

## Workspace

```text
kaya-cli/
├── crates/
│   ├── app/          # runtime, bootstrap, coordination
│   ├── commands/     # slash-command parser
│   ├── events/       # internal event bus and counters
│   ├── files/        # metadata, chunking, hashing, transfer sessions
│   ├── mesh/         # route table, relay envelopes, TTL, scoring
│   ├── peer/         # presence, peer cache, timeouts
│   ├── persistence/  # sled-backed local config/history/cache
│   ├── protocol/     # packet schema, validation, JSON encode/decode
│   ├── rooms/        # room membership and message routing
│   ├── security/     # identity, signatures, trust, encrypted DMs
│   ├── shared/       # constants, errors, node ids, utilities
│   ├── transport/    # UDP multicast discovery and datagrams
│   └── ui/           # ratatui/crossterm terminal interface
├── docs/
├── labs/
├── scripts/
└── tests/
```

## Install

Requirements:

- Rust stable toolchain
- Local network that allows IPv4 UDP multicast

Build:

```bash
cargo build
```

Run:

```bash
cargo run -p kaya-app --bin kaya
```

KAYA stores local config, identity, trust state, peer cache, and basic history in `~/.kaya` by default. Override with:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
```

Configuration is stored as TOML:

```toml
nickname = "Helio"
multicast_address = "239.71.0.1"
multicast_port = 42424
heartbeat_interval_secs = 3
peer_timeout_secs = 12
theme = "kaya-dark"
packet_max_bytes = 65536
default_room = "geral"
last_room = "semana-info"
log_level = "kaya=info"

[file_transfer]
enabled = true
max_file_size_mb = 50
chunk_size_kb = 64
accept_from_unknown = true
download_dir = "~/.kaya/files/completed"

[mesh]
enabled = true
max_ttl = 5
allow_relay_for_unknown = true
allow_relay_for_blocked = false
relay_encrypted_only = false
route_expiry_seconds = 120
max_seen_packets = 5000
```

Identity is stored separately in `~/.kaya/identity.toml`. It contains the persistent node id plus Ed25519 and X25519 key material. Do not share this file.

## Usage

Open two or three terminals on the same LAN.

Terminal 1:

```text
KAYA callsign: Helio
> /join semana-info
> recebido por alguem?
```

Terminal 2:

```text
KAYA callsign: Ana
> /who
> recebido
```

Terminal 3:

```text
KAYA callsign: Bruno
> /msg Helio teste privado
> /secure-msg Helio teste privado cifrado
> /send Helio ./docs/PROTOCOL.md
> /routes
```

## Commands

- `/help`
- `/who`
- `/peers --fingerprints`
- `/rooms`
- `/create <room>`
- `/join <room>`
- `/leave <room>`
- `/current`
- `/room <message>`
- `/msg <callsign|node-id> <message>`
- `/secure-msg <callsign|node-id> <message>`
- `/send <callsign|node-id> <path>`
- `/accept-file <file_id>`
- `/reject-file <file_id>`
- `/files`
- `/cancel-file <file_id>`
- `/open-folder`
- `/file-info <file_id>`
- `/presence <online|away|busy|invisible>`
- `/identity`
- `/fingerprint`
- `/trust <peer>`
- `/untrust <peer>`
- `/block <peer>`
- `/trust-list`
- `/sessions`
- `/close-session <peer>`
- `/routes`
- `/route <node-id|callsign>`
- `/mesh-status`
- `/mesh-clear`
- `/history [room]`
- `/dm-history <peer>`
- `/status`
- `/logs`
- `/clear`
- `/exit`

## Quality Gates

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Event System](docs/EVENT_SYSTEM.md)
- [Transport](docs/TRANSPORT.md)
- [State Management](docs/STATE_MANAGEMENT.md)
- [UI System](docs/UI_SYSTEM.md)
- [Protocol](docs/PROTOCOL.md)
- [File Transfer](docs/FILE_TRANSFER.md)
- [Mesh](docs/MESH.md)
- [Commands](docs/COMMANDS.md)
- [Testing](docs/TESTING.md)
- [Roadmap](docs/ROADMAP.md)
- [Security](docs/SECURITY.md)

## Labs

- [LAB-01 peer discovery](labs/LAB-01-peer-discovery.md)
- [LAB-02 room sync](labs/LAB-02-room-sync.md)
- [LAB-03 private messaging](labs/LAB-03-private-messaging.md)
- [LAB-04 node failure](labs/LAB-04-node-failure.md)
- [LAB-06 room membership](labs/LAB-06-room-membership.md)
- [LAB-07 direct messages](labs/LAB-07-direct-messages.md)
- [LAB-08 presence](labs/LAB-08-presence.md)
- [LAB-09 history](labs/LAB-09-history.md)
- [LAB-10 identity and fingerprints](labs/LAB-10-identity-and-fingerprints.md)
- [LAB-11 trust store](labs/LAB-11-trust-store.md)
- [LAB-12 encrypted DMs](labs/LAB-12-encrypted-dms.md)
- [LAB-13 blocked peers](labs/LAB-13-blocked-peers.md)
- [LAB-14 file offer](labs/LAB-14-file-offer.md)
- [LAB-15 file transfer](labs/LAB-15-file-transfer.md)
- [LAB-16 encrypted file transfer](labs/LAB-16-encrypted-file-transfer.md)
- [LAB-17 corrupted chunk](labs/LAB-17-corrupted-chunk.md)
- [LAB-18 blocked peer file transfer](labs/LAB-18-blocked-peer-file-transfer.md)
- [LAB-19 route table](labs/LAB-19-route-table.md)
- [LAB-20 mesh relay](labs/LAB-20-mesh-relay.md)
- [LAB-21 multihop DM](labs/LAB-21-multihop-dm.md)
- [LAB-22 mesh TTL and dedup](labs/LAB-22-mesh-ttl-and-dedup.md)
- [LAB-23 blocked relay policy](labs/LAB-23-blocked-relay-policy.md)
- [Packet loss](labs/packet-loss.md)
- [Peer timeout](labs/peer-timeout.md)
- [Malformed packets](labs/malformed-packets.md)
- [Simultaneous joins](labs/simultaneous-joins.md)
- [Multi-room sync](labs/multi-room-sync.md)
