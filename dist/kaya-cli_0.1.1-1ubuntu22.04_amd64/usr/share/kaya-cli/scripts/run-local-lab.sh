#!/usr/bin/env bash
set -euo pipefail

profile="${1:-operator}"
if [[ $# -gt 0 ]]; then
	shift
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
data_dir="${KAYA_HOME:-/tmp/kaya-lab-${profile}}"

export KAYA_HOME="${data_dir}"

printf 'Starting KAYA lab profile=%s data_dir=%s\n' "${profile}" "${data_dir}"

cargo run --manifest-path "${repo_root}/Cargo.toml" -p kaya-app --bin kaya -- --profile lab --data-dir "${data_dir}" "$@"
