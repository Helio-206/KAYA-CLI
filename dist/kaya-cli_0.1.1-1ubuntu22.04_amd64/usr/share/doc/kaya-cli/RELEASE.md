# KAYA Release

This document captures the minimum release-candidate flow for KAYA CLI.

## Quality Gates

Run these before packaging:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

For an app-focused validation pass during active development:

```bash
cargo test -p kaya-app -p kaya-ui -p kaya-commands
```

## Package

Build the release archive with:

```bash
./scripts/package-release.sh
KAYA_TARGET=x86_64-pc-windows-gnu ./scripts/package-release.sh
KAYA_TARGET=x86_64-apple-darwin ./scripts/package-release.sh
./scripts/package-ubuntu-22.04.sh
```

The script:

- builds `kaya` in release mode
- creates a versioned directory under `dist/`
- copies the binary, core docs, and helper scripts
- emits a `.tar.gz` archive for distribution

The Ubuntu 22.04 package script:

- builds `kaya` against glibc 2.35 via `cargo-zigbuild`
- assembles a Debian package layout under `dist/`
- emits a `.deb` package for Ubuntu 22.04 amd64

Generate checksums after packaging:

```bash
./scripts/generate-checksums.sh
```

Run the packaged-artifact smoke test before publishing:

```bash
./scripts/test-release-e2e.sh
```

The release bundle now includes:

- `bin/kaya`
- installation scripts
- installation, distribution, SDK, release, and versioning docs
- `RELEASE_NOTES.md`

## Release Candidate Checklist

- verify startup flags: `--demo`, `--profile`, `--data-dir`, `--version`, `--about`
- verify demo commands from [docs/DEMO_MODE.md](docs/DEMO_MODE.md)
- verify persistence isolation between `default`, `demo`, `lab`, and `paranoid`
- verify the splash screen, command hints, empty states, and modal overlays in the TUI
- verify secure session expiry, route timeout cleanup, and stale file-transfer expiry
- verify the produced archive launches with `bin/kaya`
- verify `./scripts/install-local.sh` installs the packaged binary cleanly
- verify `./scripts/uninstall.sh` removes the binary cleanly
- verify `dist/SHA256SUMS` matches the published artifacts

## Recommended Smoke Test

```bash
./scripts/run-demo.sh rc-smoke
```

Then inside the TUI:

```text
> /demo-peers 3
> /demo-message semana-info 3
> /demo-mesh-route
> /demo-file-offer
> /demo-security-warning
> /status
```
