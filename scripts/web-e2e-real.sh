#!/usr/bin/env bash
# Runs the web Playwright suite against the REAL local stack: validator + program, local USDC,
# x402 facilitator, coordinator, and a browser wallet that signs with .localnet/keys/consumer.json.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT/scripts/localnet.sh" start
cleanup() {
  [[ -n "${COORDINATOR_PID:-}" ]] && kill "$COORDINATOR_PID" 2>/dev/null || true
  "$ROOT/scripts/localnet.sh" stop
}
trap cleanup EXIT

cargo test -q -p replicata-e2e --manifest-path "$ROOT/Cargo.toml" --test web_seed -- --ignored
"$ROOT/scripts/coordinator-local.sh" >"$ROOT/.localnet/coordinator.log" 2>&1 &
COORDINATOR_PID=$!
for _ in $(seq 1 60); do curl -sf http://127.0.0.1:8787/health >/dev/null && break; sleep 1; done

set -a
# shellcheck disable=SC1091
source "$ROOT/target/web-scenarios.env"
set +a
E2E_REAL=1 \
  NEXT_PUBLIC_CLUSTER=localnet \
  NEXT_PUBLIC_RPC_URL=http://127.0.0.1:8899 \
  NEXT_PUBLIC_COORDINATOR_URL=http://127.0.0.1:8787 \
  E2E_WALLET_KEYPAIR="$ROOT/.localnet/keys/consumer.json" \
  pnpm -C "$ROOT/apps/web" test:e2e
