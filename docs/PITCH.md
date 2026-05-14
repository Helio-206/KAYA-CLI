# KAYA CLI Pitch

## The Problem

Most communication tools assume stable internet, cloud infrastructure, accounts, and centralized identity. That assumption breaks down in classrooms, temporary events, campus networks, field operations, workshops, incident response, and community spaces where people are physically together but the internet is weak, unavailable, unwanted, or inappropriate.

The gap is not only technical. It is social. When people share a place, they often need a temporary digital room that appears with them, works locally, and disappears when the group disperses.

## The Solution

KAYA CLI is an offline-first, terminal-based communication system for local networks. Devices on the same LAN discover each other automatically, join shared rooms, exchange local messages, establish encrypted direct messages, transfer files, and optionally relay traffic across an experimental local mesh.

There is no login, no cloud dependency, and no central server. The network exists because the people are present on the same local infrastructure.

## What Makes KAYA Different

- It treats local presence as a first-class primitive instead of a degraded fallback.
- It is designed for temporary digital communities, not permanent centralized platforms.
- It keeps the interface operational and inspectable through a serious terminal UI.
- It combines rooms, direct messages, trust fingerprints, encrypted DMs, file transfer, and mesh relay in one coherent local-first tool.
- It is architected as a clean Rust workspace, so transport, protocol, security, persistence, commands, and UI evolve independently.

## Why Offline-First Matters

Offline-first here does not mean "cache the cloud later." It means the product remains useful when the internet is absent.

That matters in at least four cases:

- resilience: communication continues during outages or degraded connectivity
- autonomy: a group can coordinate locally without depending on external providers
- privacy: local traffic can remain inside the local environment
- accessibility: workshops, classrooms, and ad hoc teams can use the system without account setup or infrastructure provisioning

## Why Terminal-First

Terminal-first is a deliberate product choice.

- It makes the interface lightweight and fast on modest hardware.
- It fits technical operators, labs, and evaluation settings where inspectability matters.
- It avoids the overhead of building a GUI before the interaction model is proven.
- It makes architecture and state transitions easier to explain in a technical presentation.

KAYA is not claiming the terminal is the final interface for every audience. It is claiming the terminal is the right interface for the current stage: operational clarity, fast iteration, and technical credibility.

## Why Mesh

Direct local communication is ideal, but local networks are messy. Peers may not always have clean direct reachability, and transient topologies happen in practice. The mesh layer gives KAYA a way to experiment with resilient multi-hop local delivery without jumping immediately to internet-scale networking.

Mesh matters because it allows KAYA to explore:

- relayed delivery when direct reach is unavailable
- route visibility and diagnostics inside the UI
- end-to-end encrypted payloads crossing intermediate nodes without exposing message plaintext
- a path toward more capable decentralized networking later

## Honest Limitations

KAYA is promising a strong local-first demo and a credible technical foundation, not a finished production communications platform.

Current limits are explicit:

- it is constrained to LAN environments that allow IPv4 UDP multicast
- mesh routing is experimental
- public rooms are plaintext by design today
- file chunks are not relayed over mesh yet
- there is no NAT traversal, WAN transport, or mobile client
- the cryptography has not undergone an external security audit

## Why This Is Worth Building

KAYA argues that proximity should be enough to create useful digital coordination. If people are in the same place, they should be able to communicate, organize, and exchange information even when the internet is missing or undesirable.

That makes KAYA both a practical systems project and a product thesis: local networks can be social infrastructure, not just plumbing.
