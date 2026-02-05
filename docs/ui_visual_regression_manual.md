# Tabulensis UI Visual Regression Manual

**Last updated:** 2026-02-05

This manual is a step‑by‑step guide for running and extending the desktop UI visual regression pipeline. It is written for humans who want reliable UI improvements and for AI agents that need deterministic, reviewable feedback.

---

## 1) What You Get

When the pipeline is set up and running, you can:
- Launch the desktop UI in a deterministic scenario.
- Capture a screenshot automatically.
- Compare the screenshot to a baseline.
- Generate a report explaining what changed and how it affects UI quality.

The pipeline produces three core artifacts per scenario:
- `current.png`: the new screenshot.
- `diff.png`: the visual delta.
- `diff.json`: quantitative metrics.

Optional:
- `desktop/ui_reports/<scenario>/<tag>.md`: a human‑readable review report.

---

## 2) One‑Minute Quickstart

From the repo root:

```
scripts/ui_pipeline.sh compare_basic
```

That runs capture → diff → review for the `compare_basic` scenario.

---

## 3) Dependencies

Install the following on your dev machine:

**Linux (Ubuntu):**
```
sudo apt-get update
sudo apt-get install -y xvfb xdotool imagemagick
```

**macOS:**
```
brew install imagemagick
```

**Windows:**
- Install ImageMagick.
- Screenshot automation is best run in WSL or on a Linux CI agent.

**Optional (for pixel‑perfect diffs via Node):**
```
cd scripts
npm install pixelmatch pngjs
```

If you don’t install Node deps, the diff script falls back to ImageMagick.

---

## 4) Scenarios (The Most Important Part)

Scenarios make screenshots reproducible. Each scenario is defined by a JSON file:

```
desktop/ui_scenarios/<name>/scenario.json
```

### Required fields
- `oldPath`, `newPath` – workbook or PBIX files
- `autoRunDiff` – automatically trigger a diff
- `stableWaitMs` – wait time after completion

### Example
```
{
  "name": "Compare Basic",
  "description": "Single-cell diff with a small workbook",
  "oldPath": "../../../fixtures/generated/single_cell_value_a.xlsx",
  "newPath": "../../../fixtures/generated/single_cell_value_b.xlsx",
  "autoRunDiff": true,
  "stableWaitMs": 800,
  "expectMode": "payload",
  "focusPanel": "summary",
  "preset": "balanced",
  "trustedFiles": true
}
```

### Recommended scenarios
- `compare_basic` – minimal diff, fast.
- `compare_large_mode` – triggers large mode.
- `pbix_no_mashup` – error‑handling UI state.

Note: `compare_large_mode` and `pbix_no_mashup` rely on generated fixtures under `fixtures/generated/`.
Generate them with:
```
generate-fixtures --manifest fixtures/manifest_perf_e2e.yaml --force --clean
```

---

## 5) Capturing Screenshots

### Basic
```
scripts/ui_capture.sh compare_basic
```

This will:
- Start the desktop app headlessly (via Xvfb if needed).
- Load the scenario.
- Wait for the “ready” signal.
- Capture a screenshot.

The “ready” signal is a JSON file written to the path in `EXCEL_DIFF_UI_READY_FILE`.

### Output
- `desktop/ui_snapshots/compare_basic/current.png`
- `desktop/ui_snapshots/compare_basic/current.json`
- `desktop/ui_snapshots/compare_basic/runs/<tag>.png`

### Useful environment variables
- `EXCEL_DIFF_WINDOW_SIZE=1280x720`
- `EXCEL_DIFF_UI_HEADLESS=1` (force Xvfb)
- `EXCEL_DIFF_UI_CAPTURE_TIMEOUT=90`
- `EXCEL_DIFF_UI_CAPTURE_DELAY_MS=200` (extra settle time after ready signal)
- `EXCEL_DIFF_UI_CMD="cargo run -p desktop_wx --bin desktop_wx"` (override run command)
- `EXCEL_DIFF_UI_BIN=target/debug/desktop_wx` (use prebuilt binary)
- `EXCEL_DIFF_UI_DISABLE_STATE=1` (ignore saved window state for determinism)

---

## 6) Diffing Against Baselines

```
node scripts/ui_diff.js --scenario compare_basic
```

### If baseline is missing
```
node scripts/ui_diff.js --scenario compare_basic --update-baseline
```

Baselines are stored at:
```
desktop/ui_snapshots/<scenario>/baseline.png
```

### Thresholds
Default threshold is `0.15%` mismatch. Override via:
```
node scripts/ui_diff.js --scenario compare_basic --threshold 0.05
```

---

## 7) Generating Review Reports

```
python3 scripts/ui_review.py \
  --scenario compare_basic \
  --metrics desktop/ui_snapshots/compare_basic/diff.json \
  --image desktop/ui_snapshots/compare_basic/current.png \
  --diff desktop/ui_snapshots/compare_basic/diff.png
```

Reports are written to:
```
desktop/ui_reports/<scenario>/<tag>.md
```

---

## 8) AI‑Assisted Screenshot Review (Optional)

The review script can call an external vision‑capable tool by setting:

```
export EXCEL_DIFF_UI_REVIEW_CMD='python3 scripts/your_vision_wrapper.py'
```

The command receives a JSON payload on stdin and must output Markdown on stdout. If the command fails, the script falls back to a heuristic report.
The payload includes the scenario metadata, diff metrics, screenshot size, and the contents of `docs/ui_guidelines.md`.

This gives you:
- “What I see” descriptions.
- UI‑quality assessment tied to `docs/ui_guidelines.md`.
- Suggested code locations.

---

## 9) Full Pipeline (Recommended)

```
scripts/ui_pipeline.sh compare_basic
```

### Updating baselines intentionally
```
scripts/ui_pipeline.sh compare_basic --update-baseline
```

---

## 10) Interpreting Results

### Pass
- Diff below threshold.
- Report indicates no visible regression.

### Fail
- Diff above threshold.
- Inspect `diff.png` to see exact changes.
- Decide if change is intended:
  - If yes, update baseline.
  - If no, fix the UI and re-run.

---

## 11) CI Integration (Optional)

You can wire this into a GitHub Action or CI job. The typical flow:
1. Install `xvfb`, `xdotool`, `imagemagick`.
2. Run `scripts/ui_pipeline.sh compare_basic`.
3. Upload `desktop/ui_snapshots/*` and `desktop/ui_reports/*` as artifacts.

In this repo, the workflow `UI Visual Regression` runs when:
- Triggered manually via **workflow_dispatch**, or
- A PR is labeled `ui-visual`.

---

## 12) Troubleshooting

**The app never signals ready**
- Check the scenario files and paths.
- Ensure `EXCEL_DIFF_DEV_SCENARIO` matches a scenario directory.

**No window found**
- Verify window title (`EXCEL_DIFF_UI_WINDOW_TITLE`).
- Ensure the app is not crashing on startup.

**Diff fails due to size mismatch**
- Ensure window size is fixed and consistent.
- Set `EXCEL_DIFF_WINDOW_SIZE=1280x720` and `EXCEL_DIFF_START_MAXIMIZED=0`.

**Baseline drift**
- Update baselines only for intentional UI changes.
- Keep scenario files stable.

---

## 13) Best Practices

- Keep scenarios small and deterministic.
- Don’t add new scenarios without adding a baseline.
- Update `docs/ui_guidelines.md` when introducing new UI patterns.
- Treat visual regressions as test failures.

---

## 14) Reference Files

- `docs/ui_visual_regression.md` – pipeline overview
- `docs/ui_guidelines.md` – UI standards
- `desktop/ui_scenarios/` – scenarios
- `desktop/ui_snapshots/` – baselines and outputs
- `scripts/ui_capture.sh` – capture tool
- `scripts/ui_diff.js` – visual diff tool
- `scripts/ui_review.py` – report generator
- `scripts/ui_pipeline.sh` – one‑shot pipeline
