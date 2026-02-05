# UI Visual Regression + Screenshot Review Plan

**Last updated:** 2026-02-05

## Goals
Create a repeatable, automated UI pipeline that captures deterministic screenshots, diffs them against baselines, and generates a human-readable report that explains what changed and why it matters for the UI quality and codebase.

## Non-Goals
This plan does not replace functional UI testing or end-to-end correctness checks. It focuses on visual regression and UX quality signals.

## Summary of Approach
1. Add deterministic UI scenarios that auto-load known files and reach a steady UI state.
2. Capture screenshots programmatically or via headless display, with strict environment control.
3. Compare screenshots to baselines and emit diff images and numeric metrics.
4. Generate an AI-assisted review report that describes what the screenshot shows and how it impacts UI quality.
5. Gate in CI with a small scenario set.

## Key Design Principles
- Determinism first. Eliminate layout drift by controlling window size, theme, fonts, and timing.
- Small, curated scenarios. Keep the default set fast and stable.
- Clear artifacts. Every run emits PNGs, diff images, and a JSON summary.
- Explainability. Reports should map what changed to UI guidelines and likely code locations.

## Proposed Repo Additions
New files and directories are shown below. The list is descriptive; it does not require these exact names, but consistency matters.

- `desktop/ui_scenarios/README.md`
- `desktop/ui_scenarios/<scenario>/old.xlsx`
- `desktop/ui_scenarios/<scenario>/new.xlsx`
- `desktop/ui_scenarios/<scenario>/scenario.json`
- `desktop/ui_snapshots/README.md`
- `desktop/ui_snapshots/<scenario>/baseline.png`
- `desktop/ui_snapshots/<scenario>/baseline.json`
- `desktop/ui_snapshots/<scenario>/diff.png`
- `desktop/ui_reports/<scenario>/<git-sha>.md`
- `scripts/ui_capture.sh`
- `scripts/ui_diff.js`
- `scripts/ui_review.py`
- `docs/ui_guidelines.md`
- `docs/ui_visual_regression.md`

## Scenario Definition
Each scenario should define a deterministic UI state that can be reproduced from scratch.

Each `scenario.json` should contain:
- `name`
- `description`
- `old_path`
- `new_path`
- `auto_run_diff` set to true
- `expect_mode` such as `normal` or `large`
- `stable_wait_ms` to ensure UI has settled
- `focus_panel` to keep capture consistent

## Desktop App Changes
Add a development-only scenario harness to the desktop app.

Required behavior:
- When `EXCEL_DIFF_DEV_SCENARIO=<name>` is set, the app loads `desktop/ui_scenarios/<name>/scenario.json`.
- It opens the `old_path` and `new_path`, runs a diff, and waits for `stable_wait_ms`.
- It sets a deterministic window size when `EXCEL_DIFF_WINDOW_SIZE=WxH` is set.
- It disables auto-maximize when running scenarios.
- It optionally forces a known theme when `EXCEL_DIFF_UI_THEME=<theme>` is set.
- It optionally disables animations when `EXCEL_DIFF_UI_DISABLE_ANIM=1` is set.

## Screenshot Capture Strategy
Start with a robust external capture method, then optionally add an in-app capture path.

### Phase A: Headless External Capture
- Linux target via `xvfb-run`.
- Use a fixed window title and geometry.
- Use `xdotool` to activate the window and `import` from ImageMagick to capture.
- Default output to `desktop/ui_snapshots/<scenario>/<git-sha>.png`.

### Phase B: Optional In-App Capture
- Add a desktop debug command that uses wxWidgets to capture the window contents.
- This removes dependency on `xdotool` and reduces timing flakiness.

## Visual Diff and Metrics
Add a diff script that emits both a diff PNG and metrics JSON.

Requirements:
- Input: baseline PNG and new PNG.
- Output: diff PNG, mismatch percent, and pixel count.
- Define a strict threshold for failures, such as `0.15%` mismatch for stable screens.

Implementation options:
- Node script using `pixelmatch`.
- ImageMagick `compare` as a fallback.

## AI Review Report
Add a review script that summarizes the screenshot and ties it to UI quality.

Inputs:
- New screenshot path.
- Diff metrics JSON.
- `docs/ui_guidelines.md`.
- Optional: relevant files like `desktop/wx/ui/main.xrc` and `desktop/wx/src/main.rs`.

Output:
- Markdown report under `desktop/ui_reports/<scenario>/<git-sha>.md`.
- Sections should include:
  - What the UI shows.
  - What changed since baseline.
  - Impact on usability or visual consistency.
  - Likely code locations.
  - Suggested follow-up checks.

## CI Integration
Add a lightweight CI job that runs a small scenario set.

Requirements:
- Runs in headless Linux with a fixed window size.
- Captures screenshots and diffs them against baselines.
- Fails if mismatch exceeds thresholds.
- Uploads artifacts for review on failure.

## Dependency Inventory
Linux packages for CI or local capture:
- `xvfb`
- `xdotool`
- `imagemagick`

Node packages for diff:
- `pixelmatch`
- `pngjs`

Python packages for review script if needed:
- `pillow`

## UI Guidelines Document
Add `docs/ui_guidelines.md` and keep it short and prescriptive.

Suggested contents:
- Font choices, sizes, and weight rules.
- Spacing scale and padding rules.
- Alignment and grid rules.
- Primary action placement rules.
- Color and contrast rules.
- Constraints for empty states and error states.

## Minimal Initial Scenario Set
Define a small set that runs quickly and is stable.

Suggested scenarios:
- `compare_basic` with a small diff that lights up summary and details panels.
- `compare_large_mode` that triggers large mode.
- `pbix_no_mashup` to ensure error handling UI stability.

## Rollout Phases

**Phase 1: Foundations**
1. Add `docs/ui_guidelines.md` and `docs/ui_visual_regression.md`.
2. Add `desktop/ui_scenarios/` and at least one scenario.
3. Add the env-var based scenario harness to the desktop app.

**Phase 2: Capture and Diff**
1. Add `scripts/ui_capture.sh` with deterministic window sizing.
2. Add `scripts/ui_diff.js` and baseline storage under `desktop/ui_snapshots/`.
3. Document a local workflow in `docs/ui_visual_regression.md`.

**Phase 3: Review and CI**
1. Add `scripts/ui_review.py` to generate reports.
2. Add CI job to run capture and diff.
3. Upload `desktop/ui_snapshots/` and `desktop/ui_reports/` artifacts.

**Phase 4: Hardening**
1. Add optional in-app capture to reduce headless flakiness.
2. Add more scenarios and tune thresholds.
3. Add a “baseline update” command with explicit approval.

## Determinism Checklist
Use this checklist in the implementation to reduce flaky diffs.

- Fixed window size via `EXCEL_DIFF_WINDOW_SIZE`.
- Disable auto-maximize via `EXCEL_DIFF_START_MAXIMIZED=0`.
- Disable animations via `EXCEL_DIFF_UI_DISABLE_ANIM=1`.
- Force a known GTK theme if needed.
- Ensure the scenario harness waits for a stable UI state.
- Prefer X11 backend for headless capture.

## Example Local Workflow
This is the shape of the developer workflow; final commands may change.

1. `EXCEL_DIFF_DEV_SCENARIO=compare_basic EXCEL_DIFF_WINDOW_SIZE=1280x720 EXCEL_DIFF_START_MAXIMIZED=0 cargo run -p desktop_wx --bin desktop_wx`
2. `scripts/ui_capture.sh compare_basic`
3. `node scripts/ui_diff.js compare_basic`
4. `python3 scripts/ui_review.py compare_basic`

## Success Criteria
- A new contributor can run one command and get a screenshot and report in under two minutes.
- Baseline updates are explicit and reviewable.
- Visual diffs are stable across runs on the same machine.
- Reports highlight meaningful UI regressions with minimal noise.
