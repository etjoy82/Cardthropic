#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "${cmd} is required but not installed." >&2
    exit 1
  fi
}

require_cmd sed
require_cmd rg
require_cmd awk

cargo_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
meson_version="$(sed -n "s/.*version: '\([^']*\)'.*/\1/p" meson.build | head -n1)"
readme_version="$(awk -F'`' '/^Current version: `/{print $2; exit}' README.md)"
metainfo_latest_version="$(
  sed -n 's/.*<release version="\([^"]*\)".*/\1/p' data/io.codeberg.emviolet.cardthropic.metainfo.xml.in \
    | head -n1
)"
changelog_latest_version="$(
  sed -n 's/^## \[\([^]]*\)\] - .*/\1/p' CHANGELOG.md \
    | rg -v '^Unreleased$' \
    | head -n1
)"

for pair in \
  "Cargo.toml:${cargo_version}" \
  "meson.build:${meson_version}" \
  "README.md:${readme_version}" \
  "metainfo:${metainfo_latest_version}" \
  "CHANGELOG.md:${changelog_latest_version}"
do
  key="${pair%%:*}"
  value="${pair#*:}"
  if [[ -z "${value}" ]]; then
    echo "Missing version value in ${key}" >&2
    exit 1
  fi
done

if [[ "${cargo_version}" != "${meson_version}" ]] \
  || [[ "${cargo_version}" != "${readme_version}" ]] \
  || [[ "${cargo_version}" != "${metainfo_latest_version}" ]] \
  || [[ "${cargo_version}" != "${changelog_latest_version}" ]]; then
  echo "Release version mismatch detected:" >&2
  echo "  Cargo.toml:   ${cargo_version}" >&2
  echo "  meson.build:  ${meson_version}" >&2
  echo "  README.md:    ${readme_version}" >&2
  echo "  metainfo:     ${metainfo_latest_version}" >&2
  echo "  CHANGELOG.md: ${changelog_latest_version}" >&2
  exit 1
fi

echo "Release consistency check passed (version ${cargo_version})."
