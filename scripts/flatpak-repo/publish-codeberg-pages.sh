#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  publish-codeberg-pages.sh [--checkout-dir <dir>] [--source-repo <dir>] [--branch <name>] [--dry-run]

Defaults:
  --checkout-dir $HOME/Projects/Cardthropic-flatpak
  --source-repo  <cardthropic-root>/build-repo
  --branch       pages
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECKOUT_DIR="${HOME}/Projects/Cardthropic-flatpak"
SOURCE_REPO="${ROOT_DIR}/build-repo"
BRANCH="pages"
DRY_RUN=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

canonical_path() {
  local path="$1"
  if [[ -d "${path}" ]]; then
    (cd "${path}" && pwd -P)
  else
    local base
    base="$(dirname "${path}")"
    local name
    name="$(basename "${path}")"
    (cd "${base}" && printf "%s/%s\n" "$(pwd -P)" "${name}")
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --checkout-dir) CHECKOUT_DIR="${2:-}"; shift 2 ;;
    --source-repo) SOURCE_REPO="${2:-}"; shift 2 ;;
    --branch) BRANCH="${2:-}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

require_cmd git
require_cmd rsync

if [[ ! -d "${SOURCE_REPO}" ]]; then
  echo "Source repo not found: ${SOURCE_REPO}" >&2
  echo "Run scripts/flatpak/bundle.sh first." >&2
  exit 1
fi

if [[ ! -d "${CHECKOUT_DIR}/.git" ]]; then
  echo "Checkout dir is not a git repository: ${CHECKOUT_DIR}" >&2
  echo "Run init-codeberg-pages.sh first." >&2
  exit 1
fi

if [[ "$(canonical_path "${CHECKOUT_DIR}")" == "$(canonical_path "${ROOT_DIR}")" ]]; then
  echo "Refusing to publish into source repository checkout: ${CHECKOUT_DIR}" >&2
  echo "Use a dedicated pages checkout directory." >&2
  exit 1
fi
if [[ "$(canonical_path "${CHECKOUT_DIR}")" == "$(canonical_path "${SOURCE_REPO}")" ]]; then
  echo "Checkout dir and source repo cannot be the same path." >&2
  exit 1
fi

cd "${CHECKOUT_DIR}"
git switch "${BRANCH}"

echo "Syncing ${SOURCE_REPO} -> ${CHECKOUT_DIR}"
if [[ "${DRY_RUN}" -eq 1 ]]; then
  rsync -an --delete --exclude='.git/' "${SOURCE_REPO}/" "${CHECKOUT_DIR}/"
  echo "DRY-RUN: touch .nojekyll"
  echo "DRY-RUN: git add -A"
  echo "DRY-RUN: git commit -m \"Publish Flatpak repo update\""
  echo "DRY-RUN: git push origin ${BRANCH}"
else
  rsync -a --delete --exclude='.git/' "${SOURCE_REPO}/" "${CHECKOUT_DIR}/"
  touch .nojekyll

  git add -A
  if git diff --cached --quiet; then
    echo "No changes to publish."
    exit 0
  fi

  git commit -m "Publish Flatpak repo update"
  git push origin "${BRANCH}"
  echo "Published to origin/${BRANCH}"
fi
