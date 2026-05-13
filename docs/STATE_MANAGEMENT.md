# KAYA State Management

KAYA separates state into small, purpose-specific crates.

## Peer State

`crates/peer`

Tracks:

- node id;
- callsign;
- rooms observed for that peer;
- last seen instant;
- online/offline state;
- optional latency.

Peers are keyed by node id, so duplicate heartbeats update a single record instead of creating duplicate peers.

## Room State

`crates/rooms`

Tracks:

- current local room;
- known rooms;
- room members;
- room message history;
- accepted direct messages.

Incoming room messages do not switch the local current room.

## Persistence

`crates/persistence`

Stores:

- `~/.kaya/config.toml`;
- sled history records;
- sled known-peer cache.

## Event-Driven Updates

State mutation happens inside the runtime after validated events:

```text
PacketReceived
  -> dedup
  -> peer registry
  -> room routing
  -> domain events
  -> UI projection
```

The UI is a projection of runtime state, not the owner of network or protocol state.
