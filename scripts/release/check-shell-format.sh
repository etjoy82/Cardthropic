#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/release/check-shell-format.sh [--write]

Behavior:
  - Enforces shell formatting with shfmt
  - Style: indent=2, switch-case indent enabled

Options:
  --write    Rewrite files in place instead of diff-check only
  -h, --help Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

WRITE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --write)
      WRITE=1
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

if ! command -v shfmt >/dev/null 2>&1; then
  echo "shfmt is required but not installed." >&2
  exit 1
fi

mapfile -t sh_files < <(scripts/release/list-shell-scripts.sh)
if [[ ${#sh_files[@]} -eq 0 ]]; then
  echo "No shell scripts found."
  exit 0
fi

if [[ "${WRITE}" -eq 1 ]]; then
  shfmt -w -i 2 -ci "${sh_files[@]}"
  echo "shfmt rewrite complete."
else
  shfmt -d -i 2 -ci "${sh_files[@]}"
  echo "shfmt check passed."
fi
