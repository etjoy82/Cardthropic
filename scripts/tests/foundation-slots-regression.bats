#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../.." && pwd)"
}

@test "foundation slot visibility stays mode-correct under resize (opt-in integration)" {
  if [[ "${RUN_FOUNDATION_SLOTS:-0}" != "1" ]]; then
    skip "Set RUN_FOUNDATION_SLOTS=1 to run foundation-slot GUI integration test."
  fi

  for cmd in xvfb-run timeout xdotool flatpak; do
    command -v "${cmd}" >/dev/null 2>&1 || skip "Missing dependency: ${cmd}"
  done
  flatpak info io.codeberg.emviolet.cardthropic >/dev/null 2>&1 ||
    skip "Flatpak app io.codeberg.emviolet.cardthropic is not installed."

  run "${REPO_ROOT}/scripts/tests/foundation-slots-regression.sh"
  [ "${status}" -eq 0 ]
}
