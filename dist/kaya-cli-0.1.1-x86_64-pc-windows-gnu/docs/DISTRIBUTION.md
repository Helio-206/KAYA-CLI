# KAYA Distribution

KAYA `0.1.1` is packaged for binary-first distribution so users do not need to clone the repository.

## Guaranteed Artifact

- `kaya-cli-0.1.1-x86_64-unknown-linux-gnu.tar.gz`

## Supported Optional Artifact

- `kaya-cli-0.1.1-x86_64-pc-windows-gnu.zip`

This archive contains:

- `bin/kaya` on Linux builds
- `bin/kaya.exe` on Windows builds
- release notes and installation docs
- install, uninstall, local-install, and checksum helper scripts

Phase 8 keeps `.deb` and macOS `.tar.gz` as optional tracks. Windows `.zip` packaging is available through the same release script, while Linux x86_64 tarballs remain the guaranteed release target for `0.1.1`.

## Release Workflow

```bash
./scripts/package-release.sh
KAYA_TARGET=x86_64-pc-windows-gnu ./scripts/package-release.sh
./scripts/generate-checksums.sh
```

Outputs under `dist/`:

- the staged unpacked directory
- the versioned `.tar.gz` or `.zip` archive
- `SHA256SUMS`

## Published Release Layout

GitHub Releases should contain:

- release archive
- `SHA256SUMS`
- `RELEASE_NOTES.md`
- install instructions referencing `install.sh` and manual tar extraction

## Install Paths

Default install path is `/usr/local/bin/kaya`.

Override with:

```bash
KAYA_INSTALL_DIR="$HOME/.local/bin" ./scripts/install-local.sh
```