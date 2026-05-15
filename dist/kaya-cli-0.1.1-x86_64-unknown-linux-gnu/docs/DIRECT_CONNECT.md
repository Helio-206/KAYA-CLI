# Direct Peer Connectivity

KAYA supports manual direct TCP connectivity as a fallback and core transport when UDP multicast discovery is unavailable.

Multicast remains enabled. Direct TCP adds an explicit path:

```text
Node A: /listen 7777
Node B: /connect 100.x.x.x:7777
```

After the handshake, peers are registered in the same runtime state used by LAN discovery. DMs, secure DMs, file control packets, file chunks, room state, presence and mesh route announcements can flow over the direct TCP session.

## Transport

Direct connectivity uses:

- `tokio::net::TcpListener`
- `tokio::net::TcpStream`
- length-prefixed frames
- JSON KAYA packets

Frame format:

```text
u32_be length
serde_json Packet bytes
```

The direct frame limit is larger than UDP packet limits so file chunks can move over TCP without UDP datagram constraints.

## Handshake

The direct handshake exchanges signed `HELLO` packets:

```text
outbound peer -> HELLO
inbound peer  -> HELLO
```

The `HELLO` payload advertises protocol version, fingerprint, and capabilities such as `direct_tcp`, `encrypted_dm`, `file_transfer`, `mesh`, `relay`, `rooms`, and `presence`.

The runtime verifies the signed packet envelope through the normal security pipeline. Blocked peers are rejected and duplicate direct sessions for the same node are dropped.

## Commands

```text
/listen <port>
/connect <ip>:<port>
/disconnect <peer>
/connections
/stop-listener
/listen-status
```

Example:

```text
> /listen 7777
> /listen-status
> /connections
```

Remote peer:

```text
> /connect 100.81.167.95:7777
> /secure-msg Helio teste seguro via direct_tcp
> /send Helio ./docs/PROTOCOL.md
```

## Route Priority

When a packet targets a specific peer, KAYA uses:

1. `direct_tcp`
2. multicast direct peer
3. mesh route
4. relay

Room and presence packets are still sent over multicast and mirrored to active direct TCP sessions. This keeps local LAN behavior unchanged while making Tailscale/VPN operation usable.

## UI

The TUI shows a `CONNECTIONS` panel with peer callsign, node id, transport type, remote address, connection state, and secure capability.

The `NETWORK` panel also shows listener address and direct connection count.

## Operational Notes

- Use `/listen <port>` to bind on all interfaces, including Tailscale and LAN.
- Open the chosen port in the OS firewall if needed.
- Secure DMs remain end-to-end encrypted; the TCP transport only carries packets.
- Trust store and block rules are shared with multicast, mesh and relay.
