#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

require_cmd cargo
require_cmd sha256sum
require_cmd ostree
require_cmd gzip

echo "==> [1/5] Maintainer quality gate"
scripts/release/maintainer-gate.sh

echo "==> [2/5] Building + installing Flatpak"
scripts/flatpak/build-install.sh

echo "==> [3/5] Building distributable bundle"
scripts/flatpak/bundle.sh

echo "==> [4/5] Verifying AppStream metadata from built repo"
scripts/flatpak-repo/verify-appstream.sh --repo "${ROOT_DIR}/build-repo"

echo "==> [5/5] Writing SHA256SUMS"
sha256sum cardthropic.flatpak > SHA256SUMS

echo
echo "Release artifacts ready:"
echo "  - cardthropic.flatpak"
echo "  - SHA256SUMS"
