#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
install_dir="${KAYA_INSTALL_DIR:-/usr/local/bin}"
source_path="${1:-}"
tmp_dir="$(mktemp -d)"
cleanup() {
    rm -rf "${tmp_dir}"
}
trap cleanup EXIT

if [[ -z "${source_path}" ]]; then
    host_triple="$(rustc -vV | sed -n 's/^host: //p')"
    version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${repo_root}/Cargo.toml" | head -n 1)"
    source_path="${repo_root}/dist/kaya-cli-${version}-${host_triple}.tar.gz"
fi

if [[ -f "${source_path}" && "${source_path}" == *.tar.gz ]]; then
    tar -xzf "${source_path}" -C "${tmp_dir}"
    binary_path="$(find "${tmp_dir}" -path '*/bin/kaya' -type f | head -n 1)"
elif [[ -f "${source_path}" ]]; then
    binary_path="${source_path}"
else
    cargo build --release -p kaya-app --bin kaya
    binary_path="${repo_root}/target/release/kaya"
fi

if [[ ! -f "${binary_path}" ]]; then
    printf 'Unable to find a kaya binary to install\n' >&2
    exit 1
fi

mkdir -p "${install_dir}" 2>/dev/null || true
if [[ -w "${install_dir}" ]]; then
    install -m 0755 "${binary_path}" "${install_dir}/kaya"
else
    sudo install -m 0755 "${binary_path}" "${install_dir}/kaya"
fi

printf 'Installed local kaya binary to %s/kaya\n' "${install_dir}"