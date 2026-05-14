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
- file metadata validation and safe filename handling;
- path traversal rejection;
- file chunk splitting and reassembly;
- chunk hash and final hash validation;
- file transfer accept/reject/cancel state transitions;
- encrypted file chunk decrypt success and tamper failure;
- transfer metadata persistence;
- mesh TTL decrement, hop count increment, and route trace update;
- duplicate mesh packet rejection;
- blocked peer relay denial and no-loop relay behavior;
- route scoring, expiry, request/response, and table clear;
- encrypted DM payload opacity through relay;
- mesh config validation;
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

File transfer benchmarks:

```bash
cargo run -p kaya-app --bin kaya-bench-file-chunking -- 8
cargo run -p kaya-app --bin kaya-bench-file-reassembly -- 8
```

Mesh benchmarks:

```bash
cargo run -p kaya-app --bin kaya-bench-route-table -- 10000
cargo run -p kaya-app --bin kaya-bench-mesh-dedup -- 10000
cargo run -p kaya-app --bin kaya-bench-mesh-relay-simulated -- 10000
```

These are lightweight operational benchmarks, not scientific performance suites.
