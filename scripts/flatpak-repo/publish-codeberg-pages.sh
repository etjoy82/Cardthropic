#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  publish-codeberg-pages.sh [--checkout-dir <dir>] [--source-repo <dir>] [--branch <name>]

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

while [[ $# -gt 0 ]]; do
  case "$1" in
    --checkout-dir) CHECKOUT_DIR="${2:-}"; shift 2 ;;
    --source-repo) SOURCE_REPO="${2:-}"; shift 2 ;;
    --branch) BRANCH="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

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

cd "${CHECKOUT_DIR}"
git switch "${BRANCH}"

echo "Syncing ${SOURCE_REPO} -> ${CHECKOUT_DIR}"
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
