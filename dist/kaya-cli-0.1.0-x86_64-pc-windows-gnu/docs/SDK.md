# KAYA SDK

`kaya-sdk 0.1.0` is the official Rust embedding layer for KAYA.

It is built on top of `kaya-core` and intentionally hides transport, protocol, UI, and mesh internals behind a stable client API.

## Public surface

Main entry points:

- `KayaClient`
- `KayaConfig`
- `KayaEvent`

Key methods:

- `KayaClient::new`
- `KayaClient::start`
- `set_callsign`
- `join_room`
- `send_room_message`
- `send_direct_message`
- `send_secure_direct_message`
- `send_file`
- `request_route`
- `trust_peer`
- `block_peer`
- `untrust_peer`
- `list_peers`
- `list_rooms`
- `inspect_routes`
- `mesh_status`
- `secure_sessions`
- `file_transfers`
- `subscribe_events`
- `stop`

## Example

```rust
use kaya_sdk::{KayaClient, KayaConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KayaClient::new(KayaConfig::default()).await?;

    client.set_callsign("Helio").await?;
    client.join_room("geral").await?;
    client.send_room_message("geral", "hello offline").await?;

    client.stop().await?;
    Ok(())
}
```

## Event subscription

```rust
let mut events = client.subscribe_events();
while let Ok(event) = events.recv().await {
    println!("event: {event:?}");
}
```

## Testability

`kaya-core` exposes a mock transport and `kaya-sdk` exposes `with_transport`, which allows deterministic SDK tests without LAN multicast.

## Examples

Run the packaged examples:

```bash
cargo run -p kaya-sdk --example simple-node
cargo run -p kaya-sdk --example room-bot -- geral
cargo run -p kaya-sdk --example secure-dm -- KY-REPLACE "hello secure offline"
cargo run -p kaya-sdk --example file-sender -- KY-REPLACE ./README.md
cargo run -p kaya-sdk --example mesh-status
```