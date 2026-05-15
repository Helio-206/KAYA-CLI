#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${repo_root}/Cargo.toml" | head -n 1)"
host_triple="$(rustc -vV | sed -n 's/^host: //p')"
artifact="${1:-${repo_root}/dist/kaya-cli-${version}-${host_triple}.tar.gz}"
tmp_dir="$(mktemp -d)"
install_dir="${tmp_dir}/install-bin"

cleanup() {
    rm -rf "${tmp_dir}"
}
trap cleanup EXIT

if [[ ! -f "${artifact}" ]]; then
    printf 'Artifact not found: %s\n' "${artifact}" >&2
    exit 1
fi

printf 'Testing release artifact: %s\n' "${artifact}"
tar -xzf "${artifact}" -C "${tmp_dir}"

package_dir="$(find "${tmp_dir}" -mindepth 1 -maxdepth 1 -type d -name "kaya-cli-${version}-*" | head -n 1)"
if [[ -z "${package_dir}" ]]; then
    printf 'Unable to locate extracted package directory\n' >&2
    exit 1
fi

binary_path="${package_dir}/bin/kaya"
[[ -x "${binary_path}" ]]
[[ -f "${package_dir}/README.md" ]]
[[ -f "${package_dir}/docs/INSTALLATION.md" ]]
[[ -f "${package_dir}/docs/RELEASE.md" ]]
[[ -f "${package_dir}/docs/RELEASE_NOTES.md" ]]
[[ -x "${package_dir}/scripts/install-local.sh" ]]
[[ -x "${package_dir}/scripts/install.sh" ]]

version_output="$("${binary_path}" --version)"
about_output="$("${binary_path}" --about)"

printf '%s\n' "${version_output}" | grep -F "${version}" >/dev/null
printf '%s\n' "${about_output}" | grep -F "${version}" >/dev/null
printf '%s\n' "${about_output}" | grep -F 'Local-first communication for temporary digital communities.' >/dev/null

KAYA_INSTALL_DIR="${install_dir}" "${repo_root}/scripts/install-local.sh" "${artifact}"

installed_binary="${install_dir}/kaya"
[[ -x "${installed_binary}" ]]
installed_version_output="$("${installed_binary}" --version)"
printf '%s\n' "${installed_version_output}" | grep -F "${version}" >/dev/null

printf 'Release E2E smoke passed for %s\n' "${artifact}"