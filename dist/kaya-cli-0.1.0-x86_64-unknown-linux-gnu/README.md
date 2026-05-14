# KAYA CLI

![Rust](https://img.shields.io/badge/Rust-stable-orange)
![Tests](https://img.shields.io/badge/tests-passing-brightgreen)
![Release](https://img.shields.io/badge/release-v0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Offline First](https://img.shields.io/badge/offline--first-LAN--native%20%2B%20WAN--relay-222222)
![Terminal UI](https://img.shields.io/badge/terminal--ui-ratatui-5c7cfa)

KAYA CLI is an offline-first, decentralized, terminal-based communication system for local networks, with direct TCP peer links for VPN/Tailscale use and an optional WAN relay mode for peers that are not on the same LAN. It is not just an "offline chat"; it is ephemeral social infrastructure built from physical proximity.

When multiple devices enter the same LAN, KAYA creates a temporary digital space where operators can discover nearby peers, join rooms, exchange public messages, send DMs, inspect presence, verify fingerprints, establish encrypted DM sessions, exchange files, relay encrypted DMs through an experimental local mesh, and keep working without internet, cloud, or a central server. When multicast is blocked, peers can connect directly over TCP, including through Tailscale addresses, without replacing the LAN path.

## Start Here

- [Pitch](docs/PITCH.md)
- [Technical Deep Dive](docs/TECHNICAL_DEEP_DIVE.md)
- [Jury FAQ](docs/JURY_FAQ.md)
- [Direct Connect](docs/DIRECT_CONNECT.md)
- [Tailscale Guide](docs/TAILSCALE.md)
- [WAN Relay Guide](docs/WAN_RELAY.md)
- [Relay Security](docs/RELAY_SECURITY.md)
- [Release Notes](RELEASE_NOTES.md)
- [Public Roadmap](docs/ROADMAP_PUBLIC.md)

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
│   ├── direct/       # manual TCP listener, connector and frame transport
│   ├── events/       # internal event bus and counters
│   ├── files/        # metadata, chunking, hashing, transfer sessions
│   ├── mesh/         # route table, relay envelopes, TTL, scoring
│   ├── peer/         # presence, peer cache, timeouts
│   ├── persistence/  # sled-backed local config/history/cache
│   ├── protocol/     # packet schema, validation, JSON encode/decode
│   ├── relay/        # optional TCP relay server and client
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

### Installation without cloning

Linux x86_64 release install:

```bash
curl -fsSL https://github.com/natanielmatondo/KAYA-CLI/releases/download/v0.1.0/install.sh | sh
```

Manual archive install:

```bash
tar -xzf kaya-cli-0.1.0-x86_64-unknown-linux-gnu.tar.gz
sudo mv kaya-cli-0.1.0-x86_64-unknown-linux-gnu/bin/kaya /usr/local/bin/
kaya --version
```

Local repo install without cloning a second time:

```bash
./scripts/install-local.sh
```

See [Installation](docs/INSTALLATION.md) and [Distribution](docs/DISTRIBUTION.md) for the release layout, checksums, and uninstall flow.

### Build from source

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

Demo mode:

```bash
cargo run -p kaya-app --bin kaya -- --demo --profile demo
```

KAYA stores local config, identity, trust state, peer cache, and basic history in `~/.kaya` by default. Override with:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
```

Or pass an explicit startup directory:

```bash
cargo run -p kaya-app --bin kaya -- --profile lab --data-dir /tmp/kaya-lab-01
```

## Startup Flags

- `--demo` starts an isolated presentation profile with a dedicated data directory and generated demo callsign.
- `--profile <default|demo|lab|paranoid>` applies a runtime profile before startup.
- `--data-dir <path>` overrides the profile directory explicitly.
- `--relay <tcp://host:port>` connects the CLI to an optional WAN relay.
- `--local` keeps room mirroring in local-first mode when relay is enabled.
- `--version` prints the CLI version and exits.
- `--about` prints a short product summary and exits.

Relay server mode:

```bash
cargo run -p kaya-app --bin kaya -- relay --bind 0.0.0.0:7777
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

[relay]
enabled = false
url = "tcp://relay.example:7777"
bind = "0.0.0.0:7777"
prefer_local = true
heartbeat_interval_ms = 5000
connection_timeout_ms = 15000

[relay.rooms]
enabled = true
broadcast = true
bridge_local = true

[relay.file_transfer]
enabled = false
allow_chunks = false
max_file_size_mb = 20
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

WAN relay example:

Terminal A, host the relay:

```bash
cargo run -p kaya-app --bin kaya -- relay --bind 0.0.0.0:7777
```

Terminal B, home 1:

```bash
cargo run -p kaya-app --bin kaya -- --relay tcp://PUBLIC-IP:7777
```

Terminal C, home 2:

```bash
cargo run -p kaya-app --bin kaya -- --relay tcp://PUBLIC-IP:7777
```

Then inside the CLI:

```text
> /relay-status
> /relay-peers
> /msg KY-71AF92 teste por relay
> /secure-msg Ana dm cifrada por relay
```

Tailscale direct connect example:

Host:

```text
> /listen 7777
> /listen-status
```

Friend:

```text
> /connect 100.81.167.95:7777
> /connections
> /secure-msg Helio teste seguro
```

## Commands

- `/help`
- `/about`
- `/version`
- `/demo-reset`
- `/demo-peers <n>`
- `/demo-message <room> <count>`
- `/demo-mesh-route`
- `/demo-file-offer`
- `/demo-security-warning`
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
- `/listen <port>`
- `/connect <ip>:<port>`
- `/disconnect <peer>`
- `/connections`
- `/stop-listener`
- `/listen-status`
- `/mesh-status`
- `/mesh-clear`
- `/history [room]`
- `/dm-history <peer>`
- `/status`
- `/logs`
- `/clear`
- `/exit`

## Demo Flow

For a controlled presentation:

```bash
./scripts/run-demo.sh presenter
```

Inside the TUI:

```text
> /demo-peers 4
> /demo-message semana-info 4
> /demo-mesh-route
> /demo-file-offer
> /demo-security-warning
```

For a lab profile with explicit storage:

```bash
./scripts/run-local-lab.sh operator-a
```

## Release Packaging

Build a release archive with:

```bash
./scripts/package-release.sh
./scripts/generate-checksums.sh
```

## Developer SDK usage

KAYA now exposes a stable Rust SDK surface through `kaya-sdk`.

```rust
use kaya_sdk::{KayaClient, KayaConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = KayaClient::new(KayaConfig::default()).await?;

	client.set_callsign("Helio").await?;
	client.join_room("geral").await?;
	client.send_room_message("geral", "hello offline").await?;
	client.stop().await?;
	Ok(())
}
```

The SDK hides transport, protocol, UI, and mesh internals behind `KayaClient` while still exposing event subscription, peer listing, room control, secure DM, file sending, trust management, and route inspection.

Start here:

- [SDK](docs/SDK.md)
- [Embedding](docs/EMBEDDING.md)
- [Versioning](docs/VERSIONING.md)
- [Daemon Design](docs/DAEMON.md)

Run the examples with:

```bash
cargo run -p kaya-sdk --example simple-node
cargo run -p kaya-sdk --example room-bot -- geral
cargo run -p kaya-sdk --example secure-dm -- KY-REPLACE "hello secure offline"
```

## Presentation Kit

- [Presentation Flow](presentation/README.md)
- [Demo Script](presentation/DEMO_SCRIPT.md)
- [Talk Track](presentation/TALK_TRACK.md)
- [TUI Placeholder](docs/assets/screenshots/tui-main.md)
- [Secure DM Placeholder](docs/assets/screenshots/secure-dm.md)
- [Mesh Route Placeholder](docs/assets/screenshots/mesh-route.md)
- [File Transfer Placeholder](docs/assets/screenshots/file-transfer.md)

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
- [Direct Connect](docs/DIRECT_CONNECT.md)
- [Tailscale](docs/TAILSCALE.md)
- [State Management](docs/STATE_MANAGEMENT.md)
- [UI System](docs/UI_SYSTEM.md)
- [Protocol](docs/PROTOCOL.md)
- [File Transfer](docs/FILE_TRANSFER.md)
- [Mesh](docs/MESH.md)
- [Commands](docs/COMMANDS.md)
- [Pitch](docs/PITCH.md)
- [Technical Deep Dive](docs/TECHNICAL_DEEP_DIVE.md)
- [Jury FAQ](docs/JURY_FAQ.md)
- [Demo Mode](docs/DEMO_MODE.md)
- [Distribution](docs/DISTRIBUTION.md)
- [Installation](docs/INSTALLATION.md)
- [Limitations](docs/LIMITATIONS.md)
- [Release](docs/RELEASE.md)
- [SDK](docs/SDK.md)
- [Embedding](docs/EMBEDDING.md)
- [Versioning](docs/VERSIONING.md)
- [Daemon Design](docs/DAEMON.md)
- [Public Roadmap](docs/ROADMAP_PUBLIC.md)
- [Testing](docs/TESTING.md)
- [Roadmap](docs/ROADMAP.md)
- [Security](docs/SECURITY.md)

## Community

- [Contributing](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)
- [Release Notes](RELEASE_NOTES.md)

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
- [LAB-29 direct connect](labs/LAB-29-direct-connect.md)
- [LAB-30 tailscale connect](labs/LAB-30-tailscale-connect.md)
- [LAB-31 direct secure DM](labs/LAB-31-direct-secure-dm.md)
- [LAB-32 direct file transfer](labs/LAB-32-direct-file-transfer.md)
- [Packet loss](labs/packet-loss.md)
- [Peer timeout](labs/peer-timeout.md)
- [Malformed packets](labs/malformed-packets.md)
- [Simultaneous joins](labs/simultaneous-joins.md)
- [Multi-room sync](labs/multi-room-sync.md)
