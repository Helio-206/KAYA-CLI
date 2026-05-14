# KAYA Mesh Relay

Phase 5 adds an experimental local mesh layer above the existing UDP multicast transport. It does not introduce internet relays, external servers, or libp2p. Direct LAN delivery remains the preferred path.

## Goals

- Discover lightweight routes to peers not currently direct.
- Relay DMs and encrypted DMs through local peers.
- Keep encrypted payloads opaque to relay nodes.
- Apply TTL, duplicate suppression, loop prevention, and block policy.
- Expose routing diagnostics in the TUI and logs.

## Envelope

`MESH_RELAY` carries an envelope around an existing protocol packet:

```json
{
  "mesh_version": 1,
  "mesh_packet_id": "2b80144f-f84f-47fb-8e69-15c2eac8b61a",
  "source_node": "KY-71AF92",
  "destination_node": "KY-A91C0D",
  "previous_hop": "KY-AAAAAA",
  "next_hop": "KY-A91C0D",
  "ttl": 4,
  "hop_count": 1,
  "route_trace": ["KY-71AF92", "KY-AAAAAA"],
  "created_at": "1778673210123",
  "inner_packet": {}
}
```

Relay rules:

- `ttl` decreases at every relay.
- `hop_count` increases at every relay.
- `route_trace` appends the relay node id.
- duplicate `mesh_packet_id` values are dropped.
- packets from blocked peers are dropped.
- packets that would return to `source_node` or a node already in `route_trace` are dropped.

## Route Discovery

KAYA advertises route summaries with `ROUTE_ANNOUNCE`. When an operator sends `/msg` or `/secure-msg` to a non-direct node id, the runtime emits `ROUTE_REQUEST`. A peer that knows a route replies with `ROUTE_RESPONSE`.

The routing table scores routes using:

- lower hop count;
- trusted peers;
- encrypted capability;
- lower latency when known;
- route freshness;
- failure count.

## Supported Traffic

Supported via mesh in Phase 5:

- `DIRECT_MESSAGE`
- `DM_SESSION_REQUEST`
- `DM_SESSION_ACCEPT`
- `DIRECT_MESSAGE_ENCRYPTED`
- file control packets: `FILE_OFFER`, `FILE_ACCEPT`, `FILE_REJECT`, `FILE_TRANSFER_CANCEL`, `FILE_TRANSFER_ERROR`

Not enabled in Phase 5:

- file chunks over mesh;
- room multicast relays;
- internet relay;
- persistent store-and-forward.

If a file chunk would need mesh delivery, KAYA reports: `file chunks over mesh not enabled yet`.

## Config

```toml
[mesh]
enabled = true
max_ttl = 5
allow_relay_for_unknown = true
allow_relay_for_blocked = false
relay_encrypted_only = false
route_expiry_seconds = 120
max_seen_packets = 5000
```

## Commands

- `/routes`: list known mesh routes.
- `/route <node-id|callsign>`: show the best route to a target.
- `/mesh-status`: show mesh counters and last route.
- `/mesh-clear`: clear routing table and dedup cache.

## Diagnostics

The network panel includes mesh state:

```text
mesh: yes    routes: 4    relayed/dropped: 129 / 3    avg hops: 2    trace: KY-71AF92 -> KY-AAAAAA
```

Logs include:

- `ROUTE_REQUEST_SENT`
- `ROUTE_RESPONSE_ACCEPTED`
- `MESH_RELAY_FORWARD`
- `MESH_PACKET_DROPPED`
- `MESH_DELIVERED`
- `RELAY_DENIED`
