#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"
BUILD_DIR="${ROOT_DIR}/build-dir"
REPO_DIR="${ROOT_DIR}/build-repo"
BUNDLE_PATH="${ROOT_DIR}/cardthropic.flatpak"
APP_ID="io.codeberg.emviolet.cardthropic"
BRANCH="master"

if [[ ! -f "${MANIFEST}" ]]; then
  echo "Manifest not found: ${MANIFEST}"
  exit 1
fi

echo "Building local Flatpak repo..."
flatpak-builder \
  --repo="${REPO_DIR}" \
  --force-clean \
  --install-deps-from=flathub \
  "${BUILD_DIR}" \
  "${MANIFEST}"

echo "Refreshing repo metadata (summary + appstream)..."
flatpak build-update-repo "${REPO_DIR}"

echo "Creating bundle: ${BUNDLE_PATH}"
flatpak build-bundle \
  --runtime-repo="https://flathub.org/repo/flathub.flatpakrepo" \
  "${REPO_DIR}" \
  "${BUNDLE_PATH}" \
  "${APP_ID}" \
  "${BRANCH}"

echo "Bundle created at: ${BUNDLE_PATH}"
