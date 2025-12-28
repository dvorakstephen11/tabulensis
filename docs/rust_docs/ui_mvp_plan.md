What you’re describing (“visual diff” that sort-of tracks row/col adds but doesn’t *visually match the real edits*) is almost always a **row/column alignment problem**, not a “can’t show values” problem.

In your current web demo, you actually *do* have access to real cell values and formulas via the workbook snapshots returned from the WASM layer, and the renderer does display them.  But the grid renderer looks up `oldCell` and `newCell` using the **same `(row, col)` coordinates on both sides**, and then colors those cells based on ops like `RowAdded/RowRemoved/CellEdited`.  That breaks the moment a row is inserted/moved, because the “corresponding row” in old vs new is no longer the same numeric index—so the viewer starts comparing the wrong rows, which creates exactly the “passing resemblance” effect you’re seeing.

Below is a phased plan that closes that gap first (truthfulness), then turns it into a polished “spectacular MVP” UI for both web and desktop, with the *same shape* and shared components.

---

## The target UI shape you should standardize on

This is the “shape” that stays consistent across web + desktop (desktop just gets more power features):

**Top bar**

* Pick Old / New files (drag-drop + file picker)
* Diff settings (mode, key columns, ignore options)
* Export (report JSON / HTML; desktop adds more)
* Help (shortcuts, legend)

**Left sidebar (Navigator)**

* Workbook summary
* Sheets list with per-sheet counts (Added/Removed/Modified/Moved)
* Filters: “Only sheets with changes”, search sheet name

**Center (Diff Canvas)**

* Grid diff viewer with:

  * Side-by-side view (default)
  * Unified view (optional toggle)
  * Collapsed unchanged regions (like code diffs)
  * Scroll/zoom + frozen headers
  * “Diff markers” on scroll track / minimap

**Right sidebar (Inspector + Changes)**

* Selected cell inspector:

  * Old value / New value
  * Old formula / New formula
  * Classification (value change, formula change, both, blank->value, etc.)
* Change list (grouped)

  * Click item => jumps to region/cell
  * Next/Prev change navigation

This aligns well with your own differentiation direction: “GitHub Pull Request” semantics (clean overlay diff, not destructive Excel coloring). 

---

## Phase 1 — Make the diff view “truthful” by shipping an explicit alignment model

### Goal

Make the visual grid answer, deterministically and for every rendered cell:

> “Which `(row, col)` should I read from the old snapshot, and which `(row, col)` should I read from the new snapshot, for this on-screen position?”

Once that’s true, everything else (nice styling, collapse/expand, side-by-side, jump-to-change, desktop parity) becomes an “upgrade” instead of a correctness fight.

### Reality check: what’s *actually* happening in this codebase today

* The WASM layer already returns **sheet snapshots** (old + new) that include:
  * `nrows / ncols`
  * `cells[]` with `row`, `col`, `value?`, `formula?`
* The web renderer builds `oldCells` and `newCells` maps keyed by the literal `row,col` pairs from each snapshot, and then it compares `oldCells.get("r,c")` to `newCells.get("r,c")`.

That lookup strategy is only correct when “row i” in the old sheet corresponds to “row i” in the new sheet (positional diff). But your diff engine *does* detect inserts/deletes/moves (and it emits structural ops like `RowAdded`, `RowRemoved`, `BlockMovedRows`, etc.). The moment those happen, “the corresponding row” is no longer the same numeric index — so the UI starts comparing unrelated content.

**Important constraint (codebase-grounded): do not mutate the core `DiffReport` JSON shape in Phase 1.**
There’s an explicit JSON-shape test that expects only the current top-level keys for the core report. Instead, we extend the *WASM payload* (which is already a wrapper around `report + sheets`) with UI-only alignment metadata.

### Key design decisions (lock these down in Phase 1)

1. **Alignment is derived from the diff ops, not recomputed.**
   *Why:* If we rerun alignment as a separate step, we risk producing a mapping that differs from what the engine actually decided (especially around ambiguous regions and move-handling).  
   *How:* The ops already tell us which rows/cols are inserted/removed/moved. For everything else, the remaining rows/cols align in-order.

2. **The UI renders in “aligned view indices”, not old indices and not new indices.**
   *What that means:* the rendered grid has a synthetic axis:
   * a “match” entry consumes 1 old row + 1 new row
   * an “insert” entry consumes 0 old rows + 1 new row
   * a “delete” entry consumes 1 old row + 0 new rows  
   This is exactly how code diffs get a single view that can show additions and deletions without lying.

3. **Moves are represented twice (source and destination) to keep the view monotonic.**
   In a unified grid, the “source location” and “destination location” both matter. Trying to align them into a single row entry makes the axis non-monotonic and creates downstream complexity (jump-to-row, collapse, scroll markers).
   *Phase 1 choice:* represent row/col moves as:
   * `move_src` entries at the **old** location (old-only)
   * `move_dst` entries at the **new** location (new-only)
   and tie them together with a stable `move_id`.

4. **Row/column moves affect the axis; rectangle moves do not.**
   `BlockMovedRect` is a content move *within* the grid, not an axis mutation. It should influence cell highlighting and “move linking”, but it should **not** rewire row/col correspondence.

5. **Guardrail:** if a sheet’s aligned axis would exceed a sanity limit, the UI must stay truthful by not rendering the grid.
   Phase 1 keeps the HTML grid renderer (not fast); we must protect users from accidentally rendering a massive view.
   *Behavior:* show the per-op change list + warning banner, and skip the visual grid for that sheet.

### Output contract: UI alignment added to the WASM payload (not to the core report)

The existing WASM function already returns:

```json
{ "report": { ...DiffReport... }, "sheets": { "old": ..., "new": ... } }
```

Phase 1 extends it to:

```json
{
  "report": { ...DiffReport... },
  "sheets": { "old": ..., "new": ... },
  "alignments": [
    {
      "sheet": "Sheet1",
      "rows": [
        { "old": 0, "new": 0, "kind": "match" },
        { "old": null, "new": 1, "kind": "insert" },
        { "old": 1, "new": 2, "kind": "match" },
        { "old": 2, "new": null, "kind": "delete" },
        { "old": 3, "new": null, "kind": "move_src", "move_id": "r:3+2->10" },
        { "old": null, "new": 10, "kind": "move_dst", "move_id": "r:3+2->10" }
      ],
      "cols": [
        { "old": 0, "new": 0, "kind": "match" }
      ],
      "moves": [
        { "id": "r:3+2->10", "axis": "row", "src_start": 3, "dst_start": 10, "count": 2 }
      ],
      "skipped": false
    }
  ]
}
```

#### Types and invariants

* `sheet` is a resolved sheet name string (matches `SheetSnapshot.name` so the web layer can join without string-table gymnastics).
* `rows[]` / `cols[]` are the aligned view axes.
* An axis entry:

  * `kind: "match"` → `old != null && new != null`
  * `kind: "insert" | "move_dst"` → `old == null && new != null`
  * `kind: "delete" | "move_src"` → `old != null && new == null`
  * `move_id` is present iff `kind` is `move_src` or `move_dst`
* `moves[]` are ranges so the UI can:

  * show “moved from/to” badges on headers
  * link-highlight both ends on hover/click
* `skipped` means “don’t attempt to render the visual grid for this sheet” (but still show ops list + counts).

### How the alignment is built (deterministic, derived from ops + dimensions)

For each sheet:

1. Collect per-sheet structural ops:

   * Row axis:

     * `RowAdded(row_idx)` contributes to `added_new_rows`
     * `RowRemoved(row_idx)` contributes to `removed_old_rows`
     * `BlockMovedRows(src_start_row, dst_start_row, row_count)` contributes:

       * old rows in `[src_start_row, src_start_row+row_count)` → `move_src_rows[old_row] = move_id`
       * new rows in `[dst_start_row, dst_start_row+row_count)` → `move_dst_rows[new_row] = move_id`
   * Col axis:

     * `ColumnAdded(col_idx)` → `added_new_cols`
     * `ColumnRemoved(col_idx)` → `removed_old_cols`
     * `BlockMovedColumns(...)` → `move_src_cols` / `move_dst_cols`

2. For alignment assembly, treat moves as delete+insert (with special kinds):

   * effective removed rows = `removed_old_rows ∪ move_src_rows.keys()`
   * effective added rows = `added_new_rows ∪ move_dst_rows.keys()`

3. Build axis entries with a single linear scan:

Pseudo-code for rows (same for cols):

```
i = 0 // old index
j = 0 // new index
entries = []

while i < old_nrows or j < new_nrows:
  if j < new_nrows and j in (added_new OR move_dst):
    entries.push({ old: null, new: j, kind: move_dst? else insert, move_id? })
    j++
    continue

  if i < old_nrows and i in (removed_old OR move_src):
    entries.push({ old: i, new: null, kind: move_src? else delete, move_id? })
    i++
    continue

  if i < old_nrows and j < new_nrows:
    entries.push({ old: i, new: j, kind: match })
    i++; j++
    continue

  // Tail safety (only one side remaining)
  if i < old_nrows:
    entries.push({ old: i, new: null, kind: delete })
    i++
  else:
    entries.push({ old: null, new: j, kind: insert })
    j++
```

4. Consistency checks:

   * If `old_nrows - effective_removed != new_nrows - effective_added`, the scan still produces a view, but it means we have an inconsistency (likely partial report). In that case:

     * set `skipped = true` for the sheet alignment (visual grid would otherwise lie by forcing pairings)
     * render only the textual op list for that sheet

5. Complexity:

   * Time is `O(old_nrows + new_nrows + move_count)` for rows and similarly for cols.
   * No expensive re-alignment or signature work; this is pure bookkeeping.

### What code is added / changed (grounded to current file layout)

#### WASM layer (`wasm/`)

**1) Add new types (serialized into the existing payload)**

*File:* `wasm/src/lib.rs`

* Change `DiffWithSheets` from:

  * `report: DiffReport`
  * `sheets: SheetPairSnapshot`

  to additionally include:

  * `alignments: Vec<SheetAlignment>`

* Add (or import from a new module):

  * `enum AxisKind { Match, Insert, Delete, MoveSrc, MoveDst }` (serialized as lower-case strings)
  * `struct AxisEntry { old: Option<u32>, new: Option<u32>, kind: AxisKind, move_id: Option<String> }`
  * `struct MoveGroup { id: String, axis: String, src_start: u32, dst_start: u32, count: u32 }`
  * `struct SheetAlignment { sheet: String, rows: Vec<AxisEntry>, cols: Vec<AxisEntry>, moves: Vec<MoveGroup>, skipped: bool }`

**2) Add an alignment builder module**

*File (new):* `wasm/src/alignment.rs`

Functions to add:

* `fn group_ops_by_sheet(report: &DiffReport) -> HashMap<String, Vec<&DiffOp>>`

  * Resolves `op.sheet` via `report.strings[...]` to get a stable sheet name key.

* `fn build_sheet_alignment(sheet: &str, old_sheet: Option<&SheetSnapshot>, new_sheet: Option<&SheetSnapshot>, ops: &[&DiffOp]) -> SheetAlignment`

  * Extracts dims (`nrows/ncols`) from snapshots (or 0 if missing).
  * Builds row/col axes using the scan algorithm above.
  * Extracts `moves[]` and generates stable IDs:

    * row move id: `"r:{src}+{count}->{dst}"`
    * col move id: `"c:{src}+{count}->{dst}"`

* `fn build_axis_entries(old_len: u32, new_len: u32, added: &HashSet<u32>, removed: &HashSet<u32>, move_src: &HashMap<u32, String>, move_dst: &HashMap<u32, String>) -> (Vec<AxisEntry>, bool /*consistent*/)`

  * Returns entries + whether the axis cardinalities line up.

**3) Wire it into the existing entrypoint**

*File:* `wasm/src/lib.rs`

Inside `diff_files_with_sheets_json(...)`:

* Keep:

  * read packages
  * compute `report = pkg_old.diff(&pkg_new, &cfg)`
  * compute `sheets = snapshot_workbook(...)`
* Add:

  * `let alignments = build_alignments(&report, &sheets);`
  * return JSON of `{ report, sheets, alignments }`

No changes to the core diff engine or the core report JSON serializer in Phase 1.

#### Web UI (`web/`)

**1) Plumb the new payload field**

*File:* `web/main.js`

* Parse `payload.alignments || null`
* Call renderer as: `renderReportHtml(report, sheets, alignments)`

**2) Make the renderer use alignment for all cell lookups**

*File:* `web/render.js`

Changes to make:

* Update `renderReportHtml(report, sheets)` to accept `alignments`.
* Build `alignmentBySheetName` for quick lookup.
* In `renderSheetSection(...)`, pass the sheet’s alignment object into `buildSheetGridData(...)`.
* In `buildSheetGridData(...)`:

  * Use `alignment.rows.length` and `alignment.cols.length` as the grid size (view size).
  * Build:

    * `newRowToView` and `newColToView` maps from the alignment arrays (for placing `CellEdited` ops).
  * Build `cellEdits` keyed by **view coordinates**, not raw `new (row,col)`.
* In `renderSheetGrid(...)`:

  * For each `(viewRow, viewCol)`:

    * look up `rowEntry = alignment.rows[viewRow]`, `colEntry = alignment.cols[viewCol]`
    * `oldCell` comes from `(rowEntry.old, colEntry.old)` if both exist
    * `newCell` comes from `(rowEntry.new, colEntry.new)` if both exist
  * Cell classification:

    * if `cellEdits` contains this view cell → render old→new (as today)
    * else if row/col entry is `insert` → show new-only cell
    * else if `delete` → show old-only cell
    * else if `move_src` / `move_dst` → show old-only or new-only cell, styled as “moved”
    * else (`match`) → show unchanged cell (prefer new value if present)

**3) Add minimal UI affordances so moves don’t look like adds/deletes**

*File:* `web/style.css` (or whatever stylesheet owns cell classes)

* New classes:

  * `.cell-move-src` and `.cell-move-dst`
  * header badges for moved ranges
* Update legend to include “Moved” semantics.

### Acceptance checks (make them hard to fake)

1. **Row insertion at top**
   *Old:* a sheet where row 0 is inserted in new.
   *Expected UI outcome:* the entire rest of the sheet stays aligned; unchanged cells remain in the same visual rows.

2. **Mid-sheet row deletion**
   *Old:* delete one row in the middle.
   *Expected:* everything below shifts in view; unchanged rows align.

3. **BlockMovedRows**
   *Old:* move a contiguous block of rows from middle to bottom (or vice versa).
   *Expected:*

   * The moved block appears twice: once as `move_src` at the old location, once as `move_dst` at the destination.
   * Hovering the moved header highlights both ends.
   * No “phantom” cell edits are created by misalignment.

4. **Column insertion + row insertion combined**
   *Expected:* 2D alignment is correct: the right cells are compared, not “same index”.

5. **Safety guard**
   *Use a sheet large enough to exceed the view limit.*
   *Expected:* grid view is skipped with an explicit message; ops list still renders.

---

Grounding notes (what this Phase 1 is explicitly aligned to in the current codebase): the WASM payload is currently `{report, sheets}` via `diff_files_with_sheets_json` and `DiffWithSheets`. :contentReference[oaicite:0]{index=0} The current web renderer’s grid lookup uses identical `(row,col)` keys into `oldCells` and `newCells`, which is exactly what Phase 1 corrects. :contentReference[oaicite:1]{index=1}


---

## Phase 2 — Build a shared “Diff View Model” layer (web + desktop)

### Key terms (so we stay precise)

* **View Model (VM):** a UI-focused representation of the diff that is *derived* from engine output. It answers “what should the user see?” without being tied to how it’s rendered (HTML, canvas, desktop UI, etc.).
* **View coordinates:** the unified row/column index space introduced in Phase 1 (aligned axes). Every UI feature (rendering, selection, navigation, grouping) uses these indices, not raw old/new indices.
* **Region (a.k.a. “hunk”):** a bounding box in view coordinates that encloses a cluster of changes, plus some configurable surrounding context. Regions are how we avoid rendering or navigating 10,000 individual cell edits.

### Goal

Create a single transformation pipeline that *all* UIs use:

**Engine output (DiffReport + snapshots + Phase-1 alignment) → View Model → Renderer**

This is the separation we don’t have today: `web/render.js` currently interprets ops, fetches cells, classifies changes, and emits HTML in one pass. That makes Phase 3 (canvas grid) and any desktop UI likely to re-implement semantics and diverge over time.

### Non-goals (explicit, to keep Phase 2 bounded)

* Do **not** replace the renderer yet (Phase 3 does that). Phase 2 keeps the current HTML output, but makes it consume a VM.
* Do **not** change the core `DiffReport` JSON shape (still treated as engine output).
* Do **not** build the desktop wrapper yet. Phase 2 only ensures the VM layer can be reused by a desktop shell later (same semantics, same contract).

### Design decisions (lock these down in Phase 2)

1. **VM lives in the web UI code as a plain ES module (no build step).**
   *Why this is codebase-realistic:* the current UI is plain `web/*.js` ESM imported by `index.html`, and the Node-based `web/test_render.js` imports `render.js` directly. A plain JS VM module fits both without introducing a bundler or TypeScript.

2. **Payload normalization is part of the VM layer.**
   Today `web/main.js` parses the WASM JSON and then does `payload.report || payload` and `payload.sheets || null`. The VM builder should accept:
   * a raw `DiffReport` object (used by `web/test_render.js` today)
   * a WASM payload wrapper like `{ report, sheets, alignments }` (Phase 1+)
   It should also tolerate the current sheet snapshot nesting shape (`{ old: { sheets: [...] }, new: { sheets: [...] } }`) by normalizing to `oldSheets[]/newSheets[]` internally.

3. **The VM is mostly data, but it may include a hot-path helper function.**
   We want renderers to be “dumb”, but we also need performance. So we allow `sheetVm.cellAt(viewRow, viewCol)` as a function that closes over precomputed indices/maps. Everything else should be JSON-like data so it can be inspected and tested easily.

4. **Numeric keys, not string keys, for cell lookup.**
   The current renderer builds `Map` keys like `"${row},${col}"`. That is fine for a tiny grid, but Phase 3 will call `cellAt` many times per frame. The VM uses numeric keys:
   * `key = row * 16384 + col` (Excel’s max columns is 16,384, so this is collision-free).
   This one choice is the difference between “works in tests” and “feels instant” later.

5. **Precedence rules for classification are defined once in the VM.**
   The renderer never decides whether a cell is “edited vs moved vs added”. It asks the VM and just paints classes. This is how we prevent web and desktop semantics from drifting.

6. **Moves are represented consistently across layers.**
   * Axis moves (rows/cols) come from Phase 1 alignment entries (`move_src` and `move_dst` with a shared `move_id`) and become first-class in the VM’s axis arrays.
   * Rectangular moves (`BlockMovedRect`) become region annotations (source + destination regions linked by a shared `move_id`), but they do not attempt per-cell “moved” classification (too expensive and ambiguous without a full cell mapping).

7. **Compaction is region-first (Git-style hunks), not “render the whole used range”.**
   Even before canvas virtualization, we can make the HTML UI truthful and usable by rendering a small number of region grids (each bounded + context) instead of a single huge bounding box or entire sheet.

---

### Deliverables (concrete, code-level)

#### 1) A canonical `WorkbookVM` and `SheetVM`

**New file:** `web/view_model.js`

Exports a single top-level builder:

* `buildWorkbookViewModel(payloadOrReport, opts?) -> WorkbookVM`

Where `payloadOrReport` can be either:
* a `DiffReport` (legacy path)
* a WASM payload wrapper: `{ report, sheets, alignments }`

**WorkbookVM fields (minimum):**

* `report`: the original report (kept for deep inspection and for existing renderers that still show raw op details)
* `warnings: string[]` (copied from report)
* `counts: { added, removed, modified, moved }` (same meaning as today’s summary cards)
* `sheets: SheetVM[]` (sorted by sheet name)
* `other`: grouped non-sheet ops (VBA, named ranges, charts, Power Query, measures), already resolved to display strings where needed

**SheetVM fields (minimum):**

* `name: string`
* `axis: { rows: AxisVM, cols: AxisVM }`
* `cellAt(viewRow, viewCol) -> CellVM` (fast)
* `changes:`
  * `items: ChangeItemVM[]` (grouped + compact list)
  * `regions: ChangeRegionVM[]` (bounding boxes used for navigation and “visual hunks”)
* `renderPlan:`
  * `regionsToRender: string[]` (ordered region IDs)
  * `status: { kind: "ok" | "skipped" | "missing", message?: string }`

#### 2) A fast `cellAt(viewRow, viewCol)`

**CellVM shape (returned by `cellAt`):**

* `viewRow`, `viewCol`
* `old: { row, col, cell } | null` (null if this view position has no old mapping)
* `new: { row, col, cell } | null` (null if this view position has no new mapping)
* `diffKind: "empty" | "unchanged" | "added" | "removed" | "moved" | "edited"`
* `moveId?: string` and `moveRole?: "src" | "dst"` (only when moved)
* `edit?: { fromValue, toValue, fromFormula, toFormula }` (only when edited)
* `display:`
  * `text: string` (what to render in the grid cell by default)
  * `tooltip: string` (what hover/inspector shows)

**Classification precedence (must be deterministic):**

1. If the cell is in `editMap` → `diffKind = "edited"` (even if its row/col is moved/insert/delete). Attach `moveId/moveRole` if relevant so renderers can style “edited inside moved block” later.
2. Else if either axis entry is `insert` → `diffKind = "added"` (new-only).
3. Else if either axis entry is `delete` → `diffKind = "removed"` (old-only).
4. Else if either axis entry is `move_src`/`move_dst` → `diffKind = "moved"` with `moveRole`.
5. Else → `diffKind = "unchanged"` if there is content on either side, otherwise `"empty"`.

#### 3) Grouping + compaction rules (implemented in the VM builder)

The UI already has a “Detailed Changes” list, but it is one op per line. Phase 2 introduces grouping so it stays reviewable.

**Row/column ranges**
* Build view-index sets for `RowAdded/RowRemoved/ColumnAdded/ColumnRemoved`.
* Sort by view index (so order matches the aligned grid).
* Group consecutive indices into a single item:
  * Example label: `Rows 120–138 added` (1-based, new-side numbering)
  * Example label: `Columns D–F removed` (old-side lettering)

**Moves**
* Prefer Phase-1 axis `move_id` as the canonical move identity.
* Produce one `ChangeItem` per move with both source + destination ranges so the UI can “jump” to either end.

**Cell edits -> regions**
* Each `CellEdited` becomes a 1x1 change point in view space.
* Cluster change points into regions using a deterministic algorithm:
  1. group edits by `viewRow`
  2. compress each row into contiguous column “runs”
  3. merge runs into regions if they overlap (or nearly overlap) in columns and are on adjacent rows
  4. stop region growth when `maxCellsPerRegion` is exceeded (start a new region to keep UI usable)

**Rectangular ops**
* `RectReplaced` becomes a region directly (view-space bounding box).
* `BlockMovedRect` becomes two linked regions (src + dst) that share `move_id`.

**Compacted rendering plan**
* Instead of rendering a single “whole sheet” grid, render one mini-grid per region ID in `regionsToRender`.
* Each mini-grid expands the region bounds by `contextRows/contextCols` (configurable) but is still capped by `maxVisualCells`.
* If there are *no* regions and only non-visual ops (e.g., Power Query changes), the visual section is omitted entirely.

#### 4) Deterministic sorting (one rule, reused everywhere)

VM must define a stable ordering so navigation and tests don’t flake.

* Workbook: sort sheets by name (case-insensitive compare if we want to mirror engine sheet ordering rules).
* Within a sheet:
  1. sort regions by `(topLeftViewRow, topLeftViewCol)`
  2. then by kind order (moves, structural, rect, cell edits)
  3. then by stable `id`

---

### What code is added / changed (grounded to the current file layout)

#### New: `web/view_model.js` (core of Phase 2)

Functions to add (names are explicit so implementation is straightforward):

* `normalizePayload(payloadOrReport) -> { report, sheets, alignments }`
  * Accepts `DiffReport` or `{report, sheets, alignments}`.
  * Normalizes sheet snapshots to `{ oldSheets: [], newSheets: [] }` whether the input is arrays or `{sheets: [...]}` objects.
* `buildWorkbookViewModel(payloadOrReport, opts) -> WorkbookVM`
  * Internally reuses (or moves) the existing `categorizeOps` logic from `web/render.js` so we keep the exact same meaning for counts and section partitioning.
* `buildSheetViewModel({ report, sheetName, ops, oldSheet, newSheet, alignment, opts }) -> SheetVM`
  * Builds `AxisVM` (rows+cols), cell maps, edit maps, regions, and change items.
* `makeAxisVm(entries, sideDims) -> AxisVM`
  * Builds `entries[]` plus `oldToView/newToView` arrays for O(1) mapping.
* `makeCellMap(sheetSnapshot) -> Map<number, SheetCell>`
  * Numeric key version of `buildCellMap`.
* `makeEditMap(report, sheetOps, rowsVm, colsVm) -> Map<number, CellEditVM>`
  * Maps op addresses into view-space keys.
* `clusterEditsToRegions(editKeys, opts) -> ChangeRegionVM[]`
  * Deterministic clustering described above.
* `groupConsecutive(indices) -> Array<{ start, end, count }>`
  * Shared helper for row/col range items.

#### Change: `web/render.js` becomes a pure renderer over the VM

* Add `import { buildWorkbookViewModel } from "./view_model.js";`
* Replace the top-level `renderReportHtml(report, sheetData)` path with:
  * `const vm = buildWorkbookViewModel(payloadOrReportOrReport, { ...defaults });`
  * `return renderWorkbookVm(vm);`
* Keep existing HTML structure and CSS classes where possible so Phase 2 is not a style rework.

#### Change: `web/main.js` passes the payload through

Today it does:

```js
const payload = JSON.parse(json);
const report = payload.report || payload;
const sheets = payload.sheets || null;

byId("results").innerHTML = renderReportHtml(report, sheets);
```

Phase 2 simplifies to:

```js
const payload = JSON.parse(json);
byId("results").innerHTML = renderReportHtml(payload);
```

`renderReportHtml` remains backwards-compatible for the Node test that calls it with a raw report.

#### New: `web/test_view_model.js` (unit tests for correctness)

Add a test that asserts VM-level truthfulness independent of HTML:

* **Row insertion mapping:** unchanged cells map old row i to new row i+1 via view coordinates.
* **Move identity:** `move_src` and `move_dst` share the same `move_id` and classify as moved.
* **Grouping:** 10 consecutive `RowAdded` ops become 1 change item.
* **Region compaction:** 2000 `CellEdited` ops become a small number of regions (bounded by config).

This complements the existing `web/test_render.js`, which should continue to pass without sheets and without alignments.

---

### Acceptance checks (make them hard to fake)

1. **VM correctness beats HTML correctness**
   *Given:* a fixture with a row inserted at the top and no other changes.
   *Assert:* `cellAt(viewRow=1, viewCol=0)` returns `old.row=0` and `new.row=1` (or equivalent for your fixture), and `diffKind === "unchanged"`.

2. **Moves are not displayed as add/delete**
   *Given:* a fixture that emits `BlockMovedRows` (Phase 1 also produces row-axis move entries).
   *Assert:* inside the moved block, `cellAt` reports `diffKind === "moved"` and includes `moveId`.

3. **Compaction works on far-apart edits**
   *Given:* two clusters of edits separated by 10,000 rows.
   *Expected UI behavior:* two mini-grids render (two regions), not one enormous grid.

4. **Graceful degradation**
   *Given:* a report without sheets (like `web/test_render.js` today).
   *Expected:* VM build succeeds, visual diffs are skipped with `status.kind = "missing"`, and the non-grid sections (Power Query, Measures) still render.

### Why this matters

It prevents your web UI and desktop UI from diverging: once Phase 2 exists, Phase 3 can swap renderers (HTML → canvas) without re-implementing diff semantics, and a desktop shell can reuse the exact same VM builder module.

---


---

Below is the current Phase 3 block in `ui_mvp_plan.md` (this is the exact section to replace). 

```md
## Phase 3 — Replace the current HTML grid with a real interactive grid viewer (virtualized)

### Key term: “virtualization”

Instead of creating DOM elements for every cell (which explodes for large sheets), you render only what’s visible, and recycle rows/cells as the user scrolls.

### Goal

A fast grid viewer that feels spreadsheet-native.

### Deliverables

1. **Rendering approach (recommended for MVP): Canvas-based grid**

   * One canvas for the cell area, optional separate canvases for row/col headers
   * Benefits: extremely fast, consistent across web + desktop (same WebView)
   * Supports huge sheets without DOM blowups

2. **Side-by-side mode (default)**

   * Old on left, New on right
   * Synchronized scrolling
   * Row gutter shows +/- markers for inserts/deletes, and a move indicator for moved blocks

3. **Unified mode (toggle)**

   * One grid where:

     * Added rows show only “new”
     * Removed rows show only “old”
     * Edited cells show “old → new” inline (two-line or split-cell rendering)
   * This is the “PR diff” feel applied to a sheet.

4. **Interaction**

   * Hover tooltip: full value + formula (no truncation)
   * Click selects cell; right inspector updates
   * Keyboard: arrows to move, Enter to open inspector, `n/p` to next/prev change (desktop especially)

### Acceptance checks

* Smooth scroll at large sizes (no jank)
* Selecting a cell always shows correct old/new values after row inserts/moves (proof alignment works)

---

## Phase 3 — Replace the current HTML grid with a real interactive grid viewer (virtualized)

### Grounding notes (what this phase is replacing in the current codebase)

Today the “Visual Diff” for a sheet is rendered as a literal DOM grid:

* `web/render.js` builds a `<div class="sheet-grid"> ... </div>` with one `.grid-cell` element per visible cell via `renderSheetGrid()` → `renderAlignedGrid()` (or legacy fallback). The grid relies on CSS `position: sticky` headers and the browser DOM to stay responsive. This is the main source of jank and “skipped” previews as sheets get larger.
* Sheet expand/collapse is currently done via an inline `onclick` on `.sheet-header`, and `web/main.js` simply `innerHTML = renderReportHtml(...)` after parsing the WASM payload.

Phase 3 moves the grid rendering responsibility into a purpose-built viewer that:
1) consumes the Phase 2 `SheetVM` (`axis + cellAt() + changes/regions`), and
2) renders only the visible rows/cols on each frame (virtualization), instead of materializing a DOM node per cell.

This phase is intentionally designed to fit the existing “no build step / plain ES modules / string-rendered HTML” structure.

### Key terms

**View space (aligned coordinates)**  
The grid viewer never “guesses” correspondence. Every rendered cell is addressed in *view* coordinates (`viewRow`, `viewCol`), and the VM maps those to `old(row,col)` and/or `new(row,col)` via the aligned axis.

**Virtualization**  
Render only what is visible in the viewport, and redraw as the scroll offsets change. The cost becomes proportional to viewport size, not sheet size.

**Two display modes**
* **Side-by-side:** the same view-space window is drawn twice (Old pane and New pane), scroll-locked.
* **Unified:** a single pane shows the canonical PR-diff representation.

### Goal

A fast, spreadsheet-native grid viewer that:

* stays truthful to alignment (the selected cell always resolves to the correct old/new mapping),
* supports large aligned views without DOM blowups,
* becomes the foundation for Phase 4’s “review workflow” (jump-to, filtering, next/prev change).

### Non-goals (explicit, so we don’t overbuild)

* Not implementing “real Excel rendering” (merged cells, conditional formats, fonts per cell, column widths from the workbook, etc.).
* Not implementing formula evaluation or recalculation.
* Not implementing edit-in-place.
* Not implementing perfect parity with Excel scroll/selection semantics (we only need reviewer-grade navigation).

### Design decisions (the “complicated calls” made explicitly)

#### 1) Canvas 2D (not DOM, not WebGL)

**Decision:** Use Canvas 2D rendering for the cell area and headers.

**Why this is the right MVP tradeoff in this codebase:**
* The current renderer is DOM-heavy by construction; Canvas removes the “N DOM nodes per cell” scaling problem.
* Canvas works in the current web demo (plain `index.html` + module imports), and it is the same primitive a desktop WebView will use.
* WebGL is unnecessary complexity for a MVP grid (text, borders, background fills, simple highlight overlays).

#### 2) Fixed cell metrics (row height / column width)

**Decision:** Use fixed metrics that match the existing CSS grid’s feel:
* `rowHeightPx = 36` (matches `.grid-cell { min-height: 36px; }`)
* `colWidthPx = 100` (matches the existing `minmax(100px, 1fr)` intent)
* `rowHeaderWidthPx = 50` (matches the current `grid-template-columns: 50px ...`)

**Why:** Variable sizing looks nicer but makes virtualization, hit-testing, and scroll math much harder. Fixed metrics keep the viewer deterministic and fast.

A later phase can introduce:
* “auto-fit column” (per visible region only),
* optional “compact density mode” (smaller rowHeight/colWidth).

#### 3) One scroll model for both panes (side-by-side)

**Decision:** Side-by-side uses one shared scroll model (`scrollTop`, `scrollLeft`), and draws Old and New panes from the same scroll offsets.

**Why:** This guarantees that “Old row X / New row Y” alignment stays visually locked. It also avoids “two scrollbars drifting apart”.

#### 4) Truth-first classification and styling comes from the VM

**Decision:** The grid viewer does not re-derive diff semantics from ops. It asks the VM for:
* `AxisEntryVM` for headers (insert/delete/move markers)
* `CellVM` for each visible view cell (`diffKind`, `moveId`, display text, tooltip text, old/new cell payload)

**Why:** The current codebase already has one place where classification is complicated (`renderAlignedGrid` decides moved-vs-deleted-vs-inserted-vs-edited). That logic must live in a single “source of truth” (Phase 2 VM), so Phase 3 can focus on rendering + interaction.

#### 5) Move highlighting behavior

**Decision:** Moves are treated as “paired endpoints” linked by `moveId`, with hover and selection behavior:

* Hover a moved header or moved cell:
  * highlight *that endpoint* (src or dst) with a stronger tint
  * also “ghost highlight” the corresponding paired endpoint so the relationship is obvious
* Click a moved header:
  * selects that moved region and exposes “Jump to other end” in the inspector (Phase 3 can implement the jump, even if Phase 4 adds the richer workflow)

**Why:** Without this, moved blocks are visually confusing; the current HTML grid already carries `move_id` data in headers/cells, but it isn’t leveraged yet.

### Viewer UX: what the user can do in Phase 3

#### Modes
1. **Side-by-side (default)**
   * Left pane: Old values/formulas
   * Right pane: New values/formulas
   * Shared row header gutter and column headers
   * Same structural markers (+/-/M) as the current grid header styling

2. **Unified (toggle)**
   * Inserted rows/cols show new content only
   * Deleted rows/cols show old content only (old text drawn with strike-through)
   * Edited cells show a two-line rendering:
     * old (smaller, strike-through, red)
     * new (slightly larger, green)

#### Interaction
* **Hover tooltip:** a positioned DOM tooltip (not `title=`) showing full value + formula for old/new, with the exact old/new addresses and view address.
* **Click selection:** click selects a cell; selection is visually outlined; the inspector updates.
* **Keyboard (when viewer has focus):**
  * Arrow keys move selection by 1 cell in view space
  * `Shift+Arrow` expands selection rectangle (optional, if easy)
  * `Enter` toggles “pin inspector” (keeps the inspector visible even when mouse moves)
  * `n` / `p` jumps to next/previous *change anchor* (see below)

**Change anchor definition (deterministic):**
* Use Phase 2 `sheetVm.changes.regions[]` sorted by `(minViewRow, minViewCol)`.
* The anchor point for a region is `(minViewRow, minViewCol)`.
* “Next/prev” moves between anchors and scroll-centers the viewport on the anchor.

### Implementation plan (concrete, tied to current file layout)

#### Step 1: Replace the HTML grid markup with a viewer mount point

In the renderer (Phase 2 `web/render.js`), the “Visual Diff” section should output:

* a stable mount `<div>` that the browser JS can find and hydrate
* enough `data-*` to connect it to the correct `SheetVM` and initial state

Example shape (exact attribute names are part of the plan so it’s implementable):

* `div.grid-viewer-mount`
  * `data-sheet="SheetName"`
  * `data-initial-mode="side_by_side"`
  * `data-initial-anchor="0"` (optional; index into region anchors)

The renderer remains a pure string builder (so `web/test_render.js` remains viable). No DOM work happens in `render.js`.

#### Step 2: Add a dedicated grid viewer module (browser-only hydration)

**New file:** `web/grid_viewer.js`

Exports:
* `mountSheetGridViewer({ mountEl, sheetVm, opts }) -> { destroy(), focus(), jumpTo(viewRow, viewCol), jumpToRegion(idx) }`

Responsibilities:
* Build the internal DOM structure inside `mountEl`:
  * toolbar (mode toggle, optional “show formulas” toggle placeholder)
  * viewer layout (headers + scroll container + canvas)
  * inspector panel (right side)
  * tooltip overlay
* Own all event listeners and redraw scheduling.

Important constraint for this repo:
* `grid_viewer.js` is imported by `web/main.js` only (so Node-based renderer tests are not coupled to Canvas/DOM APIs).

#### Step 3: Add a pure painter module for Canvas rendering

**New file:** `web/grid_painter.js`

Exports:
* `paintGrid(ctx, paintModel)` where `paintModel` contains:
  * `sheetVm`
  * `mode` ("side_by_side" | "unified")
  * viewport info (`scrollTop`, `scrollLeft`, `viewportWidth`, `viewportHeight`)
  * metrics (rowHeight, colWidth, header sizes)
  * selection/hover state (`selectedCell`, `hoveredCell`, `hoverMoveId`)
  * theme colors (see below)

Painter responsibilities:
* Compute visible view-row/view-col ranges:
  * `firstRow = floor(scrollTop / rowHeight)`
  * `rowCount = ceil(viewportHeight / rowHeight) + 1`
  * same for cols
* For each visible cell:
  * call `sheetVm.cellAt(viewRow, viewCol)` and render based on `diffKind`
  * draw background fill, border, and cell text
  * in unified mode, render “edited” as two-line old/new text

Painter must be deterministic:
* same inputs -> same pixels (no random jitter, no reliance on map iteration order)

#### Step 4: Theme integration (reuse the existing CSS variables)

**New file:** `web/grid_theme.js`

Exports:
* `readGridTheme(rootEl) -> GridTheme`

Implementation:
* read CSS variables from `getComputedStyle(document.documentElement)`:
  * `--bg-primary`, `--bg-secondary`, `--bg-tertiary`
  * `--border-primary`, `--border-secondary`
  * `--diff-add-bg`, `--diff-remove-bg`, `--diff-modify-bg`, `--diff-move-bg`, `--diff-move-border`
  * `--accent-green`, `--accent-red`, `--accent-yellow`, `--accent-purple`

Add (small) CSS variable for move-destination tint so we don’t hardcode RGBA in JS:
* `--diff-move-dst-bg` (matches current `.cell-move-dst` background intent)

#### Step 5: Hook hydration into the existing `web/main.js` lifecycle

Modify `web/main.js` (after Phase 2 VM introduction) so `runDiff()` does:

1. `payload = JSON.parse(diff_files_with_sheets_json(...))`
2. `workbookVm = buildWorkbookViewModel(payload, opts)`
3. `resultsEl.innerHTML = renderWorkbookVm(workbookVm)` (Phase 2 renderer)
4. `hydrateGridViewers(resultsEl, workbookVm)` (Phase 3)

Hydration function responsibilities:
* locate all `.grid-viewer-mount` elements
* mount viewers lazily:
  * eagerly mount only for the initially expanded sheet
  * mount on expand for subsequent sheets (event delegation rather than inline onclick)

This phase should also remove the inline `onclick` in `renderSheetSection` and attach a real click listener in `main.js` so:
* expand/collapse logic lives in one place
* viewer mounting can be reliably triggered when a sheet becomes expanded

#### Step 6: Minimal CSS additions in `web/index.html`

Because styles are currently in the `<style>` block, Phase 3 adds:

* layout container for viewer + inspector:
  * `.grid-viewer { display: grid; grid-template-columns: 1fr 320px; gap: 16px; }` (values tuned in implementation)
* canvas wrapper styling:
  * `.grid-canvas-wrap { position: relative; border: 1px solid var(--border-primary); border-radius: 12px; overflow: hidden; background: var(--bg-primary); }`
  * `.grid-scroll { overflow: auto; height: 520px; }` (MVP fixed height; later can become responsive)
* tooltip styling:
  * `.grid-tooltip { position: absolute; pointer-events: none; ... }`
* inspector styling:
  * `.grid-inspector { background: var(--bg-secondary); border: 1px solid var(--border-primary); border-radius: 12px; padding: 12px; }`

### What code is added / changed (explicit inventory)

#### New files

1. `web/grid_viewer.js`
   * `mountSheetGridViewer(...)`
   * internal state model:
     * `mode`
     * `scrollTop/scrollLeft`
     * `selected { viewRow, viewCol }`
     * `hover { viewRow, viewCol, moveId? }`
     * `anchorIndex` for n/p navigation
   * event wiring:
     * scroll -> schedule redraw via `requestAnimationFrame`
     * pointermove -> hover hit-test + tooltip
     * click -> selection + inspector update + focus
     * keydown -> selection move + n/p anchor jumps
   * lifecycle:
     * `destroy()` removes listeners and observers

2. `web/grid_painter.js`
   * `paintGrid(ctx, model)`
   * helpers for:
     * cell background selection (from `diffKind` + move role)
     * text truncation to cell width (canvas `measureText` with small cache)
     * drawing strike-through for old text in unified mode
     * drawing selection outline and hover outline

3. `web/grid_theme.js`
   * `readGridTheme(...)`
   * `resolveCssVar(name, fallback)` helper

(Optional but recommended for cleanliness)
4. `web/grid_metrics.js`
   * constants + “max scroll size clamp” logic (so we have one place to adjust metrics)

#### Changed files

1. `web/render.js`
   * remove DOM-heavy HTML grid generation in the Visual Diff section
   * instead emit `.grid-viewer-mount` placeholders
   * remove inline onclick on `.sheet-header` (so expand/collapse can be owned by `main.js`)
   * ensure Node render test remains stable (no Canvas imports; renderer stays string-only)

2. `web/main.js`
   * build VM (Phase 2) and render VM (Phase 2)
   * hydrate viewers after `innerHTML` insertion (Phase 3)
   * implement expand/collapse listeners and lazy viewer mounting
   * ensure first sheet auto-expands and hydrates (keeps current behavior)

3. `web/index.html` (style block only)
   * add viewer/inspector/tooltip styles
   * add any new CSS vars used by the canvas theme (e.g., `--diff-move-dst-bg`)

(If we want to unlock “huge sheet support” beyond the Phase 1 HTML guardrails)
4. `wasm/src/alignment.rs` (optional but strongly recommended once the HTML grid is removed)
   * revisit the `MAX_VIEW_CELLS`-based skip logic:
     * that limit existed to prevent rendering a massive DOM grid
     * with virtualization, the viewer cost scales with viewport size, so `rows * cols` is no longer the right skip metric
   * keep row/col axis length sanity caps (they bound JSON size), but relax/remove the “cells” cap

### Acceptance checks (manual + deterministic)

#### Performance / feel
* Smooth scroll (60fps-ish) in a sheet that would previously skip or lag (large view axis; sparse content).
* No “DOM explosion” as you scroll; memory remains stable (no growing node count).

#### Correctness / truthfulness
* Pick a workbook with:
  * a row insertion above an edited cell
  * a moved row block with at least one edited cell inside it
* Clicking the visually edited cell shows:
  * correct old address/value/formula
  * correct new address/value/formula
  * correct diffKind classification (edited beats moved/insert/delete)

#### Navigation
* `n` cycles through region anchors in stable order; `p` goes backward.
* Jump centers the viewport on the anchor, and selection updates accordingly.


---

## Phase 4 — Navigation, filtering, and "review workflow" polish

### Goal

Make the UI *reviewable as a workflow*, not just “viewable as a demo”.

Phase 3 gives us a real canvas viewer with selection, hover, an inspector, and per-sheet `n/p` navigation through `sheetVm.changes.regions`. Phase 4 turns that into an experience where a reviewer can:

* find the sheets that matter,
* walk every meaningful change (including structural row/col changes, not just cell-edit regions),
* jump to a change from the list and immediately see *where* it is in the grid,
* filter the grid to focus on what changed,
* understand warnings/limitations without guessing.

### Grounding notes (what exists in the current codebase)

The plan below is specifically designed to fit the existing architecture:

* The HTML is rendered as a string in `web/render.js` via `renderWorkbookVm(vm)` / `renderSheetVm(sheetVm)` and inserted with `innerHTML` (no framework). Sheets are `<section class="sheet-section" data-sheet="...">` with `.sheet-header` toggling `.expanded`.  
* The grid viewer is mounted lazily by `hydrateGridViewers(rootEl, workbookVm)` in `web/main.js`. It finds `.grid-viewer-mount` inside expanded sheets and calls `mountSheetGridViewer({ mountEl, sheetVm, opts })`.  
* “Detailed Changes” is already grouped (rows/cols/cells/moves/other) using `sheetVm.changes.items`, but items are currently static, non-clickable rows.  
* `sheetVm.renderPlan.status.kind` already exists (`ok | skipped | missing`) and is used to decide whether a `.grid-viewer-mount` exists or a warning message is shown.  
* Viewer navigation (`n/p`) is currently tied to `sheetVm.changes.regions` only, meaning “next change” may skip important *structural* changes (row/col add/remove/moves) that are not represented as regions.

Phase 4 builds directly on those facts; it does **not** introduce a framework or change the “string-rendered HTML + JS hydration” model.

---

### Key model: “review anchors” + “review cursor”

**Review anchor**  
A “review anchor” is a *navigable point* in a sheet that represents a meaningful change. It is the unit of:

* next/prev navigation,
* jump-to from the changes list,
* “pulse highlight” feedback when arriving somewhere.

In Phase 4, anchors must include:
* cell/rect change regions (already in `changes.regions`)
* row/col additions/removals/replacements (structural)
* row/col moves (Phase 1 move identities)
* block moved rectangles (source and destination endpoints)

**Review cursor**  
The “review cursor” is the current anchor within the workbook. It is a workbook-level concept (not just per sheet) so we can implement “Next/Prev change across sheets” in a deterministic order.

---

### Deliverables (concrete, code-level)

#### 1) Add `SheetVM.changes.anchors` as the canonical navigation sequence

**Design decision:** anchors are VM-owned, not renderer-owned.  
We do *not* derive navigation by parsing human-readable strings (labels) or DOM position. The VM already has the aligned view indices and knows what each change means; it should also own the “where do I jump?” decision so the semantics stay consistent across web + desktop.

**Add to `SheetVM.changes`:**

* `anchors: ChangeAnchorVM[]`

Where:

```ts
type ChangeAnchorVM = {
  id: string                    // stable ID used for syncing + jump-to
  group: "rows" | "cols" | "cells" | "moves" | "other"
  changeType: "added" | "removed" | "modified" | "moved"
  label: string                 // short human string ("Rows 10–12 added", "Cells B2:C4 modified", ...)
  // Primary target for navigation
  target:
    | { kind: "grid", viewRow: number, viewCol: number, regionId?: string, moveId?: string }
    | { kind: "list", elementId: string } // fallback for skipped/missing sheets
  // Optional: additional context shown in UI ("to row 50", "to column D", ...)
  detail?: string
}
```

**Anchor ID scheme (stable, deterministic):**

* Cell/rect/move regions: `region:${region.id}`
* Row structural: `row:${changeType}:${startViewRow}-${endViewRow}`
* Col structural: `col:${changeType}:${startViewCol}-${endViewCol}`
* Row/col moves (Phase-1 move id):

  * `move:${moveId}:src` and `move:${moveId}:dst`
* “Other” (non-grid ops on a sheet): `other:${op.kind}:${idx}` (only if we decide to anchor them; acceptable to omit for MVP)

**Ordering (within a sheet):**

1. Sort by target location in view space: `(viewRow, viewCol)`
2. Tie-break by group priority (moves first, then structural, then rect, then cell), then `id`
   This preserves today’s region sort behavior and makes “next change” feel spatially consistent.

**How anchors get created (grounded in current VM):**

* For each `changes.regions` entry:

  * create a grid anchor at `(region.top, region.left)` with `regionId: region.id` and optional `moveId`
* For row/col add/remove/replace items:

  * create a grid anchor at `(startViewRow, 0)` or `(0, startViewCol)` (if axis length is zero, fall back to list anchor)
* For row/col moves:

  * create two anchors (src + dst) at the start location in the moved axis
* For sheets where `renderPlan.status.kind !== "ok"`:

  * create list anchors pointing at the corresponding change item DOM IDs (see Deliverable 2)

This closes an important gap: Phase 3 viewer `n/p` currently only walks `regions`. After Phase 4, `n/p` walks *all* meaningful anchors.

---

#### 2) Make “Detailed Changes” a true changes panel with jump-to + pulse highlight

Phase 4 upgrades the existing per-sheet “Detailed Changes” list into an interactive changes panel.

**Design decision:** keep the UI structure, add interactivity.
We do not replace the “Detailed Changes” HTML with a new panel component; we make existing `change-item`s actionable and add minimal extra markup.

**`ChangeItemVM` additions (VM-level, so renderer doesn’t have to guess):**

* Add `navTargets?: Array<{ anchorId: string, label?: string }>`

  * Most items: one target (e.g., row range item → jump)
  * Move items: two targets (`From`, `To`) so a user can hop between endpoints without relying on inspector-only controls

**Renderer changes (`web/render.js`):**

* Render each change item as a `<button type="button">` (or keep as `<div>` but add a nested button) with:

  * a stable element id for list-target anchors:

    * `id="change-${sheetName}-${item.id}"`
  * one or more jump buttons:

    * `<button class="change-jump" data-sheet="..." data-anchor="...">Jump</button>`
    * for move items, render two: `From` and `To`

**Main JS behavior (`web/main.js`):**

* Use event delegation on the results root:

  * On click `.change-jump`, call `navigateToAnchor(sheetName, anchorId)`
* `navigateToAnchor` must:

  1. Expand the sheet section (if collapsed)
  2. Ensure the grid viewer is mounted (if the sheet has a `.grid-viewer-mount`)
  3. If anchor target is `grid`, call `viewer.jumpToAnchor(anchorId)` and `viewer.flashAnchor(anchorId)`
  4. If anchor target is `list`, scroll the change item element into view and briefly flash its background (CSS class with timeout)
  5. Update global review cursor (for next/prev)

**Pulse highlight (viewer-level):**

* When arriving at an anchor via jump/next/prev:

  * flash the region bounds (if `regionId`), else flash the row header / col header, else flash the selected cell
* This must be implemented inside the canvas painter so feedback is immediate and doesn’t rely on DOM overlays.

---

#### 3) Sheet list becomes actionable (index + search + “diff density” cues)

**Design decision:** add a “Sheet Index” at the top, not a left sidebar.
This keeps the page layout stable and fits the current HTML structure (single-column results with sections). If we later want a sidebar, this index can be re-skinned without changing the underlying wiring.

**Renderer changes (`web/render.js`):**
Add a new block right after summary cards:

* `renderReviewToolbar(vm)` — a sticky-ish bar containing:

  * sheet search input (filters by name)
  * global next/prev change buttons
  * filter toggles (see Deliverable 5)

* `renderSheetIndex(vm)` — list of sheets with:

  * sheet name
  * change count badge (`sheetVm.opCount`)
  * anchor count badge (`sheetVm.changes.anchors.length`)
  * a “density” bar (relative to max `opCount` or max anchor count across sheets)
  * status pill: `OK`, `SKIPPED`, `MISSING` based on `sheetVm.renderPlan.status.kind`

Each sheet index item is a `<button>` with `data-sheet="..."`.

**Main JS behavior (`web/main.js`):**

* Clicking a sheet index item:

  * scrolls to that `.sheet-section`
  * expands it
  * if it has anchors, optionally jumps to the first anchor (configurable: do this only if user clicks the “start review” icon)
* Search input:

  * filters both index items and sheet sections by name (no re-render; toggle `hidden` or a CSS class)
  * does not destroy mounted viewers; it only hides sections

---

#### 4) Next/Prev change across sheets (workbook-level cursor)

**Design decision:** workbook-level next/prev must be deterministic and sync with per-viewer navigation.

We want:

* Viewer-local `n/p` (keyboard) to update the global cursor, so toolbar “Next” continues from where the reviewer is.
* Toolbar “Next/Prev” to work even if the reviewer is mid-scroll.

**Implementation approach:**

* Build a flattened `reviewOrder` in `web/main.js` after rendering:

  * array of `{ sheetName, anchorId }`, in sheet order (`vm.sheets` is already sorted), then anchor order (`sheetVm.changes.anchors` sorted)
* Maintain `reviewState = { activeSheetName, activeAnchorId }`

**Sync mechanism (no framework, but still robust):**

* Add custom events in `grid_viewer.js` dispatched from the mount element:

  * `gridviewer:focus` when the viewer gets focus (sets active sheet)
  * `gridviewer:anchor` when anchor changes (sets active anchor)
* `web/main.js` listens for those events on the results root and updates `reviewState`.

**Behavior of global buttons:**

* `Next change`:

  * if there is an active anchor, advance to the next entry in `reviewOrder`
  * else, jump to the first entry
* `Prev change`:

  * analogous, backwards
* When moving to a different sheet:

  * expand the sheet
  * scroll it into view
  * mount viewer if needed
  * jump to anchor and flash

**Edge case (skipped/missing sheets):**

* If a sheet has `anchors` but they are list-target anchors (no grid), global next/prev still works:

  * expand sheet
  * scroll to change item element
  * flash the change row

This prevents “silent skipping” of the hardest sheets (the ones that often have the most need for review).

---

#### 5) Filters that match how spreadsheet reviews actually happen

Phase 4 adds *review-time filters* that affect rendering and navigation without changing the underlying diff semantics.

##### 5.1 Show only changed rows / changed columns (“focus mode”)

**Design decision:** “focus mode” is a viewer-level projection, not a VM rebuild.
We already have `axis` + `cellAt()`; the fastest path is to compute a list of visible row indices / col indices and map the viewer’s scroll/paint/hit-test through that mapping.

**Viewer state additions (`web/grid_viewer.js`):**

* `focus: { rows: boolean, cols: boolean }`
* `rowMap: number[] | null` and `colMap: number[] | null`

  * mapping from visible index → underlying `viewRow/viewCol`

**How to compute maps (deterministic):**

* Start from a “changed index set”:

  * any row/col where axis entry `kind !== "match"`
  * plus rows/cols covered by any `changes.regions` bounds
  * plus rows/cols covered by move endpoints (row/col moves)
* Expand by context:

  * reuse `sheetVm.renderPlan.contextRows/contextCols` as the default context in focus mode
* Convert to sorted unique arrays → `rowMap/colMap`

**Painter + hit-testing changes:**

* All operations that currently assume `(visibleIndex === viewIndex)` need to map:

  * visible row i → viewRow = rowMap[i]
  * visible col j → viewCol = colMap[j]
* Row/col headers must show the underlying view index (so gaps are obvious) rather than renumbering.

**MVP choice (explicit):** we do *not* render “gap rows” (`...`) in Phase 4.
We accept that row/col labels jump (e.g., 12, 13, 98, 99). This keeps implementation simple and truthful. “Pretty hunk separators” can be Phase 6 polish if needed.

##### 5.2 Ignore blank-to-blank edits (default ON)

**Design decision:** blank-to-blank is a VM-level filter that triggers a cheap re-VM + re-render.
It is the only Phase 4 filter that changes what constitutes a “change”, because it affects:

* whether a cell is `diffKind: edited`,
* whether that edit participates in region clustering,
* whether it appears in the changes list.

Trying to apply it purely at paint-time would make the change list and anchor list disagree with the grid.

**Implementation:**

* Add `opts.ignoreBlankToBlank: boolean` to `buildWorkbookViewModel(payloadOrReport, opts)` (default true).
* In `makeEditMap(...)`, skip inserting an edit when:

  * `fromValue`, `toValue`, `fromFormula`, `toFormula` are all empty strings after formatting/resolution.
* Wire the toolbar checkbox:

  * when toggled, rebuild VM from the already-parsed payload
  * re-render the results HTML
  * re-run hydration
  * preserve:

    * sheet expansion states (which `.sheet-section` were expanded)
    * current active sheet name
    * filter settings
    * (optional) current anchor position if the anchor still exists

This is a deliberate trade: “toggle is rare; correctness is mandatory”.

##### 5.3 Show formulas vs values vs both

**Design decision:** this is a viewer-only display preference.
The underlying cell mapping doesn’t change; only the text we paint and what we prioritize in inspector.

**Add to viewer options:**

* `contentMode: "values" | "formulas" | "both"`

**Rendering behavior:**

* `values` (default): display values first, formula secondary in inspector/tooltip (current behavior)
* `formulas`: display formula if present, else value
* `both`:

  * Side-by-side mode: paint two lines when space permits (value on top, formula below), but only when formula exists and differs from value
  * Unified mode: keep the current “old → new” two-line layout to avoid turning each edited cell into a 4-line block; show both value+formula in inspector/tooltip instead

This keeps the grid readable while still giving formula reviewers what they need.

---

#### 6) Warnings surfaced clearly (and where they matter)

Phase 4 makes warnings actionable and local.

**Current reality:** we already show report-level warnings at the top, and per-sheet grid availability is encoded as `sheetVm.renderPlan.status.kind` with a message.

**Enhancements:**

1. **Sheet header status pill**

   * Add a small pill next to the change badge:

     * `OK` (grid available)
     * `SKIPPED` (aligned view too large/inconsistent)
     * `MISSING` (alignment or snapshots missing)
   * Clicking the pill (or a “?” icon) scrolls to the sheet’s grid warning message and expands the “what this means” explanation.

2. **Sheet index status**

   * Mirror the pill in the sheet index list so reviewers can prioritize:

     * “OK + many anchors” first for rapid review
     * “SKIPPED/MISSING” as “needs manual Excel check” buckets

3. **One plain-language explanation block**

   * Add a short static “Preview limitations” explanation near the warnings section:

     * “Skipped” means the aligned view was too large or inconsistent to render; detailed change list is still valid but visual diff is unavailable.
     * “Missing” means snapshots/alignment weren’t present for that sheet in the payload.

This avoids the current ambiguity where a reviewer must infer why a grid is absent.

---

### What code is added / changed (grounded to current file layout)

#### `web/view_model.js` (VM additions for navigation + filter correctness)

**Changes:**

* Add `DEFAULT_OPTS.ignoreBlankToBlank = true`
* Update `makeEditMap(...)` to optionally skip blank-to-blank edits
* Extend `buildChangeItems(...)` to attach navigation metadata (so we don’t parse labels later):

  * row/col range items should carry `viewStart/viewEnd` for their axis
  * move items should carry `moveId`, `axis`, `srcStart`, `dstStart`, `count`
  * region-derived items should carry `regionId` (already implicit in the id string; make it explicit)

**New helper:**

* `buildChangeAnchors({ sheetName, status, items, regions, rowsVm, colsVm }) -> ChangeAnchorVM[]`

  * Produces `changes.anchors` with correct grid/list targets depending on `status.kind`
  * Ensures deterministic sorting

**Sheet VM shape update:**

* `changes: { items, regions, anchors }`

#### `web/render.js` (interactive markup + sheet index + toolbar)

**Changes:**

* Add `renderReviewToolbar(vm)` (search + global next/prev + filter controls)
* Add `renderSheetIndex(vm)` (actionable list with counts + status)
* Update `renderWorkbookVm(vm)` to insert those blocks after summary cards/warnings and before sheets
* Update `renderChangeItemVm(item, sheetName)` (signature change is fine; this is internal):

  * give each item a stable DOM id: `change-${sheetName}-${item.id}`
  * render jump buttons from `item.navTargets`:

    * `.change-jump` buttons with `data-sheet` and `data-anchor`
* Update `renderSheetVm(sheetVm)`:

  * add header status pill based on `sheetVm.renderPlan.status.kind`
  * optionally show anchor count badge

**CSS expectations:**

* Add styles to make change items look like current rows even if implemented as `<button>`
* Add `.change-jump` styles (small, unobtrusive)
* Add `.sheet-index` and `.review-toolbar` styles

#### `web/main.js` (review state + navigation wiring)

**Changes:**

* Extend existing hydration flow with a new setup step:

  * `setupReviewWorkflow(rootEl, workbookVm, payloadCache)`
* Add helpers:

  * `getSheetSection(sheetName)`
  * `expandSheet(sheetName)`
  * `ensureViewer(sheetName) -> viewer | null`
  * `navigateToAnchor(sheetName, anchorId)`
  * `buildReviewOrder(workbookVm) -> Array<{sheetName, anchorId}>`
* Add event delegation handlers:

  * click on `.sheet-index-item` → expand + scroll
  * click on `.change-jump` → navigateToAnchor
  * input on sheet search → filter sections + index
  * toolbar next/prev → traverse reviewOrder
  * filter changes (contentMode, focusRows, focusCols) → apply to all mounted viewers and remember for future mounts
  * ignoreBlankToBlank toggle → rebuild VM from cached payload and re-render, preserving expansion state

**Sync with viewer events:**

* Listen for `gridviewer:focus` and `gridviewer:anchor` custom events to keep `reviewState` correct even when the reviewer uses keyboard inside the viewer.

#### `web/grid_viewer.js` (anchor-based navigation + options + flash)

**Changes:**

* Switch anchor source from `sheetVm.changes.regions` to `sheetVm.changes.anchors` filtered to `target.kind === "grid"`
* Add methods to returned viewer API:

  * `jumpToAnchor(anchorIdOrIndex)`
  * `nextAnchor()` / `prevAnchor()` (return boolean for “moved”)
  * `setDisplayOptions({ contentMode, focusRows, focusCols })`
  * `flashAnchor(anchorId)` (or integrated into jump)
* Emit custom events from `mountEl`:

  * `gridviewer:focus` on focus
  * `gridviewer:anchor` whenever the active anchor changes

#### `web/grid_painter.js` (display mode + flash overlay + focus mapping)

**Changes:**

* Accept a `paintModel` that includes:

  * `contentMode`
  * optional `rowMap/colMap` mapping (for focus mode)
  * optional `flash` object with bounds + alpha
* Implement “both” display mode as described (two-line only when it makes sense)
* Render flash overlay:

  * region bounds (if known)
  * row/col header flash for structural anchors
  * cell flash fallback
* Maintain performance:

  * no per-cell allocations in hot loops (reuse small arrays or compute primitives)
  * no DOM operations during paint

#### `web/index.html` (CSS only)

**Add styles:**

* review toolbar container, sticky behavior (optional but recommended)
* sheet index list and density bars
* status pills (`ok/skipped/missing`)
* interactive change items (button reset styles)
* “active sheet” highlight in sheet index (set by JS via a class)

---

### Acceptance checks (make them hard to fake)

1. **Row/col changes are navigable**
   *Given:* a fixture with only `RowAdded` / `ColumnRemoved` ops (no `CellEdited`).
   *Expected:* `sheetVm.changes.anchors` is non-empty, and viewer `n/p` visits those anchors (not “no changes”).

2. **Jump-to from list works even when the sheet is collapsed**
   *Given:* page rendered with all sheets collapsed.
   *Action:* click a “Jump” button in a change item.
   *Expected:* the sheet expands, viewer mounts (if available), scrolls to the anchor, and a flash highlight is visible.

3. **Move items support both endpoints**
   *Given:* a fixture with a row move or rect move.
   *Expected:* the move list item has `From` and `To` jump actions, and each lands in the correct location.

4. **Global next/prev crosses sheets deterministically**
   *Given:* two sheets with anchors each.
   *Action:* click global “Next change” repeatedly.
   *Expected:* it walks anchors in sheet order, then location order; it never gets stuck on a sheet, and it never skips unpreviewable sheets if they have list anchors.

5. **Focus mode doesn’t lie**
   *Given:* a sheet with changes far apart (e.g., edits at row 10 and row 10,000).
   *Action:* enable “Show only changed rows”.
   *Expected:* the viewer shows a compact set of rows, and row header labels clearly jump (e.g., 11 then 10001), proving we didn’t renumber.

6. **Ignore blank-to-blank toggling is consistent**
   *Given:* a fixture containing at least one `CellEdited` op that resolves to empty → empty.
   *Expected:* with ignore ON (default), it does not appear in `changes.items`, `regions`, or `anchors`.
   *When toggled OFF:* it appears everywhere consistently, and navigation can reach it.

7. **Warnings are local and explicit**
   *Given:* a sheet with `renderPlan.status.kind === "skipped"` or `"missing"`.
   *Expected:* status pill is shown in sheet header and sheet index; global next/prev can still bring you to the first relevant change item (list anchor) with a visible highlight.


---

## Phase 5 — Web app hardening: performance, privacy UX, and workerization

### Grounding notes (what exists today in this repo, and what this phase hardens)

This phase is explicitly grounded in the current web + WASM structure:

* The browser entrypoint is `web/index.html` + `web/main.js` (plain ES modules; no bundler).
* `web/main.js` currently does the heavy work on the main thread:
  * reads files via `readFileBytes(file) -> Uint8Array`
  * calls WASM synchronously: `diff_files_with_sheets_json(oldBytes, newBytes, oldName, newName)`
  * then `JSON.parse(...)`, `buildWorkbookViewModel(payload)`, `renderWorkbookVm(...)`, and hydrates grid viewers.
  This can freeze the UI during diff and during large JSON parse / VM build.
* The WASM entrypoint `wasm/src/lib.rs::diff_files_with_sheets_json(...)` currently:
  * clones the full file buffers (`Cursor::new(old_bytes.to_vec())`), doubling peak memory at the worst moment,
  * snapshots *all* non-empty cells for every “touched” sheet (`snapshot_sheet` iterates `sheet.grid.iter_cells()` with no cap),
  * serializes to a giant JSON string.
* `wasm/src/alignment.rs` still contains the old DOM-era guardrail `MAX_VIEW_CELLS` (“keep the HTML grid from exploding”), but the current UI uses a canvas viewer that is already virtualized and does not need a `rows * cols` cap to stay performant.
* The UI already has the right architectural seams to harden:
  * Rendering is pure string generation (`web/render.js`), so it remains testable via Node (`web/test_render.js`).
  * Canvas/DOM code is isolated in `web/grid_viewer.js` and is imported only by `web/main.js`.
  * Viewer hydration is lazy (mounted on expand) in `hydrateGridViewers(...)`, which makes it practical to delay per-sheet heavy indexing.

This phase makes the web demo feel “instant, safe, and respectful”: no jank, no surprise network requests, and predictable memory behavior.

---

### Key terms (so the design choices are precise)

**Web Worker (module worker)**  
A background JS thread that runs computation without blocking the UI thread. “Module” workers can use `import` and share the same ES module style as the rest of `web/`.

**Transferables**  
A way to pass `ArrayBuffer`s to a worker *without copying* (`postMessage(msg, [buffer])`). The sender’s buffer becomes “detached” (released) after transfer.

**Structured clone**  
The browser’s built-in mechanism to copy JS objects between threads. It can clone plain objects/arrays efficiently, but it can still be expensive for extremely large payloads.

**Partial preview (payload truncation)**  
A deliberate, explicitly-labeled behavior where we keep diff truthfulness (ops + alignment) but limit how much sheet cell content we ship to the UI to prevent huge JSON/memory spikes.

---

### Goal

A web UI that:

1. stays responsive during diff computation (no main-thread freezes),
2. communicates “what’s happening” with progress stages and cancellation,
3. avoids accidental data leakage / surprise network requests,
4. degrades gracefully for large workbooks (never crashes the tab),
5. produces shareable artifacts (report JSON and a self-contained HTML report).

---

### Non-goals (explicit, to keep Phase 5 bounded)

* Not implementing a fully streaming diff pipeline (ops trickling in over time). The core diff is currently a batch operation.
* Not implementing on-demand “tile fetching” from WASM for arbitrary viewports (that would require a stateful workbook cache in WASM and a query API).
* Not implementing perfect Excel rendering fidelity (formats, widths, merged cells, etc.).

---

### Design decisions (the complicated calls made explicitly)

#### 1) WASM runs in a Worker, not on the UI thread

**Decision:** The main thread will never call `diff_files_with_sheets_json` directly. A dedicated module worker owns:
* WASM initialization (`init()`),
* version retrieval (`get_version()`),
* and the diff call itself.

**Why (in this codebase):**
* `diff_files_with_sheets_json(...)` is synchronous and can take seconds; moving it to a worker immediately removes UI freezes.
* Keeping WASM out of the main thread avoids double-loading WASM (and double memory usage).
* This fits the existing “no build step” structure: a `web/diff_worker.js` module can import `./wasm/excel_diff_wasm.js` directly.

#### 2) Cancellation is “terminate the worker”, not cooperative interruption

**Decision:** “Cancel” stops the diff by terminating the worker (`worker.terminate()`), then re-creating a fresh worker.

**Why:**
* The diff call is currently a single WASM function call with no checkpoints; there is no safe mid-flight cancellation without changing the core engine.
* Worker termination is a hard stop that reliably returns control to the UI and releases the worker’s JS/WASM heap.

**User-visible contract:**
* Cancel is immediate.
* Cancelling discards the in-progress result (there is no partial report).
* After cancel, the UI returns to the ready state.

#### 3) Progress UX is stage-based (not percent-based)

**Decision:** We show progress as discrete stages, not “42%”.
Stages are chosen to reflect actual pipeline boundaries we control:

Main thread stages:
1. Validating inputs
2. Reading files into `ArrayBuffer`
3. Transferring buffers to worker
4. Building view model + rendering DOM
5. Hydrating the grid viewer (optional / lazy)

Worker stages:
1. Initializing WASM (first run only)
2. Diffing + snapshotting + alignment (single batch call)
3. Returning payload

**Why:**
* Percent bars are misleading without true internal progress signals.
* Stage messaging is truthful, stable, and doesn’t require changing the diff engine.

#### 4) Payload-size control must happen in WASM, and must preserve “edited-cell truth”

**Decision:** Add a WASM-side snapshot limiter so we don’t serialize massive `sheets[].cells` arrays into JSON.

**Important truthfulness constraint:**  
Even if sheet cell snapshots are truncated, **edited cell values remain exact**, because `CellEdited` ops already include `from` and `to` (the VM uses those to render edited cells). So truncation reduces *context*, not correctness for edits.

**Implementation stance (MVP-hardening tradeoff):**
* We do **not** attempt to ship the entire used-range for large sheets.
* We do ship:
  * sheet dimensions (`nrows`, `ncols`),
  * alignment (bounded by row/col caps),
  * all ops (subject to a conservative op cap),
  * and a *bounded* subset of non-empty cells for context.

**UI contract:**
* When truncation happens, the UI displays a clear “Preview limited” message, but still shows:
  * structural markers,
  * accurate edited-cell old/new values,
  * and jump-to navigation between change regions.

#### 5) VM should stop eagerly indexing all sheet cells up-front

**Decision:** Update `web/view_model.js` so `makeCellMap(oldSheet/newSheet)` is lazy per sheet (built only when the viewer needs it).

**Why (in the current code):**
* `hydrateGridViewers(...)` already mounts the heavy canvas viewer lazily on expand.
* But `buildWorkbookViewModel(...)` currently builds `oldCells/newCells` maps for *every* sheet VM immediately, even for collapsed sheets.
* For large snapshots, that indexing is an avoidable main-thread spike, even after workerization.

This “lazy cell indexing” pairs naturally with payload truncation: small snapshots index quickly; big sheets don’t punish initial render.

#### 6) Export artifacts are small + safe by default

**Decision:** Provide two exports:

1) **Report JSON export**: export the `DiffReport` (plus minimal metadata), not the full `{report, sheets, alignments}` payload.  
2) **Self-contained HTML export**: a static HTML file with:
   * the rendered summary + change lists,
   * the report JSON embedded in a `<pre>` section,
   * and (optional) PNG snapshots of grid viewer canvases for sheets the user expanded.

**Why:**
* Exporting full sheet snapshots can create huge files and encourages sharing raw spreadsheet content.
* Report JSON is typically enough for audit trails and is stable across UI evolution.
* A static HTML export is portable and avoids the complexity of bundling ESM modules into a single offline interactive app.

#### 7) Privacy hardening means “no third-party requests”, not just “we don’t upload your files”

**Decision:** Make the demo *network-quiet* after initial load and eliminate third-party asset loads:

* Remove Google Fonts `<link>`s from `web/index.html`.
* Use system font stacks only.
* Add a tight CSP (via `<meta http-equiv="Content-Security-Policy">`) appropriate for:
  * module scripts,
  * WASM compilation,
  * workers,
  * and `data:` images (for export previews).

**Why:**
* Today `web/index.html` includes `fonts.googleapis.com` / `fonts.gstatic.com`. That’s a third-party request and undermines “privacy posture”.
* The best privacy UX is: “Open DevTools → Network: no third-party requests, and nothing gets posted.”

---

### Implementation plan (concrete, tied to current file layout)

#### Step 1: Add a module worker that owns WASM + diff

**New file:** `web/diff_worker.js`

Responsibilities:
* Import and initialize WASM:
  * `import init, { diff_files_with_sheets_json, get_version } from "./wasm/excel_diff_wasm.js";`
* Handle messages:
  * `{ type: "init", requestId }`
  * `{ type: "diff", requestId, oldName, newName, oldBuffer, newBuffer, options }`
* Emit messages:
  * `{ type: "ready", version }`
  * `{ type: "progress", requestId, stage, detail? }`
  * `{ type: "result", requestId, payload }`
  * `{ type: "error", requestId, message }`

Implementation details (important and codebase-realistic):
* Use `Uint8Array` views on received `ArrayBuffer`s:
  * `const oldBytes = new Uint8Array(msg.oldBuffer);`
  * `const json = diff_files_with_sheets_json(oldBytes, newBytes, oldName, newName);`
* Worker parses JSON before posting back (keeps main thread out of `JSON.parse`):
  * `const payload = JSON.parse(json);`
  * `postMessage({ type: "result", requestId, payload });`
* Release large references ASAP in worker:
  * set `oldBytes/newBytes/json/payload` to `null` after post (so GC can reclaim).

#### Step 2: Add a small worker client wrapper for request/response + cancellation

**New file:** `web/diff_worker_client.js`

Exports:
* `createDiffWorkerClient({ onStatus }) -> { ready(), diff(files, options), cancel(), dispose() }`

Design:
* The client owns:
  * worker lifecycle (create / terminate / recreate),
  * requestId generation,
  * response routing (only resolve the current request),
  * error normalization (worker errors → `Error` objects),
  * and a simple “busy” state.

This keeps `web/main.js` readable and reduces the risk of UI bugs.

#### Step 3: Update `web/main.js` to run diff through the worker + stage UX

**Change file:** `web/main.js`

Changes:
* Remove direct WASM imports:
  * remove `import init, { diff_files_with_sheets_json, get_version } ...`
* Replace with:
  * `import { createDiffWorkerClient } from "./diff_worker_client.js";`
* Replace `main()` init logic:
  * create client
  * `await client.ready()` → set `#version` from worker’s `ready` message
* Replace `runDiff()`:
  1. Validate old/new file presence.
  2. Disable “Compare” button; enable “Cancel”.
  3. Update status to “Reading files…”.
  4. Read as `ArrayBuffer` (not `Uint8Array`) so we can transfer ownership:
     * `const oldBuffer = await oldFile.arrayBuffer();`
     * `const newBuffer = await newFile.arrayBuffer();`
  5. Transfer buffers to worker via client:
     * `const payload = await client.diff({ oldName, newName, oldBuffer, newBuffer }, options)`
  6. Yield a frame, then build VM + render:
     * `await nextFrame()` to ensure status paints before heavy DOM updates.
  7. `workbookVm = buildWorkbookViewModel(payload)`
  8. `resultsEl.innerHTML = renderWorkbookVm(workbookVm)`
  9. Hydrate viewers as today (`hydrateGridViewers(...)`), and keep a handle for export.

**New UI controls (wired here):**
* `#cancel` button: calls `client.cancel()`, resets UI state, and clears in-progress status.
* Export buttons (see Step 6): call into an `export.js` helper.

#### Step 4: Make WASM memory behavior less risky (remove extra file-buffer clones)

**Change file:** `wasm/src/lib.rs`

**Critical low-risk improvement:** remove the redundant `.to_vec()` clones:

Current:
* `let old_cursor = Cursor::new(old_bytes.to_vec());`
* `let new_cursor = Cursor::new(new_bytes.to_vec());`

Replace with:
* `let old_cursor = Cursor::new(old_bytes);`
* `let new_cursor = Cursor::new(new_bytes);`

Rationale:
* JS → WASM already requires a copy into linear memory.
* The `.to_vec()` adds a second full copy in WASM heap; this is exactly what causes peak-memory spikes on large files.

#### Step 5: Implement snapshot truncation metadata + caps in WASM

**Change file:** `wasm/src/lib.rs`

Add fields to `SheetSnapshot`:

* `truncated: bool`
* `included_cells: u32` (or `usize` serialized as number)
* `total_non_empty_cells: u32`
* (optional) `note: Option<String>` with a short human-readable reason

Add constants (explicit so behavior is stable):
* `MAX_SNAPSHOT_CELLS_PER_SHEET: usize = 50_000` (tunable)
* `MAX_SNAPSHOT_CELLS_TOTAL: usize = 200_000` (across all sheets)
* `STRUCTURAL_PREVIEW_MAX_ROWS: u32 = 200`
* `STRUCTURAL_PREVIEW_MAX_COLS: u32 = 80`
* `SNAPSHOT_CONTEXT_ROWS: u32 = 1` (match `web/view_model.js` default)
* `SNAPSHOT_CONTEXT_COLS: u32 = 1`

Add a new snapshot function:
* `snapshot_sheet_limited(sheet, pool, sheet_ops, caps) -> SheetSnapshot`

Algorithm (explicit and implementable with existing core APIs):
1. Compute `total_non_empty_cells = sheet.grid.cell_count()`.
2. If `total_non_empty_cells == 0`: return empty cells, `truncated=false`.
3. Derive a small list of “interest rectangles” from ops for that sheet:
   * For `CellEdited`: add a 1x1 rect at `(row,col)` (old/new values are still in ops; this is for context only).
   * For `RectReplaced` and `BlockMovedRect`: add the rect for src/dst/top_left/size.
   * For row/col structural ops:
     * Use “strips” but clamp the opposite axis:
       * rows: `[row .. row+count)` × `[0 .. min(ncols, STRUCTURAL_PREVIEW_MAX_COLS))`
       * cols: `[0 .. min(nrows, STRUCTURAL_PREVIEW_MAX_ROWS))` × `[col .. col+count)`
4. Expand each rect by context rows/cols and clamp within `[0..nrows)` / `[0..ncols)`.
5. Estimate the total area of interest rects. Choose a sampling strategy:
   * If total area is small: iterate rect coordinates and call `sheet.grid.get(row,col)` (avoids scanning the whole sheet).
   * If total area is huge but non-empty cell count is smaller: iterate `sheet.grid.iter_cells()` and include those that fall in any rect.
6. Stop when either:
   * per-sheet cap reached, or
   * global cap reached (tracked by the parent `snapshot_workbook` call).
   Set `truncated=true` and include a short note.

Modify `snapshot_workbook(...)` to:
* group ops by sheet name (you already do this in `alignment.rs`; reuse the same approach in `lib.rs` locally),
* pass per-sheet op slices into `snapshot_sheet_limited`,
* and track global snapshot cell budget.

**Key UX implication:**  
Even with truncated snapshots, the UI remains truthful for edited cells because `CellEdited.from/to` is authoritative.

#### Step 6: Revisit alignment skip logic to match the canvas viewer reality

**Change file:** `wasm/src/alignment.rs`

* Remove `MAX_VIEW_CELLS` gating (or raise it dramatically), because canvas rendering is viewport-bound.
* Keep the axis caps:
  * `MAX_VIEW_ROWS` and `MAX_VIEW_COLS` still protect:
    * JSON size (axis arrays),
    * and the browser scroll model (huge scroll extents).
* Update the `skipped` reason message to be explicit:
  * e.g., `"Preview disabled: sheet has 250,000 columns (cap is 200)."`

This aligns the backend guardrails with the current renderer (canvas virtualization).

#### Step 7: Make `web/view_model.js` lazily build old/new cell maps per sheet

**Change file:** `web/view_model.js`

Modify `buildSheetViewModel(...)` so it does not eagerly call:
* `const oldCells = makeCellMap(oldSheet);`
* `const newCells = makeCellMap(newSheet);`

Instead:
* store `oldSheet` and `newSheet` on the sheet VM
* create `oldCells/newCells` as `null` initially
* implement a small internal `ensureCellMaps()` that:
  * builds maps on first use,
  * then caches them for subsequent `cellAt` calls.

Expose (optional but clean):
* `sheetVm.ensureCellIndex()` which can be called by the viewer on mount (so first render isn’t surprised).

Also propagate truncation metadata into the VM:
* `sheetVm.preview = { truncatedOld, truncatedNew, note? }`
* `sheetVm.renderPlan.status.kind` becomes:
  * `"ok"`: full preview
  * `"partial"`: preview available but truncated
  * `"skipped"` / `"missing"` as today

#### Step 8: Teach the renderer to display “partial preview” warnings

**Change file:** `web/render.js`

Update `renderSheetGridVm(sheetVm)` behavior:

* If `status.kind === "partial"`:
  * render the viewer mount as usual,
  * plus render a small non-blocking warning element above the viewer:
    * “Preview limited for performance; edited cells remain exact.”
* If `status.kind === "skipped" | "missing"`: keep existing skip warning behavior.

This keeps the UX honest while still enabling the viewer.

#### Step 9: Add export helpers (report JSON + self-contained HTML)

**New file:** `web/export.js`

Exports:
* `downloadReportJson({ report, meta })`
  * `meta` includes `{ version, oldName, newName, createdAtIso }`
  * output filename: `excel-diff-report__old__new__YYYY-MM-DD.json`
* `downloadHtmlReport({ title, meta, renderedResultsHtml, cssText, reportJsonText, gridPreviews })`
  * `gridPreviews` is optional map `{ sheetName -> dataUrlPng }`

Implementation notes grounded in the current UI:
* CSS source of truth is the `<style>` block in `web/index.html`. At runtime we can do:
  * `const cssText = document.querySelector("style")?.textContent || ""`
* `renderedResultsHtml` can be taken from `#results.innerHTML` after diff.
* If we choose to include preview images:
  * extend `mountSheetGridViewer(...)` to expose `capturePng()` that returns `canvas.toDataURL("image/png")`
  * only capture images for mounted viewers (first expanded sheet), or optionally mount-capture-destroy for each sheet (slower but complete).

This export is intentionally static (no scripts) so it opens safely anywhere.

#### Step 10: Privacy / security polish in `web/index.html`

**Change file:** `web/index.html`

1) Remove Google Fonts `<link>` tags.
2) Ensure font stack uses system fonts only.
3) Add a small privacy note in the UI near uploads:
   * “Runs locally in your browser. Files are not uploaded.”
4) Add a CSP `<meta http-equiv="Content-Security-Policy" ...>` compatible with:
   * module scripts
   * workers
   * WASM
   * inline styles (since the page uses a `<style>` block)

Example CSP shape (final string should be tested in-browser):
* `default-src 'self';`
* `script-src 'self' 'wasm-unsafe-eval';`
* `style-src 'self' 'unsafe-inline';`
* `img-src 'self' data:;`
* `connect-src 'self';`
* `worker-src 'self';`
* `object-src 'none'; base-uri 'none'; frame-ancestors 'none';`

---

### What code is added / changed (explicit inventory)

#### New files

1) `web/diff_worker.js`
* Owns WASM init + diff
* Implements the worker message protocol
* Emits stage progress messages

2) `web/diff_worker_client.js`
* Creates/owns the worker instance
* `ready()`, `diff()`, `cancel()`, `dispose()`
* Ensures only one in-flight request is active and cancels safely

3) `web/export.js`
* `downloadReportJson(...)`
* `downloadHtmlReport(...)`
* Shared helper `downloadBlob(filename, mime, textOrBytes)`

#### Changed files

1) `web/main.js`
* Replace direct WASM import with worker client
* Add stage-based status updates
* Add cancel button wiring
* Wire export buttons after successful diff
* Keep `hydrateGridViewers(...)` behavior, but retain viewer references for export previews

2) `web/index.html`
* Remove Google Fonts
* Add cancel + export controls
* Add privacy note
* Add CSP meta tag
* Add minimal CSS for:
  * progress stages / spinner
  * export bar
  * partial-preview warning

3) `wasm/src/lib.rs`
* Remove `.to_vec()` buffer clones (use `Cursor::new(&[u8])`)
* Implement capped/truncated snapshots:
  * add snapshot metadata to `SheetSnapshot`
  * snapshot selection by interest rects derived from ops
  * global + per-sheet caps

4) `wasm/src/alignment.rs`
* Remove/relax `MAX_VIEW_CELLS` cap
* Keep row/col caps
* Improve `skipped` messaging

5) `web/view_model.js`
* Lazy old/new cell map building per sheet
* Propagate truncation metadata into VM status (`partial`)
* Optional `sheetVm.ensureCellIndex()` for viewer mount

6) `web/render.js`
* Render `partial` status warnings without disabling preview

7) `web/grid_viewer.js` (optional but recommended if we include image previews in HTML export)
* Add `capturePng()` on the returned viewer object

8) `.github/workflows/web_ui_tests.yml` (recommended hardening)
* Run both:
  * `node web/test_render.js`
  * `node web/test_view_model.js`
  so VM performance/behavior changes stay covered.

---

### Acceptance checks (manual + deterministic)

#### Responsiveness (hard to fake)
* Start a diff of two moderately large `.xlsx` files (tens of MB).
* Expected:
  * UI remains responsive (you can scroll the page, expand/collapse sections).
  * Status updates appear immediately (“Reading…”, “Diffing…”, “Rendering…”).
  * Cancel works: the worker is terminated and UI returns to ready state.

#### Memory behavior (specific to this codebase)
* In DevTools Performance/Memory:
  * Peak memory should be noticeably lower after removing WASM-side `.to_vec()` clones.
  * Large files should not cause an immediate tab crash.

#### Payload truncation truthfulness
* Use a case with many edits in a large sheet.
* Expected:
  * UI shows a clear “Preview limited” warning if caps trigger.
  * Edited cells still show correct old/new values (from ops), even if surrounding context is sparse/blank.

#### Privacy posture
* Open DevTools → Network, hard refresh.
* Expected:
  * No requests to `fonts.googleapis.com` / `fonts.gstatic.com` (they’re removed).
  * No other third-party requests.
* Optional: verify CSP blocks accidental external loads.

#### Export
* “Download report JSON”:
  * downloads a small JSON file
  * includes `report.ops` and metadata (version/filenames/timestamp)
* “Download HTML report”:
  * opens offline (airplane mode) with readable summary + change lists
  * if preview PNGs are enabled: at least the expanded sheet includes a visual snapshot

#### Regression checks
* `node web/test_render.js` still passes (renderer remains pure; no Worker/Canvas imports).
* `node web/test_view_model.js` passes (VM changes preserve semantics).
* WASM size budget remains under the existing workflow limit (10MB).

---

## Phase 6 — Desktop app foundation: same UI, native diff engine, better IO

### Goal

Desktop app = same shape, higher ceilings. :contentReference[oaicite:0]{index=0}

Concretely: keep the current `web/` UI (same VM + renderer + canvas viewer), but replace “WASM in a worker reading `File` bytes” with “native Rust diff reading from disk”, and add desktop-only ergonomics (open dialog, drag/drop, recents, rerun). 

---

### Grounding notes: what already exists and should be preserved

1. **The UI already has a clean “diff engine” seam.**  
   `web/main.js` constructs a `diffClient` and uses a small API surface: `ready()`, `diff(...)`, `cancel()`, `dispose()`, plus stage/status updates via `onStatus`. This is exactly the seam we should preserve and swap behind. 

2. **The UI already has a stage-based progress vocabulary.**  
   `web/main.js` maps `{ stage, detail }` to user-facing status strings (e.g., `diff`, `parse`, `render`) and uses `detail` when present, so a desktop backend can stream “truthful stage text” without needing percent-perfect progress. :contentReference[oaicite:3]{index=3}

3. **There is an existing, stable payload contract to keep parity.**  
   The WASM entrypoint `diff_files_with_sheets_json(...)` returns a JSON wrapper `{ report, sheets, alignments }` (not just a `DiffReport`). Desktop should emit the same shape so `web/view_model.js` / `web/render.js` remain reusable and semantics do not fork. 

4. **Workbook vs PBIX/PBIT handling already exists in Rust code.**  
   The CLI has `host_kind_from_path(...)` and `open_host(...)` that decide how to open `.xlsx/.xlsm/.xltx/.xltm` vs `.pbix/.pbit`. Desktop should reuse the same extension rules so behavior matches the CLI and web demo. :contentReference[oaicite:5]{index=5}

5. **Repo structure reality:**  
   The Cargo workspace currently has `core`, `cli`, `wasm` only. Desktop will be a new member (or a Tauri `src-tauri` project that still depends on `core`). :contentReference[oaicite:6]{index=6}

---

### Success criteria for Phase 6 (what “foundation” means)

**Parity (must):**
- Desktop renders the same summary cards, sheet list, detailed changes, and grid viewer behavior as the web UI, because it uses the same `web/` modules and the same `{ report, sheets, alignments }` payload contract. 

**Ceiling-lift (must):**
- Desktop can diff substantially larger files than the browser demo because it:
  - reads from disk directly (no `File.arrayBuffer()` duplication in JS),
  - runs native Rust (no WASM constraints),
  - can allocate more memory without tab-crash dynamics.

**Desktop ergonomics (must):**
- “Open…” dialogs for old/new.
- Drag/drop from Finder/Explorer into “Old” / “New” zones.
- Recent comparisons list persisted across app restarts.
- One-click “Re-run” for a recent entry.

**Progress + cancellation (must):**
- Status updates are visible during diff.
- Cancel reliably stops the in-flight diff and returns to “ready” state (no hung UI).

---

### Design decisions (lock these down to prevent web/desktop drift)

#### 1) Shell choice: default to Tauri (WebView + Rust backend), keep Electron as contingency

**Default recommendation: Tauri**  
Why it fits *this* repo:
- The project is already Rust-first (core engine is Rust), so a Rust backend is a natural fit.
- The UI is already a self-contained set of ESM files under `web/` (no bundler). A WebView shell can load those assets as-is.
- We get native dialogs + file IO without introducing a Node runtime into production.

**Contingency: Electron**  
Keep as a fallback *only if* WebView constraints block one of the key requirements (module workers / canvas / CSP / file-drop behavior). The goal is UI parity, not “which framework”. The plan below is written so the UI changes are minimal and mostly shell-agnostic.

#### 2) Payload contract is sacred: desktop backend must emit the same wrapper shape

Desktop should return the same JSON-serializable struct shape the web demo already consumes:  
`{ report: DiffReport, sheets: SheetPairSnapshot, alignments: SheetAlignment[] }`. :contentReference[oaicite:8]{index=8}

This keeps:
- `web/view_model.js` and `web/render.js` unchanged (or nearly unchanged),
- the canvas viewer semantics unchanged,
- the “truthfulness” guarantees from Phases 1–5 intact.

#### 3) Keep the `diffClient` interface identical across web and desktop

The worker client already defines the right minimal contract (`ready/diff/cancel/dispose`) and the right status callback shape. Desktop should implement the same so `web/main.js` only needs a small “platform selection” shim. 

#### 4) Cancellation semantics: prefer cooperative cancellation in the native engine, but allow “hard cancel” as a fallback

In web we cancel by terminating the worker, because the WASM call is synchronous.   
On desktop we can do better:

- **Preferred:** cooperative cancel checkpoints (fast, safe) in the native diff pipeline so “Cancel” stops the job without killing the whole app process.
- **Fallback:** run the diff in an isolated worker thread/process and hard-stop it (thread cancellation via process boundary if needed). This is a pragmatic “never hang” guarantee.

---

### Implementation plan (concrete, codebase-realistic)

#### Step 1: Add a desktop project without introducing a front-end build step

**Repo layout proposal (low-friction):**
- Add a new directory, e.g. `desktop/` (or `app/`), containing the desktop shell project.
- Configure the shell to load frontend assets from the existing `web/` directory (the same `index.html` + ESM modules the browser demo uses). :contentReference[oaicite:11]{index=11}

**Workspace wiring:**
- Add the desktop crate/project to the workspace alongside `core`, `cli`, `wasm`, so the desktop backend can depend on the same `excel_diff` crate the CLI uses. 

**Key acceptance gate for Step 1:**
- Desktop launches and renders `web/index.html` with all scripts/styles loaded locally.
- The UI is visible and interactive (even before diff is wired).

#### Step 2: Introduce a “platform adapter” in `web/` that selects the diff backend

Add a small module (new file) that answers:
- “Am I running in desktop shell?” (Tauri detection, etc.)
- “Which diff client should I create?”

**New file (example):** `web/platform.js`
- `isDesktop() -> boolean`
- `createAppDiffClient({ onStatus }) -> diffClient`

Implementation approach:
- If desktop: return `createNativeDiffClient({ onStatus })`
- Else: return existing `createDiffWorkerClient({ onStatus })` :contentReference[oaicite:13]{index=13}

**Change file:** `web/main.js`
- Replace direct construction of `createDiffWorkerClient(...)` with the platform factory.
- Keep the rest of the flow unchanged as much as possible (same stage UI, same error handling, same exports wiring). 

**Acceptance gate for Step 2:**
- Browser demo still uses the worker client and works unchanged.
- Desktop still loads, but now constructs the native client (even if it’s stubbed initially).

#### Step 3: Implement the native diff client (front-end side) with the same contract

**New file (example):** `web/native_diff_client.js`  
Export: `createNativeDiffClient({ onStatus }) -> { ready, diff, cancel, dispose }`

Requirements to match the existing worker client behavior:
- Only one in-flight diff at a time (match the worker client’s “busy” semantics). :contentReference[oaicite:15]{index=15}
- `ready()` resolves to a version string for `#version` display (mirrors worker’s `ready` message that includes version). 
- `diff(...)` resolves to the payload object (`{ report, sheets, alignments }`), not a JSON string.
- `cancel()` is always safe to call; it reliably returns UI to ready state (even if cancel is best-effort). The UI already assumes cancel resets state immediately. 

**Status updates contract:**
- Call `onStatus({ stage, detail, source: "native" })` during:
  - validate
  - read/open
  - diff
  - (optional) snapshot/alignment
- This plugs directly into the existing `handleWorkerStatus(...)` stage mechanism, because it only needs `{ stage, detail }`. :contentReference[oaicite:18]{index=18}

#### Step 4: Move payload-building logic into a shared Rust module so WASM and desktop cannot drift

Right now, the web payload wrapper logic lives in the WASM crate: snapshot selection + caps + alignment building, producing `DiffWithSheets { report, sheets, alignments }`. :contentReference[oaicite:19]{index=19}  
Desktop needs the same logic.

**Best practice (to prevent divergence):** extract a new Rust crate in the workspace, e.g. `ui_payload/`:
- Pure Rust (no wasm-bindgen, no Tauri dependency).
- Depends on `excel_diff` (core) and `serde`.
- Contains:
  - the payload structs (the same serializable field names and nesting),
  - snapshot generation (including truncation metadata),
  - alignment building (currently in `wasm/src/alignment.rs`),
  - host-kind detection helpers (can be copied from CLI’s `host_kind_from_path` rules). 

**Then:**
- WASM crate becomes a thin wrapper:
  - open packages from bytes,
  - call `ui_payload::build_payload_from_packages(...)`,
  - serialize to JSON string.
- Desktop backend calls the same shared builder but opens packages from file paths.

**Acceptance gate for Step 4 (high-value):**
- A small golden fixture diff run via WASM and via desktop returns payloads that are structurally equivalent (same keys, same kinds, same alignment lengths for the same caps).

#### Step 5: Desktop backend command that runs diff from disk paths and returns the payload

Desktop backend responsibilities:
- Accept two file paths (old/new).
- Determine host kind (Workbook vs PBIX/PBIT) using the same extension logic already proven in CLI. :contentReference[oaicite:21]{index=21}
- Open packages from `File::open(path)` and run the diff using `excel_diff` core.
- Build the `{ report, sheets, alignments }` wrapper using the shared `ui_payload` builder (Step 4). :contentReference[oaicite:22]{index=22}

**Important parity detail:** PBIX/PBIT currently yields a payload with empty sheets and empty alignments (the WASM path explicitly does this). Desktop should do the same so the UI behavior remains consistent. :contentReference[oaicite:23]{index=23}

**Version command:**
- Implement a simple backend `get_version` that returns the desktop app version and/or engine version; the web UI already expects a version string to display. 

#### Step 6: Native file handling in the UI (open dialogs + drag/drop + paths)

Today `web/main.js` reads from `<input type=file>` and passes `ArrayBuffer`s to the worker.   
Desktop should switch to **path-based selection** (to avoid JS memory blowups and enable rerun).

**UI behavior changes (desktop only):**
- Clicking the “Old file” or “New file” dropzone opens a native “Open File…” dialog and stores:
  - `oldPath/newPath` (full path)
  - `oldName/newName` (basename for display)
- Drag/drop:
  - Dropping a file onto the Old zone sets `oldPath`.
  - Dropping onto New zone sets `newPath`.
  - Dropping two files onto the page can fill both in order (optional).

**Minimal code strategy to avoid HTML churn:**
- Keep the existing dropzone HTML structure so CSS/layout remain stable.
- In desktop mode:
  - ignore (or hide/disable) the `<input type=file>` element,
  - update the same `nameOld/nameNew` DOM nodes with basename strings so the UI looks identical.

**Recents persistence:**
- Maintain a recent list of comparisons (`[{ oldPath, newPath, lastRunIso, oldName, newName }]`), stored in app-local data as JSON.
- Render a “Recent” section in the UI:
  - each item: “Old → New”, timestamp
  - actions: “Load”, “Re-run”, “Swap” (optional)
- Keep list small (e.g., last 20) to reduce UI clutter.

**Acceptance gate for Step 6:**
- Selecting files via dialog populates the UI without reading bytes into JS.
- Rerun works after app restart (recents persisted).

#### Step 7: Progress + cancellation implemented end-to-end

**Backend execution model:**
- Run the diff on a background task/thread so the UI thread is never blocked (the webview stays responsive).
- Stream progress as discrete stages via events:
  - `read` (opening/parsing packages)
  - `diff` (core diff execution)
  - `snapshot` (building sheet snapshots)
  - `align` (building alignment arrays)
  - `done` (ready to render)

**Frontend behavior:**
- The native diff client listens to progress events and calls `onStatus({ stage, detail })`.
- `web/main.js` already displays those stages via `showStage(...)`. :contentReference[oaicite:26]{index=26}

**Cancellation semantics:**
- Frontend “Cancel” immediately resets UI state (already implemented pattern). 
- Backend cancels the in-flight job:
  - Preferred: cooperative cancel token checked in the diff pipeline.
  - Fallback: if a cooperative cancel is not feasible everywhere yet, ensure cancel never hangs by isolating the job (thread/process boundary) and hard-stopping it if needed.

**Acceptance gate for Step 7:**
- Start a long diff, see status change at least through `read → diff`.
- Press Cancel:
  - UI returns to ready immediately,
  - backend stops work promptly (no CPU pegged for minutes),
  - a subsequent diff run works (no “stuck busy” state).

#### Step 8: Desktop packaging + smoke checks (minimum viable “distributable”)

**Build outputs (foundation-level):**
- A dev build that runs locally via one command.
- A release build that produces a single installable artifact per OS (details can be refined in Phase 7).

**Smoke checks for packaging:**
- App launches offline.
- No unexpected network requests (keep the “local-only” posture from the web UI). 
- Diff works on a small sample.

---

### What code is added / changed (explicit inventory)

#### New (web)

- `web/platform.js`  
  Platform detection + diff client selection.

- `web/native_diff_client.js`  
  Desktop implementation of the `diffClient` contract matching the worker client’s API surface (`ready/diff/cancel/dispose`). :contentReference[oaicite:29]{index=29}

- (Optional but recommended) `web/recents.js`  
  Pure helpers for rendering and normalizing recent entries (keep `main.js` readable).

#### Changed (web)

- `web/main.js`  
  - Construct diff client via `platform.createAppDiffClient(...)` instead of `createDiffWorkerClient(...)`.
  - Allow “selected inputs” to be either:
    - web `File` objects (browser path), or
    - desktop `{ path, name }` records (desktop path).
  - Skip “transfer to worker” stage in desktop mode; keep the same stage UI plumbing. 

- `web/index.html` (+ inline CSS in the `<style>` block)  
  - Add a small “Recent” section (desktop-only rendering is fine; the DOM can exist in web but be hidden).
  - Add minimal affordances for “Open…” on click (can be purely JS-driven without HTML changes if desired). :contentReference[oaicite:31]{index=31}

#### New (Rust)

- `ui_payload/` (new crate, recommended)  
  Shared “build `{ report, sheets, alignments }`” logic extracted from WASM so desktop and web cannot drift. :contentReference[oaicite:32]{index=32}

#### Changed (Rust)

- `wasm/`  
  Refactor to call into `ui_payload` for payload creation (wrapper remains `wasm_bindgen`-exposed). :contentReference[oaicite:33]{index=33}

- `Cargo.toml` (workspace)  
  Add desktop and `ui_payload` crates as workspace members. :contentReference[oaicite:34]{index=34}

- `desktop/` (new desktop shell project)  
  Rust backend:
  - command: `get_version`
  - command: `diff_paths_with_sheets(...)` (plus progress events + cancel command)
  - recents storage helpers

---

### Acceptance checks (manual + deterministic)

#### Parity (web vs desktop)
1. **Same payload shape**
   - Run the same small fixture in web (WASM) and desktop (native).
   - Expected: both return `{ report, sheets, alignments }` and the UI renders without any desktop-only renderer code. 

2. **Same sheet-level truthfulness**
   - Use a case with row insertion/move.
   - Expected: the desktop UI behaves identically to web (because it’s the same VM + viewer). 

#### Native file handling
3. **Open dialog populates old/new without loading bytes into JS**
   - Expected: dropzone labels update to basenames; diff runs via paths.

4. **Drag/drop behavior**
   - Drop onto Old sets old; drop onto New sets new; diff runs.

5. **Recents persist**
   - Run a diff, quit app, relaunch.
   - Expected: the comparison is present in “Recent” and can be re-run.

#### Progress + cancellation
6. **Progress is visible**
   - Start a medium/large diff.
   - Expected: status transitions through at least `read` and `diff`, with meaningful `detail` text (stage-based). :contentReference[oaicite:37]{index=37}

7. **Cancel is reliable**
   - Start a diff, click Cancel.
   - Expected: UI returns to ready state immediately; a new diff can be started right away. 

#### “Higher ceilings” sanity checks
8. **Large file behavior**
   - Compare files that are painful in the browser (large `.xlsx`).
   - Expected: desktop completes more reliably (no tab crash dynamics), with the UI staying responsive during the run.

#### Regression safety (don’t break the existing repo discipline)
9. **Web UI tests still pass**
   - `node web/test_render.js`
   - `node web/test_view_model.js`  
   Expected: unchanged, because renderer/VM remain pure and desktop integration stays behind `web/main.js` + platform adapter. 


---

## Phase 7 — Desktop-only “spectacular” enhancements that actually matter

These are the “greater resources” wins that users feel immediately:

1. **Very large file support**

   * More memory headroom
   * Better caching (store parsed workbook structures for quick re-diffs)

2. **Export formats**

   * “Audit workbook” export: a generated XLSX that lists changes with hyperlinks (high perceived value)
   * Rich HTML export with embedded screenshots/regions (optional)

3. **Batch / folder compare**

   * Compare many files (versioned monthly reports, etc.)
   * Output a summary table + drill-down

4. **Deep search**

   * Search for a value/formula across old/new and show hits as navigable results
   * This is painful in web for big sheets; desktop can do it comfortably.

---

## What “spectacular MVP UI” means in concrete, testable terms

If you implement Phases 1–7, you should be able to truthfully claim:

* **Correctness**

  * Row/col inserts/moves do not create “phantom” edits (alignment is explicit and visual)
* **Clarity**

  * Users can answer “what changed?” in 30 seconds via summary + sheet list + next/prev change
* **Depth**

  * Clicking a cell shows old/new value + formula, with confident classification
* **Speed**

  * UI doesn’t freeze during diff (web worker) and scroll remains smooth (virtualized/canvas)
* **Parity**

  * Same layout, same concepts, same keyboard shortcuts across web and desktop
* **Desktop advantage**

  * Desktop handles huge files and exports audit artifacts

---

## One strategic recommendation: start by fixing truthfulness, not aesthetics

You already have a visually polished *theme*, but the viewer loses trust when it misrepresents changes. The fastest path to “wow” is:

1. **Phase 1 alignment model** (engine → UI contract), then
2. **Phase 3 interactive aligned grid viewer** (canvas + virtualization), then
3. everything else becomes additive polish.

That’s the path from “pretty demo” to “people will rely on this in real workflows.”
