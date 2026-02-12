#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUNDLE_PATH="${ROOT_DIR}/cardthropic.flatpak"

if [[ ! -f "${BUNDLE_PATH}" ]]; then
  echo "Bundle not found: ${BUNDLE_PATH}"
  echo "Build it first with: scripts/flatpak/bundle.sh"
  exit 1
fi

echo "Ensuring Flathub remote exists (runtime dependency source)..."
flatpak remote-add --if-not-exists \
  flathub \
  https://flathub.org/repo/flathub.flatpakrepo

echo "Installing bundle..."
flatpak install -y --user "${BUNDLE_PATH}"

echo "Installed. Run with:"
echo "  flatpak run io.codeberg.emviolet.cardthropic"

