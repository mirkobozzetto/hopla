#!/bin/sh
# Installs the latest hopla on macOS or Linux:
#   curl -fsSL https://raw.githubusercontent.com/mirkobozzetto/hopla/main/install.sh | sh
set -eu

REPO=mirkobozzetto/hopla
BIN_DIR="${HOPLA_BIN_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
  Darwin) os=apple-darwin ;;
  Linux) os=unknown-linux-gnu ;;
  *)
    echo "On Windows, in PowerShell:" >&2
    echo "  irm https://raw.githubusercontent.com/$REPO/main/install.ps1 | iex" >&2
    exit 1 ;;
esac
case "$(uname -m)" in
  arm64 | aarch64) arch=aarch64 ;;
  x86_64 | amd64) arch=x86_64 ;;
  *) echo "no hopla build for $(uname -m) yet" >&2; exit 1 ;;
esac

name="hopla-$arch-$os"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "https://github.com/$REPO/releases/latest/download/$name.tar.gz" | tar xz -C "$tmp"
mkdir -p "$BIN_DIR"
install -m 755 "$tmp/$name/hopla" "$BIN_DIR/hopla"

echo "hopla is in $BIN_DIR. Run: hopla"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "Add $BIN_DIR to your PATH first." ;;
esac
