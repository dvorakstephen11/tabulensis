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

## Phase 4 — Navigation, filtering, and “review workflow” polish

### Goal

Make it *usable for real audits*, not just demos.

### Deliverables

1. **Changes panel with jump-to**

   * Grouped by type: Structural / Cells / Moves
   * Clicking an item scrolls and briefly pulses the region in the grid

2. **Sheet list becomes actionable**

   * Each sheet row shows counts and a small “diff density” sparkline or indicator
   * Search sheets by name

3. **Filters**

   * “Show only changed rows”
   * “Show only changed columns”
   * “Ignore blank-to-blank” (default on)
   * “Show formulas” vs “Show values” vs “Show both”

4. **Warnings surfaced clearly**

   * If the engine warns about ambiguous alignment or low-information regions, show it near the sheet name and explain what it means for the viewer’s confidence.

Your current renderer already has a notion of warnings and sections, so you’re evolving what exists, not inventing the concept. 

---

## Phase 5 — Web app hardening: performance, privacy UX, and workerization

### Goal

A web UI that feels instant and safe.

### Deliverables

1. **Run diff in a Web Worker**

   * Key term: a “Web Worker” is a background thread in the browser so the UI doesn’t freeze.
   * This is crucial if users drop large files.

2. **Streaming-ish UX even if results are batch**

   * Show stages: “Parsing”, “Aligning”, “Diffing”, “Rendering”
   * Display file sizes + “runs locally in your browser” messaging

3. **Payload size control**

   * For web, avoid shipping entire-sheet snapshots when a sheet is huge:

     * Prefer: “changed region tiles” + “on-demand fetch tiles” (via additional WASM calls), or
     * Fallback: cap snapshot to bounding boxes + context.

Your WASM layer currently snapshots all cells for selected sheets. That’s fine for small-to-medium, but you’ll want control knobs for large. 

4. **Export**

   * Download report JSON (already basically possible)
   * Download HTML report (self-contained)

---

## Phase 6 — Desktop app foundation: same UI, native diff engine, better IO

### Goal

Desktop app = same shape, higher ceilings.

### Deliverables

1. **Choose a shell that preserves UI parity**

   * A WebView-based shell (Electron or Tauri) lets you reuse the web UI nearly 1:1.
   * Desktop then calls the native Rust diff engine directly (no WASM constraints).

2. **Native file handling**

   * Open dialogs, drag/drop from Finder/Explorer
   * Recent comparisons list
   * “Re-run diff” quickly after file changes

3. **Progress + cancellation**

   * Desktop can run diff on a background thread and allow cancel mid-run (much harder in pure web).

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
