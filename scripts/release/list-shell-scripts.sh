#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

if ! command -v rg >/dev/null 2>&1; then
  echo "rg is required but not installed." >&2
  exit 1
fi

while IFS= read -r script; do
  shebang="$(sed -n '1p' "${script}")"
  if [[ "${script}" == *.sh || "${shebang}" == "#!/usr/bin/env bash" ]]; then
    echo "${script}"
  fi
done < <(rg --files scripts | sort)
