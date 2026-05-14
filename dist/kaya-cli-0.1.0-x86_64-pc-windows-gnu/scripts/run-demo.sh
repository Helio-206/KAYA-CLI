#!/usr/bin/env bash
set -euo pipefail

profile="${1:-presenter}"
if [[ $# -gt 0 ]]; then
    shift
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
data_dir="${KAYA_HOME:-/tmp/kaya-demo-${profile}}"

export KAYA_HOME="${data_dir}"

printf 'Starting KAYA demo profile=%s data_dir=%s\n' "${profile}" "${data_dir}"
printf 'Suggested in-app flow: /demo-peers 4 ; /demo-message semana-info 4 ; /demo-mesh-route\n'

cargo run --manifest-path "${repo_root}/Cargo.toml" -p kaya-app --bin kaya -- --demo --profile demo --data-dir "${data_dir}" "$@"
