# KAYA Event System

KAYA uses an internal Tokio event bus to reduce coupling between networking, runtime state, and UI rendering.

## Crate

`crates/events`

## Transport

The event bus uses `tokio::sync::broadcast` with a bounded channel. Every subsystem can subscribe to the same stream without owning another subsystem directly.

Default capacity:

```text
1024 events
```

## Events

- `PeerDiscovered`
- `PeerTimedOut`
- `PacketReceived`
- `PacketSent`
- `IdentityLoaded`
- `IdentityCreated`
- `PacketSignatureValid`
- `PacketSignatureInvalid`
- `RoomCreated`
- `RoomJoined`
- `RoomLeft`
- `RoomMessageReceived`
- `DirectMessageSent`
- `DirectMessageReceived`
- `EncryptedMessageReceived`
- `PresenceUpdated`
- `PeerTrusted`
- `PeerBlocked`
- `SecureSessionStarted`
- `SecureSessionClosed`
- `SecurityWarning`
- `ErrorOccurred`
- `NetworkStarted`
- `ShutdownInitiated`

## Runtime Flow

```text
UDP socket
  -> network reader task
  -> PacketReceived event
  -> runtime handler
  -> security inspection
  -> peer/room/session state update
  -> domain events
  -> UI state and diagnostics
```

The UI never talks directly to networking. It receives already-normalized runtime state.

## Counters

`EventCounters` tracks per-kind counts for observability. The TUI technical panel displays total events and the hottest event counters.

## Failure Behavior

If a receiver lags, the runtime logs the skipped event count in the technical panel. The channel is intentionally bounded so floods cannot create unbounded memory growth.
