#!/usr/bin/env bash
set -euo pipefail

REPO="iredox/tbook"
ASSET="tbook-linux-x86_64.tar.gz"
PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

mkdir -p "$BIN_DIR"

echo "Downloading tbook..."
curl -fsSL "https://github.com/${REPO}/releases/latest/download/${ASSET}" -o "$TMP_DIR/${ASSET}"
tar -xzf "$TMP_DIR/${ASSET}" -C "$TMP_DIR"

if [ -f "$TMP_DIR/tbook" ]; then
  install -m755 "$TMP_DIR/tbook" "$BIN_DIR/tbook"
elif [ -f "$TMP_DIR/bin/tbook" ]; then
  install -m755 "$TMP_DIR/bin/tbook" "$BIN_DIR/tbook"
else
  echo "tbook binary not found in release archive" >&2
  exit 1
fi

for bin in pdftotext pdftoppm; do
  if [ -f "$TMP_DIR/$bin" ]; then
    install -m755 "$TMP_DIR/$bin" "$BIN_DIR/$bin"
  elif [ -f "$TMP_DIR/bin/$bin" ]; then
    install -m755 "$TMP_DIR/bin/$bin" "$BIN_DIR/$bin"
  fi
done

if ! command -v pdftotext >/dev/null 2>&1 || ! command -v pdftoppm >/dev/null 2>&1; then
  echo "poppler-utils not found; installing via system package manager..."
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y poppler-utils
  elif command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y poppler-utils
  elif command -v pacman >/dev/null 2>&1; then
    sudo pacman -Sy --noconfirm poppler
  elif command -v zypper >/dev/null 2>&1; then
    sudo zypper install -y poppler-tools
  else
    echo "Please install poppler-utils manually to enable PDF support." >&2
  fi
fi

echo "Installed to ${BIN_DIR}. Ensure it is in your PATH."
