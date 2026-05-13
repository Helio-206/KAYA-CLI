# LAB: Peer Timeout

Goal: verify timeout behavior under abrupt node failure.

## Steps

1. Start two nodes.
2. Confirm `/who` shows both.
3. Kill one process without `/exit`.
4. Wait longer than `peer_timeout_secs` from `~/.kaya/config.toml`.

## Expected Behavior

- The surviving node emits `PeerTimedOut`.
- The peer remains known but is marked offline.
- No duplicate offline events are emitted for the same timeout window.
