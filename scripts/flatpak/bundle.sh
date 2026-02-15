#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"
BUILD_DIR="${ROOT_DIR}/build-dir"
REPO_DIR="${ROOT_DIR}/build-repo"
BUNDLE_PATH="${ROOT_DIR}/cardthropic.flatpak"
APP_ID=""
BRANCH=""

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

if [[ ! -f "${MANIFEST}" ]]; then
  echo "Manifest not found: ${MANIFEST}"
  exit 1
fi

require_cmd flatpak
require_cmd flatpak-builder
require_cmd jq

APP_ID="$(jq -r '.id // empty' "${MANIFEST}")"
if [[ -z "${APP_ID}" ]]; then
  echo "Manifest is missing required .id: ${MANIFEST}" >&2
  exit 1
fi
BRANCH="$(jq -r '.branch // "master"' "${MANIFEST}")"

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
