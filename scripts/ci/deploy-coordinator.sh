#!/usr/bin/env bash
# Rolls the production coordinator (Render, image-backed service) to the exact image that passed the
# devnet canary, then waits until /version reports that build (the old instance stays healthy while
# Render swaps it, so /health alone cannot prove the deploy happened).
#
# Env: RENDER_DEPLOY_HOOK, IMAGE_REF (registry.gitlab.com/...:<sha>), EXPECTED_VERSION (<sha>),
#      COORDINATOR_URL.
set -euo pipefail

COORDINATOR_URL=${COORDINATOR_URL:-https://replicata-coordinator.onrender.com}
ENCODED=$(printf '%s' "$IMAGE_REF" | jq -sRr @uri)

curl -fsS -X POST "$RENDER_DEPLOY_HOOK&imgURL=$ENCODED" >/dev/null
echo "requested Render deploy of $IMAGE_REF"
for _ in $(seq 1 60); do
  live=$(curl -fsS --max-time 30 "$COORDINATOR_URL/version" 2>/dev/null || true)
  [[ $live == "$EXPECTED_VERSION" ]] && { echo "coordinator serving $live"; exit 0; }
  sleep 15
done
echo "coordinator still serving '${live:-unknown}', expected $EXPECTED_VERSION" >&2
exit 1
