# KAYA CLI Jury FAQ

## Does this work without internet?

Yes. KAYA is designed for local-network operation and does not require internet connectivity for peer discovery, room messaging, direct messages, encrypted DMs, or local file transfer.

## Does it need a server?

No. KAYA is serverless in its current design. Peers discover each other over the local network and exchange packets directly.

## How do peers find each other?

Peers use IPv4 UDP multicast on the local network. A node announces itself and listens for nearby KAYA traffic. That makes peer discovery automatic inside a multicast-capable LAN.

## Are direct messages really encrypted?

Encrypted direct messages are. Public room messages are not. KAYA uses X25519 for session setup, HKDF-SHA256 for key derivation, and ChaCha20-Poly1305 for authenticated encryption of secure direct-message payloads.

## Can a relay node read encrypted messages?

Not the secure DM plaintext. Relay nodes can see that a mesh packet exists and can inspect mesh envelope metadata such as route behavior, but they do not get the decrypted contents of encrypted DM payloads.

## What is the limitation of UDP multicast?

UDP multicast is usually limited to a LAN or multicast-enabled network segment. Some networks block or heavily constrain multicast. Delivery is best-effort, unordered, and not suitable as a production WAN transport by itself.

## What happens if a peer crashes or disappears?

The runtime uses heartbeats, peer timeout logic, and leave/offline handling to remove stale peers from the active projection. Route state and stale transfer state are also pruned over time.

## Can this be used outside the LAN?

Not as a general production solution today. There is no NAT traversal, no internet relay infrastructure, and no WAN transport layer yet. The roadmap explicitly separates the current LAN-first design from future work such as binary protocol evolution and QUIC.

## What is still missing before production?

Several things:

- a transport story beyond UDP multicast on a LAN
- stronger security review and audit work
- hardened replay protection and broader key-management workflows
- mature mesh behavior under larger or noisier network conditions
- production-grade packaging, observability, and operational testing
- non-terminal clients if broader user adoption is the goal

## Why terminal-first instead of a GUI?

Because this stage of the project prioritizes operational visibility, speed of iteration, and architectural clarity. A terminal UI is lighter, easier to demo in technical environments, and easier to evaluate for systems behavior.

## Is the mesh production-ready?

No. The mesh layer is explicitly experimental. It is useful as a demonstration of multi-hop local delivery and routing diagnostics, but it should not be overstated as production-grade routing.

## Why keep public rooms plaintext?

Because KAYA currently separates broad shared-room communication from sensitive peer-to-peer traffic. Public rooms remain compatible and observable, while sensitive traffic is handled via encrypted direct messages.
