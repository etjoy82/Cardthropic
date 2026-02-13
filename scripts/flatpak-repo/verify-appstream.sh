#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  verify-appstream.sh [--repo <ostree_repo_dir>]

Default:
  --repo <cardthropic-root>/build-repo
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_DIR="${ROOT_DIR}/build-repo"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) REPO_DIR="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ ! -d "${REPO_DIR}" ]]; then
  echo "Repo not found: ${REPO_DIR}" >&2
  exit 1
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

if ! ostree --repo="${REPO_DIR}" refs | "${SEARCH_BIN}" "${SEARCH_QUIET_OPTS[@]}" '^appstream/x86_64$'; then
  echo "appstream/x86_64 ref not found in ${REPO_DIR}" >&2
  exit 1
fi

echo "Inspecting appstream metadata in ${REPO_DIR}..."
ostree --repo="${REPO_DIR}" cat appstream/x86_64 /appstream.xml.gz \
  | gzip -dc \
  | "${SEARCH_BIN}" "${SEARCH_OPTS[@]}" "id>|project_license|screenshot|image|release version" \
  || true

echo
echo "Expected:"
echo "- project_license should be present"
echo "- screenshot/image URL should be present"
