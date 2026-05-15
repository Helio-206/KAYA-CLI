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
- voice command parsing, protocol validation, and runtime/UI voice state projection;
- push-to-talk keyboard handling in the TUI;
- voice config TOML validation;
- transport packet deduplication;
- config TOML validation.

## Voice Validation

Default build without native audio host integration:

```bash
cargo test -p kaya-voice -p kaya-persistence -p kaya-protocol
cargo test -p kaya-ui -p kaya-events -p kaya-rooms -p kaya-app
```

Linux voice media path uses `arecord` and `aplay` at runtime, so a functional end-to-end voice check also needs those binaries available on the host.
Windows voice media path uses the native `cpal` backend and can be cross-checked with:

```bash
cargo check -p kaya-app --target x86_64-pc-windows-gnu
```

Optional Linux native audio validation:

```bash
sudo apt-get install -y libasound2-dev
cargo test -p kaya-voice -p kaya-app --features kaya-voice/native-audio
```

The default build keeps `native-audio` disabled so the workspace compiles even when ALSA development headers are unavailable.

## Release E2E Smoke

Validate the packaged release artifact end to end on Linux with:

```bash
./scripts/test-release-e2e.sh
```

The script extracts the packaged archive, checks the expected release layout, verifies `--version` and `--about`, and exercises `install-local.sh` against the packaged binary in an isolated install directory.

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
