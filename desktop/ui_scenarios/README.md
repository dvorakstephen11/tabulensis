# UI Scenarios

UI scenarios define deterministic states for screenshot capture and visual regression.

Each scenario lives under `desktop/ui_scenarios/<name>/scenario.json` and should include:
- `oldPath` + `newPath`: paths to input files (relative to the scenario directory or absolute).
- `autoRunDiff`: whether the app should run a diff automatically.
- `stableWaitMs`: how long to wait after diff completion before signaling readiness.
- `focusPanel`: which UI panel to focus (compare/recents/batch/search/summary/details).
- `expectMode`: `payload` or `large` (optional, used for readiness metadata).

## Included scenarios
- `compare_basic`: uses committed templates (`fixtures/templates/`) so it works out of the box.
- `compare_large_mode`: uses generated fixtures (`fixtures/generated/`) and may require fixture generation.
- `pbix_no_mashup`: uses generated PBIX fixtures (`fixtures/generated/`) and may require fixture generation.

Use `EXCEL_DIFF_DEV_SCENARIO=<name>` to load a scenario.
