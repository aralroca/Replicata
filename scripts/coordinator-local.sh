#!/usr/bin/env bash
# Runs the coordinator against the stack started by `scripts/localnet.sh start`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/.localnet/env"
export BIND="${BIND:-127.0.0.1:8787}"
export PUBLIC_URL="${PUBLIC_URL:-http://127.0.0.1:8787}"
export POLL_MS="${POLL_MS:-1000}"

cargo build -q -p replicata-coordinator --manifest-path "$ROOT/Cargo.toml"
exec "$ROOT/target/debug/replicata-coordinator"
