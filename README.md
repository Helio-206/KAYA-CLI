# KAYA CLI

KAYA CLI is an offline-first, decentralized, terminal-based communication system for local networks. It is not just an "offline chat"; it is ephemeral social infrastructure built from physical proximity.

When multiple devices enter the same LAN, KAYA creates a temporary digital space where operators can discover nearby peers, join rooms, exchange public messages, send DMs, inspect presence, and keep working without internet, cloud, or a central server.

```text
+-------------------- KAYA --------------------+
| SPACE: semana-info      ROOM: #semana-info   |
| NODE: KY-71AF92        CALLSIGN: Helio       |
| STATUS: CONNECTED                            |
+------------------- TRAFFIC ------------------+
| [#geral] Ana: alguem recebe?                 |
| [#geral] Helio: recebido                     |
| [DM] Bruno -> Helio: teste privado           |
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
│   ├── peer/         # presence, peer cache, timeouts
│   ├── persistence/  # sled-backed local config/history/cache
│   ├── protocol/     # packet schema, validation, JSON encode/decode
│   ├── rooms/        # room membership and message routing
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

KAYA stores local config, peer cache, and basic history in `~/.kaya` by default. Override with:

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
```

## Phase 1 Usage

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
```

## Commands

- `/help`
- `/who`
- `/rooms`
- `/join <room>`
- `/room [room]`
- `/msg <callsign|node-id> <message>`
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
- [Commands](docs/COMMANDS.md)
- [Testing](docs/TESTING.md)
- [Roadmap](docs/ROADMAP.md)
- [Security](docs/SECURITY.md)

## Labs

- [LAB-01 peer discovery](labs/LAB-01-peer-discovery.md)
- [LAB-02 room sync](labs/LAB-02-room-sync.md)
- [LAB-03 private messaging](labs/LAB-03-private-messaging.md)
- [LAB-04 node failure](labs/LAB-04-node-failure.md)
- [Packet loss](labs/packet-loss.md)
- [Peer timeout](labs/peer-timeout.md)
- [Malformed packets](labs/malformed-packets.md)
- [Simultaneous joins](labs/simultaneous-joins.md)
- [Multi-room sync](labs/multi-room-sync.md)
