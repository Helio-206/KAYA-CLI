# KAYA CLI v0.1.1 Release Notes

## Summary

`v0.1.1` is the current patch release of KAYA CLI: an offline-first, terminal-based communication system for local networks.

It provides:

- local peer discovery over IPv4 UDP multicast
- public room messaging
- direct messages and encrypted direct messages
- trust fingerprints and blocked-peer handling
- explicit file offers and local file transfer
- experimental mesh relay for selected traffic
- demo mode, packaging scripts, presentation-ready documentation, and an embeddable Rust SDK layer
- live audio spaces with Linux runtime capture/playback and Windows-native voice backend support

## Main Features

- Rust workspace with separated crates for transport, protocol, rooms, security, files, mesh, persistence, commands, and UI
- ratatui-based terminal interface with diagnostics, logs, splash screen, hints, and modal overlays
- profile-driven startup with `default`, `demo`, `lab`, and `paranoid`
- release candidate demo commands for controlled presentations
- release packaging script that produces a versioned archive in `dist/`

## Limitations

- constrained to multicast-capable LAN environments
- no NAT traversal or WAN transport
- public rooms are plaintext
- mesh routing is experimental
- file chunks over mesh are not implemented
- no mobile client
- no external cryptographic audit yet

See [docs/LIMITATIONS.md](docs/LIMITATIONS.md) for the full list.

## Command Highlights

- `/help`
- `/about`
- `/version`
- `/join <room>`
- `/room <message>`
- `/msg <peer> <message>`
- `/secure-msg <peer> <message>`
- `/send <peer> <path>`
- `/accept-file <file_id>`
- `/trust <peer>`
- `/routes`
- `/mesh-status`
- `/status`
- `/demo-peers <n>`
- `/demo-message <room> <count>`
- `/demo-mesh-route`

See [docs/COMMANDS.md](docs/COMMANDS.md) for the full command reference.

## Installation

Requirements:

- Rust stable
- a local network that allows IPv4 UDP multicast

Build locally:

```bash
cargo build --release
```

Run locally:

```bash
cargo run -p kaya-app --bin kaya
```

Package for distribution:

```bash
./scripts/package-release.sh
./scripts/generate-checksums.sh
```

Install from a published release:

```bash
curl -fsSL https://github.com/natanielmatondo/KAYA-CLI/releases/download/v0.1.1/install.sh | sh
```

Install from a downloaded archive:

```bash
tar -xzf kaya-cli-0.1.1-x86_64-unknown-linux-gnu.tar.gz
sudo mv kaya-cli-0.1.1-x86_64-unknown-linux-gnu/bin/kaya /usr/local/bin/
kaya --version
```

Developer SDK:

```bash
cargo add kaya-sdk
```

See [docs/SDK.md](docs/SDK.md) and [docs/INSTALLATION.md](docs/INSTALLATION.md) for the public API and install flow.

## Validation Passed

The repository passed the following checks for this release candidate:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo build --release
./scripts/package-release.sh
```
