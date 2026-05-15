# KAYA WAN Relay

KAYA remains LAN-native. The WAN relay is optional and additive: local multicast and local mesh continue to work, and relay is used when peers are not reachable on the same network.

## Modes

- `LAN only`: default multicast and local mesh, no relay.
- `Local-first relay`: multicast and mesh first, relay used for WAN fallback and room mirroring.
- `Relay-only rooms`: room traffic is mirrored to the relay without keeping local-first room bridging.

## Start a Relay

```bash
cargo run -p kaya-app --bin kaya -- relay --bind 0.0.0.0:7777
```

The process prints the `tcp://` address and stays running until `Ctrl+C`.

## Connect a CLI Node

```bash
cargo run -p kaya-app --bin kaya -- --relay tcp://HOST:7777
```

Inside the CLI:

```text
> /relay-status
> /relay-peers
> /relay-mode local-first
```

## Routing Order

Direct delivery order is:

1. local direct peer
2. local mesh route
3. WAN relay

Room traffic is still sent on LAN. When relay room bridging is enabled, the same signed room packet is also wrapped and mirrored through the relay.

## Operations

- `/relay-connect <tcp://host:port>` connects to a relay without restart.
- `/relay-disconnect` disconnects the current relay session.
- `/relay-mode [local-first|relay-only]` shows or changes room mirroring strategy.
- `/relay-status` shows connection status, peer count, mode, and URL.

## Limitations

- Relay metadata can see node ids, callsigns, room names, and delivery timing.
- Encrypted DMs stay encrypted end-to-end, but room messages are not end-to-end encrypted.
- File relay remains conservative by default and is disabled in config unless enabled explicitly.