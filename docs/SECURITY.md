# KAYA Security Notes

KAYA Phase 3 introduces local cryptographic identity, signed packets, a trust store, and encrypted direct messages. Public rooms intentionally remain plaintext multicast traffic for compatibility and shared-room observability.

## Current Threats

### Packet Spoofing

Any device on the multicast-capable LAN can emit JSON packets. Phase 3 signs packets emitted by current KAYA nodes and rejects invalid signatures, but unsigned legacy packets may still be accepted for compatibility.

### Callsign Impersonation

Callsigns are user-chosen labels. Duplicate callsigns are detected during DM target resolution. Fingerprints, not callsigns, are the security identity.

### Passive Eavesdropping

Room messages and legacy `DIRECT_MESSAGE` packets are still JSON over UDP multicast. `DIRECT_MESSAGE_ENCRYPTED` payloads use X25519 session setup, HKDF-SHA256 key derivation, and ChaCha20-Poly1305 encryption.

### Replay

KAYA includes packet ids, timestamps, and runtime duplicate suppression for recently seen packet ids. Encrypted DMs use nonces, but Phase 3 does not yet implement a persistent cryptographic replay window.

### Relay Attacks

A malicious node could rebroadcast packets outside the intended local context.

## Mitigations Planned

- Persistent local identity hardening and optional passphrase protection.
- Duplicate suppression.
- Replay windows.
- Optional room keys.
- Trust-on-first-use warnings for changed fingerprints.
- Safer identity export/import workflows.

## Implemented in Phase 3

- `~/.kaya/identity.toml` stores the local node id, callsign, Ed25519 keypair, X25519 keypair, and public fingerprint.
- Packet signatures use Ed25519. Private keys are never logged or transmitted.
- The trust store in `~/.kaya/trust.toml` records known peers, first/last seen timestamps, fingerprints, and `unknown`/`trusted`/`blocked` state.
- Blocked peers are filtered from UI peer projections and their packets are rejected before chat routing.
- Secure DMs use `DM_SESSION_REQUEST`, `DM_SESSION_ACCEPT`, and `DIRECT_MESSAGE_ENCRYPTED`.
- Secure file chunks can use the existing secure session with HKDF context `kaya-file-transfer-v1`.
- File offers are manual accept/reject. Blocked peers cannot route file offers or chunks into the transfer manager.
- The UI exposes identity fingerprint, trusted/blocked counts, active secure sessions, and security warning count.

## Operational Guidance

Use public rooms for non-sensitive coordination. Use `/secure-msg` for sensitive local direct messages, and verify peer fingerprints out of band before trusting a peer.
