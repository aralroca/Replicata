#!/usr/bin/env bash
# Deploys apps/web to Vercel. `preview` prints STAGING_URL=<url> (a dotenv line for the pipeline);
# `production` deploys and aliases https://replicata.vercel.app.
#
# Env: VERCEL_TOKEN, VERCEL_ORG_ID, VERCEL_PROJECT_ID.
set -euo pipefail

TARGET=${1:?preview|production}
ARGS=(deploy --yes --token "$VERCEL_TOKEN")
[[ $TARGET == production ]] && ARGS+=(--prod)

URL=$(npx --yes vercel@57 "${ARGS[@]}" 2>/dev/null | tail -n 1)
[[ $URL == https://* ]] || { echo "vercel deploy failed: $URL" >&2; exit 1; }
curl -fsS --retry 5 --retry-delay 5 -o /dev/null "$URL"
echo "STAGING_URL=$URL"
