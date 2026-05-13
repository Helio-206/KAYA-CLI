#!/usr/bin/env bash
set -euo pipefail

profile="${1:-operator}"
export KAYA_HOME="/tmp/kaya-${profile}"

cargo run -p kaya-app --bin kaya
