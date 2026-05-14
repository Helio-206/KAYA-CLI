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
- whether the local node joined each room;
- room members;
- room message history;
- accepted direct messages.
- encrypted marker for secure DMs.

Incoming room messages do not switch the local current room. Messages for rooms the local node has not joined are ignored at routing time.

## Presence State

`crates/peer`

Peers carry presence:

- `online`
- `away`
- `busy`
- `invisible`
- `offline`

`offline` is derived from `LEAVE` or heartbeat timeout.

## Persistence

`crates/persistence`

Stores:

- `~/.kaya/config.toml`;
- sled history records;
- sled known-peer cache.

History records include:

- timestamp;
- room or DM target;
- sender;
- body;
- direct/public marker;
- event marker.
- encrypted marker.

## Security State

`crates/security`

Tracks:

- persistent local identity from `~/.kaya/identity.toml`;
- public fingerprint;
- trust store records from `~/.kaya/trust.toml`;
- trusted/blocked peer counts;
- in-memory secure DM sessions;
- pending secure messages waiting for handshake acceptance.

## File Transfer State

`crates/files`

Tracks:

- incoming and outgoing transfer sessions;
- metadata and completed file paths;
- chunk buffers for reassembly;
- progress counters;
- final hash verification status;
- persisted transfer records under `~/.kaya/files/metadata`.

## Event-Driven Updates

State mutation happens inside the runtime after validated events:

```text
PacketReceived
  -> dedup
  -> signature/trust inspection
  -> peer registry
  -> room or secure-session routing
  -> file-transfer routing
  -> domain events
  -> UI projection
```

The UI is a projection of runtime state, not the owner of network or protocol state.
