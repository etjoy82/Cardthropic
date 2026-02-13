#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUNDLE_PATH="${ROOT_DIR}/cardthropic.flatpak"

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

if [[ ! -f "${BUNDLE_PATH}" ]]; then
  echo "Bundle not found: ${BUNDLE_PATH}"
  echo "Build it first with: scripts/flatpak/bundle.sh"
  exit 1
fi

require_cmd flatpak

echo "Ensuring Flathub remote exists (runtime dependency source)..."
flatpak remote-add --if-not-exists \
  flathub \
  https://flathub.org/repo/flathub.flatpakrepo

echo "Installing bundle..."
flatpak install -y --user "${BUNDLE_PATH}"

echo "Installed. Run with:"
echo "  flatpak run io.codeberg.emviolet.cardthropic"
