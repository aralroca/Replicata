#!/bin/sh
# Replicata worker installer.
#   curl -sSf https://<site>/install.sh | sh
# Downloads `replicata` from the latest GitHub release of aralroca/Replicata
# (asset `replicata-<target>.tar.gz`, target = darwin-arm64 | linux-x86_64) into $REPLICATA_INSTALL_DIR (default ~/.local/bin).
set -eu

REPO="aralroca/Replicata"
VERSION="${REPLICATA_VERSION:-latest}"
INSTALL_DIR="${REPLICATA_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
  echo "replicata-install: $*" >&2
  exit 1
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os-$arch" in
    Darwin-arm64 | Darwin-aarch64) echo "darwin-arm64" ;;
    Linux-x86_64 | Linux-amd64) echo "linux-x86_64" ;;
    *) fail "unsupported platform $os-$arch (supported: macOS arm64, Linux x86_64)" ;;
  esac
}

download_url() {
  if [ "$VERSION" = "latest" ]; then
    echo "https://github.com/$REPO/releases/latest/download/replicata-$1.tar.gz"
  else
    echo "https://github.com/$REPO/releases/download/$VERSION/replicata-$1.tar.gz"
  fi
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

TARGET="${REPLICATA_TARGET:-$(detect_target)}"
URL="$(download_url "$TARGET")"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT INT TERM

echo "Downloading $URL"
curl -fsSL "$URL" -o "$TMP/replicata.tar.gz" || fail "download failed ($URL)"
tar -xzf "$TMP/replicata.tar.gz" -C "$TMP"
BIN="$(find "$TMP" -type f -name replicata | head -n 1)"
[ -n "$BIN" ] || fail "archive does not contain a 'replicata' binary"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$BIN" "$INSTALL_DIR/replicata" 2>/dev/null || {
  cp "$BIN" "$INSTALL_DIR/replicata"
  chmod 0755 "$INSTALL_DIR/replicata"
}

echo "Installed $INSTALL_DIR/replicata"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo "Add it to your PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
    # Make `install.sh | sh && replicata ...` work in this same shell invocation is not
    # possible from a child process; link into a PATH dir if one is writable.
    for dir in /usr/local/bin /opt/homebrew/bin; do
      if [ -d "$dir" ] && [ -w "$dir" ]; then
        ln -sf "$INSTALL_DIR/replicata" "$dir/replicata" && echo "Linked $dir/replicata" && break
      fi
    done
    ;;
esac
command -v docker >/dev/null 2>&1 || echo "Note: the worker needs Docker to run experiments."
