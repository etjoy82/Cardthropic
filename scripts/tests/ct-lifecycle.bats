#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../.." && pwd)"
}

@test "ct lifecycle script passes (opt-in integration)" {
  if [[ "${RUN_CT_LIFECYCLE:-0}" != "1" ]]; then
    skip "Set RUN_CT_LIFECYCLE=1 to run GUI lifecycle integration test."
  fi

  for cmd in xvfb-run timeout xdotool; do
    command -v "${cmd}" >/dev/null 2>&1 || skip "Missing dependency: ${cmd}"
  done

  run "${REPO_ROOT}/scripts/tests/ct-lifecycle.sh"
  [ "${status}" -eq 0 ]
}
