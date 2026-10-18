#!/usr/bin/env bash
# CI helper: installs the Agave CLI (solana, solana-test-validator, spl-token, cargo build-sbf)
# and anchor-cli, and exposes them on GITHUB_PATH.
set -euo pipefail

# `cargo install` puts anchor in $CARGO_HOME/bin (GitLab caches CARGO_HOME inside the project).
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
sh -c "$(curl -sSfL "https://release.anza.xyz/${AGAVE_VERSION:-v4.1.2}/install")"
SOLANA_BIN="$HOME/.local/share/solana/install/active_release/bin"
echo "$SOLANA_BIN" >>"${GITHUB_PATH:-/dev/null}"
export PATH="$SOLANA_BIN:$PATH"

ANCHOR_VERSION=${ANCHOR_VERSION:-1.2.1}
ANCHOR_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/anchor"
# Prefer the release binary (Linux x86_64): compiling anchor-cli takes most of a CI job.
install_anchor() {
  local url="https://github.com/solana-foundation/anchor/releases/download/v$ANCHOR_VERSION/anchor-$ANCHOR_VERSION-x86_64-unknown-linux-gnu"

  mkdir -p "$(dirname "$ANCHOR_BIN")"
  if [[ "$(uname -s)-$(uname -m)" == Linux-x86_64 ]] && curl -sSfL -o "$ANCHOR_BIN" "$url"; then
    chmod +x "$ANCHOR_BIN"
  else
    cargo install anchor-cli --version "$ANCHOR_VERSION" --locked
  fi
}

if ! command -v anchor >/dev/null || ! anchor --version | grep -q "$ANCHOR_VERSION"; then
  install_anchor
fi
solana-keygen new --no-bip39-passphrase --silent --force -o "$HOME/.config/solana/id.json" >/dev/null
solana --version
anchor --version
