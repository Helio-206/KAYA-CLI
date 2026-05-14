# LAB-19 Route Table

## Objective

Verify that KAYA builds and displays mesh route table entries from local peers and route announcements.

## Setup

Open two or three terminals with separate `KAYA_HOME` values on the same LAN.

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-bruno cargo run -p kaya-app --bin kaya
```

## Commands

On each node:

```text
> /who
> /routes
> /mesh-status
```

## Topology

All nodes are on the same multicast LAN for this lab. Route entries may show as direct because each process can hear the others.

## Expected Result

- `/routes` lists direct peers with `hops=1`.
- `/mesh-status` shows `enabled=true` and a non-zero route count after peers exchange packets.
- Technical logs include route discovery or route announcement activity.

## Troubleshooting

- If routes stay empty, send `/room ping` from each node to force packet observation.
- Verify multicast is allowed on the network.
- Confirm each terminal uses a different `KAYA_HOME` so identities are distinct.
