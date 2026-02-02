Below is a concrete plan to add an in-app visual diff viewer for Excel workbook comparisons in this codebase, centered on a **true grid visualization** for grid-related diffs (cell edits, row/col inserts/deletes, moved blocks, replaced regions), while still surfacing non-grid changes in a structured, reviewable way.

I’ll assume the goal is a customer-grade “review changes” experience: fast orientation, easy navigation, and clear trust signals when previews are limited.

---

## Product outcome and UX goals

### What customers should be able to do (core workflow)

1. **Orient**: Immediately understand *what changed* and *which sheets* are impacted.
2. **Inspect**: Click a sheet and **see changes in a grid**, not just a list of operations.
3. **Navigate**: Jump to next/previous change; filter by change type; search.
4. **Understand**: For any highlighted cell/row/rect, see old vs new values/formulas and the “why” (moved vs edited vs replaced).
5. **Trust**: When the preview is partial (budgets / caps), the UI must say so clearly and offer alternatives (e.g., export audit workbook).

### Design principles (to keep the UI “obvious” and scalable)

* **Grid-first** for sheet diffs: people think in cells and ranges, not JSON.
* **Progressive disclosure**: start with “change hunks” / focused regions; allow expanding to larger context.
* **Graceful degradation**: never “freeze”; for large sheets or enormous diffs, switch to region-based rendering and summaries.
* **Keyboard-first review**: next/prev change, search, copy cell details.
* **Explain the model**: show aligned row/col headers when structural changes exist.

---

## Where this fits in the existing architecture

You already have almost everything needed on the backend side:

* The diff runner supports **two modes**: `Payload` vs `Large` and can load per-sheet payload on demand via `load_sheet_payload`. 
* UI payloads already include:

  * **Sparse sheet snapshots** (cells with value/formula, plus truncation note). 
  * **Row/column alignments** for visualizing structural diffs, with explicit “insert/delete/move” axis entries and skip reasons for very large sheets. 
* Snapshot/preview guardrails exist:

  * Structural preview caps: **200 rows x 80 cols** (used to choose interesting regions). 
  * Alignment caps: **10,000 rows** / **200 cols** (preview disabled beyond this). 
* The wx desktop app already supports a WebView UI and an RPC bridge with methods like `loadSheetPayload` and `exportAuditXlsx`. 

So the plan is mainly a **front-end visual diff viewer**, plus a small set of backend/RPC enhancements to make large cases pleasant.

(Also: the current legacy desktop UI shows JSON/summary text only. )

---

## Target UI: information architecture

### Top-level “Results” layout

* **Left sidebar: Sheet Navigator**

  * Sheet name
  * Counts (added/removed/modified/moved)
  * Badges for special states: added sheet, removed sheet, renamed sheet
  * Filter toggles: “Only changed sheets”, “Only structural changes”, “Only moved”, etc.

* **Main panel: Sheet Diff Viewer**

  * Header: sheet name, change summary, preview status (“preview limited” if truncated)
  * Tabs:

    1. **Grid** (default when sheet has grid ops)
    2. **Non-grid changes** (named ranges, queries, VBA, charts, model…)
    3. **Operations list** (raw, filterable table; also acts as navigation)

* **Bottom/right inspector (collapsible)**

  * Selected cell/range details: old vs new value, formula, change classification
  * Move metadata (move group id, src/dst ranges)
  * Copy buttons (“Copy old value”, “Copy new value”, “Copy A1 address(es)”)

### Grid view: two complementary modes

1. **Aligned side-by-side grid (primary)**

   * Old grid on left, new grid on right
   * Synchronized scrolling
   * Row/col headers reflect alignment:

     * Insert row/col: blank placeholder on old side with “+”
     * Delete row/col: blank placeholder on new side with “–”
     * MoveSrc / MoveDst: markers with a move-id badge
   * Cell-level highlights for edits

   Alignment data is already produced as `SheetAlignment { rows: Vec<AxisEntry>, cols: Vec<AxisEntry>, moves: Vec<MoveGroup> }`. 

2. **Change hunks view (best for huge sheets / skipped alignment)**

   * Shows a vertical list of “regions of interest” (like diff hunks in code review)
   * Each hunk renders a small side-by-side mini-grid (e.g., up to 30x20, with context)
   * Great when alignment is skipped due to row/col caps or when rendering a full aligned grid would be noisy

This directly matches how snapshots are already generated: `collect_interest_rects(...)` builds rectangles around cell edits, row/col ops, rect moves, etc. 

---

## Data-to-visual mapping (the heart of the grid viewer)

### 1) Normalize what the UI needs into a “SheetViewModel”

For a selected sheet, the web UI should build (client-side) an in-memory view model:

* `rowAxis[]`: from `alignment.rows` (or a synthetic axis if alignment skipped)
* `colAxis[]`: from `alignment.cols`
* `oldCellMap`: map `(row,col) -> {value, formula}` from snapshot cells
* `newCellMap`: same for new
* `changeMarkers`:

  * row markers: added/removed/replaced/moved
  * col markers: added/removed/moved
  * rect markers: moved rect, replaced rect
  * cell markers: edited cell

The raw diff ops are available inside `DiffWithSheets.report`. 

### 2) Coordinate systems (avoid subtle bugs)

You’ll display in **aligned axis coordinates**, not raw workbook coordinates:

* Let `i` be an index into `rowAxis`, `j` be an index into `colAxis`.
* Each axis entry gives:

  * `oldRow?: u32`, `newRow?: u32`
  * `oldCol?: u32`, `newCol?: u32`
* For a displayed cell `(i,j)`:

  * Old cell lookup uses `(oldRow, oldCol)` if both exist
  * New cell lookup uses `(newRow, newCol)` if both exist
  * If one side is missing, render an “empty placeholder” for inserts/deletes

This makes structural diffs readable and keeps both sides visually aligned.

### 3) Derive render “cell state” from ops

For each cell/row/col/rect, determine a primary visual classification:

* **Inserted row/col** (green-ish accent + “+” marker in header)
* **Deleted row/col** (red-ish accent + “–” marker)
* **Moved blocks** (distinct style + move badge; hover highlights both src/dst)
* **Edited cell**:

  * value changed
  * formula changed
  * both changed
  * “equivalent after shift” (if formula diff indicates) — show in inspector
* **Replaced row/rect**:

  * treat as “large replacement”; highlight entire region and show a message like “Dense change; individual cell edits suppressed.”

Why this matters: the diff engine emits `RowReplaced` / `RectReplaced` for dense edits rather than many cell edits. 
Your grid view must treat those as first-class operations, not “missing detail.”

### 4) What to display inside cells

Default display: show the **value**.
On hover / selection: show value + formula.

Snapshot cells store formula with the leading `=` already applied in `push_cell`. 

---

## Implementation plan by layers

## Layer A — UI foundation choice

### Preferred: build the visual diff viewer in the WebView UI (recommended)

Rationale:

* Grid virtualization and rich interactions are dramatically easier and more consistent in a web UI.
* You already have a WebView mode and an RPC bridge including `loadSheetPayload` and `exportAuditXlsx`. 

**Deliverable:** a self-contained `web/` UI that can render the new grid diff viewer and drive the existing backend via RPC.

### Optional: integrate visuals into the legacy wx UI incrementally

Even if you keep the current legacy interface, you can add a **“Visual” tab** that hosts a WebView instance and uses the same viewer. This avoids duplicating grid rendering in wx widgets and keeps a fallback for environments without web assets.

The legacy UI today only shows counts or JSON in a text control when selecting sheets. 
This is the strongest, lowest-risk way to add visuals without rewriting everything at once.

---

## Layer B — Web UI: components and state

### 1) RPC client + session state

Create a thin RPC client in the web UI:

* `diff(oldPath, newPath, options)` -> `DiffOutcome`
* `loadDiffSummary(diffId)` -> summary
* `loadSheetPayload(diffId, sheetName)` -> `DiffWithSheets`
* `exportAuditXlsx(diffId)` -> path or null

These are already implemented server-side in the wx host. 

Maintain a `DiffSession` state:

* `diffId`, `mode`, `summary`
* cached `sheetPayloadsByName` (LRU cache to avoid re-fetching)
* currently selected sheet

### 2) Sheet Navigator

Render from `summary.sheets`:

* group by: changed vs unchanged
* allow sorting by: op_count, name, modified count
* show an icon for “preview limited” once you’ve loaded payload and see `SheetSnapshot.truncated`

### 3) Sheet Diff Viewer shell

Once a sheet is selected:

* if `mode=Payload` and you already have the payload: hydrate immediately
* if `mode=Large`: call `loadSheetPayload` and show a skeleton/progress state

This matches how the backend loads per-sheet payload in large mode. 

---

## Layer C — Grid Diff Viewer (aligned side-by-side)

### 1) Rendering approach (performance + UX)

Use a **2D virtualized grid** for each side, with sticky row/col headers, synchronized scroll.

* DOM approach: a virtualized grid component (e.g., fixed-size cells) that only renders visible rows/cols + small overscan.
* Keep cell rendering very lightweight (text truncation, minimal DOM nesting).
* Provide zoom by changing cell size + font scale.

Why: even though snapshots are sparse, the alignment axes can be large (up to 10k x 200 before preview is disabled). 
You must not render a full matrix.

### 2) Synchronized scrolling

* A single scroll container drives both grids.
* Each grid uses the same `rowAxis`/`colAxis` and computes old/new cell content based on axis entries.
* Selection/hover should highlight both sides.

### 3) Headers: show alignment semantics

Row header shows:

* old row number (if present)
* new row number (if present)
* markers:

  * Insert (only new row exists)
  * Delete (only old row exists)
  * MoveSrc / MoveDst (move badge)

This uses `AxisKind` and `move_id` from `AxisEntry`. 

Similarly for columns.

### 4) Cell styling rules (readable, accessible)

Don’t rely on color alone:

* Use a subtle background + an icon glyph/border:

  * Added: plus marker
  * Removed: minus marker
  * Modified: outline + dot
  * Moved: dashed outline + move badge
* Provide a legend (collapsible) and tooltips.

### 5) Inspector panel

On selecting a cell:

* show old/new A1 addresses (can be derived from axis mapping)
* show old/new value + formula
* show change category and any move information
* provide copy actions

Note: `CellAddress` serializes as A1 strings already, which is great for UI display and for copying. 

---

## Layer D — Change Hunks View (for large sheets / better review)

### Why it matters

Even when alignment is available, customers often prefer reviewing “only what changed” rather than scrolling through mostly empty context.

The backend snapshot system already chooses rectangles around meaningful ops, including moved blocks, replaced regions, row/col ops, and duplicate key clusters. 

### Implementation options

**Option 1 (client-side hunks):**

* Re-implement the interest-rect generation in the web UI using the same logic:

  * cell edits -> 1x1 rect
  * row ops -> 1 row x previewCols rect
  * col ops -> previewRows x 1 col rect
  * moved blocks -> include both src and dst rects
  * expand by context rows/cols

You can align this with the current caps (200x80, context 1) found in `ui_payload`. 
This is quick, but you must keep logic in sync.

**Option 2 (preferred, server-side hunks):**
Extend `ui_payload` to return `interestRects` per sheet in the payload (they’re already computed internally). 
This makes the UI simpler and ensures parity across desktop/web clients.

### Hunk rendering UX

Each hunk card:

* Title: `Sheet1!A120:D150` (old and new range if different)
* Badges: “moved”, “replaced region”, “row inserted”, etc.
* Mini grid (two panes) with limited size; click “Open in full grid” to jump/zoom.

---

## Layer E — Non-grid diffs: make them visual too (without pretending they are grids)

Grid diffs are the star, but customers also need clarity on:

* Power Query changes
* Named range changes
* VBA module changes
* Model changes (relationships, columns, measures) when enabled
* Charts / metadata

You already export these into separate sheets in the audit workbook (Summary, Warnings, Cells, Structure, PowerQuery, Model, OtherOps). 
Use that as an information architecture guide inside the UI:

* Render these as tables with:

  * change type column
  * entity name
  * old/new values
  * “diff” view for long text (query definitions, M expressions, VBA text)

For long text diffs: use a line-based diff component, and allow “copy old/new”.

---

## Layer F — Backend/RPC enhancements (to make it feel “instant” on big diffs)

You can ship an MVP without changing backend APIs. But to make this maximally useful under real enterprise-sized workbooks, add two strategic capabilities.

### 1) “Open exported audit” / “reveal in file explorer”

When user exports audit XLSX via `exportAuditXlsx`, provide an “Open” button.

* Add an RPC method like `openPath({ path })` that uses OS-specific shell open (you already use `std::process::Command` in the wx host). 
  This avoids the “now go find it manually” pain.

### 2) Range-based loading (advanced, but pays off)

Problem: `loadSheetPayload` returns ops + snapshots. For certain sheets, that can still be huge.

You already store ops in SQLite with indexed fields such as `sheet_id`, `row`, `col`, `row_end`, `col_end`, `move_id`, etc. (see how index fields are derived for ops). 

Add APIs that let the web UI request just what it needs:

* `loadSheetMeta(diffId, sheetName)` -> sheet dimensions, alignment availability, truncation flags
* `loadOpsInRange(diffId, sheetName, rowStart, rowEnd, colStart, colEnd)` -> only ops intersecting a viewport/hunk
* `loadCellsInRange(diffId, sheetName, side, rowStart, rowEnd, colStart, colEnd)` -> sparse cell values/formulas from the workbook cache

This allows:

* true infinite scroll on huge sheets
* “jump to change” without preloading everything
* faster initial render

---

## Layer G — Making preview limitations honest and helpful

The payload already contains explicit signals:

* `SheetSnapshot.truncated` plus a human note like “Preview limited: showing X of Y non-empty cells.” 
* `SheetAlignment.skipped` with a concrete `skip_reason` when rows/cols exceed caps. 

UX requirements:

* Show a small banner in the sheet viewer:

  * “Preview limited” with a short reason
  * Buttons: “Export audit workbook”, “Show change hunks”, “Try loading more context” (if you implement range loading)
* Never silently hide: if alignment is skipped, explicitly switch to Hunks View as default.

---

## Testing strategy (so grid visuals are trustworthy)

### 1) Golden-file fixtures for visual logic

Create a set of small workbook fixtures that exercise:

* single cell edit
* row inserted above edits
* column removed
* moved row block
* moved rect block
* dense row replaced / dense rect replaced
* renamed sheet
* added/removed sheet

For each fixture, validate:

* alignment axis correctness (in `ui_payload`)
* change markers mapping correctness (in UI)
* navigation order (“next diff” visits expected coordinates)

### 2) Contract tests for RPC payloads

Add a TypeScript schema validation layer (zod/io-ts) on the web UI side:

* fail fast if payload shape changes
* makes refactors safer

### 3) Performance regression guardrails

Automate:

* time-to-first-grid-render on a moderate sheet
* memory usage for loading a large-mode diff and opening 3–5 sheets
* verify that virtualization keeps DOM node count bounded

---

## Phased rollout plan

### Phase 1 — MVP (high value, low risk)

* Build web UI grid viewer (Aligned + Inspector)
* Implement Sheet Navigator + sheet selection
* Render cell edits + row/col insert/delete + rect replaced + moved blocks (highlight + inspector)
* Show preview-limit banners and fall back to Hunks View when alignment is skipped
* Wire `exportAuditXlsx` into the UI

Uses existing APIs as-is (`loadSheetPayload`, etc.). 

### Phase 2 — Review ergonomics

* Next/prev change navigation within a sheet
* Search integration (hook into existing backend search endpoints if desired)
* Better move visualization: hover highlights both src/dst; click “jump to counterpart”
* “Show formulas” toggle + formula diff explanation in inspector

### Phase 3 — Big-sheet excellence

* Add range-based loading APIs
* Implement viewport ops/cells fetching for near-infinite sheets
* Optional: “minimap” overview of change distribution

### Phase 4 — Desktop polish and defaults

* Make the visual UI the default desktop experience (remove reliance on an env flag), while keeping legacy fallback
* Add “Open exported audit workbook” / “Reveal in folder”
* Ship web assets reliably (bundle `web/` next to the binary, consistent with index resolution logic)

---

## What you’ll end up with

A desktop experience where customers can:

* pick a sheet, immediately see a familiar spreadsheet-like grid,
* understand structural changes via aligned headers,
* inspect exact old/new values and formulas,
* review large diffs safely via hunks and progressive loading,
* and always have an escape hatch (export audit workbook) when previews are necessarily limited.

If you’d like, I can also outline a clean TypeScript data model (types/interfaces) that mirrors the Rust payloads (`DiffWithSheets`, `SheetSnapshot`, `SheetAlignment`, `DiffOp`) and the exact marker derivation rules for each grid-relevant `DiffOp` variant—so implementation stays deterministic and testable.
