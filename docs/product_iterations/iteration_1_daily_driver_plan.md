# Iteration 1 Plan: "Make It a Daily Driver"

**Last updated:** 2026-02-08  
**Roadmap anchor:** `product_roadmap.md` -> "I upgrades (make it a daily driver)"

## 0) Executive Summary

Iteration 1 is a UX and workflow iteration: reduce time-to-answer on real workbooks by making results scannable, adding noise controls, and providing a first "why did this number change?" explanation surface. This iteration should be shippable without expanding Excel OpenXML coverage or changing the core diff algorithm.

Primary surfaces:
- Desktop native UI (wx/XRC): `desktop/wx/ui/main.xrc`, `desktop/wx/src/main.rs`
- Desktop backend summary/store: `desktop/backend/src/**`
- Web/WASM demo: `web/**`, `wasm/**`
- Shared payload/summary helpers: `ui_payload/**`

## 1) Goals (What "Done" Looks Like)

1. A user can open a diff and answer "what changed?" in under 30 seconds without reading raw JSON.
2. A user can toggle off common noise classes (formatting-only changes, moved-block spam, etc.) and see summary counts update accordingly.
3. A user can click a changed value and get a best-effort explanation: formula change vs upstream query/model change, plus pointers to the relevant artifacts.
4. Users can save and reuse comparison profiles ("presets") beyond `fastest/balanced/most_precise`.
5. Basic performance instrumentation is visible in the UI, and limit-triggered partial results are obvious.

## 2) Non-Goals (Explicitly Not In Scope)

- New artifact coverage (pivot diffs, conditional formatting, comments, etc.). Those are Iteration 3+.
- Any "write back" feature (merge/apply diffs). Iteration 4+.
- Building a complete dependency/lineage graph across query/model/pivot consumers. Iteration 1 only ships best-effort linking.
- Major engine refactors or tuning changes (avoid perf-risk scope creep).

## 3) Current State (What Already Exists)

Desktop:
- Run summary header exists (paths + mode/op counts/warnings/complete): `desktop/wx/ui/main.xrc`, `desktop/wx/src/main.rs`.
- Sheet list supports filtering and sorts by ops (already helpful for scanning): `desktop/wx/ui/main.xrc`, `desktop/wx/src/main.rs`.
- Summary/Details tabs are currently plain text (good for debug, not for scanning): `desktop/wx/ui/main.xrc`.

Data model:
- Ops already carry semantic classification fields for key noise controls:
  - `DiffOp::QueryDefinitionChanged.change_kind` (semantic vs formatting-only vs renamed): `core/src/diff.rs`.
  - `DiffOp::MeasureDefinitionChanged.change_kind` (semantic vs formatting-only vs unknown): `core/src/diff.rs`.
  - `DiffOp::CellEdited.formula_diff` (formatting-only vs semantic vs filled, etc.): `core/src/diff.rs`.
- Engine can suppress formatting-only M and DAX changes via config (`semantic_noise_policy`): `core/src/config.rs`, `core/src/m_diff.rs`, `core/src/model_diff.rs`.

Web demo:
- Summary "cards" (added/removed/modified/moved) already exist: `web/render.js`, `web/main.js`.
- A filter UI exists for grid-oriented views (focus rows/cols, structural, moved-only, etc.): `web/render.js`, `web/main.js`.

## 4) User-Facing Feature Spec

### 4.1 Change Summary Panel (Workbook-Level and Sheet-Level)

Deliver a scannable Summary view that answers:
- What changed? (counts, categories)
- Where did it change? (top sheets/artifacts)
- How risky is it? (warnings, severity)

Minimum Summary content:
- Added/Removed/Modified/Moved cards (existing counts).
- "Categories" breakdown (at least):
  - Grid (sheet structure + cells)
  - Power Query (M)
  - Model (DAX/tabular)
  - Objects (charts, named ranges, VBA)
- "Top changes" list (top N sheets/artifacts by severity then op-count).
- "Run health": `complete` flag, warnings count, and a visible warning banner when incomplete.

Desktop UI target:
- Summary tab becomes a structured panel, not a single raw text dump.
- Details tab keeps the raw debug text (and config JSON), for power users.

Web target:
- Keep existing summary cards, add category + severity summaries, and add deep-links to the "why" view.

### 4.2 Noise Controls (First-Class)

Noise controls must be:
- Deterministic (no AI).
- Reflected in summary counts and sheet ordering (so filters feel "real").
- Saved as part of a comparison profile (see 4.4).

Noise controls to ship in Iteration 1:
- Toggle: "Hide formatting-only M changes"
  - Engine-level option: set `semantic_noise_policy=SuppressFormattingOnly` (preferred).
  - UI-level fallback (if needed): filter ops where `QueryDefinitionChanged.change_kind=FormattingOnly`.
- Toggle: "Hide formatting-only DAX changes" (when DAX semantic diff enabled)
  - Engine-level option: `semantic_noise_policy=SuppressFormattingOnly` plus `enable_dax_semantic_diff=true`.
  - UI-level fallback: filter ops where `MeasureDefinitionChanged.change_kind=FormattingOnly`.
- Toggle: "Hide formatting-only formula changes" (when formula semantic diff enabled)
  - UI-level filter: hide `CellEdited` where `formula_diff=FormattingOnly`.
- Toggle: "Collapse moved blocks"
  - UI-level change: group `BlockMoved*` ops by `move_id` (desktop store already computes move_id in `diff_ops`).
  - UI-level behavior: show a single move item per move_id; hide subordinate cell edits inside moved regions (best-effort; start with just collapsing move ops).
- Toggle: "Semantic-only mode (Power Query + DAX)"
  - A single convenience switch that:
    - enables semantic diff where available (`enable_m_semantic_diff`, `enable_dax_semantic_diff`)
    - sets `semantic_noise_policy=SuppressFormattingOnly`
    - leaves grid diff behavior unchanged

Important behavior rule:
- Toggling noise controls should not silently change the underlying diff unless it is explicitly an "Engine noise suppression" control. If a control changes engine config, the UI should communicate that it will re-run the diff (or require rerun), and should persist both the raw and filtered views if feasible.

### 4.3 "Why Did This Number Change?" View (Best-Effort v1)

Goal: provide a small, reliable explanation surface that helps users find the most likely upstream cause, without pretending to fully solve lineage.

Definition of "v1":
- Works well for the common cases:
  - changed cell formula (semantic or textual change)
  - changed Power Query output (query definition changed; sheet load metadata indicates the sheet is query-backed)
  - changed DAX measure definition (if model diff is present)
- For unknown cases, show "No strong attribution available" and provide navigational pointers rather than guesses.

UI placement:
- Desktop: add an "Explain" action in the Grid tab (context menu or button), plus a read-only panel in the Details tab that shows the explanation for the selected cell.
- Web: add an "Explain" button for selected cell diffs and a sidebar/detail pane.

Explanation content (minimum):
- Cell identity: sheet + address.
- Old/New values and formulas (if present), plus `formula_diff`.
- Relevant upstream changes:
  - Power Query: query name(s) likely backing this sheet (see "Linking heuristics" below), and the `QueryDefinitionChanged` summary (semantic vs formatting-only).
  - Model: measures/tables changed (if present), focusing on semantic changes first.
- A "jump list": links to the relevant Query/Model change items in the changes list.

Linking heuristics (v1):
- If a query has `QueryMetadataChanged(field=LoadToSheet)` and the new value references a sheet name, consider it a candidate for that sheet.
- If query group path suggests the sheet name (simple substring match), consider it a weak candidate.
- If no metadata exists, do not guess.

### 4.4 Saved Comparison Presets ("Profiles")

Iteration 1 should introduce user-facing profiles that are broader than the engine preset.

Profile contains:
- Engine preset: `fastest|balanced|most_precise`
- Trusted files toggle
- Noise controls (4.2)
- Optional hardening limits (max memory, timeout, max ops, on-limit behavior)
- Optional semantic toggles (M/formula/DAX semantic diff)

Profiles to ship:
- Built-in profiles:
  - "Default (Balanced)"
  - "Finance model review" (biased toward semantic diffs and noise suppression; may map to most-precise)
  - "Data pipeline workbook" (noise suppression + stronger hardening limits; biased toward stability)
  - "Power BI model review" (DAX semantic enabled, formatting-only suppressed)
- User-defined profiles:
  - Save current settings as a named profile
  - Rename/delete profiles
  - Export/import a profile JSON (optional; if added, keep it local-only and do not include file paths)

Storage:
- Desktop: new file `compare_profiles.json` in the app data dir (do not overload `ui_state.json` with an unbounded list).
- Web demo: localStorage (namespaced key).

### 4.5 Performance Instrumentation and Run Health

Must show:
- Duration: started/finished and computed elapsed time.
- Mode: payload vs large.
- Ops count.
- Complete flag and warnings list, with clear "incomplete" banner.

Nice-to-have (dev mode, optional for Iteration 1):
- Engine perf metrics if built with `perf-metrics`:
  - phase timings, peak memory, cells compared, moves detected
- Display rule: only show the metrics panel when present; do not require the feature.

## 5) Technical Plan (Implementation Breakdown)

### 5.1 Data/Model Work (Shared)

1. Add a shared op classification helper used by both desktop and web.
   - Rust location: `ui_payload` (preferred) or `core` (if it must be shared outside UI payloads).
   - Output:
     - `OpCategory`: Grid, PowerQuery, Model, Objects, Other
     - `OpSeverity`: Low/Medium/High
     - `OpNoiseClass`: FormattingOnly, RenameOnly, Structural, Move, ValueChange, FormulaChange, Unknown
2. Define severity rules (deterministic and conservative).
   - High:
     - Query semantic changes
     - DAX semantic changes
     - Formula semantic changes
     - Duplicate key clusters
     - Any incomplete run (warnings present)
   - Medium:
     - Grid structural changes (row/col adds/removes, rect replaced)
     - Move ops
     - Unknown semantic classification
   - Low:
     - Formatting-only changes
     - Pure renames
3. Define how summary counts should respond to noise filters.
   - Counts displayed in Summary should reflect the active noise filter set.
   - Run health (complete/warnings) must always reflect the underlying run, not filtered view.

### 5.2 Desktop UI Work (wx/XRC)

1. Summary tab redesign:
   - Add a "cards row" (Added/Removed/Modified/Moved) similar to web.
   - Add a Category breakdown table.
   - Add a "Top sheets/artifacts" list sorted by severity then op count.
   - Keep `summary_text` only as an expandable "Raw summary" section or move it to Details tab.
2. Noise controls UI:
   - Add a "Filters" panel near the sheet list (or in the Summary tab) with the Iteration 1 toggles.
   - Persist toggles in the selected profile (see 5.4).
3. "Explain" UI:
   - Add a right-click/context action for the selected diff cell in Grid view (or a button).
   - Add an explanation render panel (simple read-only text is acceptable for v1).
4. Plumbing:
   - Ensure filter changes trigger:
     - pure UI filter (no rerun) when possible
     - rerun when the filter requires changing engine config (semantic-only suppression)

Validation:
- Update/extend deterministic UI scenarios in `desktop/ui_scenarios/**`.
- Capture a new baseline run for at least `compare_grid_basic` and a scenario with M changes (new scenario may be needed).
- Follow `docs/ui_visual_regression.md`.

### 5.3 Desktop Backend / Store Work

1. Ensure we can compute category counts efficiently for both payload and large modes.
   - Payload mode: compute from in-memory `DiffReport.ops`.
   - Large mode: compute via SQL from `diff_ops.kind` groupings (avoid loading all ops).
2. Add an API for "summary breakdown" if needed (desktop backend already returns `DiffRunSummary`).
   - Option A: extend `DiffRunSummary` (requires schema version bump and migration).
   - Option B (preferred for Iteration 1): add a new RPC endpoint returning a computed breakdown for a `diff_id`.
3. If engine perf metrics are enabled:
   - Extend store schema to persist metrics JSON per run, or return it transiently for payload mode.
   - Keep this behind a feature or env flag to avoid release risk.

### 5.4 Web/WASM Demo Work

1. Expand summary UI:
   - Category + severity display in the summary panel.
   - Filter state visible and sticky.
2. Add Iteration 1 noise controls:
   - Implement filtering based on op fields already present in the payload (query change kind, formula_diff, etc).
3. Add "Explain" view for a selected cell:
   - Implement minimal heuristics (see 4.3).
4. Add comparison profiles:
   - Profile dropdown
   - Save/rename/delete in localStorage

### 5.5 CLI (Optional, Only If Low Cost)

Iteration 1 is primarily UI, but two CLI improvements are optionally in-scope if they are cheap:
- A `tabulensis summary --format concise` output that matches the Summary cards/categories.
- A `--noise-policy suppress-formatting-only` convenience flag mapping to `semantic_noise_policy`.

## 6) Test Plan

### 6.1 Unit Tests

- Rust:
  - Op classification (category/severity/noise) for representative ops: `ui_payload` or `core`.
  - Summary recomputation under filters (ensure counts stable).
- JS:
  - Web view-model filtering and summary recompute: `web/test_view_model.js`, `web/test_outcome_payload.js`.

### 6.2 Integration Tests

- Desktop backend:
  - Breakdown queries against `diff_ops` (large mode) return correct counts without full-load.
  - Profile persistence and round-trip JSON schema.

### 6.3 UI Regression

- Update scenarios:
  - Existing: `desktop/ui_scenarios/compare_grid_basic`
  - Add: a scenario with Power Query diffs and a scenario with formula semantic diffs.
- Run capture + summary validation:
  - `./scripts/ui_capture.sh compare_grid_basic --tag iter1_premerge`
  - `python3 scripts/ui_snapshot_summary.py desktop/ui_snapshots/compare_grid_basic/runs/iter1_premerge.json`

## 7) Performance Plan

Iteration 1 should not require the full perf cycle unless engine behavior is modified in perf-sensitive paths.

Default gate:
- `python scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv`

If we add new SQL aggregation queries for large mode:
- Add a micro-benchmark (or at least a regression test) to ensure summary breakdown does not become O(total ops loaded into Rust).

If `perf-metrics` is enabled in desktop:
- Ensure metrics collection is not enabled in release builds unless explicitly desired.

## 8) Security/Privacy Considerations

- Profiles must not store file contents, only settings.
- Profile export/import must avoid including file paths by default (or must clearly warn).
- "Explain" view must be local-only and deterministic; do not call external services.

## 9) Packaging / Pricing / Marketing Notes (Iteration 1)

Pricing:
- No SKU change required for Iteration 1.

Marketing:
- Update positioning from "diff tool" to "explains changes":
  - "Change Summary"
  - "Noise suppression"
  - "Why did this number change?"

## 10) Work Breakdown and Estimates

Assumptions:
- Single developer.
- No major engine changes.
- Desktop native UI is the primary ship target; web demo is updated in parallel but can lag by a few days.

Estimated effort:
- Summary panel + category/severity (desktop + web): 2-4 days
- Noise controls (desktop + web) including rerun semantics: 2-5 days
- "Explain" v1 (desktop + web): 3-7 days
- Profiles (desktop + web): 2-5 days
- Tests + UI scenarios + docs: 2-4 days

Total:
- Lean scope (best-effort explain, minimal profiles): ~1-2 weeks
- Full scope (robust explain UX, import/export profiles, metrics): ~2-3 weeks

## 11) Checklist (Suggested Order)

- [x] Define OpCategory/Severity/NoiseClass mapping (Rust + JS).
- [x] Implement category + severity summary computation for payload mode (Rust) and web demo (JS).
- [x] Add large-mode category counts via SQL aggregation (desktop backend).
- [x] Redesign desktop Summary tab to structured layout; keep raw data in Details.
- [x] Add noise control UI and persistence in profiles.
- [x] Implement rerun semantics and UI messaging for engine-changing toggles.
- [x] Implement "Explain" v1 on web, then port to desktop.
- [x] Add/extend UI scenarios and refresh deterministic baselines.
- [x] Add unit/integration tests for classification and filtering.
- [x] Run quick perf thresholds suite and sanity-check results (full perf cycle: `benchmarks/perf_cycles/2026-02-08_123202/`).
