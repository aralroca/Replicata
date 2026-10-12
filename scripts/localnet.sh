#!/usr/bin/env bash
# Local Replicata stack for development and end-to-end tests.
#
#   scripts/localnet.sh start    validator + program + USDC mint + funded keys + facilitator + Postgres
#   scripts/localnet.sh stop     stop everything started by `start`
#   scripts/localnet.sh fund <pubkey> [usdc]   airdrop 10 SOL and mint USDC (default 100)
#   scripts/localnet.sh env      print the env file path
#
# Everything lives in .localnet/ (keys, ledger, logs, env). Requires: solana, spl-token, docker, curl,
# and `anchor build` having produced target/deploy/replicata.so.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE="$ROOT/.localnet"
KEYS="$STATE/keys"
RPC="http://127.0.0.1:8899"
PROGRAM_ID="G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9"
FACILITATOR_PORT="${FACILITATOR_PORT:-8402}"
USERS=(author agent worker1 worker2 worker3 consumer crank)

log() { printf '\033[1;34m[localnet]\033[0m %s\n' "$*"; }

keypair() {
  local file="$KEYS/$1.json"
  [[ -f "$file" ]] || solana-keygen new --no-bip39-passphrase --silent --force -o "$file" >/dev/null
  echo "$file"
}

pubkey() { solana-keygen pubkey "$(keypair "$1")"; }

wait_for() {
  local what="$1"; shift
  for _ in $(seq 1 60); do
    if "$@" >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  echo "timed out waiting for $what" >&2
  exit 1
}

mint_address() { solana-keygen pubkey "$(keypair usdc-mint)"; }

fund() {
  local owner="$1" amount="${2:-100}" mint
  mint="$(mint_address)"
  solana airdrop 10 "$owner" -u "$RPC" >/dev/null
  spl-token create-account "$mint" --owner "$owner" --fee-payer "$(keypair mint-authority)" -u "$RPC" >/dev/null 2>&1 || true
  spl-token mint "$mint" "$amount" --recipient-owner "$owner" --mint-authority "$(keypair mint-authority)" \
    --fee-payer "$(keypair mint-authority)" -u "$RPC" >/dev/null
}

start_validator() {
  [[ -f "$ROOT/target/deploy/replicata.so" ]] || { echo "run 'anchor build' first" >&2; exit 1; }
  log "starting solana-test-validator"
  solana-test-validator --reset --quiet --ledger "$STATE/ledger" \
    --gossip-port 18000 --dynamic-port-range 18001-18030 \
    --bpf-program "$PROGRAM_ID" "$ROOT/target/deploy/replicata.so" >"$STATE/validator.log" 2>&1 &
  echo $! >"$STATE/validator.pid"
  wait_for "validator" solana cluster-version -u "$RPC"
}

create_mint() {
  log "creating local USDC mint $(mint_address)"
  solana airdrop 100 "$(pubkey mint-authority)" -u "$RPC" >/dev/null
  spl-token create-token "$(keypair usdc-mint)" --decimals 6 --mint-authority "$(keypair mint-authority)" \
    --fee-payer "$(keypair mint-authority)" -u "$RPC" >/dev/null
}

fund_users() {
  for user in "${USERS[@]}"; do
    fund "$(pubkey "$user")" 1000
    log "funded $user $(pubkey "$user")"
  done
  solana airdrop 100 "$(pubkey facilitator)" -u "$RPC" >/dev/null
}

start_facilitator() {
  log "starting x402 facilitator on :$FACILITATOR_PORT"
  (cd "$ROOT" && cargo build -q -p replicata-facilitator)
  RPC_URL="$RPC" FEE_PAYER="$(keypair facilitator)" PORT="$FACILITATOR_PORT" \
    "$ROOT/target/debug/replicata-facilitator" >"$STATE/facilitator.log" 2>&1 &
  echo $! >"$STATE/facilitator.pid"
  wait_for "facilitator" curl -sf "http://127.0.0.1:$FACILITATOR_PORT/supported"
}

start_postgres() {
  log "starting postgres (docker)"
  docker run -d --rm --name replicata-postgres -p "${PG_PORT:-54329}":5432 --tmpfs /var/lib/postgresql/data \
    -e POSTGRES_USER=replicata -e POSTGRES_PASSWORD=replicata -e POSTGRES_DB=replicata postgres:17-alpine >/dev/null
  wait_for "postgres" docker exec replicata-postgres pg_isready -U replicata -h 127.0.0.1
}

write_env() {
  cat >"$STATE/env" <<EOF
export CLUSTER=localnet
export RPC_URL=$RPC
export USDC_MINT=$(mint_address)
export FACILITATOR_URL=http://127.0.0.1:$FACILITATOR_PORT
export DATABASE_URL=postgres://replicata:replicata@127.0.0.1:${PG_PORT:-54329}/replicata
export CRANK_KEYPAIR=$(keypair crank)
export KEYS_DIR=$KEYS
EOF
  log "env written to $STATE/env"
}

stop() {
  for svc in facilitator validator; do
    if [[ -f "$STATE/$svc.pid" ]]; then
      kill "$(cat "$STATE/$svc.pid")" 2>/dev/null || true
      rm -f "$STATE/$svc.pid"
    fi
  done
  docker rm -f replicata-postgres >/dev/null 2>&1 || true
  log "stopped"
}

start() {
  mkdir -p "$KEYS"
  stop
  start_validator
  create_mint
  fund_users
  start_postgres
  start_facilitator
  write_env
  log "ready: source $STATE/env"
}

case "${1:-}" in
  start) start ;;
  stop) stop ;;
  fund) fund "$2" "${3:-100}" ;;
  env) echo "$STATE/env" ;;
  *) echo "usage: $0 start|stop|fund <pubkey> [usdc]|env" >&2; exit 1 ;;
esac
