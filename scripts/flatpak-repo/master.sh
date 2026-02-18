#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Cardthropic Flatpak Repo Master Script

Runs the full publish flow:
  1) Build Flatpak payload (scripts/flatpak/bundle.sh)
  2) Refresh AppStream refs offline (preserve screenshots/URLs in no-net environments)
  3) Init/update Codeberg Pages checkout
  4) Publish build-repo -> pages
  5) Generate .flatpakrepo descriptor
  6) (Optional) add/update local test remote and install app
  7) Verify appstream metadata in build-repo

Usage:
  scripts/flatpak-repo/master.sh [options]

Options:
  --repo-url <url>        Codeberg git URL for Flatpak hosting repo
                          default: https://codeberg.org/emviolet/Cardthropic-flatpak.git
  --base-url <url>        Pages URL for Flatpak hosting repo
                          default: https://emviolet.codeberg.page/Cardthropic-flatpak/
  --checkout-dir <dir>    Local checkout for hosting repo
                          default: $HOME/Projects/Cardthropic-flatpak
  --remote <name>         Flatpak remote name for local test install
                          default: cardthropic
  --out <path>            Output path for .flatpakrepo descriptor
                          default: <cardthropic-root>/cardthropic.flatpakrepo
  --test-remote-no-gpg-verify
                          For local testing only: disable GPG verification when
                          adding the test remote.
  --skip-test-remote      Skip local remote add/install test
  --skip-bundle           Skip scripts/flatpak/bundle.sh (reuse existing build-repo)
  --dry-run               Print commands without executing publish actions
  -h, --help              Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_URL="https://codeberg.org/emviolet/Cardthropic-flatpak.git"
BASE_URL="https://emviolet.codeberg.page/Cardthropic-flatpak/"
CHECKOUT_DIR="${HOME}/Projects/Cardthropic-flatpak"
REMOTE_NAME="cardthropic"
FLATPAKREPO_OUT="${ROOT_DIR}/cardthropic.flatpakrepo"
SKIP_TEST_REMOTE=0
SKIP_BUNDLE=0
DRY_RUN=0
TEST_REMOTE_NO_GPG_VERIFY=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

run() {
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "DRY-RUN: $*"
  else
    "$@"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-url)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --repo-url" >&2
        exit 2
      fi
      REPO_URL="${2:-}"
      shift 2
      ;;
    --base-url)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --base-url" >&2
        exit 2
      fi
      BASE_URL="${2:-}"
      shift 2
      ;;
    --checkout-dir)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --checkout-dir" >&2
        exit 2
      fi
      CHECKOUT_DIR="${2:-}"
      shift 2
      ;;
    --remote)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --remote" >&2
        exit 2
      fi
      REMOTE_NAME="${2:-}"
      shift 2
      ;;
    --out)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --out" >&2
        exit 2
      fi
      FLATPAKREPO_OUT="${2:-}"
      shift 2
      ;;
    --skip-test-remote)
      SKIP_TEST_REMOTE=1
      shift
      ;;
    --test-remote-no-gpg-verify)
      TEST_REMOTE_NO_GPG_VERIFY=1
      shift
      ;;
    --skip-bundle)
      SKIP_BUNDLE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
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

require_cmd git
if [[ "${SKIP_BUNDLE}" -eq 0 ]]; then
  require_cmd flatpak
  require_cmd flatpak-builder
fi
if [[ "${SKIP_TEST_REMOTE}" -eq 0 ]]; then
  require_cmd flatpak
fi
require_cmd ostree
require_cmd appstreamcli

publish_args=(
  --checkout-dir "${CHECKOUT_DIR}"
  --source-repo "${ROOT_DIR}/build-repo"
)
if [[ "${DRY_RUN}" -eq 1 ]]; then
  publish_args+=(--dry-run)
fi

echo "== Cardthropic Flatpak Repo Publish =="
echo "Repo URL:      ${REPO_URL}"
echo "Base URL:      ${BASE_URL}"
echo "Checkout dir:  ${CHECKOUT_DIR}"
echo "Remote name:   ${REMOTE_NAME}"
echo "Descriptor:    ${FLATPAKREPO_OUT}"
echo "Dry run:       $([[ "${DRY_RUN}" -eq 1 ]] && echo yes || echo no)"
echo

if [[ "${SKIP_BUNDLE}" -eq 0 ]]; then
  echo "[1/7] Building Flatpak payload..."
  run "${ROOT_DIR}/scripts/flatpak/bundle.sh"
else
  echo "[1/7] Skipped Flatpak payload build."
fi

echo "[2/7] Refreshing AppStream refs (offline compose)..."
run "${ROOT_DIR}/scripts/flatpak-repo/refresh-appstream-offline.sh" \
  --repo "${ROOT_DIR}/build-repo" \
  --base-url "${BASE_URL}"

echo "[3/7] Initializing Codeberg Pages checkout..."
run "${ROOT_DIR}/scripts/flatpak-repo/init-codeberg-pages.sh" \
  --repo-url "${REPO_URL}" \
  --checkout-dir "${CHECKOUT_DIR}"

echo "[4/7] Publishing build-repo to Pages checkout..."
run "${ROOT_DIR}/scripts/flatpak-repo/publish-codeberg-pages.sh" \
  "${publish_args[@]}"

echo "[5/7] Generating .flatpakrepo descriptor..."
run "${ROOT_DIR}/scripts/flatpak-repo/make-flatpakrepo.sh" \
  --base-url "${BASE_URL}" \
  --out "${FLATPAKREPO_OUT}"

if [[ "${SKIP_TEST_REMOTE}" -eq 0 ]]; then
  echo "[6/7] Adding/updating local test remote and installing app..."
  test_remote_args=(
    --replace
    --remote "${REMOTE_NAME}"
    --url "${BASE_URL}"
  )
  if [[ "${TEST_REMOTE_NO_GPG_VERIFY}" -eq 1 ]]; then
    test_remote_args+=(--no-gpg-verify)
  fi
  run "${ROOT_DIR}/scripts/flatpak-repo/add-test-remote.sh" \
    "${test_remote_args[@]}"
else
  echo "[6/7] Skipped local test remote."
fi

echo "[7/7] Verifying AppStream metadata in build-repo..."
run "${ROOT_DIR}/scripts/flatpak-repo/verify-appstream.sh" \
  --repo "${ROOT_DIR}/build-repo"

echo
echo "Done."
echo "Published Flatpak repo: ${BASE_URL}"
echo "Descriptor file: ${FLATPAKREPO_OUT}"
