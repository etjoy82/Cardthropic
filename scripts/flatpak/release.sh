#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

echo "==> [1/6] Formatting"
cargo fmt

echo "==> [2/6] Checking"
cargo check

echo "==> [3/6] Testing"
cargo test -q

echo "==> [4/6] Building + installing Flatpak"
scripts/flatpak/build-install.sh

echo "==> [5/6] Building distributable bundle"
scripts/flatpak/bundle.sh

echo "==> [6/6] Writing SHA256SUMS"
sha256sum cardthropic.flatpak > SHA256SUMS

echo
echo "Release artifacts ready:"
echo "  - cardthropic.flatpak"
echo "  - SHA256SUMS"

