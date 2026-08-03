#!/usr/bin/env bash
# Pull the rolling GitHub Release tag `nightly` for this platform.
# Requires: gh (authenticated), unzip.
set -euo pipefail

REPO="${DPL_NIGHTLY_REPO:-EonsofStupid/devpulselabs}"
DEST_ROOT="${DPL_NIGHTLY_DIR:-${HOME}/Downloads/devpulse-nightly}"
DATE="$(date -u +%Y-%m-%d)"
ARCH="$(uname -m)"
OS="$(uname -s)"

case "${OS}-${ARCH}" in
  Linux-x86_64)
    PATTERN="devpulselabs-linux-x64.zip"
    ;;
  Linux-aarch64|Linux-arm64)
    # Hosted ubuntu-24.04-arm publishes this after the 2026-07-30 estate
    # runners were deregistered (self-hosted ARM asset was retired; hosted
    # ARM restored so aarch64 boxes can pull again).
    PATTERN="devpulselabs-linux-aarch64.zip"
    ;;
  Darwin-*)
    echo "error: macOS nightly not published yet (dpl-mac staged)." >&2
    exit 1
    ;;
  *)
    echo "error: unsupported platform ${OS}-${ARCH}; use scripts/pull-nightly.ps1 on Windows." >&2
    exit 1
    ;;
esac

OUT="${DEST_ROOT}/${DATE}"
mkdir -p "$OUT"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading ${PATTERN} from ${REPO} @ nightly → ${OUT}"
gh release download nightly \
  -R "$REPO" \
  -p "$PATTERN" \
  -D "$TMP" \
  --clobber

unzip -o "$TMP/$PATTERN" -d "$OUT"
echo "Ready: ${OUT}"
ls -la "$OUT"
