# Relay Security Notes

The relay is a transport bridge, not a trusted message authority.

## Preserved Properties

- Signed packets still arrive signed end-to-end.
- Encrypted DMs remain encrypted across the relay.
- Trust-store checks still apply on the receiving peer.
- Blocked peers are still denied by local policy.

## Exposed Metadata

The relay can observe:

- node ids
- callsigns
- room names
- packet timing
- packet size patterns
- connection and disconnect events

## Operational Guidance

- Prefer encrypted DM for sensitive content.
- Treat room chat over relay as transport-visible.
- Run the relay on an operator-controlled host whenever possible.
- Use a tunnel only to expose the relay, not as a security boundary.
- Keep file relay disabled unless the environment is controlled.

## Current Policy Defaults

- room bridging: enabled
- file relay: disabled
- local-first preference: enabled
- blocked-peer relay forwarding: denied by local node policy