#!/usr/bin/env bash
# Rolls the production coordinator (Render, image-backed service) to the exact image that passed the
# devnet canary, then waits until the new instance answers /health.
#
# Env: RENDER_DEPLOY_HOOK, IMAGE_REF (registry.gitlab.com/...:<sha>), COORDINATOR_URL.
set -euo pipefail

COORDINATOR_URL=${COORDINATOR_URL:-https://replicata-coordinator.onrender.com}
ENCODED=$(printf '%s' "$IMAGE_REF" | jq -sRr @uri)

curl -fsS -X POST "$RENDER_DEPLOY_HOOK&imgURL=$ENCODED" >/dev/null
echo "requested Render deploy of $IMAGE_REF"
sleep 30
for _ in $(seq 1 40); do
  curl -fsS --max-time 30 "$COORDINATOR_URL/health" >/dev/null && { echo "coordinator healthy"; exit 0; }
  sleep 15
done
echo "coordinator did not become healthy after the deploy" >&2
exit 1
