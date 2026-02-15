#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  verify-appstream.sh [--repo <ostree_repo_dir>] [--arch <flatpak_arch>]

Default:
  --repo <cardthropic-root>/build-repo
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_DIR="${ROOT_DIR}/build-repo"
ARCH=""

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

require_cmd ostree
require_cmd gzip

if [[ -z "${ARCH}" ]]; then
  case "$(uname -m)" in
    x86_64 | amd64) ARCH="x86_64" ;;
    aarch64 | arm64) ARCH="aarch64" ;;
    *) ARCH="$(uname -m)" ;;
  esac
fi

if command -v rg >/dev/null 2>&1; then
  SEARCH_BIN="rg"
  SEARCH_OPTS=(-n)
  SEARCH_QUIET_OPTS=(-q)
else
  SEARCH_BIN="grep"
  SEARCH_OPTS=(-nE)
  SEARCH_QUIET_OPTS=(-qE)
fi

APPSTREAM_REF="appstream/${ARCH}"
if ! ostree --repo="${REPO_DIR}" refs | "${SEARCH_BIN}" "${SEARCH_QUIET_OPTS[@]}" "^${APPSTREAM_REF}$"; then
  echo "${APPSTREAM_REF} ref not found in ${REPO_DIR}" >&2
  exit 1
fi

echo "Inspecting appstream metadata (${APPSTREAM_REF}) in ${REPO_DIR}..."
ostree --repo="${REPO_DIR}" cat "${APPSTREAM_REF}" /appstream.xml.gz |
  gzip -dc |
  "${SEARCH_BIN}" "${SEARCH_OPTS[@]}" "id>|project_license|screenshot|image|release version" ||
  true

echo
echo "Expected:"
echo "- project_license should be present"
echo "- screenshot/image URL should be present"
