# KAYA CLI Public Roadmap

This roadmap is written for public readers. It separates what exists now from what is intentionally future work.

## v0.1.0 Release Candidate

Focus:

- offline-first LAN communication
- room messaging
- direct messages
- encrypted direct messages
- file offers and local file transfer
- experimental mesh relay for selected traffic
- demo mode, packaging, and presentation-ready documentation

Status:

- available now as the current release candidate
- suitable for technical evaluation, demos, and local labs
- not positioned as production-ready

## v0.2.0 Binary Protocol

Target:

- replace or complement the current JSON-heavy packet layer with a more compact binary protocol
- improve bandwidth efficiency and parsing overhead
- make protocol evolution more disciplined for larger test scenarios

Expected outcomes:

- tighter packet sizes
- cleaner versioning strategy
- better path toward larger or noisier local-network scenarios

## v0.3.0 QUIC Transport

Target:

- explore a transport path beyond multicast-only local networking
- introduce a more reliable and modern transport option for selected scenarios

Expected outcomes:

- stronger delivery guarantees than raw UDP multicast alone
- a path toward broader network topologies
- groundwork for future WAN-capable operation

## v0.4.0 Mobile and Android

Target:

- extend KAYA beyond terminal-only operation
- introduce mobile participation for local-first communication scenarios

Expected outcomes:

- Android-first mobile client exploration
- more realistic field usage beyond laptops and developer machines
- validation of KAYA as a broader product, not only a CLI system

## v0.5.0 Production Security Review

Target:

- subject the security architecture and implementation to external review
- harden identity, key management, replay handling, and threat boundaries

Expected outcomes:

- formal review of cryptographic assumptions and implementation choices
- sharper production/non-production boundaries
- a realistic basis for any future production claims

## Roadmap Discipline

This roadmap is intentionally conservative. KAYA should grow by hardening proven boundaries, not by promising unsupported capabilities too early.
