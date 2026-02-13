#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Hotfix Release Flow Helper

This script runs local release sanity checks and prints the exact git/flatpak
commands for a hotfix branch publish flow.

Usage:
  scripts/release/hotfix-flow.sh --version <x.y.z> [--with-bundle]

Options:
  --version <x.y.z>  Required release version (example: 0.3.2)
  --with-bundle      Also run scripts/flatpak/bundle.sh
  -h, --help         Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION=""
WITH_BUNDLE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="${2:-}"; shift 2 ;;
    --with-bundle) WITH_BUNDLE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "${VERSION}" ]]; then
  echo "--version is required." >&2
  usage
  exit 1
fi

cd "${ROOT_DIR}"

echo "[1/4] Running cargo check..."
cargo check

echo "[2/4] Validating appstream metadata..."
appstreamcli validate --no-net data/io.codeberg.emviolet.cardthropic.metainfo.xml.in

if [[ "${WITH_BUNDLE}" -eq 1 ]]; then
  echo "[3/4] Building flatpak bundle..."
  scripts/flatpak/bundle.sh
else
  echo "[3/4] Skipped flatpak bundle build (use --with-bundle to enable)."
fi

echo "[4/4] Local status summary:"
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
