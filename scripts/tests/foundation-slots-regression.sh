#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.
#
# Regression check: foundation slot visibility must be mode-correct.
# - Klondike: 4 visible
# - Spider: 8 visible
# - FreeCell: 4 visible
#
# Uses layout_debug invariants emitted by the app:
#   mode=... fslots_vis=... fslots_exp=... fslots_ok=...

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_ID="io.codeberg.emviolet.cardthropic"
BIN_PATH="${CARDTHROPIC_BIN:-${ROOT_DIR}/target/debug/cardthropic}"
LAUNCH_CMD="${CARDTHROPIC_CMD:-}"

require_cmd() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing required command: ${cmd}" >&2
    exit 2
  fi
}

ensure_binary() {
  if [[ -x "${BIN_PATH}" ]]; then
    return
  fi
  echo "Binary not found at ${BIN_PATH}; building debug binary..."
  (cd "${ROOT_DIR}" && cargo build -q)
  if [[ ! -x "${BIN_PATH}" ]]; then
    echo "Expected executable missing after build: ${BIN_PATH}" >&2
    exit 2
  fi
}

resolve_launch_cmd() {
  if [[ -n "${LAUNCH_CMD}" ]]; then
    return
  fi
  if command -v flatpak >/dev/null 2>&1 && flatpak info "${APP_ID}" >/dev/null 2>&1; then
    LAUNCH_CMD="flatpak run --env=G_APPLICATION_NON_UNIQUE=1 ${APP_ID}"
    return
  fi
  ensure_binary
  LAUNCH_CMD="${BIN_PATH}"
}

main() {
  require_cmd xvfb-run
  require_cmd xdotool
  require_cmd timeout
  resolve_launch_cmd
  echo "Launch command: ${LAUNCH_CMD}"

  timeout 90s xvfb-run -a bash -s -- "${LAUNCH_CMD}" <<'EOS'
set -euo pipefail

LAUNCH_CMD="$1"
LOG_FILE="/tmp/cardthropic-foundation-slots.log"
: >"${LOG_FILE}"

bash -lc "${LAUNCH_CMD}" >"${LOG_FILE}" 2>&1 &
APP_PID="$!"

cleanup() {
  kill "${APP_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
}
trap cleanup EXIT

find_window() {
  local id=""
  for _ in {1..300}; do
    if ! kill -0 "${APP_PID}" 2>/dev/null; then
      echo "App exited before window appeared." >&2
      tail -n 120 "${LOG_FILE}" >&2 || true
      return 1
    fi
    id="$(xdotool search --pid "${APP_PID}" 2>/dev/null | head -n1 || true)"
    if [[ -z "${id}" ]]; then
      id="$(xdotool search --name '[Cc]ardthropic' 2>/dev/null | head -n1 || true)"
    fi
    if [[ -n "${id}" ]]; then
      echo "${id}"
      return 0
    fi
    sleep 0.1
  done
  echo "Unable to find Cardthropic window." >&2
  tail -n 120 "${LOG_FILE}" >&2 || true
  return 1
}

WINDOW_ID="$(find_window)"
xdotool windowactivate "${WINDOW_ID}" || true
xdotool windowsize "${WINDOW_ID}" 900 800 || true
sleep 0.2

# Ensure robot debug mode is on so layout_debug lines are emitted.
if ! grep -q "layout_debug" "${LOG_FILE}"; then
  xdotool key --window "${WINDOW_ID}" F8 || true
  sleep 0.2
  xdotool windowsize "${WINDOW_ID}" 880 800 || true
  sleep 0.3
fi
if ! grep -q "layout_debug" "${LOG_FILE}"; then
  echo "No layout_debug output after enabling debug mode." >&2
  tail -n 160 "${LOG_FILE}" >&2 || true
  exit 1
fi

assert_mode_slots() {
  local mode="$1"
  local key_combo="$2"
  local expected="$3"
  local line=""

  xdotool key --window "${WINDOW_ID}" "${key_combo}" || true
  sleep 0.2

  for i in {1..40}; do
    local w=$((900 - i * 11))
    if [[ "${w}" -lt 360 ]]; then
      w=360
    fi
    xdotool windowsize "${WINDOW_ID}" "${w}" 800 || true
    sleep 0.1
    line="$(grep "layout_debug" "${LOG_FILE}" | grep "mode=${mode} " | tail -n1 || true)"
    if [[ -z "${line}" ]]; then
      continue
    fi
    if [[ "${line}" == *"fslots_vis=${expected}"* && "${line}" == *"fslots_exp=${expected}"* && "${line}" == *"fslots_ok=true"* ]]; then
      echo "PASS ${mode}: ${line}"
      return 0
    fi
  done

  echo "FAIL ${mode}: expected ${expected} visible foundation slots." >&2
  echo "Last matching line: ${line}" >&2
  tail -n 200 "${LOG_FILE}" >&2 || true
  exit 1
}

assert_mode_slots "Klondike" "shift+1" 4
assert_mode_slots "Spider" "ctrl+4" 8
assert_mode_slots "FreeCell" "ctrl+shift+3" 4

echo "Foundation slot visibility regression checks passed."
EOS
}

main "$@"
