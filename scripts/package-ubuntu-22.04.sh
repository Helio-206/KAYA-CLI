#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
out_dir="${1:-${repo_root}/dist}"

cd "${repo_root}"

version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "${version}" ]]; then
    printf 'Unable to determine workspace version from Cargo.toml\n' >&2
    exit 1
fi

if ! command -v cargo-zigbuild >/dev/null 2>&1; then
    printf 'cargo-zigbuild is required to build the Ubuntu 22.04 package\n' >&2
    exit 1
fi

if ! command -v dpkg-deb >/dev/null 2>&1; then
    printf 'dpkg-deb is required to build the Ubuntu 22.04 package\n' >&2
    exit 1
fi

package_version="${version}-1ubuntu22.04"
package_name="kaya-cli_${package_version}_amd64"
staging_dir="${out_dir}/${package_name}"
archive_path="${out_dir}/${package_name}.deb"
build_target="x86_64-unknown-linux-gnu.2.35"
build_output_dir="target/x86_64-unknown-linux-gnu/release"

rm -rf "${staging_dir}"
mkdir -p \
    "${staging_dir}/DEBIAN" \
    "${staging_dir}/usr/bin" \
    "${staging_dir}/usr/share/doc/kaya-cli" \
    "${staging_dir}/usr/share/kaya-cli/scripts"

cargo zigbuild --release -p kaya-app --bin kaya --target "${build_target}"

install -m 0755 "${build_output_dir}/kaya" "${staging_dir}/usr/bin/kaya"
install -m 0644 README.md LICENSE RELEASE_NOTES.md "${staging_dir}/usr/share/doc/kaya-cli/"
install -m 0644 \
    docs/COMMANDS.md \
    docs/DISTRIBUTION.md \
    docs/INSTALLATION.md \
    docs/RELEASE.md \
    docs/SECURITY.md \
    docs/TAILSCALE.md \
    docs/WAN_RELAY.md \
    "${staging_dir}/usr/share/doc/kaya-cli/"
install -m 0755 \
    scripts/install.sh \
    scripts/uninstall.sh \
    scripts/run-demo.sh \
    scripts/run-local-lab.sh \
    "${staging_dir}/usr/share/kaya-cli/scripts/"

cat > "${staging_dir}/DEBIAN/control" <<EOF
Package: kaya-cli
Version: ${package_version}
Section: net
Priority: optional
Architecture: amd64
Maintainer: KAYA CLI Contributors
Homepage: https://github.com/natanielmatondo/KAYA-CLI
Depends: libc6 (>= 2.35)
Description: Local-first communication CLI for temporary digital communities
 KAYA CLI is an offline-first, terminal-based communication system for local
 networks, direct TCP links, and relay-assisted operation.
EOF

chmod 0644 "${staging_dir}/DEBIAN/control"
rm -f "${archive_path}"
dpkg-deb --root-owner-group --build "${staging_dir}" "${archive_path}"

printf 'Created Ubuntu 22.04 package: %s\n' "${archive_path}"
printf 'Staging directory: %s\n' "${staging_dir}"