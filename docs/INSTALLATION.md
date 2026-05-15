# KAYA Installation

## Without cloning

Install from GitHub Releases:

```bash
curl -fsSL https://github.com/natanielmatondo/KAYA-CLI/releases/download/v0.1.1/install.sh | sh
```

The installer downloads the Linux x86_64 archive, extracts it, and places `kaya` in `/usr/local/bin` by default.

Environment overrides:

- `KAYA_VERSION`
- `KAYA_TARGET`
- `KAYA_INSTALL_BASE_URL`
- `KAYA_INSTALL_DIR`

## Manual archive install

```bash
tar -xzf kaya-cli-0.1.1-x86_64-unknown-linux-gnu.tar.gz
sudo mv kaya-cli-0.1.1-x86_64-unknown-linux-gnu/bin/kaya /usr/local/bin/
kaya --version
```

## Ubuntu 22.04 package install

```bash
sudo dpkg -i kaya-cli_0.1.1-1ubuntu22.04_amd64.deb
kaya --version
```

## Local install from this repository

```bash
./scripts/install-local.sh
```

This installs from:

- a supplied archive path if given
- the matching `dist/` archive if present
- otherwise a fresh local release build

## Uninstall

```bash
./scripts/uninstall.sh
```

## Verification

```bash
kaya --version
sha256sum -c dist/SHA256SUMS
```