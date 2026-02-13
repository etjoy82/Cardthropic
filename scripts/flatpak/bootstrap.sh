#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"

if ! command -v flatpak >/dev/null 2>&1; then
  echo "flatpak is required but not installed."
  echo "Install it first, then rerun this script."
  exit 1
fi

if ! command -v flatpak-builder >/dev/null 2>&1; then
  echo "flatpak-builder is required but not installed."
  echo "Install flatpak-builder using your distro package manager, then rerun."
  exit 1
fi

if [[ ! -f "${MANIFEST}" ]]; then
  echo "Manifest not found: ${MANIFEST}"
  exit 1
fi

echo "Adding Flathub remote (if missing)..."
flatpak remote-add --if-not-exists \
  flathub \
  https://flathub.org/repo/flathub.flatpakrepo

echo "Installing GNOME 48 build dependencies (user scope)..."
flatpak install -y --user flathub \
  org.gnome.Platform//48 \
  org.gnome.Sdk//48 \
  org.freedesktop.Sdk.Extension.rust-stable//24.08

echo "Bootstrap complete."
