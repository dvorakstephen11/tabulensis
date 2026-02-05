#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/ui_pipeline.sh <scenario> [--tag <tag>] [--update-baseline] [--no-review]
USAGE
}

if [[ $# -lt 1 ]]; then
  usage
  exit 1
fi

SCENARIO="$1"
shift
TAG=""
UPDATE_BASELINE=0
NO_REVIEW=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG="$2"; shift 2 ;;
    --update-baseline)
      UPDATE_BASELINE=1; shift 1 ;;
    --no-review)
      NO_REVIEW=1; shift 1 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "Unknown arg: $1" >&2
      usage
      exit 1
      ;;
  esac
done

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
OUT_DIR="$ROOT/desktop/ui_snapshots/$SCENARIO"

CAPTURE_ARGS=("$SCENARIO")
if [[ -n "$TAG" ]]; then
  CAPTURE_ARGS+=("--tag" "$TAG")
fi

scripts/ui_capture.sh "${CAPTURE_ARGS[@]}"

DIFF_ARGS=("--scenario" "$SCENARIO")
if [[ $UPDATE_BASELINE -eq 1 ]]; then
  DIFF_ARGS+=("--update-baseline")
fi
set +e
node scripts/ui_diff.js "${DIFF_ARGS[@]}"
DIFF_EXIT=$?
set -e

if [[ $NO_REVIEW -eq 0 ]]; then
  python3 scripts/ui_review.py \
    --scenario "$SCENARIO" \
    --metrics "$OUT_DIR/diff.json" \
    --image "$OUT_DIR/current.png" \
    --diff "$OUT_DIR/diff.png"
fi
exit ${DIFF_EXIT:-0}
