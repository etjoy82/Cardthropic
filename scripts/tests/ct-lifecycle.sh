#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
#
# Regression check for app lifecycle termination behavior:
# - quit accelerator (Ctrl+Q -> app.quit) should terminate process cleanly
# - main-window close should terminate process cleanly

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

run_case() {
  local mode="$1" # quit | close
  if command -v flatpak >/dev/null 2>&1; then
    flatpak kill "${APP_ID}" >/dev/null 2>&1 || true
  fi
  timeout 45s xvfb-run -a bash -s -- "${LAUNCH_CMD}" "${mode}" <<'EOS'
set -euo pipefail

LAUNCH_CMD="$1"
MODE="$2"

bash -lc "${LAUNCH_CMD}" >/tmp/cardthropic-lifecycle.log 2>&1 &
APP_PID="$!"

cleanup() {
  kill "${APP_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
}
trap cleanup EXIT

if ! command -v xdotool >/dev/null 2>&1; then
  echo "xdotool is required for lifecycle UI automation." >&2
  exit 2
fi

# Wait for main window. Prefer PID match; fall back to title match.
window_id=""
for _ in {1..300}; do
  if ! kill -0 "${APP_PID}" 2>/dev/null; then
    echo "Cardthropic exited before creating a window." >&2
    echo "--- /tmp/cardthropic-lifecycle.log ---" >&2
    tail -n 120 /tmp/cardthropic-lifecycle.log >&2 || true
    exit 1
  fi
  window_id="$(xdotool search --pid "${APP_PID}" 2>/dev/null | head -n 1 || true)"
  if [[ -z "${window_id}" ]]; then
    window_id="$(xdotool search --name '[Cc]ardthropic' 2>/dev/null | head -n 1 || true)"
  fi
  if [[ -n "${window_id}" ]]; then
    break
  fi
  sleep 0.1
done
if [[ -z "${window_id}" ]]; then
  echo "Unable to find Cardthropic window." >&2
  echo "--- /tmp/cardthropic-lifecycle.log ---" >&2
  tail -n 120 /tmp/cardthropic-lifecycle.log >&2 || true
  exit 1
fi

xdotool windowactivate "${window_id}" || true

# Open auxiliary windows first to stress cleanup on termination.
xdotool key --window "${window_id}" F1 || true
xdotool key --window "${window_id}" ctrl+shift+h || true
xdotool key --window "${window_id}" ctrl+shift+a || true

if [[ "${MODE}" == "quit" ]]; then
  sent=0
  for _ in {1..5}; do
    if xdotool key --window "${window_id}" ctrl+q 2>/dev/null; then
      sent=1
      break
    fi
    sleep 0.1
  done
  if [[ "${sent}" -eq 0 ]]; then
    echo "Unable to send Ctrl+Q to window ${window_id}; falling back to window close." >&2
    xdotool windowclose "${window_id}" || true
  fi
elif [[ "${MODE}" == "close" ]]; then
  xdotool windowclose "${window_id}"
else
  echo "Unknown mode: ${MODE}" >&2
  exit 2
fi

# Ensure process exits.
for _ in {1..120}; do
  if ! kill -0 "${APP_PID}" 2>/dev/null; then
    exit 0
  fi
  sleep 0.1
done

echo "Cardthropic process remained alive after ${MODE} case." >&2
ps -p "${APP_PID}" -o pid=,stat=,cmd= >&2 || true
exit 1
EOS
}

main() {
  require_cmd xvfb-run
  require_cmd xdotool
  require_cmd timeout
  resolve_launch_cmd
  echo "Launch command: ${LAUNCH_CMD}"

  echo "[1/2] lifecycle quit-action case..."
  if ! run_case quit; then
    echo "FAIL: quit-action case." >&2
    exit 1
  fi
  echo "PASS: app.quit terminated process."

  echo "[2/2] lifecycle main-window close case..."
  if ! run_case close; then
    echo "FAIL: main-window close case." >&2
    exit 1
  fi
  echo "PASS: main-window close terminated process."

  echo "Lifecycle regression checks completed."
}

main "$@"
