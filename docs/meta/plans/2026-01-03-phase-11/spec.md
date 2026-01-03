# Phase 11 implementation plan: stable sheet identity for rename-safe diffs

Phase 11’s intent is to stop treating sheet renames as “remove + add” by carrying a workbook-internal sheet identifier through the IR and using it during matching, with a deterministic fallback when that identifier is missing/invalid.

Below is a concrete plan that fits the current architecture: OpenXML parsing already extracts `sheetId`, but the IR discards it; the diff engine matches sheets by `(lowercased name, kind)` and therefore can’t recognize renames; and the UI payload/viewer groups everything by sheet *name*, so a rename-aware engine needs a bridging story to keep “before/after” previews aligned.

---

## Success criteria

### Functional

1. **A sheet rename no longer emits `SheetRemoved` + `SheetAdded`** when the underlying workbook identity is stable; instead it is treated as the “same” sheet for grid diffing, and a dedicated rename signal is emitted (or a deterministic equivalent).
2. **Grid ops remain attributed to the renamed sheet correctly** (including edits, row/col ops, moves), and ordering of operations remains deterministic.
3. **No regressions in duplicate/ambiguous situations**: if the internal ID is missing/duplicated/unusable, behavior falls back to today’s name-based matching with warnings (and stable ordering).
4. **The UI remains coherent**: renamed sheets still render a usable old/new structural preview instead of losing one side due to name-keying.

### Determinism / stability

5. Existing determinism expectations (sort by `lowercase(name)` then `SheetKind`) remain true for non-rename cases, and rename cases have a well-defined deterministic order.

---

## Current state (what the code actually does today)

### Parsing: the ID exists, but gets dropped

* `parse_workbook_xml` produces `SheetDescriptor { name, rel_id, sheet_id }`, where `sheet_id` is read from the workbook XML. 
* `open_workbook_from_container` iterates those descriptors, opens each sheet XML, and constructs `Sheet { name, kind, grid }` — **without preserving `sheet_id`**.

### Engine: sheet identity is name-based

* The core workbook diff builds `HashMap<SheetKey, &Sheet>` for old/new where `SheetKey = { name_lower, kind }` via `make_sheet_key`, then unions keys and sorts by `(name_lower, kind-order)` before emitting ops. 
* A rename necessarily changes `name_lower`, so it becomes “old key missing + new key missing” → `SheetRemoved` + `SheetAdded`, and it does **not** diff the grids across that rename.

### Tests: rename is explicitly asserted as add/remove

There is an existing fixture test that asserts rename results in exactly one add and one remove (and no grid ops). That test is expected to change in this phase. 

### UI / payload: everything is keyed by sheet name

* The payload builder collects sheet IDs from ops (currently only ops with a `sheet` field), snapshots sheets by matching `sheet.name` (StringId), and groups ops by resolved sheet name string.
* The web viewer builds lookups keyed by `sheet.name` (string) and then, for each sheet name in `sheetOps`, picks `oldLookup.get(sheetName)` and `newLookup.get(sheetName)`. A rename breaks that assumption immediately.

---

## Target design: stable identity vs display name

The important conceptual shift:

* **Matching identity**: something stable across rename (OpenXML `sheetId` when available).
* **Display identity**: the sheet’s name (what users see, what the report resolves via the string pool).

The current report schema uses `SheetId = StringId` (sheet name). 
So the phase should keep *display* keyed on names, but use *internal IDs* for matching, and add a bridging signal so downstream systems can associate old-name snapshots with new-name ops.

---

## Work plan

### 1) Upgrade the IR: preserve workbook sheet IDs

#### 1.1 Add a workbook-internal ID field to `Sheet`

**File:** `core/src/workbook.rs` 

Proposed minimal addition:

* Add `pub workbook_sheet_id: Option<u32>` (or a newtype like `WorkbookSheetId(u32)` if you want stronger typing).
* Keep existing fields (`name: StringId`, `kind: SheetKind`, `grid: Grid`) unchanged in meaning.

Why this is the right place:

* Sheet identity is currently derived from `name` + `kind`; carrying a stable ID at the IR layer makes identity improvements available to all diff paths, not just OpenXML-specific code.

API/compat note:

* `Sheet` is a public struct today; adding a field is a breaking change for downstream constructors. If you care about external users, consider adding a `Sheet::new(...)` constructor and making the struct `#[non_exhaustive]` as part of the same phase (or accept the breaking change since this is a fast-moving project).

#### 1.2 Populate it during OpenXML parsing

**File:** `core/src/open_xml.rs` (the function that builds `Sheet { ... }`)

In the loop over `sheets_in_order`:

* Set `workbook_sheet_id: sheet.sheet_id` on the constructed IR `Sheet`.

This directly satisfies “preserve workbook‑internal sheet IDs when available.”

#### 1.3 Keep deterministic behavior for in-memory workbooks

Many unit tests construct `Sheet` directly. For those:

* Default `workbook_sheet_id: None` unless a test explicitly wants rename-aware matching.

This ensures existing name-based test semantics remain valid unless deliberately updated.

---

### 2) Update sheet matching in the diff engine

**File:** `core/src/engine/...` (the implementation that currently builds `old_sheets/new_sheets` keyed by `SheetKey` and sorts keys) 

#### 2.1 Replace “single key” matching with a two-tier match plan

You want deterministic, rename-safe matching without sacrificing existing behavior when IDs are absent/broken.

A concrete algorithm that fits the current structure:

1. Precompute per-sheet metadata:

   * `name_lower` (existing behavior)
   * `kind`
   * `workbook_sheet_id` (new)

2. Build **uniqueness maps** for workbook IDs separately for old and new:

   * `id_counts_old: HashMap<u32, usize>`
   * `id_counts_new: HashMap<u32, usize>`

3. Partition sheets into:

   * **ID-matchable:** `workbook_sheet_id = Some(id)` where `id_counts[...] == 1`
   * **Fallback-matchable:** everything else (None, invalid, duplicated IDs)

4. Build match maps:

   * `old_by_id: HashMap<u32, &Sheet>`
   * `new_by_id: HashMap<u32, &Sheet>`
   * `old_by_name: HashMap<SheetKey, &Sheet>` (existing `SheetKey { name_lower, kind }`)
   * `new_by_name: HashMap<SheetKey, &Sheet>`

5. Produce a unified list of “entries” to diff:

   * First, entries for the union of `old_by_id.keys ∪ new_by_id.keys`:

     * `Both(old, new)` if present in both
     * `OldOnly(old)` / `NewOnly(new)` otherwise
   * Then, for fallback (name-based) keys not already consumed by an ID-match, do the existing union behavior.

This preserves current fallback semantics and makes use of the internal ID when it’s safe.

#### 2.2 Deterministic ordering

Today, ordering is derived from `SheetKey.name_lower` then kind order.

For the new entries:

* Define a **sort key** per entry:

  * Prefer **new name_lower** when `new` exists (this makes “current workbook” naming dominate output).
  * Else use old name_lower.
* Tiebreakers:

  * `SheetKind` order (existing)
  * Then, for stable deterministic ordering in pathological cases, a final tiebreaker:

    * if ID-based entry: the numeric `workbook_sheet_id`
    * if name-based: the `name_lower` string (already used)

This keeps output stable even if multiple sheets share the same lowercased name in corrupt inputs.

#### 2.3 Warnings and “deterministic fallback”

The existing engine emits warnings when duplicate identities occur and later sheets overwrite earlier ones. 

Extend warnings to cover workbook ID issues:

* If `id_counts_old[id] > 1`, emit a warning like:

  * “duplicate workbook sheetId in old workbook: id=…, falling back to name-based matching for those sheets”
* Same for new.

Crucially:

* Do **not** drop the sheets; push them to fallback matching so you still produce a report.
* Keep “later definition overwrites earlier one” behavior only within the fallback name map, as currently done, to preserve determinism and existing warning semantics. 

---

### 3) Emit an explicit rename operation

This is the bridging mechanism that makes the rest of the system (payload + UI) workable.

#### 3.1 Add `DiffOp::SheetRenamed`

**File:** `core/src/diff.rs` (where `DiffOp` is defined and serialized via `#[serde(tag="kind")]`) 

Recommended shape (optimized for downstream grouping):

* `SheetRenamed { sheet: SheetId, from: SheetId, to: SheetId }`

  * `sheet` should equal `to` (the new name), so any helper that extracts “the sheet for this op” stays simple.
  * `from` gives you the old display name.
  * `to` is redundant but makes the payload/UI logic simpler and future-proofs (and mirrors `QueryRenamed { from, to }` style).

#### 3.2 When to emit it

Only emit `SheetRenamed` when:

* The sheet match is via **workbook_sheet_id**, and
* `old_name_lower != new_name_lower`

This preserves the project’s explicit intent to treat case-only changes as “no change” (consistent with existing case-insensitive behavior/tests).

#### 3.3 Grid ops should use the new sheet name

In the matched-sheet path, the engine currently sets `sheet_id = old_sheet.name` before calling `try_diff_grids_internal`. 

Change that rule:

* If matched via workbook ID, pass `sheet_id = new_sheet.name`.
* Otherwise keep `old_sheet.name` (status quo), so name-based matching doesn’t change semantics.

This ensures that once a rename is recognized, all grid diffs appear under the “current” name.

---

### 4) Update sinks/consumers that assume only add/remove for sheets

This phase introduces a new op kind; anything matching over `DiffOp` needs to handle it.

#### 4.1 Excel export (“Structure” sheet)

There is a match arm handling `SheetAdded` and `SheetRemoved` (and many other ops). Add support for `SheetRenamed` so exports remain complete. 

Expected user-facing message:

* “Sheet renamed: OldName -> NewName”

#### 4.2 Desktop store classification

Where the store classifies ops into added/removed/modified:

* Treat `SheetRenamed` as **modified** (or its own subtype if you want to expose it later).

This keeps summary counts consistent with existing UI patterns.

#### 4.3 Text/CLI outputs

Any human-readable formatting that lists sheet changes should include rename.

If there is a “sheet change count” summary, decide whether rename counts as modified or as both removed+added (recommended: modified).

---

### 5) Make UI payload + web viewer rename-aware

Without this, rename-aware engine output will degrade the visual experience because everything is keyed by names.

#### 5.1 Payload builder: include rename sheets in snapshot selection

**File:** `ui_payload/src/lib.rs`

Right now `collect_sheet_ids` and `group_ops_by_sheet` only pull a single `sheet` id from ops that have one. 

Update both functions to recognize `DiffOp::SheetRenamed`:

* Add `sheet` (the new name) to the set as usual
* Also add `from` (the old name) to the set, so **old snapshot includes the old sheet** even though ops are grouped under the new name.

This guarantees both sides exist in the snapshot material available to the viewer.

#### 5.2 Payload builder: optionally precompute a rename map for alignments

Alignments are keyed by a sheet string name; a rename will prevent old/new from aligning unless you apply aliasing. The cleanest fix is to build a rename map in Rust and apply it when building the alignment lookup and/or when emitting alignment objects.

If you keep alignments purely name-keyed, you’ll need to do this in JS anyway. Doing it in Rust reduces duplicated logic across web/desktop.

#### 5.3 Web viewer: resolve old sheet snapshot via rename mapping

**Files:** web viewer logic contains:

* `categorizeOps(report)` and `buildWorkbookViewModel(...)`

Plan:

1. In `categorizeOps`, treat `SheetRenamed` as a sheet op:

   * Group under the **new** sheet name (resolve `op.sheet`).
   * Track a `renameMapNewToOld.set(newName, oldName)` using `op.from` and `op.to/op.sheet`.
2. In `buildWorkbookViewModel`, when selecting `oldSheet` for a `sheetName`:

   * If `oldLookup.get(sheetName)` is missing, check `renameMapNewToOld.get(sheetName)` and try that old name against `oldLookup`.

This one change makes renamed sheets render with both sides present, without needing to mutate snapshot names.

---

### 6) Optional but strongly recommended: avoid chart noise on sheet rename

If the diff engine recognizes sheet renames but chart matching is still keyed by sheet name, you risk spurious `ChartRemoved` + `ChartAdded` when only the containing sheet was renamed.

Today, chart diff keys use `(sheet_lower, chart_name_lower)`. 

**Recommended within Phase 11** (because it’s the same identity problem):

1. Extend `ChartObject` IR to include `workbook_sheet_id: Option<u32>` (same source as the parent sheet).
2. During parsing, populate it while iterating sheets (you already have the descriptor’s `sheet_id` there).
3. Update chart keying:

   * Prefer `(workbook_sheet_id, chart_name_lower)` when id is present and unique
   * Fallback to the existing `(sheet_lower, chart_name_lower)` otherwise

This keeps charts stable across sheet renames and aligns with the “reduce name-only ambiguity” spirit of the phase.

(If you don’t do this now, document it as an expected artifact: “sheet rename may still show chart add/remove noise” — but it’s better to fix while you’re touching identity.)

---

## Test plan

### 1) Update the existing rename fixture test

The current PG6.3 test asserts rename = add + remove. It should be updated to assert the new behavior. 

New expectations:

* Exactly one `SheetRenamed` from `OldName` to `NewName`
* No `SheetAdded`/`SheetRemoved`
* No grid ops (if the fixture truly only renames)

This is the end-to-end “grounded in reality” proof because it uses actual workbook fixtures opened through OpenXML parsing (where the new `workbook_sheet_id` will exist).

### 2) Add targeted unit tests for matching edge cases

Add new unit tests in the engine test module (where other identity tests live).

Key cases:

1. **Rename with same workbook ID, grid changes**

   * Old/new have same `workbook_sheet_id`
   * Names differ
   * One cell edit exists
   * Expect: `SheetRenamed` + `CellEdited` (sheet field should resolve to new name)

2. **Swap names across two sheets**

   * Two sheets with IDs 1 and 2; names A and B.
   * New workbook: ID 1 is B, ID 2 is A.
   * Expect: two `SheetRenamed` ops, and grid diffs apply to correct IDs (not name-matched crosswise).

3. **Duplicate workbook IDs in one workbook**

   * Two old sheets share `workbook_sheet_id = 1`.
   * New workbook is empty.
   * Expect:

     * Warning emitted about duplicate workbook IDs
     * Deterministic fallback behavior (likely remove ops based on name-keying rules)
   * Assert no panic and stable output ordering.

4. **Fallback still respects kind**

   * Ensure `(workbook_sheet_id, kind)` matching doesn’t accidentally match across kinds (consistent with the existing “identity includes kind” intent). 

### 3) UI/unit tests

If you have JS tests for view model construction, add one minimal payload test:

* A report containing `SheetRenamed` plus one grid op under the new name
* `sheets.old` contains old name snapshot, `sheets.new` contains new name snapshot
* Assert `buildWorkbookViewModel` produces a single sheet VM with both old+new present.

The relevant code paths are exactly the ones that currently fail for rename scenarios (name-keyed lookups).

---

## Risk register and mitigations

1. **Public API churn (adding fields to public structs)**

   * Mitigation: add constructors/builders and consider `#[non_exhaustive]` for IR structs that are likely to evolve.

2. **“Unrelated workbook” comparisons may match by numeric IDs**

   * If you match primarily by `sheetId`, you may compare sheet 1 vs sheet 1 even when names differ.
   * Mitigation options:

     * Keep name-based matching as the primary pass and only use workbook IDs to resolve ambiguities (but note: this fails on name swaps, which is exactly where IDs help).
     * Or add a heuristic/config flag later (“prefer name matching”) if this becomes a practical concern.
   * Document expected semantics clearly.

3. **Downstream consumers missing the new op kind**

   * Mitigation: update all in-repo sinks (export, UI payload, web viewer, store) in the same PR/phase, and add a compilation check that exhaustively matches where applicable.

---

## Deliverables checklist (what “done” looks like)

* IR:

  * `Sheet` carries `workbook_sheet_id: Option<u32>` and OpenXML parsing sets it.
* Engine:

  * Workbook diff matches by workbook ID when safe; falls back deterministically to current `SheetKey` behavior. 
  * `SheetRenamed` emitted for true renames (case-insensitive change), and grid ops use new name.
* UI:

  * Payload includes both old+new snapshots for renamed sheets, and web viewer correctly associates them.
* Tests:

  * PG6.3 updated to assert rename op instead of add/remove. 
  * New targeted unit tests for name swaps + duplicate IDs + determinism.
* Optional (recommended):

  * Chart diff becomes rename-robust by using the same sheet identity.

