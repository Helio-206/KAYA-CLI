# LAB-28 Relay Security Metadata

## Goal

Verify what remains protected and what metadata is visible when relay is in the path.

## Steps

1. Connect two peers through relay.
2. Send a room message.
3. Send an encrypted DM.
4. Observe relay logs or diagnostics.

## Expected Result

- room message routing metadata is visible to the relay operator
- encrypted DM body is not visible as plaintext
- node ids, timing, and delivery events remain observable