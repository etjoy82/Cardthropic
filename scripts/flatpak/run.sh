#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

if ! command -v flatpak >/dev/null 2>&1; then
  echo "flatpak is required but not installed." >&2
  exit 1
fi

flatpak run io.codeberg.emviolet.cardthropic "$@"
