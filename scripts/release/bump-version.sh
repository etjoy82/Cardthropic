#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/release/bump-version.sh --version <x.y.z> [--date YYYY-MM-DD] [--dry-run]

Behavior:
  - Updates version references in:
    - Cargo.toml
    - meson.build
    - README.md (Current version line)
  - Adds a new release section to CHANGELOG.md after [Unreleased]
  - Prepends a new <release> entry in AppStream metadata

Options:
  --version <x.y.z>   Required semver (major.minor.patch)
  --date <YYYY-MM-DD> Release date for changelog/metainfo (default: today)
  --dry-run           Print planned changes without writing files
  -h, --help          Show this help
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION=""
RELEASE_DATE="$(date +%F)"
DRY_RUN=0

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
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
    --date)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "Missing value for --date" >&2
        exit 2
      fi
      RELEASE_DATE="${2:-}"
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

if [[ ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Version must be semver like x.y.z (example: 0.6.0)." >&2
  exit 1
fi

if [[ ! "${RELEASE_DATE}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "Date must be YYYY-MM-DD." >&2
  exit 1
fi

require_cmd awk
require_cmd sed
require_cmd rg
require_cmd mktemp

cd "${ROOT_DIR}"

if rg -q "^## \\[${VERSION}\\] - " CHANGELOG.md; then
  echo "CHANGELOG.md already contains version ${VERSION}." >&2
  exit 1
fi
if rg -q "<release version=\"${VERSION}\" date=\"${RELEASE_DATE}\">" data/io.codeberg.emviolet.cardthropic.metainfo.xml.in; then
  echo "AppStream metadata already contains version ${VERSION} with date ${RELEASE_DATE}." >&2
  exit 1
fi

apply_or_preview() {
  local src="$1"
  local tmp="$2"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "DRY-RUN: would update ${src}"
    rm -f "${tmp}"
  else
    mv "${tmp}" "${src}"
    echo "Updated ${src}"
  fi
}

# Cargo.toml: [package].version
tmp="$(mktemp -p . .tmp.cargo.XXXXXX)"
awk -v version="${VERSION}" '
  BEGIN { in_package=0; done=0 }
  /^\[package\]/ { in_package=1; print; next }
  /^\[/ && $0 !~ /^\[package\]/ { in_package=0 }
  in_package && !done && /^version = / {
    print "version = \"" version "\""
    done=1
    next
  }
  { print }
  END {
    if (!done) {
      print "ERROR: did not find [package] version in Cargo.toml" > "/dev/stderr"
      exit 1
    }
  }
' Cargo.toml >"${tmp}"
apply_or_preview "Cargo.toml" "${tmp}"

# meson.build: project version
tmp="$(mktemp -p . .tmp.meson.XXXXXX)"
awk -v version="${VERSION}" '
  BEGIN { done=0 }
  {
    if (!done && $0 ~ /version:[[:space:]]*'\''[^'\'']+'\''/) {
      gsub(/version:[[:space:]]*'\''[^'\'']+'\''/, "version: '\''" version "'\''")
      done=1
    }
    print
  }
  END {
    if (!done) {
      print "ERROR: did not find project version in meson.build" > "/dev/stderr"
      exit 1
    }
  }
' meson.build >"${tmp}"
apply_or_preview "meson.build" "${tmp}"

# README.md: Current version line
tmp="$(mktemp -p . .tmp.readme.XXXXXX)"
awk -v version="${VERSION}" '
  BEGIN { done=0 }
  /^Current version: `/ {
    suffix=$0
    sub(/^Current version: `[^`]*`/, "", suffix)
    print "Current version: `" version "`" suffix
    done=1
    next
  }
  { print }
  END {
    if (!done) {
      print "ERROR: did not find Current version line in README.md" > "/dev/stderr"
      exit 1
    }
  }
' README.md >"${tmp}"
apply_or_preview "README.md" "${tmp}"

# CHANGELOG.md: insert section after Unreleased block
tmp="$(mktemp -p . .tmp.changelog.XXXXXX)"
awk -v version="${VERSION}" -v date="${RELEASE_DATE}" '
  BEGIN { inserted=0; in_unreleased=0 }
  /^## \[Unreleased\]/ { in_unreleased=1; print; next }
  /^## \[/ && in_unreleased && !inserted {
    print ""
    print "## [" version "] - " date
    print ""
    print "### Changed"
    print "- TODO: summarize release changes."
    print ""
    inserted=1
    in_unreleased=0
  }
  { print }
  END {
    if (!inserted) {
      print ""
      print "## [" version "] - " date
      print ""
      print "### Changed"
      print "- TODO: summarize release changes."
      print ""
    }
  }
' CHANGELOG.md >"${tmp}"
apply_or_preview "CHANGELOG.md" "${tmp}"

# AppStream metainfo: prepend new release under <releases>
tmp="$(mktemp -p . .tmp.metainfo.XXXXXX)"
awk -v version="${VERSION}" -v date="${RELEASE_DATE}" '
  BEGIN { inserted=0 }
  /<releases>/ && !inserted {
    print
    print "    <release version=\"" version "\" date=\"" date "\">"
    print "      <description>"
    print "        <p>TODO: summarize release changes.</p>"
    print "      </description>"
    print "    </release>"
    inserted=1
    next
  }
  { print }
  END {
    if (!inserted) {
      print "ERROR: did not find <releases> in metainfo file" > "/dev/stderr"
      exit 1
    }
  }
' data/io.codeberg.emviolet.cardthropic.metainfo.xml.in >"${tmp}"
apply_or_preview "data/io.codeberg.emviolet.cardthropic.metainfo.xml.in" "${tmp}"

if [[ "${DRY_RUN}" -eq 1 ]]; then
  echo "Dry run complete."
else
  echo "Version bump complete: ${VERSION} (${RELEASE_DATE})"
fi
