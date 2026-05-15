#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
target_dir="${1:-${repo_root}/dist}"
output_file="${target_dir}/SHA256SUMS"

if [[ ! -d "${target_dir}" ]]; then
    printf 'Directory not found: %s\n' "${target_dir}" >&2
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    checksum_cmd=(sha256sum)
elif command -v shasum >/dev/null 2>&1; then
    checksum_cmd=(shasum -a 256)
else
    printf 'Neither sha256sum nor shasum is available\n' >&2
    exit 1
fi

tmp_file="$(mktemp)"
find "${target_dir}" -maxdepth 1 -type f \
    \( -name '*.tar.gz' -o -name '*.zip' -o -name '*.deb' \) \
    -print0 \
    | sort -z \
    | while IFS= read -r -d '' artifact; do
        (cd "${target_dir}" && "${checksum_cmd[@]}" "$(basename "${artifact}")")
    done > "${tmp_file}"

mv "${tmp_file}" "${output_file}"
printf 'Wrote checksums: %s\n' "${output_file}"