#!/usr/bin/env bash
# Release gate: the coordinator image built by this pipeline (running as a CI service) must take a
# real experiment on Solana devnet from publish to a verified x402 purchase, replicated by three
# independent workers, and the sealed hash must equal the one recorded in the repository.
#
#   scripts/ci/devnet-canary.sh <replicata-cli.tar.gz>
#
# Env: CANARY_KEYS (file: {"author":[..],"worker1":[..],"worker2":[..],"worker3":[..],"consumer":[..]}),
#      CANARY_COORDINATOR (default http://coordinator:8787), CANARY_EXPERIMENT (default lorenz-int).
set -euo pipefail

API=${CANARY_COORDINATOR:-http://coordinator:8787}
NAME=${CANARY_EXPERIMENT:-lorenz-int}
WORK=${TMPDIR:-/tmp}/canary
EXAMPLE=examples/experiments/$NAME
STARTED=$(date +%s)

log() { printf '\033[1;34m[canary]\033[0m %s\n' "$*"; }

report() {
  jq -n --arg status "$1" --arg id "${ID:-}" --arg hash "${HASH:-}" --arg expected "$(cat "$EXAMPLE/expected_hash")" \
    --arg api "$API" --argjson secs "$(($(date +%s) - STARTED))" \
    '{status: $status, experiment: $id, result_hash: $hash, expected_hash: $expected, coordinator: $api, seconds: $secs,
      explorer: ("https://explorer.solana.com/address/" + $id + "?cluster=devnet")}' | tee canary.json
  echo "CANARY_STATUS=$1" >canary.env
  echo "CANARY_EXPERIMENT_ID=${ID:-}" >>canary.env
}

fail() {
  log "FAILED: $*"
  report failed
  exit 1
}

setup() {
  mkdir -p "$WORK"
  tar -xzf "$1" -C "$WORK"
  for key in author worker1 worker2 worker3 consumer; do
    jq -c ".$key" "$CANARY_KEYS" >"$WORK/$key.json"
  done
  for _ in $(seq 1 60); do curl -sf "$API/health" >/dev/null && return; sleep 2; done
  fail "coordinator $API never became healthy"
}

publish() {
  # Small amounts: the reward and the access fee circulate between the canary's own keys.
  jq '.reward = 300000 | .bond = 100000 | .access_price = 10000' "$EXAMPLE/publish.json" >"$WORK/publish.json"
  ID=$("$WORK/replicata" publish --coordinator "$API" --keypair "$WORK/author.json" "$WORK/publish.json" |
    sed -n 's/^published experiment \([1-9A-HJ-NP-Za-km-z]*\).*/\1/p')
  [[ -n $ID ]] || fail "publish did not return an experiment id"
  log "published $ID"
}

replicate() {
  for _ in $(seq 1 30); do curl -sf "$API/v1/experiments/$ID" >/dev/null && break; sleep 2; done
  for n in 1 2 3; do
    REPLICATA_HOME="$WORK/w$n" "$WORK/replicata" run --once "$ID" --coordinator "$API" \
      --keypair "$WORK/worker$n.json" >"$WORK/w$n.log" 2>&1 &
  done
  wait || true
  for _ in $(seq 1 60); do
    [[ $(curl -sf "$API/v1/experiments/$ID" | jq -r .status) == replicated ]] && return
    sleep 5
  done
  tail -n 5 "$WORK"/w*.log
  fail "experiment $ID was not replicated"
}

buy() {
  local code

  code=$(curl -s -o /dev/null -w '%{http_code}' "$API/v1/experiments/$ID/result")
  [[ $code == 402 ]] || fail "unpaid result returned HTTP $code instead of 402"
  HASH=$("$WORK/replicata" buy "$ID" --coordinator "$API" --keypair "$WORK/consumer.json" -o "$WORK/result.tar.gz" |
    sed -n 's/^verified result \([0-9a-f]*\).*/\1/p')
  [[ $HASH == "$(cat "$EXAMPLE/expected_hash")" ]] || fail "sealed hash $HASH != expected $(cat "$EXAMPLE/expected_hash")"
}

setup "$1"
publish
replicate
buy
log "replicated 3/3, sold over x402, hash $HASH matches the repository"
report passed
