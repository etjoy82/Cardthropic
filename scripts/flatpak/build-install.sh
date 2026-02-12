#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"
BUILD_DIR="${ROOT_DIR}/build-dir"

if [[ ! -f "${MANIFEST}" ]]; then
  echo "Manifest not found: ${MANIFEST}"
  exit 1
fi

echo "Building and installing Cardthropic Flatpak (user scope)..."
flatpak-builder \
  --user \
  --install \
  --force-clean \
  --install-deps-from=flathub \
  "${BUILD_DIR}" \
  "${MANIFEST}"

echo "Installed: io.codeberg.emviolet.cardthropic"
echo "Run with: scripts/flatpak/run.sh"

