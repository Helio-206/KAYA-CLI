# KAYA Transport

KAYA uses IPv4 UDP multicast for local peer discovery and message fanout. It also supports manual direct TCP sessions for VPNs, Tailscale, corporate networks, and other environments where multicast is blocked.

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

## Direct TCP

Direct TCP is started manually from the TUI:

```text
> /listen 7777
> /connect 100.81.167.95:7777
```

Direct TCP uses length-prefixed JSON packet frames. The handshake exchanges signed `HELLO` packets and advertises capabilities such as `encrypted_dm`, `file_transfer`, `mesh`, and `direct_tcp`.

Targeted packet route priority is:

1. direct TCP;
2. multicast direct peer;
3. mesh;
4. relay.

Room, presence, and route-announcement packets are still sent over multicast and mirrored to active direct TCP peers. This keeps LAN behavior intact while making Tailscale and VPN operation reliable.

## Packet Lifecycle

```text
Packet
  -> protocol validation
  -> JSON encode
  -> UDP multicast send or direct TCP frame
  -> UDP receive or direct TCP frame receive
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

Active direct TCP sessions are closed during shutdown. `/stop-listener` stops only the listener; existing direct sessions remain active until `/disconnect <peer>` or `/exit`.
