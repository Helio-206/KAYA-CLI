# KAYA Tests

The workspace keeps executable tests close to their crates:

- `crates/shared`: node id generation and normalization.
- `crates/protocol`: packet serialization, validation, and decoding.
- `crates/commands`: command parser.
- `crates/peer`: peer discovery and timeout.
- `crates/rooms`: room routing and DMs.
- `crates/persistence`: config and history storage.
- `crates/transport`: datagram decode guardrails.

Run all tests:

```bash
cargo test
```
