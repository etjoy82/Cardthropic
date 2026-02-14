#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/release/test-shell.sh [--strict-tools]

Behavior:
  - Runs shell tests under scripts/tests/*.bats
  - Skips when no tests are present
  - In strict mode, requires bats when tests are present

Options:
  --strict-tools  Require bats to be installed when tests are present
  -h, --help      Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

STRICT_TOOLS=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --strict-tools)
      STRICT_TOOLS=1
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

mapfile -t bats_files < <(find scripts/tests -type f -name '*.bats' 2>/dev/null | sort || true)
if [[ ${#bats_files[@]} -eq 0 ]]; then
  echo "No shell tests found."
  exit 0
fi

if ! command -v bats >/dev/null 2>&1; then
  if [[ "${STRICT_TOOLS}" -eq 1 ]]; then
    echo "bats is required (--strict-tools) but not installed." >&2
    exit 1
  fi
  echo "bats not installed; skipping shell tests (non-strict mode)."
  exit 0
fi

bats "${bats_files[@]}"
echo "Shell tests passed."
