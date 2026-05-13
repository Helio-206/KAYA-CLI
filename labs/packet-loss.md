# LAB: Packet Loss

Goal: observe KAYA behavior when UDP datagrams are dropped.

## Manual Simulation

Run two nodes:

```bash
KAYA_HOME=/tmp/kaya-a cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-b cargo run -p kaya-app --bin kaya
```

Send bursts of room messages from one node while temporarily disabling Wi-Fi or changing networks on the other.

## Expected Behavior

- No process crash.
- Missing UDP packets are not recovered in Phase 1.
- Heartbeats restore presence when connectivity returns.
- Peer timeout marks absent nodes offline.

## Observability

Watch:

- packet tx/rx counters;
- peer timeout logs;
- event counters;
- duplicate/malformed counters.
