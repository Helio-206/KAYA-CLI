# Embedding KAYA

Use `kaya-sdk` when you want to embed a KAYA node inside another Rust application.

## What you do not need to import

Applications using the SDK should not need to import:

- `kaya-transport`
- `kaya-protocol`
- `kaya-ui`
- mesh relay internals

The SDK keeps those details behind `KayaClient`.

## Embedding model

`KayaClient::new` starts a headless node backed by `kaya-core`.

Your application can then:

- subscribe to `KayaEvent`
- render its own UI
- bridge events into another runtime
- wrap KAYA in bots, services, or local desktop apps

## Recommended pattern

1. Create one `KayaClient` per embedded node.
2. Set a callsign early.
3. Subscribe to events in a dedicated task.
4. Expose only your own application-level commands to the user.
5. Call `stop()` during shutdown.

## Current boundary

For `0.1.1`, the SDK is intended for single-process embedding. The daemon model is documented separately in [DAEMON.md](DAEMON.md).