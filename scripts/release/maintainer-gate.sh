#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/release/maintainer-gate.sh [--skip-cargo] [--strict-tools]

Behavior:
  - Enforces shell script standards checks
  - Enforces release consistency checks
  - Runs optional shellcheck lint (if installed, or required via --strict-tools)
  - Runs cargo fmt/check/test unless --skip-cargo is provided

Options:
  --skip-cargo    Skip cargo fmt/check/test
  --strict-tools  Require shellcheck to be installed
  -h, --help      Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

SKIP_CARGO=0
STRICT_TOOLS=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-cargo) SKIP_CARGO=1; shift ;;
    --strict-tools) STRICT_TOOLS=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

require_cmd bash
require_cmd find
require_cmd rg

echo "[gate 1/4] Shell script policy checks"
scripts/release/check-shell-scripts.sh

echo "[gate 2/4] Release consistency checks"
scripts/release/check-release-consistency.sh

echo "[gate 3/4] Shell lint (shellcheck)"
if command -v shellcheck >/dev/null 2>&1; then
  mapfile -t sh_files < <(find scripts -maxdepth 3 -type f -name '*.sh' | sort)
  shellcheck --rcfile .shellcheckrc "${sh_files[@]}"
  echo "shellcheck passed."
else
  if [[ "${STRICT_TOOLS}" -eq 1 ]]; then
    echo "shellcheck is required (--strict-tools) but not installed." >&2
    exit 1
  fi
  echo "shellcheck not installed; skipping (non-strict mode)."
fi

echo "[gate 4/4] Cargo validation"
if [[ "${SKIP_CARGO}" -eq 1 ]]; then
  echo "Cargo checks skipped (--skip-cargo)."
else
  require_cmd cargo
  cargo fmt --check
  cargo check
  cargo test -q
fi

echo "Maintainer gate passed."
