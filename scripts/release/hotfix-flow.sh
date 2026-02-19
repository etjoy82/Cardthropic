#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Hotfix Release Flow Helper

This script runs local release sanity checks and prints the exact git/flatpak
commands for a hotfix branch publish flow.

Usage:
  scripts/release/hotfix-flow.sh --version <semver> [--skip-bundle]

Options:
  --version <semver> Required release version (example: 0.3.2 or 0.3.2-beta.1)
  --skip-bundle      Skip scripts/flatpak/bundle.sh + appstream repo verification
  -h, --help         Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION=""
SKIP_BUNDLE=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --version" >&2
        exit 2
      fi
      VERSION="${2:-}"
      shift 2
      ;;
    --skip-bundle)
      SKIP_BUNDLE=1
      shift
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

if [[ -z "${VERSION}" ]]; then
  echo "--version is required." >&2
  usage
  exit 1
fi
semver_re='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
if [[ ! "${VERSION}" =~ ${semver_re} ]]; then
  echo "Version must be SemVer (example: 0.6.0 or 0.6.0-beta.1)." >&2
  exit 1
fi

cd "${ROOT_DIR}"

require_cmd cargo
require_cmd appstreamcli
require_cmd git
if [[ "${SKIP_BUNDLE}" -eq 0 ]]; then
  require_cmd flatpak
  require_cmd flatpak-builder
  require_cmd ostree
  require_cmd gzip
fi

echo "[1/5] Maintainer quality gate..."
scripts/release/maintainer-gate.sh

echo "[2/5] Validating appstream metadata template..."
appstreamcli validate --no-net data/io.codeberg.emviolet.cardthropic.metainfo.xml.in

echo "[3/5] Building flatpak bundle..."
if [[ "${SKIP_BUNDLE}" -eq 0 ]]; then
  scripts/flatpak/bundle.sh
else
  echo "Skipped flatpak bundle build (--skip-bundle)."
fi

echo "[4/5] Verifying appstream metadata from built repo..."
if [[ "${SKIP_BUNDLE}" -eq 0 ]]; then
  scripts/flatpak-repo/verify-appstream.sh --repo "${ROOT_DIR}/build-repo"
else
  echo "Skipped appstream repo verification (--skip-bundle)."
fi

echo "[5/5] Local status summary:"
git status --short

cat <<EOF

Next commands:

  git switch -c ${VERSION}
  git add -A
  git commit -m "release: v${VERSION}"
  git push -u origin ${VERSION}

  git switch main
  git fetch origin
  git pull --rebase origin main
  git merge --no-ff ${VERSION} -m "Merge release ${VERSION}"
  git push origin main

  scripts/flatpak-repo/master.sh --skip-bundle --skip-test-remote

EOF
