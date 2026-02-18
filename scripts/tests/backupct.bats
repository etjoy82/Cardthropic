#!/usr/bin/env bats

setup() {
  TEST_TMPDIR="$(mktemp -d)"
  export TEST_TMPDIR
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../.." && pwd)"
}

teardown() {
  rm -rf "${TEST_TMPDIR}"
}

@test "backupct appends sanitized description to generated --name" {
  local repo_dir="${TEST_TMPDIR}/repo"
  local release_dir="${repo_dir}/scripts/release"
  mkdir -p "${release_dir}"

  cp "${REPO_ROOT}/scripts/release/backupct" "${release_dir}/backupct"

  cat > "${release_dir}/zip-working-tree.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "${CAPTURE_FILE}"
EOF
  chmod +x "${release_dir}/backupct" "${release_dir}/zip-working-tree.sh"

  export CAPTURE_FILE="${TEST_TMPDIR}/capture.txt"
  run "${release_dir}/backupct" "My quick/fix #1"
  [ "${status}" -eq 0 ]

  local name_value
  name_value="$(awk 'prev=="--name"{print; exit}{prev=$0}' "${CAPTURE_FILE}")"
  [[ "${name_value}" == *"-my-quick-fix-1" ]]
}

@test "backupct respects explicit --name without appending description" {
  local repo_dir="${TEST_TMPDIR}/repo"
  local release_dir="${repo_dir}/scripts/release"
  mkdir -p "${release_dir}"

  cp "${REPO_ROOT}/scripts/release/backupct" "${release_dir}/backupct"

  cat > "${release_dir}/zip-working-tree.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "${CAPTURE_FILE}"
EOF
  chmod +x "${release_dir}/backupct" "${release_dir}/zip-working-tree.sh"

  export CAPTURE_FILE="${TEST_TMPDIR}/capture.txt"
  run "${release_dir}/backupct" --name "manual-name" "ignored desc"
  [ "${status}" -eq 0 ]

  local name_value
  name_value="$(awk 'prev=="--name"{print; exit}{prev=$0}' "${CAPTURE_FILE}")"
  [ "${name_value}" = "manual-name" ]
}

@test "backupct installed outside repo resolves repo from current working directory" {
  local repo_dir="${TEST_TMPDIR}/repo"
  local release_dir="${repo_dir}/scripts/release"
  local bin_dir="${TEST_TMPDIR}/bin"
  mkdir -p "${release_dir}" "${bin_dir}"

  cp "${REPO_ROOT}/scripts/release/backupct" "${bin_dir}/backupct"

  cat > "${release_dir}/zip-working-tree.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "${CAPTURE_FILE}"
EOF
  chmod +x "${bin_dir}/backupct" "${release_dir}/zip-working-tree.sh"

  export CAPTURE_FILE="${TEST_TMPDIR}/capture.txt"
  pushd "${repo_dir}" >/dev/null
  run "${bin_dir}/backupct"
  popd >/dev/null
  [ "${status}" -eq 0 ]

  local repo_value
  repo_value="$(awk 'prev=="--repo"{print; exit}{prev=$0}' "${CAPTURE_FILE}")"
  [ "${repo_value}" = "${repo_dir}" ]
}
