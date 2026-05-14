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
- identity generation and identity TOML persistence;
- fingerprint generation;
- packet signing and signature validation;
- invalid signature detection after packet tampering;
- trust store state transitions and blocked peers;
- X25519 shared-secret equality through secure DM session setup;
- encrypted DM decrypt success and tamper failure;
- secure session lifecycle;
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
