#!/usr/bin/env bash
# CI helper: installs the Agave CLI (solana, solana-test-validator, spl-token, cargo build-sbf)
# and anchor-cli, and exposes them on GITHUB_PATH.
set -euo pipefail

sh -c "$(curl -sSfL "https://release.anza.xyz/${AGAVE_VERSION:-v4.1.2}/install")"
SOLANA_BIN="$HOME/.local/share/solana/install/active_release/bin"
echo "$SOLANA_BIN" >>"${GITHUB_PATH:-/dev/null}"
export PATH="$SOLANA_BIN:$PATH"

if ! command -v anchor >/dev/null || ! anchor --version | grep -q "${ANCHOR_VERSION:-1.2.1}"; then
  cargo install anchor-cli --version "${ANCHOR_VERSION:-1.2.1}" --locked
fi
solana-keygen new --no-bip39-passphrase --silent --force -o "$HOME/.config/solana/id.json" >/dev/null
solana --version
anchor --version
