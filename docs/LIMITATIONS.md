# KAYA CLI Limitations

KAYA is a strong local-first prototype and release candidate, but it is not a finished production communication platform. This document is intentionally blunt about current limits.

## Network Scope

- KAYA depends on IPv4 UDP multicast for discovery and local packet exchange.
- That generally limits operation to a LAN or multicast-enabled local network segment.
- There is no NAT traversal, internet rendezvous, or WAN transport layer.
- Some enterprise or guest networks may block multicast completely.

## Reliability

- UDP delivery is best-effort and unordered.
- KAYA compensates with heartbeats, duplicate suppression, timeout cleanup, and local diagnostics, but it does not turn UDP into guaranteed delivery.
- Mesh routing is experimental and should be treated as a controlled capability, not production infrastructure.

## Mesh Boundaries

- Multi-hop relay exists for selected traffic types.
- File chunks over mesh are not implemented in `v0.1.1`.
- Route quality is heuristic and locally observed, not globally verified.
- Relay behavior is not a substitute for a mature routed transport.

## Security Boundaries

- Public room messages are plaintext by design today.
- Encrypted DMs and secure file chunks exist, but the overall system has not undergone an external cryptographic audit.
- There is no production-grade identity recovery, federation, or enterprise key lifecycle model.
- Replay and abuse defenses are practical but not yet comprehensive enough to claim hardened production security.

## Platform Scope

- KAYA is currently terminal-first.
- There is no mobile client.
- There is no Android or iOS support.
- There is no browser client.

## Operational Maturity

- Packaging is adequate for a release candidate, not a broad software-distribution program.
- Large-scale performance characteristics are not yet validated.
- The project is optimized for clarity, local labs, demos, and technical review rather than production deployment.

## What This Means

KAYA should be evaluated honestly as:

- a compelling offline-first LAN communication prototype
- a well-structured systems project in Rust
- a credible foundation for further research and productization

It should not yet be described as:

- a production-secure messaging platform
- a WAN-ready communication system
- a mature replacement for audited, internet-scale messaging infrastructure
