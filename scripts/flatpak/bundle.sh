#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"
FLATPAK_REPO_FILE="${ROOT_DIR}/cardthropic.flatpakrepo"
BUILD_DIR="${ROOT_DIR}/build-dir"
REPO_DIR="${ROOT_DIR}/build-repo"
BUNDLE_PATH="${ROOT_DIR}/cardthropic.flatpak"
APP_ID=""
BRANCH=""
REPO_URL=""

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

if [[ -z "${REPO_URL}" && -f "${FLATPAK_REPO_FILE}" ]]; then
  REPO_URL="$(sed -n 's/^Url=//p' "${FLATPAK_REPO_FILE}" | head -n1)"
fi
if [[ -z "${REPO_URL}" ]]; then
  REPO_URL="https://emviolet.codeberg.page/Cardthropic-flatpak/"
fi
REPO_URL="${REPO_URL%/}/"

echo "Building local Flatpak repo..."
flatpak-builder \
  --repo="${REPO_DIR}" \
  --force-clean \
  --install-deps-from=flathub \
  "${BUILD_DIR}" \
  "${MANIFEST}"

echo "Refreshing repo metadata (summary + appstream)..."
flatpak build-update-repo "${REPO_DIR}"

echo "Refreshing AppStream refs (offline compose --no-net)..."
"${ROOT_DIR}/scripts/flatpak-repo/refresh-appstream-offline.sh" \
  --repo "${REPO_DIR}" \
  --base-url "${REPO_URL}"

echo "Verifying AppStream metadata in repo..."
"${ROOT_DIR}/scripts/flatpak-repo/verify-appstream.sh" --repo "${REPO_DIR}"

echo "Creating bundle: ${BUNDLE_PATH}"
flatpak build-bundle \
  --repo-url="${REPO_URL}" \
  --runtime-repo="https://flathub.org/repo/flathub.flatpakrepo" \
  "${REPO_DIR}" \
  "${BUNDLE_PATH}" \
  "${APP_ID}" \
  "${BRANCH}"

echo "Bundle created at: ${BUNDLE_PATH}"
