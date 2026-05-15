# KAYA CLI Presentation Flow

This folder is the presentation kit for a public demo, technical jury, or GitHub-facing walkthrough.

## Suggested Sequence

### 1. Opening

Start with the thesis in one sentence:

> KAYA CLI turns a local network into a temporary digital community without internet, cloud, or a central server.

Show the TUI quickly so the audience immediately understands the product shape.

### 2. Problem

Frame the problem before the implementation:

- communication tools usually assume reliable internet and centralized infrastructure
- many real situations are local-first: workshops, classrooms, campus events, field teams, temporary operations
- a group in the same place should still be able to coordinate digitally even when the internet is absent or undesirable

Use [docs/PITCH.md](../docs/PITCH.md) as the narrative source.

### 3. Demo

Run the live demo with three terminals or machines.

Use [presentation/DEMO_SCRIPT.md](DEMO_SCRIPT.md) for the exact command flow.

The recommended sequence is:

- start three nodes
- join the same room
- send a public room message
- show trust and fingerprint flow
- send a secure direct message
- demonstrate a file offer
- show a mesh route example
- finish with `/status`, `/routes`, or `/sessions`

### 4. Technical Explanation

After the demo, explain the architecture in layers:

- workspace crates
- event bus
- UDP multicast discovery
- rooms and peer state
- encrypted DMs
- file transfer
- mesh routing
- security model
- demo mode

Use [docs/TECHNICAL_DEEP_DIVE.md](../docs/TECHNICAL_DEEP_DIVE.md).

### 5. Risks and Limits

Be explicit and credible:

- LAN scope because of UDP multicast
- experimental mesh
- no NAT traversal or WAN transport
- no mobile clients yet
- no external cryptographic audit yet

Use [docs/LIMITATIONS.md](../docs/LIMITATIONS.md).

### 6. Roadmap

Position the next milestones clearly:

- `v0.1.1 RC`: current LAN-first release candidate
- `v0.2.0`: binary protocol
- `v0.3.0`: QUIC exploration
- `v0.4.0`: mobile and Android
- `v0.5.0`: production security review

Use [docs/ROADMAP_PUBLIC.md](../docs/ROADMAP_PUBLIC.md).

### 7. Closing

End with the product claim, not only the implementation claim:

> If people share a place, they should be able to create a useful digital coordination space without asking the internet for permission.

## Presenter Checklist

- verify local network conditions before the session
- keep the release archive ready as a fallback artifact
- keep screenshot placeholders ready in case live demo conditions degrade
- use demo mode for predictable on-screen state when needed
- never oversell the mesh or security posture beyond the documented boundaries
