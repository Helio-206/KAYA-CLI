# KAYA Distribution

KAYA `0.1.0` is packaged for binary-first distribution so users do not need to clone the repository.

## Guaranteed Artifact

- `kaya-cli-0.1.0-x86_64-unknown-linux-gnu.tar.gz`

This archive contains:

- `bin/kaya`
- release notes and installation docs
- install, uninstall, local-install, and checksum helper scripts

## Optional Artifacts

Phase 8 keeps `.deb`, Windows `.zip`, and macOS `.tar.gz` as optional tracks. The release pipeline is prepared around checksums and install docs, but the guaranteed release target for `0.1.0` remains Linux x86_64 tarballs.

## Release Workflow

```bash
./scripts/package-release.sh
./scripts/generate-checksums.sh
```

Outputs under `dist/`:

- the staged unpacked directory
- the versioned `.tar.gz` archive
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