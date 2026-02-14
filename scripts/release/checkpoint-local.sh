#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage: scripts/release/checkpoint-local.sh [options]

Create a local-only checkpoint:
  1) stage all changes (`git add -A`)
  2) commit locally if there are staged changes
  3) create a local tag at HEAD
  4) write a working-tree backup zip to ~/Backups (default)

No push is performed.

Options:
  --message TEXT      Commit message override
  --tag NAME          Tag name override
  --backup-name NAME  Backup zip base name override (without .zip)
  --output-dir DIR    Backup output directory (default: ~/Backups)
  --no-tag            Skip tag creation
  --no-backup         Skip backup creation
  -h, --help          Show this help
EOF
}

COMMIT_MESSAGE=""
TAG_NAME=""
BACKUP_NAME=""
OUTPUT_DIR="${HOME}/Backups"
DO_TAG=1
DO_BACKUP=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --message)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --message" >&2
        exit 2
      fi
      COMMIT_MESSAGE="${2:-}"
      shift 2
      ;;
    --tag)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --tag" >&2
        exit 2
      fi
      TAG_NAME="${2:-}"
      shift 2
      ;;
    --backup-name)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --backup-name" >&2
        exit 2
      fi
      BACKUP_NAME="${2:-}"
      shift 2
      ;;
    --output-dir)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --output-dir" >&2
        exit 2
      fi
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --no-tag)
      DO_TAG=0
      shift
      ;;
    --no-backup)
      DO_BACKUP=0
      shift
      ;;
    -h | --help)
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

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${ROOT}" ]]; then
  echo "ERROR: not inside a git repository" >&2
  exit 1
fi

cd "${ROOT}"

NOW_HUMAN="$(date +%Y-%m-%d-%I-%M-%S-%p)"
NOW_TAG="$(date +%Y%m%d-%H%M%S)"
REPO_NAME="$(basename "${ROOT}")"

if [[ -z "${COMMIT_MESSAGE}" ]]; then
  COMMIT_MESSAGE="chore(local): checkpoint ${NOW_HUMAN}"
fi
if [[ -z "${TAG_NAME}" ]]; then
  TAG_NAME="local-checkpoint-${NOW_TAG}"
fi
if [[ -z "${BACKUP_NAME}" ]]; then
  BACKUP_NAME="${REPO_NAME}-${NOW_HUMAN}"
fi

git add -A

COMMITTED=0
if git diff --cached --quiet; then
  echo "No staged changes to commit."
else
  git commit -m "${COMMIT_MESSAGE}"
  COMMITTED=1
fi

if [[ "${DO_TAG}" -eq 1 ]]; then
  if git rev-parse --verify --quiet "refs/tags/${TAG_NAME}" >/dev/null; then
    echo "Tag already exists, skipping: ${TAG_NAME}"
  else
    git tag "${TAG_NAME}"
    echo "Created tag: ${TAG_NAME}"
  fi
else
  echo "Skipping tag creation (--no-tag)."
fi

if [[ "${DO_BACKUP}" -eq 1 ]]; then
  scripts/release/zip-working-tree.sh \
    --repo "${ROOT}" \
    --output-dir "${OUTPUT_DIR}" \
    --name "${BACKUP_NAME}"
else
  echo "Skipping backup creation (--no-backup)."
fi

if [[ "${COMMITTED}" -eq 1 ]]; then
  echo "Checkpoint complete: committed, tagged/backed up per options."
else
  echo "Checkpoint complete: no commit needed, tagged/backed up per options."
fi
