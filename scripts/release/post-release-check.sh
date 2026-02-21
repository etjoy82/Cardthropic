#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/release/post-release-check.sh [options]

Behavior:
  - Verifies release version consistency across tracked files
  - Verifies local release tag and commit wiring
  - Verifies Pages site version string in pages/index.html
  - Verifies Codeberg + GitHub main refs include local HEAD/release commit
  - Verifies Codeberg + GitHub release tag points to expected release commit
  - Optionally verifies Cardthropic-flatpak pages checkout sync state

Options:
  --version <semver>        Release version (default: Cargo.toml version)
  --codeberg-remote <name>  Codeberg remote name (default: origin)
  --github-remote <name>    GitHub remote name (default: github)
  --flatpak-checkout <dir>  Cardthropic-flatpak checkout path
                            default: $HOME/Projects/Cardthropic-flatpak
  --skip-fetch              Skip `git fetch` steps
  --skip-flatpak-checkout   Skip Cardthropic-flatpak checkout verification
  --offline                 Skip all network-dependent checks
  -h, --help                Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION=""
CODEBERG_REMOTE="origin"
GITHUB_REMOTE="github"
FLATPAK_CHECKOUT="${HOME}/Projects/Cardthropic-flatpak"
SKIP_FETCH=0
SKIP_FLATPAK_CHECKOUT=0
OFFLINE=0

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

pass() {
  echo "PASS: $*"
  PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
  echo "FAIL: $*" >&2
  FAIL_COUNT=$((FAIL_COUNT + 1))
}

skip() {
  echo "SKIP: $*"
  SKIP_COUNT=$((SKIP_COUNT + 1))
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
    --codeberg-remote)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --codeberg-remote" >&2
        exit 2
      fi
      CODEBERG_REMOTE="${2:-}"
      shift 2
      ;;
    --github-remote)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --github-remote" >&2
        exit 2
      fi
      GITHUB_REMOTE="${2:-}"
      shift 2
      ;;
    --flatpak-checkout)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --flatpak-checkout" >&2
        exit 2
      fi
      FLATPAK_CHECKOUT="${2:-}"
      shift 2
      ;;
    --skip-fetch)
      SKIP_FETCH=1
      shift
      ;;
    --skip-flatpak-checkout)
      SKIP_FLATPAK_CHECKOUT=1
      shift
      ;;
    --offline)
      OFFLINE=1
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
require_cmd sed
require_cmd rg
require_cmd awk
require_cmd mktemp

cd "${ROOT_DIR}"

if [[ -z "${VERSION}" ]]; then
  VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
fi
if [[ -z "${VERSION}" ]]; then
  echo "Unable to infer release version from Cargo.toml; pass --version." >&2
  exit 1
fi
semver_re='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
if [[ ! "${VERSION}" =~ ${semver_re} ]]; then
  echo "Version must be SemVer (example: 0.6.0 or 0.6.0-beta.1)." >&2
  exit 1
fi

TAG="v${VERSION}"
LOCAL_HEAD="$(git rev-parse HEAD)"
RELEASE_COMMIT=""

echo "== Cardthropic Post-Release Verification =="
echo "Version:          ${VERSION}"
echo "Tag:              ${TAG}"
echo "Codeberg remote:  ${CODEBERG_REMOTE}"
echo "GitHub remote:    ${GITHUB_REMOTE}"
echo "Flatpak checkout: ${FLATPAK_CHECKOUT}"
echo "Offline mode:     $([[ "${OFFLINE}" -eq 1 ]] && echo yes || echo no)"
echo

if scripts/release/check-release-consistency.sh >/dev/null 2>&1; then
  pass "release consistency"
else
  fail "release consistency (run scripts/release/check-release-consistency.sh)"
fi

if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null 2>&1; then
  RELEASE_COMMIT="$(git rev-parse "${TAG}^{commit}")"
  pass "local tag exists (${TAG} -> ${RELEASE_COMMIT:0:7})"
else
  fail "local tag ${TAG} missing"
fi

if [[ -n "${RELEASE_COMMIT}" ]]; then
  if git merge-base --is-ancestor "${RELEASE_COMMIT}" "${LOCAL_HEAD}"; then
    pass "local HEAD contains release commit (${RELEASE_COMMIT:0:7})"
  else
    fail "local HEAD does not contain release commit (${RELEASE_COMMIT:0:7})"
  fi
fi

if [[ -f "pages/index.html" ]]; then
  if rg -q "Version:[[:space:]]*${VERSION}\\b" pages/index.html; then
    pass "pages/index.html shows version ${VERSION}"
  else
    fail "pages/index.html does not show version ${VERSION}"
  fi
else
  skip "pages/index.html missing; site version check skipped"
fi

if [[ -f "cardthropic.flatpak" ]]; then
  pass "release artifact exists (cardthropic.flatpak)"
else
  skip "cardthropic.flatpak missing (artifact check)"
fi
if [[ -f "SHA256SUMS" ]]; then
  pass "checksum file exists (SHA256SUMS)"
else
  skip "SHA256SUMS missing (checksum check)"
fi

check_remote_release_state() {
  local remote="$1"
  local label="$2"

  if ! git remote get-url "${remote}" >/dev/null 2>&1; then
    fail "${label}: remote '${remote}' not configured"
    return
  fi

  if [[ "${OFFLINE}" -eq 0 && "${SKIP_FETCH}" -eq 0 ]]; then
    if git fetch "${remote}" >/dev/null 2>&1; then
      pass "${label}: fetch ok"
    else
      fail "${label}: fetch failed"
    fi
  elif [[ "${OFFLINE}" -eq 1 ]]; then
    skip "${label}: fetch skipped (--offline)"
  else
    skip "${label}: fetch skipped (--skip-fetch)"
  fi

  if git show-ref --verify --quiet "refs/remotes/${remote}/main"; then
    if git merge-base --is-ancestor "${LOCAL_HEAD}" "${remote}/main"; then
      pass "${label}: main contains local HEAD (${LOCAL_HEAD:0:7})"
    else
      fail "${label}: main does not contain local HEAD (${LOCAL_HEAD:0:7})"
    fi

    if [[ -n "${RELEASE_COMMIT}" ]]; then
      if git merge-base --is-ancestor "${RELEASE_COMMIT}" "${remote}/main"; then
        pass "${label}: main contains ${TAG} commit (${RELEASE_COMMIT:0:7})"
      else
        fail "${label}: main does not contain ${TAG} commit (${RELEASE_COMMIT:0:7})"
      fi
    fi
  else
    fail "${label}: tracking ref ${remote}/main is missing"
  fi

  if [[ "${OFFLINE}" -eq 1 ]]; then
    skip "${label}: live tag check skipped (--offline)"
    return
  fi

  local tmp_err
  tmp_err="$(mktemp -p . .tmp.post-release-check.XXXXXX)"
  local remote_tag_lines
  if remote_tag_lines="$(
    git ls-remote --tags "${remote}" "refs/tags/${TAG}" "refs/tags/${TAG}^{}" 2>"${tmp_err}"
  )"; then
    local remote_tag_obj
    local remote_tag_peeled
    local remote_tag_commit
    remote_tag_obj="$(
      printf '%s\n' "${remote_tag_lines}" |
        awk '$2=="refs/tags/'"${TAG}"'"{print $1}' |
        head -n1
    )"
    remote_tag_peeled="$(
      printf '%s\n' "${remote_tag_lines}" |
        awk '$2=="refs/tags/'"${TAG}"'^{}"{print $1}' |
        head -n1
    )"
    remote_tag_commit="${remote_tag_peeled:-${remote_tag_obj}}"
    if [[ -z "${remote_tag_commit}" ]]; then
      fail "${label}: tag ${TAG} missing on remote"
    elif [[ -z "${RELEASE_COMMIT}" ]]; then
      pass "${label}: tag ${TAG} exists on remote"
    elif [[ "${remote_tag_commit}" == "${RELEASE_COMMIT}" ]]; then
      pass "${label}: tag ${TAG} points to expected commit (${RELEASE_COMMIT:0:7})"
    else
      fail "${label}: tag ${TAG} points to ${remote_tag_commit:0:7}, expected ${RELEASE_COMMIT:0:7}"
    fi
  else
    local err_line
    err_line="$(head -n1 "${tmp_err}" || true)"
    [[ -z "${err_line}" ]] && err_line="unknown error"
    fail "${label}: unable to query remote tags (${err_line})"
  fi
  rm -f "${tmp_err}"
}

check_flatpak_checkout_sync() {
  if [[ "${SKIP_FLATPAK_CHECKOUT}" -eq 1 ]]; then
    skip "flatpak checkout check skipped (--skip-flatpak-checkout)"
    return
  fi
  if [[ ! -d "${FLATPAK_CHECKOUT}" ]]; then
    skip "flatpak checkout not found at ${FLATPAK_CHECKOUT}"
    return
  fi
  if ! git -C "${FLATPAK_CHECKOUT}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    fail "flatpak checkout path is not a git repo (${FLATPAK_CHECKOUT})"
    return
  fi

  if [[ "${OFFLINE}" -eq 0 && "${SKIP_FETCH}" -eq 0 ]]; then
    if git -C "${FLATPAK_CHECKOUT}" fetch origin >/dev/null 2>&1; then
      pass "flatpak checkout fetch ok"
    else
      fail "flatpak checkout fetch failed"
    fi
  elif [[ "${OFFLINE}" -eq 1 ]]; then
    skip "flatpak checkout fetch skipped (--offline)"
  else
    skip "flatpak checkout fetch skipped (--skip-fetch)"
  fi

  if ! git -C "${FLATPAK_CHECKOUT}" show-ref --verify --quiet refs/heads/pages; then
    fail "flatpak checkout missing local pages branch"
    return
  fi
  if ! git -C "${FLATPAK_CHECKOUT}" show-ref --verify --quiet refs/remotes/origin/pages; then
    fail "flatpak checkout missing tracking ref origin/pages"
    return
  fi

  local counts
  counts="$(git -C "${FLATPAK_CHECKOUT}" rev-list --left-right --count pages...origin/pages)"
  local ahead
  local behind
  ahead="$(printf '%s' "${counts}" | awk '{print $1}')"
  behind="$(printf '%s' "${counts}" | awk '{print $2}')"
  if [[ "${ahead}" == "0" && "${behind}" == "0" ]]; then
    pass "flatpak checkout pages is synced with origin/pages"
  else
    fail "flatpak checkout pages diverged (ahead ${ahead}, behind ${behind})"
  fi

  if [[ -z "$(git -C "${FLATPAK_CHECKOUT}" status --porcelain)" ]]; then
    pass "flatpak checkout working tree clean"
  else
    fail "flatpak checkout has uncommitted changes"
  fi
}

check_remote_release_state "${CODEBERG_REMOTE}" "Codeberg"
check_remote_release_state "${GITHUB_REMOTE}" "GitHub"
check_flatpak_checkout_sync

echo
echo "Summary: ${PASS_COUNT} passed, ${FAIL_COUNT} failed, ${SKIP_COUNT} skipped."
if [[ "${FAIL_COUNT}" -ne 0 ]]; then
  exit 1
fi

echo "Post-release verification passed."
