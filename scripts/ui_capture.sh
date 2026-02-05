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
  EXCEL_DIFF_UI_HEADLESS     Force xvfb-run if set to 1.
  EXCEL_DIFF_UI_CAPTURE_TIMEOUT  Seconds to wait for readiness (default: 90).
  EXCEL_DIFF_UI_CAPTURE_DELAY_MS Additional delay after ready signal (default: 200).
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
DELAY_MS="${EXCEL_DIFF_UI_CAPTURE_DELAY_MS:-200}"

READY_FILE="$(mktemp /tmp/excel_diff_ui_ready_XXXXXX.json)"
SHOT_PATH="$RUNS_DIR/${TAG}.png"
META_PATH="$RUNS_DIR/${TAG}.json"

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
    if [[ -f "$READY_FILE" ]]; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

capture_with_display() {
  require_tool xdotool
  require_tool import

  export EXCEL_DIFF_DEV_SCENARIO="$SCENARIO"
  export EXCEL_DIFF_UI_READY_FILE="$READY_FILE"
  export EXCEL_DIFF_WINDOW_SIZE="$WINDOW_SIZE"
  export EXCEL_DIFF_START_MAXIMIZED=0
  export EXCEL_DIFF_UI_DISABLE_STATE=1
  export EXCEL_DIFF_USE_WEBVIEW=0
  export EXCEL_DIFF_SUPPRESS_GTK_WARNINGS=1

  bash -c "$APP_CMD" &
  APP_PID=$!

  cleanup() {
    if kill -0 "$APP_PID" >/dev/null 2>&1; then
      kill "$APP_PID" >/dev/null 2>&1 || true
    fi
  }
  trap cleanup EXIT

  if ! wait_for_ready; then
    echo "Timed out waiting for UI ready signal." >&2
    exit 1
  fi

  if [[ "$DELAY_MS" -gt 0 ]]; then
    sleep "$(awk "BEGIN {print $DELAY_MS/1000}")"
  fi

  WID=$(xdotool search --sync --name "$WINDOW_TITLE" | head -n 1 || true)
  if [[ -z "$WID" ]]; then
    echo "Failed to locate window titled '$WINDOW_TITLE'" >&2
    exit 1
  fi

  xdotool windowactivate --sync "$WID" >/dev/null 2>&1 || true
  import -window "$WID" "$SHOT_PATH"

  cat > "$META_PATH" <<META
{
  "scenario": "${SCENARIO}",
  "tag": "${TAG}",
  "windowSize": "${WINDOW_SIZE}",
  "windowTitle": "${WINDOW_TITLE}",
  "readyFile": "${READY_FILE}",
  "timestampUnix": "$(date +%s)",
  "command": "${APP_CMD}"
}
META

  cp "$SHOT_PATH" "$OUT_DIR/current.png"
  cp "$META_PATH" "$OUT_DIR/current.json"
}

if [[ -z "${DISPLAY:-}" || "${EXCEL_DIFF_UI_HEADLESS:-}" == "1" ]]; then
  require_tool xvfb-run
  WIDTH=${WINDOW_SIZE%x*}
  HEIGHT=${WINDOW_SIZE#*x}
  xvfb-run -a -s "-screen 0 ${WIDTH}x${HEIGHT}x24" bash -c "$(declare -f require_tool wait_for_ready capture_with_display); capture_with_display"
else
  capture_with_display
fi

echo "Captured screenshot: $SHOT_PATH"
