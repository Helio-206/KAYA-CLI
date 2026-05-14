# LAB-27 Relay Peer Disconnect

## Goal

Verify relay cleanup after peer disconnect or heartbeat timeout.

## Steps

1. Connect two nodes to the same relay.
2. Confirm `/relay-peers` lists both.
3. Close one node or force `/relay-disconnect`.
4. Wait one cleanup interval.
5. Run `/relay-peers` again on the remaining node.

## Expected Result

- the disconnected node disappears from relay peer list
- remaining node stays connected
- no stale route or panic is shown