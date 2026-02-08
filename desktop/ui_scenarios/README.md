# UI Scenarios

UI scenarios define deterministic states for screenshot capture and visual regression.

Each scenario lives under `desktop/ui_scenarios/<name>/scenario.json` and should include:
- `oldPath` + `newPath`: paths to input files (relative to the scenario directory or absolute). Required when `autoRunDiff=true`.
- `autoRunDiff`: whether the app should run a diff automatically.
- `stableWaitMs`: how long to wait before signaling readiness when `autoRunDiff` is false.
- `cancelAfterMs`: optional; when set (and `autoRunDiff=true`), the scenario will trigger Cancel after the given delay.
- `readyOnStage`: optional; when set (and `autoRunDiff=true`), signal UI readiness on the first matching progress stage (e.g. `diff`) to capture "working" mid-run states.
- `focusPanel`: which UI panel to focus (compare/recents/batch/search/summary/details/grid).
- `expectMode`: `payload` or `large` (optional, used for readiness metadata).

## Included scenarios
- `compare_empty`: empty state (no paths selected). Validates first-run UX and control states.
- `compare_no_diffs`: identical workbooks. Validates the first-class “No differences” state and empty sheets panel UX.
- `compare_basic`: small diff using committed templates (`fixtures/templates/`) so it works out of the box.
- `compare_many_sheets`: multi-sheet diff to validate default sorting and the sheets filter box.
- `compare_multi_sheet`: sheet + grid changes to validate sheets list density and Grid preview behavior.
- `compare_grid_basic`: single-cell change to exercise the Grid preview tab.
- `compare_power_query`: Power Query (M) diffs to validate category summary + noise filters.
- `compare_formula_semantic`: formula semantic diff fixture to validate formula_diff classification + Explain v1 wiring.
- `compare_large_mode`: large workbook to exercise large-mode UI behavior.
- `compare_canceled`: cancels a long-running diff to validate cancel UX and end-state messaging.
- `compare_working`: captures a mid-run progress state to validate stage labels + progress gauge behavior.
- `compare_unsupported`: unsupported extension to validate error UX and messaging.
- `compare_container_limits`: ZIP entry count exceeds default container limits to validate safety-limit error UX.
- `pbix_no_mashup`: PBIX fixture to validate error and messaging UI state.

## Canonical CI scenario set

The opt-in CI visual regression job (PR label: `ui-visual`) runs a small canonical set:

- `compare_grid_basic`
- `compare_large_mode`
- `pbix_no_mashup`

Keep this set small and stable. Adding scenarios should be an explicit, reviewed change (and should
include baseline PNGs).

## Native UX checklist (use during review)

These checks are intentionally small and stable. If a scenario capture regresses, look here first.

- Status: top status text and bottom status bar should be coherent and match the scenario intent.
- Focus: the scenario `focusPanel` should be selected (root tab and result tab where applicable).
- Control states: Compare/Cancel enabled/disabled should match the current run state.
- Sheets list: for scenarios with diffs, sheet rows should be visible (not clipped) and column headers readable.
- Grid preview: when focused on `grid`, the preview should render (or a clear placeholder should explain why).
- Errors: error code + message should be visible in Details, with no silent failure or blank screen.

Use `EXCEL_DIFF_DEV_SCENARIO=<name>` to load a scenario.
