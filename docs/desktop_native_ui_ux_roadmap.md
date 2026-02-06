# Native Desktop UI/UX Roadmap (XRC)

**Last updated:** 2026-02-06  
**Primary UI:** native wx/XRC (`desktop/wx/ui/main.xrc` + `desktop/wx/src/main.rs`)  
**Non-primary UI:** WebView (`EXCEL_DIFF_USE_WEBVIEW=1`) is optional and must not be required for a good experience.

This document is an execution roadmap for improving the desktop UI/UX while keeping the native XRC UI the primary product surface.

## Goals

- Make the Compare workflow obvious on first use (directionality, required inputs, defaults).
- Make results scannable and navigable on real-world workbooks (hundreds of sheets, large diffs).
- Improve perceived responsiveness during long operations without sacrificing determinism.
- Keep the UI consistent with `docs/ui_guidelines.md` and stable for the visual regression pipeline.

## Non-goals (for this roadmap)

- Replacing the XRC UI with the WebView UI.
- Major diff engine/parsing performance work (tracked separately in `docs/desktop_perf_ux_improvement_plan.md`).
- Large-scale visual redesign that breaks platform-native expectations.

## Constraints and Guardrails

- Deterministic layout: changes must be compatible with `docs/ui_visual_regression.md`.
- Avoid wide-scope churn: prefer targeted edits to `desktop/wx/ui/main.xrc` and `desktop/wx/src/main.rs`.
- Accessibility basics: clear labels, tooltips where truncation exists, readable contrast, keyboard paths for core actions.

## Current UX Map (Native UI)

- Compare tab:
  - Old/New file pickers, preset choice, "Trusted files" toggle, Compare/Cancel.
  - Progress gauge + status text + status pill.
  - Split view: left sheets list (DataView), right results tabs (Summary, Details, Grid).
- Recents tab: list + "Open Selected".
- Batch tab: old/new folders + include/exclude globs + results list.
- Search tab: query + scope + build index buttons + results list.

## Roadmap

### Milestone 0: Baseline and UX Debt Inventory (1-2 sessions)

**Why:** lock in repeatable validation before changing the UI, and get crisp “before/after” artifacts.

- Add or refine deterministic UI scenarios in `desktop/ui_scenarios/` to cover:
  - Empty state (no paths selected).
  - Small diff with multiple changed sheets.
  - Large mode run (sheet payload loading on selection).
  - Error states (unsupported extension, container limits, canceled).
- Capture baselines and confirm the pipeline is stable:
  - `./scripts/ui_capture.sh compare_grid_basic --tag <tag>`
  - `python3 scripts/ui_snapshot_summary.py desktop/ui_snapshots/compare_grid_basic/runs/<tag>.json`
- Add a short “Native UX checklist” section to scenario descriptions to prevent regressions (status text, enabled/disabled states, focus).

**Primary files**
- `desktop/ui_scenarios/**`
- `docs/ui_visual_regression.md`

**Validation**
- `cargo run -p desktop_wx --bin xrc_smoke`
- One scenario capture and diff.

### Milestone 1: Compare Flow Clarity (High impact, low risk)

**Problems observed in current UI**
- Old/New direction is visually subtle; pickers are unlabeled.
- “Next/Previous Difference” shortcuts navigate the sheet list, not differences (trust hit).
- Empty state feels “blank” (Summary/Details start empty).

**Changes**
- Add explicit "Old" and "New" labels (and tooltips) above file pickers.
- Add a `Swap` button for Old/New paths.
- Disable Compare until both paths are present (and show inline help text when missing).
- Replace the initial blank Summary/Details with a guided empty state ("Select old/new, pick preset, Compare").
- Rename menu items to match behavior if they remain sheet-navigation:
  - Example: "Next Sheet" / "Previous Sheet" or "Next Changed Sheet" / "Previous Changed Sheet".
  - Only keep “Difference” wording when there is actual in-sheet diff navigation.

**Primary files**
- `desktop/wx/ui/main.xrc`
- `desktop/wx/src/main.rs`

**Validation**
- Visual regression captures for empty state + post-diff state.
- Manual: Tab-order and keyboard shortcuts still work.

### Milestone 2: Results That Scan (Information design, not more data)

**Problems**
- The sheet list is the primary navigation surface, but it has no quick filter/sort and weak “what should I click next” affordances.
- Status bar shows counts but the main content area is under-utilized for quick understanding.

**Changes**
- Add a filter/search box for sheet names and counts (client-side filter of the virtual table rows).
- Sort and prioritize changed sheets by default (e.g., `Ops desc`, then name).
- Add a compact “Run summary header” above the splitter:
  - Old/New basenames (with full path tooltip).
  - Mode (payload/large), op counts, warnings count, “Complete” status.
- Make “No differences” a first-class state:
  - Clear message and remove empty sheet list confusion.

**Primary files**
- `desktop/wx/ui/main.xrc`
- `desktop/wx/src/main.rs`

**Validation**
- Scenario with zero diffs and scenario with many sheets.

### Milestone 3: Progress and Responsiveness (Perceived speed)

**Problems**
- Gauge is not meaningfully driven today (mostly 0/100).
- Status updates are text-only and can feel jittery.

**Changes**
- Improve progress semantics without fragile % math:
  - Use an indeterminate gauge for long phases.
  - Use stable stage labels already emitted by the backend (`read`, `diff`, `snapshot`) and engine phases (`parse`, `alignment`, etc).
- Optional (if worth it): extend `ProgressEvent` to include `percent` for a few reliable steps only.
- Make cancel behavior explicit:
  - When cancel requested: status becomes "Cancel requested (finishing current step)..."
  - On cancel completion: show “Canceled” state, keep previous results intact unless explicitly cleared.

**Primary files**
- `desktop/backend/src/events.rs`
- `desktop/backend/src/diff_runner.rs`
- `desktop/wx/src/main.rs`

**Validation**
- Manual: cancel mid-run leaves UI in coherent state.
- Visual regression capture for “working” and “canceled”.

### Milestone 4: Sheet Selection UX (Payload vs Large mode)

**Problems**
- Large mode loads payload per sheet; UI currently jumps from "Loading sheet payload..." to either JSON or grid without richer context.

**Changes**
- On selection in large mode:
  - Load lightweight `SheetMeta` first (dims, truncation, preview note) and display immediately.
  - Then load full sheet payload lazily (and only render full JSON when Details tab is opened, matching the existing deferred behavior).
- Improve selection affordances:
  - Double-click on a sheet selects it and switches to Grid tab (configurable).
  - Keyboard: Enter opens Grid, Ctrl+Enter opens Details.

**Primary files**
- `desktop/backend/src/diff_runner.rs` (already has `SheetMeta`)
- `desktop/wx/src/main.rs`

**Validation**
- Large mode scenario where sheet payload is slow enough to observe.

### Milestone 5: Error UX That Offers a Next Action

**Problems**
- Errors are presented as code+message only; some are actionable (trusted retry, unsupported file, corrupt file).

**Changes**
- When an error is retryable with “Trusted files” (`trusted_retry=true`):
  - Show a dialog: “This file hit safety limits. Retry as trusted?” with explicit consequences.
  - One-click toggles `Trusted files` + reruns.
- Deep-link error docs:
  - Provide a “What does this mean?” action that opens `docs/errors.md` (or shows a short excerpt).
- Preserve last successful result on a failed run; do not clear the sheet list unless the user changed inputs.

**Primary files**
- `desktop/backend/src/diff_runner.rs` (trusted retry flag already exists)
- `desktop/wx/src/main.rs`
- `docs/errors.md`

**Validation**
- Scenario that triggers container limit errors (or a controlled mock path).

### Milestone 6: Polish Pass (Consistency, spacing, theme)

**Problems**
- Native UI is light-themed; embedded grid preview HTML is dark, which can feel disjoint.

**Changes**
- Align grid preview visuals with the native theme:
  - Either light-mode grid preview by default, or provide a native toggle.
- Ensure spacing matches `docs/ui_guidelines.md` (4/8/12/16/24 px scale) and avoid one-off borders.
- Review column widths and truncation:
  - Add tooltips where values are clipped (recents, batch errors).

**Primary files**
- `desktop/wx/src/theme.rs`
- `desktop/wx/src/grid_preview.rs`
- `desktop/wx/ui/main.xrc`

**Validation**
- Visual regression runs across all core scenarios.

## Backlog (Nice-to-haves)

- Persist per-user preferences beyond the current `ui_state.json`:
  - preferred preset, “show sheets panel” default, last used tab.
- Export ergonomics:
  - After export, show “Open” and “Reveal in folder” actions (native UI already has `open_path()` helpers).
- More in-app discoverability:
  - Add a Help panel or “Getting started” section in the Compare tab empty state.

