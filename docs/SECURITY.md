# KAYA Security Notes

KAYA Phase 1 is an experimental local-network communication tool. It is not yet a secure messenger.

## Current Threats

### Packet Spoofing

Any device on the multicast-capable LAN can emit packets that look like KAYA packets. Node ids are temporary identifiers, not authenticated identities.

### Callsign Impersonation

Callsigns are user-chosen labels. Phase 1 does not prevent two peers from using the same callsign.

### Passive Eavesdropping

All packets are JSON over UDP multicast. Anyone on the local network segment can read room messages and DMs.

### Replay

Phase 1 includes packet ids and timestamps, but does not yet enforce replay windows or duplicate suppression.

### Relay Attacks

A malicious node could rebroadcast packets outside the intended local context.

## Mitigations Planned

- Long-lived local identity keys.
- Packet signatures.
- Direct-message encryption.
- Peer fingerprints.
- Duplicate suppression.
- Replay windows.
- Optional room keys.
- Trust warnings in the UI.

## Operational Guidance

Use Phase 1 for local experiments, labs, prototypes, and non-sensitive coordination. Do not send secrets, credentials, personal data, or operationally sensitive content until cryptographic identity and encryption are implemented.
