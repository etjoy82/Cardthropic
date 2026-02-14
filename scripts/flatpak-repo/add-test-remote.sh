#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  add-test-remote.sh --url <repo_url> [--remote <name>] [--app-id <id>] [--replace]

Example:
  add-test-remote.sh \
    --remote cardthropic \
    --url "https://emviolet.codeberg.page/Cardthropic-flatpak/"
EOF
}

REMOTE="cardthropic"
URL=""
APP_ID="io.codeberg.emviolet.cardthropic"
REPLACE=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --remote)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --remote" >&2
        exit 2
      fi
      REMOTE="${2:-}"
      shift 2
      ;;
    --url)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --url" >&2
        exit 2
      fi
      URL="${2:-}"
      shift 2
      ;;
    --app-id)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --app-id" >&2
        exit 2
      fi
      APP_ID="${2:-}"
      shift 2
      ;;
    --replace)
      REPLACE=1
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

if [[ -z "${URL}" ]]; then
  echo "Missing --url" >&2
  usage
  exit 1
fi

URL="${URL%/}/"
require_cmd flatpak

remote_exists=0
while IFS= read -r remote_name; do
  if [[ "${remote_name}" == "${REMOTE}" ]]; then
    remote_exists=1
    break
  fi
done < <(flatpak remotes --user --columns=name)

if [[ "${remote_exists}" -eq 1 && "${REPLACE}" -eq 0 ]]; then
  echo "Remote already exists: ${REMOTE}" >&2
  echo "Use --replace to delete and recreate it." >&2
  exit 1
fi

if [[ "${remote_exists}" -eq 1 ]]; then
  flatpak remote-delete --user "${REMOTE}"
fi

flatpak remote-add --if-not-exists --user --no-gpg-verify "${REMOTE}" "${URL}"
flatpak update --user --appstream "${REMOTE}" -y
flatpak install --user -y "${REMOTE}" "${APP_ID}"

echo "Installed ${APP_ID} from remote ${REMOTE}"
echo "Restart GNOME Software if metadata is stale:"
echo "  gnome-software --quit"
