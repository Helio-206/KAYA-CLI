#!/usr/bin/env bash
set -euo pipefail

version="${KAYA_VERSION:-0.1.0}"
target="${KAYA_TARGET:-x86_64-unknown-linux-gnu}"
install_dir="${KAYA_INSTALL_DIR:-/usr/local/bin}"
repo_url="${KAYA_INSTALL_BASE_URL:-https://github.com/natanielmatondo/KAYA-CLI/releases/download/v${version}}"
archive="kaya-cli-${version}-${target}.tar.gz"
tmp_dir="$(mktemp -d)"
cleanup() {
    rm -rf "${tmp_dir}"
}
trap cleanup EXIT

if ! command -v curl >/dev/null 2>&1; then
    printf 'curl is required to install KAYA\n' >&2
    exit 1
fi
if ! command -v tar >/dev/null 2>&1; then
    printf 'tar is required to install KAYA\n' >&2
    exit 1
fi

printf 'Downloading %s\n' "${archive}"
curl -fsSL "${repo_url}/${archive}" -o "${tmp_dir}/${archive}"
tar -xzf "${tmp_dir}/${archive}" -C "${tmp_dir}"

package_dir="${tmp_dir}/kaya-cli-${version}-${target}"
binary_path="${package_dir}/bin/kaya"
if [[ ! -f "${binary_path}" ]]; then
    printf 'Binary not found in package: %s\n' "${binary_path}" >&2
    exit 1
fi

mkdir -p "${install_dir}" 2>/dev/null || true
if [[ -w "${install_dir}" ]]; then
    install -m 0755 "${binary_path}" "${install_dir}/kaya"
else
    sudo install -m 0755 "${binary_path}" "${install_dir}/kaya"
fi

printf 'Installed kaya to %s/kaya\n' "${install_dir}"
printf 'Run: kaya --version\n'