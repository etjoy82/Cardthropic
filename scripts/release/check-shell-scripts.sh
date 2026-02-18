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

require_cmd find
require_cmd sed
require_cmd rg
require_cmd stat

portable_stat_mode() {
  local path="$1"
  if stat -c '%a' "${path}" >/dev/null 2>&1; then
    stat -c '%a' "${path}"
    return
  fi
  if stat -f '%Lp' "${path}" >/dev/null 2>&1; then
    stat -f '%Lp' "${path}"
    return
  fi
  fail "unable to determine file mode for ${path}; unsupported stat implementation"
}

fail() {
  echo "shell-check: $*" >&2
  exit 1
}

while IFS= read -r script; do
  shebang="$(sed -n '1p' "${script}")"

  mode="$(portable_stat_mode "${script}")"
  [[ "${mode}" =~ ^[0-9]+$ ]] || fail "unable to read mode for ${script}"

  [[ "${shebang}" == "#!/usr/bin/env bash" ]] || fail "bad shebang in ${script}"

  set_line="$(sed -n '2p' "${script}")"
  [[ "${set_line}" == "set -euo pipefail" ]] || fail "missing strict mode on line 2 in ${script}"

  rg -q "Maintainer-only operational script for Cardthropic\\." "${script}" ||
    fail "missing maintainer-only banner in ${script}"

  # Require owner executable bit at minimum for maintainability.
  owner_exec=$(((10#${mode} / 100) % 10))
  ((owner_exec & 1)) || fail "script is not executable by owner: ${script}"

  bash -n "${script}" || fail "bash -n failed for ${script}"
done < <(scripts/release/list-shell-scripts.sh)

echo "Shell script checks passed."
