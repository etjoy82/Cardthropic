#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/release/finalize-release-notes.sh --version <x.y.z> --note "<text>" [--note "<text>" ...] [--dry-run]

Behavior:
  - Replaces the placeholder TODO bullet in CHANGELOG.md for the target version
  - Replaces the AppStream <description> block for the same version with <p> entries

Options:
  --version <x.y.z>  Required release version
  --note "<text>"    Release note text (repeat for multiple items)
  --dry-run          Print planned changes without writing files
  -h, --help         Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION=""
DRY_RUN=0
NOTES=()

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

xml_escape() {
  sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
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
    --note)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --note" >&2
        exit 2
      fi
      NOTES+=("${2:-}")
      shift 2
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

if [[ -z "${VERSION}" ]]; then
  echo "--version is required." >&2
  usage
  exit 1
fi
if [[ "${#NOTES[@]}" -eq 0 ]]; then
  echo "At least one --note is required." >&2
  usage
  exit 1
fi

require_cmd awk
require_cmd rg
require_cmd mktemp
require_cmd sed

cd "${ROOT_DIR}"

if ! rg -q "^## \\[${VERSION}\\] - " CHANGELOG.md; then
  echo "CHANGELOG.md does not contain version ${VERSION}." >&2
  exit 1
fi
if ! rg -q "<release version=\"${VERSION}\" " data/io.codeberg.emviolet.cardthropic.metainfo.xml.in; then
  echo "AppStream metadata does not contain release version ${VERSION}." >&2
  exit 1
fi

notes_block=""
for note in "${NOTES[@]}"; do
  [[ -z "${note}" ]] && continue
  notes_block+="- ${note}"$'\n'
done
notes_block="${notes_block%$'\n'}"
if [[ -z "${notes_block}" ]]; then
  echo "All provided notes were empty." >&2
  exit 1
fi

appstream_desc="      <description>"$'\n'
for note in "${NOTES[@]}"; do
  [[ -z "${note}" ]] && continue
  escaped="$(printf '%s' "${note}" | xml_escape)"
  appstream_desc+="        <p>${escaped}</p>"$'\n'
done
appstream_desc+="      </description>"

apply_or_preview() {
  local target="$1"
  local tmp="$2"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "DRY-RUN: would update ${target}"
    rm -f "${tmp}"
  else
    mv "${tmp}" "${target}"
    echo "Updated ${target}"
  fi
}

# CHANGELOG.md replacement
tmp="$(mktemp -p . .tmp.finalize-changelog.XXXXXX)"
awk -v version="${VERSION}" -v notes_block="${notes_block}" '
  BEGIN {
    in_target=0
    replaced=0
  }
  {
    if ($0 ~ "^## \\[" version "\\] - ") {
      in_target=1
      print
      next
    }
    if (in_target && $0 ~ "^## \\[") {
      in_target=0
    }
    if (in_target && !replaced && $0 == "- TODO: summarize release changes.") {
      n = split(notes_block, lines, "\n")
      for (i = 1; i <= n; i++) {
        print lines[i]
      }
      replaced=1
      next
    }
    print
  }
  END {
    if (!replaced) {
      print "ERROR: did not find changelog TODO placeholder for version " version > "/dev/stderr"
      exit 1
    }
  }
' CHANGELOG.md >"${tmp}"
apply_or_preview "CHANGELOG.md" "${tmp}"

# AppStream metadata replacement
tmp="$(mktemp -p . .tmp.finalize-metainfo.XXXXXX)"
awk -v version="${VERSION}" -v new_desc="${appstream_desc}" '
  BEGIN {
    in_release=0
    in_old_desc=0
    replaced=0
  }
  {
    if ($0 ~ "<release version=\"" version "\" ") {
      in_release=1
      print
      next
    }
    if (in_release && !in_old_desc && $0 ~ /<description>/) {
      n = split(new_desc, lines, "\n")
      for (i = 1; i <= n; i++) {
        print lines[i]
      }
      in_old_desc=1
      replaced=1
      next
    }
    if (in_old_desc) {
      if ($0 ~ /<\/description>/) {
        in_old_desc=0
      }
      next
    }
    if (in_release && $0 ~ /<\/release>/) {
      in_release=0
      print
      next
    }
    print
  }
  END {
    if (!replaced) {
      print "ERROR: did not replace AppStream description for version " version > "/dev/stderr"
      exit 1
    }
  }
' data/io.codeberg.emviolet.cardthropic.metainfo.xml.in >"${tmp}"
apply_or_preview "data/io.codeberg.emviolet.cardthropic.metainfo.xml.in" "${tmp}"

if [[ "${DRY_RUN}" -eq 1 ]]; then
  echo "Dry run complete."
else
  echo "Release notes finalized for version ${VERSION}."
fi
