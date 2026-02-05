# UI Visual Regression Pipeline

**Last updated:** 2026-02-05

This guide documents the automated desktop UI visual regression workflow.

## Overview
The pipeline does four things:
1. Load a deterministic scenario.
2. Capture a screenshot.
3. Diff against a baseline.
4. Generate a review report.

## Key Commands
- Capture: `scripts/ui_capture.sh <scenario>`
- Diff: `node scripts/ui_diff.js --scenario <scenario>`
- Review: `python3 scripts/ui_review.py --scenario <scenario>`
- One-shot: `scripts/ui_pipeline.sh <scenario>`

## Scenario Harness
Enable with:

```
EXCEL_DIFF_DEV_SCENARIO=compare_basic \
EXCEL_DIFF_WINDOW_SIZE=1280x720 \
EXCEL_DIFF_START_MAXIMIZED=0 \
EXCEL_DIFF_UI_DISABLE_STATE=1 \
cargo run -p desktop_wx --bin desktop_wx
```

The app will load `desktop/ui_scenarios/<scenario>/scenario.json`, open the files, run the diff, then write a ready signal to the file specified by `EXCEL_DIFF_UI_READY_FILE`.

## Environment Variables
- `EXCEL_DIFF_DEV_SCENARIO`: scenario name.
- `EXCEL_DIFF_UI_SCENARIOS_ROOT`: override scenario root directory.
- `EXCEL_DIFF_UI_READY_FILE`: file path to write readiness metadata.
- `EXCEL_DIFF_WINDOW_SIZE`: fixed window size (e.g., `1280x720`).
- `EXCEL_DIFF_UI_DISABLE_STATE`: disable `ui_state.json` load/save.
- `EXCEL_DIFF_START_MAXIMIZED`: should be `0` for captures.
- `EXCEL_DIFF_UI_WINDOW_TITLE`: window title for capture (default `Tabulensis`).

## Dependencies
- `xvfb-run`, `xdotool`, `imagemagick` for screenshot capture.
- `node` if using `scripts/ui_diff.js`.
- `python3` for report generation.
`scripts/ui_diff.js` prefers `pixelmatch` + `pngjs` if installed, and falls back to ImageMagick otherwise.

## Baselines
- Baselines live at `desktop/ui_snapshots/<scenario>/baseline.png`.
- Update baselines explicitly using:

```
node scripts/ui_diff.js --scenario compare_basic --update-baseline
```

## Reports
Reports are written to `desktop/ui_reports/<scenario>/<tag>.md` by default.

## Notes
- If the diff is above the threshold, review `diff.png` and the report before updating baselines.
- Keep scenarios small and deterministic to avoid flaky diffs.
