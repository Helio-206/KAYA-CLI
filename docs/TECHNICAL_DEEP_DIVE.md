# KAYA CLI Technical Deep Dive

## Overview

KAYA CLI is a Rust workspace organized around clear subsystem boundaries. The application bootstrap and runtime coordination live in the `app` crate, while transport, protocol, rooms, security, file transfer, persistence, commands, and UI are isolated into dedicated crates.

That separation matters because KAYA is simultaneously a product demo and a systems project. The repository is meant to be readable, extensible, and easy to explain under technical scrutiny.

## Workspace Architecture

### `app`

Coordinates startup, configuration, event handling, background tasks, command execution, and UI projection. This is where the crates meet.

### `commands`

Parses slash commands into typed operations. It keeps command behavior explicit and testable.

### `events`

Defines the internal event bus and event flow used to coordinate runtime activity without collapsing everything into direct call chains.

### `protocol`

Defines packet schemas, packet encoding and decoding, validation, and message types.

### `transport`

Implements the UDP multicast socket layer that sends and receives local network packets.

### `peer`

Tracks nearby peers, presence, timeouts, and peer metadata.

### `rooms`

Maintains room membership, current-room state, and room message routing.

### `security`

Owns local identity, signatures, trust state, encrypted DM session setup, and secure payload handling.

### `files`

Tracks file offers, chunking, hashing, transfer progress, and completed-file handling.

### `mesh`

Maintains route announcements, route scoring, route requests and responses, relay rules, TTL, trace information, and duplicate suppression.

### `persistence`

Stores configuration, identity-related local metadata, peer cache, and basic history.

### `ui`

Renders the ratatui-based TUI, panels, overlays, logs, status areas, and command feedback.

### `shared`

Provides shared helpers, node identifiers, timestamps, and common error or normalization utilities.

## Event Bus

KAYA uses an internal event bus to decouple network intake, domain state changes, and UI projection.

The core flow is:

```text
transport -> protocol decode -> runtime -> domain handlers -> event bus -> UI projection
```

This pattern keeps the runtime readable:

- transport concerns remain in the transport layer
- protocol decoding remains in the protocol layer
- room, peer, security, and file logic stay in their own bounded modules
- UI state becomes a projection of runtime events, not the source of truth

This also makes demo mode and diagnostics easier, because synthetic events can exercise the same projection path used by real traffic.

## UDP Multicast Transport

KAYA uses IPv4 UDP multicast for peer discovery and local packet exchange. This is a deliberate trade-off:

- it works well for local-network discovery without central coordination
- multiple peers can observe the same traffic simultaneously
- it is easy to run in labs and demos
- it keeps the infrastructure footprint minimal

The downside is equally clear:

- multicast is typically limited to a LAN or multicast-enabled segment
- some networks disable or restrict multicast
- delivery is best-effort and unordered

KAYA handles that reality with duplicate suppression, heartbeats, peer timeout logic, and explicit diagnostics.

## Rooms

Rooms are the shared social surface. A peer can create, join, leave, and announce rooms. Membership is synchronized with lightweight room events and snapshot reconciliation.

The room model is intentionally simple:

- public messages are scoped by room
- the runtime tracks the current room and known rooms locally
- membership is eventually synchronized rather than globally centralized
- if a peer disappears, timeout logic and leave/offline handling clean up the projection

This keeps room behavior easy to reason about during local-only operation.

## Encrypted Direct Messages

Public rooms are plaintext by design today. Sensitive communication is handled through encrypted direct messages.

The secure DM path uses:

- Ed25519 for signed packet identity
- X25519 for session key agreement
- HKDF-SHA256 for key derivation
- ChaCha20-Poly1305 for authenticated encryption

The high-level flow is:

1. peers exchange or reuse trustable identity material
2. a secure session request is sent
3. the receiving peer accepts and establishes session state
4. the initial queued message is released once the session is ready
5. encrypted DM payloads are decrypted locally and rendered in the TUI as secure traffic

Relay nodes can forward encrypted payloads, but they do not get message plaintext.

## File Transfer

KAYA supports explicit file offers with local acceptance and rejection. Files are not silently pushed.

The file transfer model includes:

- safe filename validation
- chunked transfer state
- SHA-256 verification for chunks and final file integrity
- local persistence of transfer metadata and progress
- optional encrypted chunks over an active secure DM session

This keeps transfers inspectable and operationally safe. It also avoids overstating what the system can do today: mesh relay for file chunks is not part of the current release candidate.

## Mesh Routing

Mesh routing is an experimental relay layer over the local transport. It is designed to extend local reach without changing the basic local-first model.

The mesh subsystem handles:

- route announcements
- route requests and responses
- scored route selection
- TTL limits
- route trace visibility
- duplicate relay suppression
- no-loop behavior
- diagnostics exposed in the UI

KAYA uses relays for direct-message style traffic and file control packets. It does not currently relay file chunks. That boundary is intentional to avoid overstating maturity and to reduce uncontrolled bulk relay behavior.

## Security Model

KAYA's security model is pragmatic and layered.

What it does today:

- assigns each node a persistent local cryptographic identity
- signs packets with Ed25519
- exposes peer fingerprints in the UI and command surface
- stores trust decisions locally
- allows unknown, trusted, and blocked peer states
- encrypts direct messages and secure file chunks end-to-end
- keeps relay nodes blind to encrypted DM plaintext

What it does not claim today:

- audited production cryptography
- secure group messaging for public rooms
- global identity federation
- WAN-grade transport security

This is a strong research and engineering baseline, but not a finished security product.

## Demo Mode

Demo mode is intentionally built into the product surface rather than bolted on externally.

It provides:

- isolated data directories and profiles
- demo-friendly callsign behavior
- deterministic commands for seeding peers, messages, mesh traces, file offers, and warnings
- a controlled presentation flow without polluting a normal operator profile

This matters because it turns KAYA into a better public artifact. A reviewer can both inspect the code and run a convincing guided demonstration quickly.

## Why The Architecture Matters

KAYA is strongest when evaluated as both a working prototype and a disciplined architecture.

The codebase shows:

- a coherent local-first product thesis
- clear subsystem decomposition
- honest security boundaries
- operational diagnostics instead of black-box behavior
- a path to future transport and protocol upgrades without rewriting everything from scratch
