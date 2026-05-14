#!/usr/bin/env bash
set -euo pipefail

install_dir="${KAYA_INSTALL_DIR:-/usr/local/bin}"
binary_path="${install_dir}/kaya"

if [[ ! -e "${binary_path}" ]]; then
    printf 'kaya is not installed at %s\n' "${binary_path}"
    exit 0
fi

if [[ -w "${install_dir}" ]]; then
    rm -f "${binary_path}"
else
    sudo rm -f "${binary_path}"
fi

printf 'Removed %s\n' "${binary_path}"