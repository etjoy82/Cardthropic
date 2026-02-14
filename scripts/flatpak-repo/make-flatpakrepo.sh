#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  make-flatpakrepo.sh --base-url <repo_url> [--out <file>]

Example:
  make-flatpakrepo.sh \
    --base-url "https://emviolet.codeberg.page/Cardthropic-flatpak/" \
    --out "./cardthropic.flatpakrepo"
EOF
}

BASE_URL=""
OUT_FILE="./cardthropic.flatpakrepo"
TITLE="Cardthropic Flatpak Repository"
COMMENT="Official Cardthropic Flatpak repository"
DESCRIPTION="Install Cardthropic with full AppStream metadata (license, screenshots, updates)."
HOMEPAGE="https://codeberg.org/emviolet/Cardthropic"
ICON_URL=""
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"
APP_ID=""
DEFAULT_BRANCH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --base-url" >&2
        exit 2
      fi
      BASE_URL="${2:-}"
      shift 2
      ;;
    --out)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --out" >&2
        exit 2
      fi
      OUT_FILE="${2:-}"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "${BASE_URL}" ]]; then
  echo "Missing --base-url" >&2
  usage
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required but not installed." >&2
  exit 1
fi
if [[ ! -f "${MANIFEST}" ]]; then
  echo "Manifest not found: ${MANIFEST}" >&2
  exit 1
fi

APP_ID="$(jq -r '.id // empty' "${MANIFEST}")"
if [[ -z "${APP_ID}" ]]; then
  echo "Manifest is missing required .id: ${MANIFEST}" >&2
  exit 1
fi
DEFAULT_BRANCH="$(jq -r '.branch // "master"' "${MANIFEST}")"

BASE_URL="${BASE_URL%/}/"
ICON_URL="${BASE_URL}icons/128x128/${APP_ID}.png"

cat >"${OUT_FILE}" <<EOF
[Flatpak Repo]
Title=${TITLE}
Comment=${COMMENT}
Description=${DESCRIPTION}
Homepage=${HOMEPAGE}
Icon=${ICON_URL}
Url=${BASE_URL}
DefaultBranch=${DEFAULT_BRANCH}
NoEnumerate=false
EOF

echo "Generated ${OUT_FILE}"
