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

host_triple="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "${host_triple}" ]]; then
    printf 'Unable to determine rust host triple\n' >&2
    exit 1
fi

package_name="kaya-cli-${version}-${host_triple}"
staging_dir="${out_dir}/${package_name}"
archive_path="${out_dir}/${package_name}.tar.gz"

rm -rf "${staging_dir}"
mkdir -p "${staging_dir}/bin" "${staging_dir}/docs" "${staging_dir}/scripts"

cargo build --release -p kaya-app --bin kaya

install -m 0755 target/release/kaya "${staging_dir}/bin/kaya"
install -m 0644 README.md LICENSE Cargo.toml "${staging_dir}/"
install -m 0644 \
    docs/COMMANDS.md \
    docs/DEMO_MODE.md \
    docs/DISTRIBUTION.md \
    docs/INSTALLATION.md \
    docs/NGROK.md \
    docs/RELEASE.md \
    docs/RELAY_SECURITY.md \
    docs/SDK.md \
    docs/SECURITY.md \
    docs/VERSIONING.md \
    docs/WAN_RELAY.md \
    docs/FRIEND_QUICKSTART.md \
    RELEASE_NOTES.md \
    "${staging_dir}/docs/"
install -m 0755 \
    scripts/generate-checksums.sh \
    scripts/install-local.sh \
    scripts/install.sh \
    scripts/run-demo.sh \
    scripts/run-local-lab.sh \
    scripts/uninstall.sh \
    "${staging_dir}/scripts/"

tar -C "${out_dir}" -czf "${archive_path}" "${package_name}"

printf 'Created release archive: %s\n' "${archive_path}"
printf 'Staging directory: %s\n' "${staging_dir}"
