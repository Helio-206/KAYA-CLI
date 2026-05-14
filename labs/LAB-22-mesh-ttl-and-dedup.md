# LAB-22 Mesh TTL and Dedup

## Objective

Validate mesh packet TTL handling, loop prevention, and duplicate mesh packet rejection.

## Setup

Use three nodes or a packet replay harness that can resend the same `MESH_RELAY` envelope.

## Commands

On any node:

```text
> /mesh-status
> /routes
```

Replay or inject a duplicate `MESH_RELAY` with the same `mesh_packet_id`, then inspect logs.

## Topology

```text
Helio -> Ana -> Bruno
```

Create or simulate a loop by routing a packet back to a node already present in `route_trace`.

## Expected Result

- TTL decreases with each relay.
- Hop count increases with each relay.
- Duplicate `mesh_packet_id` values are dropped.
- Packets with exhausted TTL are dropped.
- Packets whose route trace already contains the local node are dropped.

## Troubleshooting

- Run `cargo test -p kaya-mesh` for deterministic TTL and duplicate tests.
- Use `/mesh-clear` to reset the seen-packet cache between manual runs.
- Check technical logs for `MESH_PACKET_DROPPED`.
