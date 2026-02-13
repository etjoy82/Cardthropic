#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  add-test-remote.sh --url <repo_url> [--remote <name>] [--app-id <id>]

Example:
  add-test-remote.sh \
    --remote cardthropic \
    --url "https://emviolet.codeberg.page/Cardthropic-flatpak/"
EOF
}

REMOTE="cardthropic"
URL=""
APP_ID="io.codeberg.emviolet.cardthropic"

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --remote) REMOTE="${2:-}"; shift 2 ;;
    --url) URL="${2:-}"; shift 2 ;;
    --app-id) APP_ID="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "${URL}" ]]; then
  echo "Missing --url" >&2
  usage
  exit 1
fi

URL="${URL%/}/"
require_cmd flatpak

flatpak remote-delete --user "${REMOTE}" >/dev/null 2>&1 || true
flatpak remote-add --if-not-exists --user --no-gpg-verify "${REMOTE}" "${URL}"
flatpak update --user --appstream "${REMOTE}" -y
flatpak install --user -y "${REMOTE}" "${APP_ID}"

echo "Installed ${APP_ID} from remote ${REMOTE}"
echo "Restart GNOME Software if metadata is stale:"
echo "  gnome-software --quit"
