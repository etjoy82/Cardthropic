#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  refresh-appstream-offline.sh [--repo <ostree_repo_dir>] [--arch <flatpak_arch>] [--origin <name>] [--base-url <url>]

Defaults:
  --repo     <cardthropic-root>/build-repo
  --origin   flatpak
  --base-url https://emviolet.codeberg.page/Cardthropic-flatpak/
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_DIR="${ROOT_DIR}/build-repo"
MANIFEST="${ROOT_DIR}/io.codeberg.emviolet.cardthropic.json"
ARCH=""
ORIGIN="flatpak"
BASE_URL="https://emviolet.codeberg.page/Cardthropic-flatpak/"

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --repo" >&2
        exit 2
      fi
      REPO_DIR="${2:-}"
      shift 2
      ;;
    --arch)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --arch" >&2
        exit 2
      fi
      ARCH="${2:-}"
      shift 2
      ;;
    --origin)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --origin" >&2
        exit 2
      fi
      ORIGIN="${2:-}"
      shift 2
      ;;
    --base-url)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --base-url" >&2
        exit 2
      fi
      BASE_URL="${2:-}"
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

if [[ ! -d "${REPO_DIR}" ]]; then
  echo "Repo not found: ${REPO_DIR}" >&2
  exit 1
fi
if [[ ! -f "${MANIFEST}" ]]; then
  echo "Manifest not found: ${MANIFEST}" >&2
  exit 1
fi

require_cmd jq
require_cmd ostree
require_cmd appstreamcli
require_cmd flatpak

if [[ -z "${ARCH}" ]]; then
  case "$(uname -m)" in
    x86_64 | amd64) ARCH="x86_64" ;;
    aarch64 | arm64) ARCH="aarch64" ;;
    *) ARCH="$(uname -m)" ;;
  esac
fi

APP_ID="$(jq -r '.id // empty' "${MANIFEST}")"
BRANCH="$(jq -r '.branch // "master"' "${MANIFEST}")"
if [[ -z "${APP_ID}" ]]; then
  echo "Manifest is missing required .id: ${MANIFEST}" >&2
  exit 1
fi

APP_REF="app/${APP_ID}/${ARCH}/${BRANCH}"
if ! ostree --repo="${REPO_DIR}" refs | grep -q "^${APP_REF}$"; then
  echo "Missing app ref in repo: ${APP_REF}" >&2
  exit 1
fi

BASE_URL="${BASE_URL%/}/"

tmp="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp}"
}
trap cleanup EXIT

echo "Checking out ${APP_REF} (user mode)..."
ostree --repo="${REPO_DIR}" checkout -U "${APP_REF}" "${tmp}/app"

if [[ ! -d "${tmp}/app/export" ]]; then
  echo "Expected export tree missing in checkout: ${tmp}/app/export" >&2
  exit 1
fi

echo "Composing AppStream metadata offline (--no-net)..."
mkdir -p "${tmp}/out" "${tmp}/media" "${tmp}/stage"
appstreamcli compose \
  --no-net \
  --prefix=/ \
  --origin="${ORIGIN}" \
  --result-root="${tmp}/out" \
  --media-dir="${tmp}/media" \
  --media-baseurl="${BASE_URL}media/" \
  --no-partial-urls \
  "${tmp}/app/export"

catalog_xml="${tmp}/out/share/swcatalog/xml/${ORIGIN}.xml.gz"
if [[ ! -f "${catalog_xml}" ]]; then
  catalog_xml="$(find "${tmp}/out/share/swcatalog/xml" -maxdepth 1 -name '*.xml.gz' | head -n1 || true)"
fi
if [[ -z "${catalog_xml}" || ! -f "${catalog_xml}" ]]; then
  echo "Composed catalog XML not found under ${tmp}/out/share/swcatalog/xml" >&2
  exit 1
fi

cp "${catalog_xml}" "${tmp}/stage/appstream.xml.gz"
if [[ -d "${tmp}/out/share/swcatalog/icons" ]]; then
  cp -a "${tmp}/out/share/swcatalog/icons" "${tmp}/stage/icons"
else
  mkdir -p "${tmp}/stage/icons"
fi

echo "Committing refreshed appstream refs..."
ostree --repo="${REPO_DIR}" commit \
  --branch="appstream/${ARCH}" \
  --tree=dir="${tmp}/stage" \
  --subject="Refresh appstream metadata (offline compose --no-net)"

ostree --repo="${REPO_DIR}" commit \
  --branch="appstream2/${ARCH}" \
  --tree=dir="${tmp}/stage" \
  --subject="Refresh appstream2 metadata (offline compose --no-net)"

echo "Updating repo summary without regenerating appstream refs..."
flatpak build-update-repo --no-update-appstream "${REPO_DIR}"

echo "AppStream refresh complete for ${APP_REF}"
