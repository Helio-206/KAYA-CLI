# KAYA Testing

Run all tests:

```bash
cargo test
```

Run quality gates:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

## Coverage Areas

- command registry, aliases, and validation;
- protocol encode/decode and hardening;
- Phase 2 room and presence packet validation;
- malformed packet rejection;
- fuzz-like invalid protocol inputs;
- event bus delivery and counters;
- peer timeout and duplicate updates;
- duplicate callsign detection;
- presence update handling;
- room synchronization and routing;
- room creation, join, leave, and member snapshots;
- DM delivery and history filtering;
- transport packet deduplication;
- config TOML validation.

## Benchmarks

Message codec benchmark:

```bash
cargo run -p kaya-app --bin kaya-bench-messages -- 50000
```

Peer registry benchmark:

```bash
cargo run -p kaya-app --bin kaya-bench-peers -- 10000
```

These are lightweight operational benchmarks, not scientific performance suites.
