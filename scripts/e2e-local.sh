#!/usr/bin/env bash
# Runs the full end-to-end suite against a fresh local stack, N times in a row (default 3)
# to catch flakiness. Requires Docker, the Solana CLI and `anchor build`.
#
#   scripts/e2e-local.sh [runs]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNS="${1:-3}"

"$ROOT/scripts/localnet.sh" start
trap '"$ROOT/scripts/localnet.sh" stop' EXIT

cargo build -q -p replicata-e2e --tests --manifest-path "$ROOT/Cargo.toml"
for run in $(seq 1 "$RUNS"); do
  echo "=== e2e run $run/$RUNS ==="
  cargo test -q -p replicata-e2e --manifest-path "$ROOT/Cargo.toml" -- --ignored --test-threads=1
done
echo "=== e2e: $RUNS/$RUNS runs green ==="
