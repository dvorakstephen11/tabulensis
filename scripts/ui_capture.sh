#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/ui_capture.sh <scenario> [--tag <tag>] [--out <dir>]

Environment variables:
  EXCEL_DIFF_UI_CMD          Override command to run the app.
  EXCEL_DIFF_UI_BIN          Direct path to desktop_wx binary.
  EXCEL_DIFF_WINDOW_SIZE     Window size (e.g. 1280x720).
  EXCEL_DIFF_UI_WINDOW_TITLE Window title to capture (default: Tabulensis).
  EXCEL_DIFF_UI_HEADLESS     Set to 0 to use current DISPLAY, otherwise uses xvfb-run (default).
  EXCEL_DIFF_UI_CAPTURE_TIMEOUT  Seconds to wait for readiness (default: 90).
  EXCEL_DIFF_UI_WINDOW_TIMEOUT   Seconds to wait for the window to appear (default: 10).
  EXCEL_DIFF_UI_CAPTURE_DELAY_MS Additional delay after ready signal (default: scenario stableWaitMs or 200).
USAGE
}

if [[ $# -lt 1 ]]; then
  usage
  exit 1
fi

SCENARIO="$1"
shift

TAG=""
OUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG="$2"; shift 2 ;;
    --out)
      OUT_DIR="$2"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$ROOT/desktop/ui_snapshots/$SCENARIO"
fi
RUNS_DIR="$OUT_DIR/runs"
mkdir -p "$RUNS_DIR"

if [[ -z "$TAG" ]]; then
  TAG="$(git rev-parse --short HEAD 2>/dev/null || date +%Y%m%d_%H%M%S)"
fi

WINDOW_SIZE="${EXCEL_DIFF_WINDOW_SIZE:-1280x720}"
WINDOW_TITLE="${EXCEL_DIFF_UI_WINDOW_TITLE:-Tabulensis}"
TIMEOUT_SEC="${EXCEL_DIFF_UI_CAPTURE_TIMEOUT:-90}"
WINDOW_TIMEOUT_SEC="${EXCEL_DIFF_UI_WINDOW_TIMEOUT:-10}"
if [[ -n "${EXCEL_DIFF_UI_CAPTURE_DELAY_MS:-}" ]]; then
  DELAY_MS="${EXCEL_DIFF_UI_CAPTURE_DELAY_MS}"
else
  # Default to the scenario's `stableWaitMs` when available.
  DELAY_MS="$(
    python3 - "$ROOT" "$SCENARIO" 2>/dev/null <<'PY' || echo 200
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
scenario = sys.argv[2]
path = root / "desktop" / "ui_scenarios" / scenario / "scenario.json"
if not path.is_file():
    print(200)
    raise SystemExit(0)

try:
    data = json.loads(path.read_text(encoding="utf-8"))
except Exception:
    print(200)
    raise SystemExit(0)

value = data.get("stableWaitMs")
print(int(value) if isinstance(value, int) and value >= 0 else 200)
PY
  )"
fi

READY_FILE="$(mktemp /tmp/excel_diff_ui_ready_XXXXXX.json)"
# `mktemp` creates the file; remove it so we can wait for the app to write it.
rm -f "$READY_FILE"
SHOT_PATH="$RUNS_DIR/${TAG}.png"
META_PATH="$RUNS_DIR/${TAG}.json"
LOG_PATH="$RUNS_DIR/${TAG}.log"
READY_COPY_PATH="$RUNS_DIR/${TAG}.ready.json"

APP_CMD="${EXCEL_DIFF_UI_CMD:-cargo run -p desktop_wx --bin desktop_wx}"
if [[ -n "${EXCEL_DIFF_UI_BIN:-}" ]]; then
  APP_CMD="${EXCEL_DIFF_UI_BIN}"
fi

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required tool: $1" >&2
    exit 1
  fi
}

wait_for_ready() {
  local deadline=$((SECONDS + TIMEOUT_SEC))
  while [[ $SECONDS -lt $deadline ]]; do
    if [[ -s "$READY_FILE" ]]; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

wait_for_window() {
  local deadline=$((SECONDS + WINDOW_TIMEOUT_SEC))
  while [[ $SECONDS -lt $deadline ]]; do
    local wid
    wid="$(xdotool search --name "$WINDOW_TITLE" 2>/dev/null | head -n 1 || true)"
    if [[ -n "$wid" ]]; then
      echo "$wid"
      return 0
    fi
    sleep 0.2
  done
  return 1
}

capture_with_display() {
  require_tool xdotool
  require_tool import
  require_tool setsid

  # Keep backend state under the run directory so capture works in restricted/sandboxed envs.
  APP_DATA_DIR="$RUNS_DIR/${TAG}_app_data"
  mkdir -p "$APP_DATA_DIR"

  export EXCEL_DIFF_LOG="${EXCEL_DIFF_LOG:-debug}"
  export EXCEL_DIFF_DEBUG_LAYOUT="${EXCEL_DIFF_DEBUG_LAYOUT:-1}"
  export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
  # Force X11 for deterministic headless capture under xvfb-run.
  export GDK_BACKEND="${GDK_BACKEND:-x11}"
  unset WAYLAND_DISPLAY
  export EXCEL_DIFF_APP_DATA_DIR="$APP_DATA_DIR"
  export EXCEL_DIFF_DEV_SCENARIO="$SCENARIO"
  export EXCEL_DIFF_UI_READY_FILE="$READY_FILE"
  export EXCEL_DIFF_WINDOW_SIZE="$WINDOW_SIZE"
  export EXCEL_DIFF_START_MAXIMIZED=0
  export EXCEL_DIFF_UI_DISABLE_STATE=1
  export EXCEL_DIFF_USE_WEBVIEW=0
  export EXCEL_DIFF_SUPPRESS_GTK_WARNINGS=1
  # UI capture should not depend on licensing state (especially for release-desktop builds).
  # Use EXCEL_DIFF_REQUIRE_LICENSE=1 to explicitly test licensing flows.
  export EXCEL_DIFF_SKIP_LICENSE="${EXCEL_DIFF_SKIP_LICENSE:-1}"

  # Run in a dedicated process group so we can reliably clean up `cargo run` and any
  # child processes it spawns.
  setsid bash -c "$APP_CMD" >"$LOG_PATH" 2>&1 &
  APP_PID=$!

  cleanup() {
    if kill -0 "$APP_PID" >/dev/null 2>&1; then
      kill -- -"$APP_PID" >/dev/null 2>&1 || true
    fi
  }
  trap cleanup EXIT

  if ! wait_for_ready; then
    echo "Timed out waiting for UI ready signal." >&2
    if [[ -f "$LOG_PATH" ]]; then
      echo "Last 200 lines of log ($LOG_PATH):" >&2
      tail -n 200 "$LOG_PATH" >&2 || true
    fi
    exit 1
  fi

  if [[ "$DELAY_MS" -gt 0 ]]; then
    sleep "$(awk "BEGIN {print $DELAY_MS/1000}")"
  fi

  # Persist the ready file alongside the screenshot so it doesn't get lost in /tmp.
  cp "$READY_FILE" "$READY_COPY_PATH"

  WID="$(wait_for_window || true)"
  if [[ -z "$WID" ]]; then
    echo "Failed to locate window titled '$WINDOW_TITLE' (xdotool). Capturing root window instead." >&2
    if [[ -f "$READY_FILE" ]]; then
      echo "Ready file contents ($READY_FILE):" >&2
      cat "$READY_FILE" >&2 || true
    fi
    if [[ -f "$LOG_PATH" ]]; then
      echo "Last 200 lines of log ($LOG_PATH):" >&2
      tail -n 200 "$LOG_PATH" >&2 || true
    fi
    WID="root"
  fi

  if [[ "$WID" != "root" ]]; then
    xdotool windowactivate --sync "$WID" >/dev/null 2>&1 || true
  fi
  import -window "$WID" "$SHOT_PATH"

  cat > "$META_PATH" <<META
{
  "scenario": "${SCENARIO}",
  "tag": "${TAG}",
  "windowSize": "${WINDOW_SIZE}",
  "windowTitle": "${WINDOW_TITLE}",
  "readyFile": "${READY_FILE}",
  "readyCopy": "${READY_COPY_PATH}",
  "logFile": "${LOG_PATH}",
  "appDataDir": "${APP_DATA_DIR}",
  "timestampUnix": "$(date +%s)",
  "command": "${APP_CMD}"
}
META

  cp "$SHOT_PATH" "$OUT_DIR/current.png"
  cp "$META_PATH" "$OUT_DIR/current.json"
  cp "$READY_COPY_PATH" "$OUT_DIR/current_ready.json"
}

if [[ "${EXCEL_DIFF_UI_HEADLESS:-1}" != "0" && -z "${EXCEL_DIFF_UI_IN_XVFB:-}" ]]; then
  require_tool xvfb-run
  WIDTH=${WINDOW_SIZE%x*}
  HEIGHT=${WINDOW_SIZE#*x}
  export EXCEL_DIFF_UI_IN_XVFB=1
  export EXCEL_DIFF_UI_HEADLESS=0
  exec xvfb-run -a -s "-screen 0 ${WIDTH}x${HEIGHT}x24" "$0" "$SCENARIO" --tag "$TAG" --out "$OUT_DIR"
fi

capture_with_display

echo "Captured screenshot: $SHOT_PATH"
