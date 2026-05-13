# KAYA Transport

KAYA Phase 1 uses IPv4 UDP multicast for local peer discovery and message fanout.

## Defaults

```text
multicast_address = "239.71.0.1"
multicast_port = 42424
heartbeat_interval_secs = 3
peer_timeout_secs = 12
packet_max_bytes = 65536
```

These values are configurable in `~/.kaya/config.toml`.

## Socket Setup

The transport crate binds a UDP socket on all local IPv4 interfaces and joins the configured multicast group.

Important properties:

- `SO_REUSEADDR` is enabled for local multi-terminal labs.
- multicast loopback is enabled so multiple terminals on the same machine can see each other.
- multicast TTL is `1` to keep traffic local.

## Packet Lifecycle

```text
Packet
  -> protocol validation
  -> JSON encode
  -> UDP multicast send
  -> UDP receive
  -> packet size check
  -> JSON decode
  -> protocol validation
  -> PacketReceived event
```

## Stabilization

The runtime maintains a packet deduplication cache keyed by `packet_id`. Duplicate datagrams are dropped before they mutate peer or room state.

Malformed datagrams are rejected by the transport/protocol boundary and published as `ErrorOccurred` events.

## Graceful Shutdown

On `/exit`, the runtime broadcasts a `LEAVE` packet before aborting the network reader task.
