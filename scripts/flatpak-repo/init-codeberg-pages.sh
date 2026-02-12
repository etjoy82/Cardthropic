#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  init-codeberg-pages.sh --repo-url <git_url> [--checkout-dir <dir>] [--branch <name>]

Examples:
  init-codeberg-pages.sh --repo-url "https://codeberg.org/emviolet/Cardthropic-flatpak.git"
  init-codeberg-pages.sh --repo-url "https://codeberg.org/emviolet/Cardthropic-flatpak.git" \
    --checkout-dir "$HOME/Projects/Cardthropic-flatpak" --branch pages
EOF
}

REPO_URL=""
CHECKOUT_DIR="${HOME}/Projects/Cardthropic-flatpak"
BRANCH="pages"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-url) REPO_URL="${2:-}"; shift 2 ;;
    --checkout-dir) CHECKOUT_DIR="${2:-}"; shift 2 ;;
    --branch) BRANCH="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "${REPO_URL}" ]]; then
  echo "Missing --repo-url" >&2
  usage
  exit 1
fi

if [[ ! -d "${CHECKOUT_DIR}/.git" ]]; then
  echo "Cloning ${REPO_URL} -> ${CHECKOUT_DIR}"
  git clone "${REPO_URL}" "${CHECKOUT_DIR}"
fi

cd "${CHECKOUT_DIR}"

if git show-ref --verify --quiet "refs/heads/${BRANCH}"; then
  git switch "${BRANCH}"
else
  if git ls-remote --exit-code --heads origin "${BRANCH}" >/dev/null 2>&1; then
    git fetch origin "${BRANCH}"
    git switch -c "${BRANCH}" "origin/${BRANCH}"
  else
    git switch --orphan "${BRANCH}"
    touch .nojekyll
    git add .nojekyll
    git commit -m "Initialize ${BRANCH} branch for Flatpak Pages hosting"
    git push -u origin "${BRANCH}"
  fi
fi

echo "Ready: ${CHECKOUT_DIR} (branch: ${BRANCH})"
