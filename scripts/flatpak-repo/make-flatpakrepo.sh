#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  make-flatpakrepo.sh --base-url <repo_url> [--name <remote_name>] [--out <file>]

Example:
  make-flatpakrepo.sh \
    --base-url "https://emviolet.codeberg.page/Cardthropic-flatpak/" \
    --name "cardthropic" \
    --out "./cardthropic.flatpakrepo"
EOF
}

BASE_URL=""
NAME="cardthropic"
OUT_FILE="./cardthropic.flatpakrepo"
TITLE="Cardthropic Flatpak Repository"
COMMENT="Official Cardthropic Flatpak repository"
DESCRIPTION="Install Cardthropic with full AppStream metadata (license, screenshots, updates)."
HOMEPAGE="https://codeberg.org/emviolet/Cardthropic"
ICON_URL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url) BASE_URL="${2:-}"; shift 2 ;;
    --name) NAME="${2:-}"; shift 2 ;;
    --out) OUT_FILE="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "${BASE_URL}" ]]; then
  echo "Missing --base-url" >&2
  usage
  exit 1
fi

BASE_URL="${BASE_URL%/}/"
ICON_URL="${BASE_URL}icons/128x128/io.codeberg.emviolet.cardthropic.png"

cat > "${OUT_FILE}" <<EOF
[Flatpak Repo]
Title=${TITLE}
Comment=${COMMENT}
Description=${DESCRIPTION}
Homepage=${HOMEPAGE}
Icon=${ICON_URL}
Url=${BASE_URL}
DefaultBranch=master
NoEnumerate=false
EOF

echo "Generated ${OUT_FILE}"
