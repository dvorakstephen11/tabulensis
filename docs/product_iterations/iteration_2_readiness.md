# Iteration 2 Readiness (PBIP/PBIR/TMDL)

Date: 2026-02-08

Iteration 2 (as defined in `product_roadmap.md`) is: **PBIP import + compare, PBIR diff viewing, and TMDL diff viewing** with a Git-oriented UX.

## Readiness Summary

The repo is **ready to start Iteration 2**, with one big caveat:

- The current “happy path” UX, payloads, and mental model are **workbook/grid centric** (sheets, cells, and worksheet-ish ops).
- PBIP/PBIR/TMDL are **project/text centric** (folders, JSON-like documents, model metadata), so we should plan Iteration 2 as a new “surface” that still reuses the same core primitives (normalization -> diff -> explain -> presentation).

## What Is Already In Place (Good News)

- A proven core diff pipeline (parse -> diff -> summarize) with a clear separation between:
  - `core/` (engine)
  - `desktop/backend/` (orchestration + storage)
  - `ui_payload/` (DTOs + protocol for UIs)
- Desktop app orchestration already supports multiple “modes” and already has patterns for:
  - Aggregated analysis for large runs (DB-side aggregation)
  - Noise filters and toggles
  - “Explain” as a first-class UI concept
- Perf harness + perf-cycle discipline exists, which matters if we end up parsing large PBIP repos.

## Main Gaps/Risks For Iteration 2

- **Data model mismatch:** PBIP/PBIR/TMDL diffs won’t map cleanly onto “sheet/cell” views.
  - We need a *new canonical diff artifact* for project diffs: documents + entities + locations.
- **Normalization policy will dominate quality:**
  - PBIR JSON is noisy (ordering, GUID churn, generated fields).
  - TMDL is text but benefits from AST-ish normalization and stable IDs.
- **UX requirements differ:**
  - Users want “what changed in the report/model?” at page/visual/measure granularity, not “what cell changed?”.
- **Fixture strategy is required up-front:**
  - PBIP projects vary widely by Power BI version and feature usage.

## Recommended Iteration 2 Breakdown (Phased)

### Phase 0: Decision Spike (1-2 days)

Define the canonical representation and scope boundaries.

- Decide what “diff” means for each artifact:
  - PBIP: folder tree + file-by-file normalize+diff
  - PBIR: normalized JSON semantic diff and a viewer
  - TMDL: normalized representation and a viewer
- Decide the first “golden” user journey:
  - Folder-to-folder compare of two PBIP project roots with a high-level summary and drilldown.

### Phase 1: Ingestion + Normalization (3-6 days)

Goal: deterministically parse + normalize without caring about UI yet.

- Build a PBIP project loader that:
  - Walks the repo tree
  - Identifies PBIR/TMDL artifacts
  - Loads documents with stable paths/IDs
- Implement normalization:
  - PBIR: canonical JSON ordering + GUID-noise suppression rules
  - TMDL: whitespace/comment normalization, stable block ordering, stable identifiers

Output: a deterministic “normalized project snapshot” suitable for diffing.

### Phase 2: Diff + Summary + Explain (4-8 days)

Goal: generate a high quality diff and presentation-oriented summary.

- Define a new diff op taxonomy for project diffs:
  - DocumentAdded/Removed/Changed
  - EntityAdded/Removed/Changed (visual, page, measure, relationship, etc.)
  - PropertyChanged with stable paths (JSON pointer / TMDL path)
- Implement:
  - Summary counts by category and severity
  - Explain generation tuned for PR review (“This visual changed because…”)

### Phase 3: Desktop Viewer UX (4-8 days)

Goal: ship a usable desktop experience for PBIP diffs.

- Add a “PBIP” entry path in the desktop app:
  - Folder pickers instead of file pickers
  - A viewer pane that is document/entity oriented
- Add a simple and fast “details” view:
  - Side-by-side normalized text/JSON
  - Collapsible sections per page/visual/entity

### Phase 4: Git UX Kit + CLI Support (2-5 days)

- `.gitattributes` + difftool templates
- CLI commands for:
  - `diff pbip <old_dir> <new_dir>`
  - export formats for PR-friendly output

## Rough Time Estimate

Assuming one engineer, focused work:

- Best case (tight scope, few surprises): **~2 weeks**
- Typical (normalization churn + viewer iteration): **~3-4 weeks**
- Worst case (artifact variability + edge cases): **~5+ weeks**

## Preflight Checklist

- [x] Pick 3 representative PBIP repos as fixtures (small/medium/large).
- [x] Write down normalization rules for PBIR and TMDL (what to ignore, what to stabilize).
- [x] Decide the first viewer UX (document list + drilldown vs page/visual tree).
- [x] Define a minimal diff op taxonomy for Iteration 2 MVP.
- [x] Add a perf baseline for PBIP parsing/diffing once Phase 1 lands.

