# LAB-24 Local vs Relay

## Goal

Verify that LAN multicast still works and that relay is added as a fallback rather than a replacement.

## Steps

1. Start two nodes on the same LAN without relay.
2. Send room chat and DM.
3. Start a relay with `kaya relay --bind 0.0.0.0:7777`.
4. Restart both nodes with `--relay tcp://HOST:7777`.
5. Send the same room chat and DM again.
6. Use `/relay-status` and `/relay-peers` on both nodes.

## Expected Result

- local room chat still appears immediately
- relay shows connected peers
- no protocol or UI crash occurs when relay is present