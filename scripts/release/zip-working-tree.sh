#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage: scripts/release/zip-working-tree.sh [--repo PATH] [--output-dir DIR] [--name NAME]

Create a zip archive of the current working tree using Git file selection:
  - includes tracked files
  - includes untracked files
  - excludes ignored files per .gitignore / .git/info/exclude / global excludes

Options:
  --repo PATH        Repository path to archive (default: current git repo)
  --output-dir DIR   Directory for the output zip (default: .)
  --name NAME        Base archive name without .zip
                     (default: <repo>-<YYYY-MM-DD-HH-MM-SS-AM/PM>)
  -h, --help         Show this help
EOF
}

REPO_PATH=""
OUTPUT_DIR="${HOME}/Backups"
NAME=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO_PATH="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --name)
      NAME="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v git >/dev/null 2>&1; then
  echo "ERROR: git is required" >&2
  exit 1
fi
if ! command -v zip >/dev/null 2>&1; then
  echo "ERROR: zip is required" >&2
  exit 1
fi

ROOT=""
if [[ -n "$REPO_PATH" ]]; then
  ROOT="$(git -C "$REPO_PATH" rev-parse --show-toplevel 2>/dev/null || true)"
else
  ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
fi
if [[ -z "$ROOT" ]]; then
  echo "ERROR: not inside a git repository" >&2
  exit 1
fi

cd "$ROOT"

if [[ -z "$NAME" ]]; then
  REPO_NAME="$(basename "$ROOT")"
  NAME="${REPO_NAME}-$(date +%Y-%m-%d-%I-%M-%S-%p)"
fi

mkdir -p "$OUTPUT_DIR"

if [[ "$OUTPUT_DIR" = /* ]]; then
  ARCHIVE_PATH="${OUTPUT_DIR%/}/${NAME}.zip"
else
  ARCHIVE_PATH="${ROOT}/${OUTPUT_DIR%/}/${NAME}.zip"
fi

mapfile -d '' -t FILES < <(git ls-files --cached --others --exclude-standard -z)

if [[ ${#FILES[@]} -eq 0 ]]; then
  echo "ERROR: no files selected by git ls-files" >&2
  exit 1
fi

if [[ -e "$ARCHIVE_PATH" ]]; then
  echo "ERROR: archive already exists: $ARCHIVE_PATH" >&2
  exit 1
fi

zip -q "$ARCHIVE_PATH" "${FILES[@]}"

echo "Created: $ARCHIVE_PATH"
echo "Files: ${#FILES[@]}"
