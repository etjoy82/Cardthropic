#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

while IFS= read -r script; do
  shebang="$(sed -n '1p' "${script}")"
  if [[ "${script}" == *.sh || "${shebang}" == "#!/usr/bin/env bash" ]]; then
    echo "${script}"
  fi
done < <(find scripts -maxdepth 3 -type f | sort)
