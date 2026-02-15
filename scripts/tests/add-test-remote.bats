#!/usr/bin/env bats

setup() {
  TEST_TMPDIR="$(mktemp -d)"
  export TEST_TMPDIR
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../.." && pwd)"
  export LOG_FILE="${TEST_TMPDIR}/flatpak.log"

  mkdir -p "${TEST_TMPDIR}/bin"
  cat > "${TEST_TMPDIR}/bin/flatpak" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

echo "$*" >> "${LOG_FILE}"

if [[ "${1:-}" == "remotes" ]]; then
  printf '%s\n' "cardthropic"
  exit 0
fi

exit 0
EOF
  chmod +x "${TEST_TMPDIR}/bin/flatpak"
  export PATH="${TEST_TMPDIR}/bin:${PATH}"
}

teardown() {
  rm -rf "${TEST_TMPDIR}"
}

@test "add-test-remote fails when remote exists without --replace" {
  run "${REPO_ROOT}/scripts/flatpak-repo/add-test-remote.sh" \
    --remote cardthropic \
    --url "https://example.com/repo/"

  [ "${status}" -eq 1 ]
  [[ "${output}" == *"Use --replace to delete and recreate it."* ]]
}

@test "add-test-remote deletes and recreates when --replace is set" {
  run "${REPO_ROOT}/scripts/flatpak-repo/add-test-remote.sh" \
    --replace \
    --remote cardthropic \
    --url "https://example.com/repo/"

  [ "${status}" -eq 0 ]
  grep -q "^remote-delete --user cardthropic$" "${LOG_FILE}"
  grep -q "^remote-add --if-not-exists --user --no-gpg-verify cardthropic https://example.com/repo/$" "${LOG_FILE}"
}
