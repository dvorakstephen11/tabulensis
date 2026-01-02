## Phase 6 goal and scope

Phase 6 in your `13_phase_plan.md` is specifically about **adding direct “leaf diff” APIs** (grid-only and sheet-only) and **eliminating the current “temporary workbook” cloning pattern** used when diffing `Sheet`/`Grid`, plus **re-scoping `Diffable`** so workbook orchestration stays a workbook concern. 

Concretely, that means:

* New engine entry points like `engine::diff_grids(...)` and `engine::diff_sheets(...)` (plus streaming variants) that call the existing grid diff pipeline directly. 
* Refactor (or remove) the `Diffable` implementations for `Sheet` and `Grid` that currently build one-sheet `Workbook`s and clone everything just to get into `engine::diff_workbooks`.
* Keep the design grounded in the current engine layering: **workbook_diff orchestrates**, **grid_diff actually compares grids**.

---

## Why this phase matters in *this* codebase

### The current bottleneck / abstraction leak

Today, `Diffable for Sheet` and `Diffable for Grid` are implemented by:

1. Cloning the sheet/grid
2. Wrapping it in a synthetic single-sheet `Workbook`
3. Calling `crate::engine::diff_workbooks(...)` 

That is explicitly clone-heavy:

* `Sheet` diff clones both sheets. 
* `Grid` diff clones both grids into new `Sheet` structs and interns the literal `"<grid>"`. 

This is “real cost” in both memory and runtime when grids are large (dense grids are common in your test/bench harnesses). It also makes the API story confusing: to diff a grid, you secretly diff a workbook.

### The engine already has the right “leaf” primitive

The core grid diff pipeline already exists as `try_diff_grids(...)`, and it takes **borrowed references** to both grids and a **read-only** `StringPool` (`&StringPool`, not `&mut`). 

That’s exactly what Phase 6 wants to expose as a first-class API: reusing `try_diff_grids` directly instead of forcing a workbook wrapper.

### Streaming constraints you must preserve

Your sinks and tests establish a strong invariant: **the string table is emitted once at `begin()` and is assumed stable**.

* `JsonLinesSink::begin()` writes a header that contains `strings: pool.strings()` and then sets `wrote_header = true`. 
* There’s test infrastructure that checks the pool length does not change after `begin` (the `FrozenPoolSink` pattern).

So any new leaf diff streaming API must ensure:

* any synthetic IDs like `"<grid>"` are interned **before** `sink.begin(pool)` is called, or else you’ll emit ops referencing IDs not present in the header.

---

## Target end state: APIs and semantics

### A quick definition: “leaf diff”

In this project, “leaf diff” means **diffing the lowest-level IR nodes that actually carry cell data** (`Grid` and `Sheet`) *without* introducing workbook-level orchestration like sheet enumeration, identity matching, or workbook-level object diffs. That orchestration already lives in `engine/workbook_diff.rs`.

### Proposed public engine APIs (what Phase 6 is asking for)

You want two families: **non-streaming** (returns `DiffReport`) and **streaming** (writes ops into a `DiffSink`, returns `DiffSummary`).

#### 1) Grid leaf diffs

A minimal, ergonomic API surface:

* `engine::diff_grids(old: &Grid, new: &Grid, pool: &mut StringPool, config: &DiffConfig) -> DiffReport`
* `engine::try_diff_grids(old: &Grid, new: &Grid, pool: &mut StringPool, config: &DiffConfig) -> Result<DiffReport, DiffError>`
* `engine::diff_grids_streaming(old: &Grid, new: &Grid, pool: &mut StringPool, config: &DiffConfig, sink: &mut impl DiffSink) -> DiffSummary`
* `engine::try_diff_grids_streaming(old: &Grid, new: &Grid, pool: &mut StringPool, config: &DiffConfig, sink: &mut impl DiffSink) -> Result<DiffSummary, DiffError>`

**Semantics**

* Use a default sheet id `"<grid>"` (interned into the provided pool before `begin`). This matches existing `Diffable for Grid` behavior today.
* Ops produced are the same grid ops you already generate for normal workbook diffs (`CellEdited`, `RowAdded`, `BlockMovedRect`, etc.) because they come from the same pipeline (`try_diff_grids`). 

**Why not require a `SheetId` parameter?**
You can add an advanced “named grid” overload later if needed, but Phase 6’s core win is eliminating workbook wrappers and clones. For most use and tests, a stable default id is fine.

(If you *do* want the “named grid” option, make it an additional function, not the only one: it’s helpful for multi-grid streaming into one sink.)

#### 2) Sheet leaf diffs

Similarly:

* `engine::diff_sheets(old: &Sheet, new: &Sheet, pool: &mut StringPool, config: &DiffConfig) -> DiffReport`
* `engine::try_diff_sheets(...) -> Result<DiffReport, DiffError>`
* `engine::diff_sheets_streaming(...) -> DiffSummary`
* `engine::try_diff_sheets_streaming(...) -> Result<DiffSummary, DiffError>`
* optional progress variants (see below)

**Semantics**

* Use `old.name` (the interned sheet name) as the `SheetId` passed into `try_diff_grids`. This aligns with how the grid pipeline formats warnings like `Sheet 'X': alignment limits exceeded...` using `pool.resolve(sheet_id)`.
* This API diffs *only* `Sheet.grid` content (and any grid metadata already surfaced as grid ops by the engine). It does **not** enumerate sheets or emit `SheetAdded`/`SheetRemoved`—that remains workbook-level behavior.

> Note: the existing `Diffable for Sheet` wrapper *incidentally* performs sheet identity orchestration by virtue of calling `diff_workbooks` on two one-sheet workbooks. If you keep `Diffable for Sheet`, you need to decide whether to preserve that edge-case behavior when names/kinds differ. (More on that below.)

---

## Implementation workstreams (highly detailed)

### Workstream A: Add grid leaf APIs in `engine/grid_diff.rs`

This is the most direct “grounded” change because:

* `try_diff_grids(...)` already lives here. 
* `diff_grids_database_mode(...)` and `try_diff_grids_database_mode_streaming(...)` already show the exact wrapper pattern you should follow: create sink, call streaming, build report, embed errors if needed. 

#### A1) Introduce a default sheet label constant

Add a constant near the existing `DATABASE_MODE_SHEET_ID`:

* `const GRID_MODE_SHEET_ID: &str = "<grid>";`

This mirrors how `Diffable for Grid` interns `"<grid>"` today. 

#### A2) Add `try_diff_grids_streaming` (spreadsheet-mode leaf)

Pattern it after `try_diff_grids_database_mode_streaming`:

Key steps in the function body:

1. **Intern required strings before begin**

   * `let sheet_id: SheetId = pool.intern(GRID_MODE_SHEET_ID);`

2. **Begin sink and guard finish**

   * `sink.begin(pool)?;`
   * `let mut finish_guard = SinkFinishGuard::new(sink);`

3. **Create engine diff context and hardening controller**

   * Use the engine’s internal `DiffContext` (from `engine/context.rs`), not the public `diffable::DiffContext`. The internal one holds `warnings` + `formula_cache`.
   * `let mut ctx = super::context::DiffContext::default();`
   * `let mut hardening = super::hardening::HardeningController::new(config, progress_opt);` (progress discussed below).

4. **Handle timeout at the leaf level**
   Follow the engine’s established behavior: if `check_timeout()` triggers early, return `DiffSummary { complete: false, ... }` after finishing the sink. This mirrors workbook streaming.

5. **Call `try_diff_grids` directly**

   * `try_diff_grids(sheet_id, old, new, config, pool, sink, op_count, &mut ctx, &mut hardening, metrics_opt)?;` 

6. **Finish and return summary**

   * `finish_guard.finish_and_disarm()?;`
   * `complete: ctx.warnings.is_empty()` is the natural rule already used in other paths (database-mode summary uses warnings to determine completeness). 

#### A3) Decide whether to expose an “op_count-in/out” signature

Database-mode streaming takes `op_count: &mut usize` as an input/output accumulator. 

For spreadsheet-mode leaf streaming, you have two viable designs:

* **Option A (consistent with database-mode):**

  * `try_diff_grids_streaming(..., op_count: &mut usize) -> Result<DiffSummary, DiffError>`
  * Pros: composable if callers want to stream multiple diffs into one sink while tracking a single op counter.
  * Cons: slightly awkward for casual use.

* **Option B (consistent with workbook streaming):**

  * `try_diff_grids_streaming(...) -> Result<DiffSummary, DiffError>` and it manages `op_count` internally.
  * Pros: easy to use.
  * Cons: less composable.

**Recommendation for this codebase:** expose **both**, but only one needs to be `pub`:

* Public: ergonomic signature (Option B).
* Internal/private helper: takes `&mut op_count` for composition, mirroring how `try_diff_grids(...)` already takes `&mut op_count`.

This keeps the top-level API friendly while preserving the “engine building block” flexibility you already use in `WorkbookPackage::diff_*_streaming_with_pool` composition patterns.

#### A4) Add non-streaming wrappers `try_diff_grids` and `diff_grids`

Follow the exact template used by:

* `try_diff_workbooks(...)` (VecSink + build report) 
* `diff_grids_database_mode(...)` (embeds error into `DiffReport`) 

Implementation pattern:

* `try_diff_grids(...)`:

  1. create `VecSink`
  2. call `try_diff_grids_streaming(...)`
  3. `let strings = pool.strings().to_vec();`
  4. `DiffReport::from_ops_and_summary(...)`

* `diff_grids(...)`:

  * call `try_diff_grids(...)`
  * on `Err(e)`, return a `DiffReport` with:

    * `complete: false`
    * warnings: `[e.to_string()]`
    * ops: whatever the sink captured before failure (same as database-mode wrapper) 

#### A5) Progress variants (optional but strongly aligned with engine conventions)

Workbook streaming already supports an optional progress callback via `HardeningController::new(config, progress)` and uses `hardening.progress("parse", 0.0)` etc.

For grid/sheet leaf diffs, you can:

* Provide `try_diff_grids_streaming_with_progress(..., progress: &dyn ProgressCallback)` and wire it into `HardeningController`. 
* Keep progress phases simple and aligned with grid diff internals (it already emits `"alignment"` / `"cell_diff"` progress within the grid pipeline).

That gives you parity with the workbook API surface without inventing new progress semantics.

---

### Workstream B: Add sheet leaf APIs (new `engine/sheet_diff.rs` or in `engine/mod.rs`)

This workstream is thin glue: `Sheet` already contains `name`, `kind`, and `grid`.

#### B1) Create a small helper to pick the `SheetId`

Use:

* `let sheet_id = old.name;`

This ensures warnings and ops refer to the old sheet name (interned) and avoids any need to mutate the pool after begin.

#### B2) Implement streaming first, then non-streaming

Implement `try_diff_sheets_streaming` as:

1. `sink.begin(pool)?;` (no new strings needed)
2. `finish_guard`
3. create engine `DiffContext` + `HardeningController`
4. call `try_diff_grids(sheet_id, &old.grid, &new.grid, ...)` 
5. finish + return summary

Then implement `try_diff_sheets` / `diff_sheets` as VecSink wrappers, mirroring workbook/grid patterns.

#### B3) Decide how to handle kind/name mismatches (important for backward-compat)

This is the one place where “grounded reality” matters because of the current behavior in `Diffable for Sheet`. 

Today:

* `Sheet.diff` wraps sheets into workbooks and then uses workbook orchestration, which will treat sheet identity as `(lowercased_name, kind)` (see workbook_diff’s `SheetKey`).

For the new leaf API, you have two consistent choices:

* **Leaf semantics (recommended):** always diff `old.grid` vs `new.grid` under `sheet_id = old.name`, regardless of `kind` or `name`.

  * This matches the phrase “leaf diff” and “make workbook orchestration explicit workbook concern.”
  * If you want guardrails, add a *warning* (not a different op type) when `old.kind != new.kind` or names differ.

* **Compatibility semantics:** if kind/name mismatch, emit `SheetRemoved`/`SheetAdded` instead of diffing grids, to match “single-sheet workbook diff” behavior.

  * This preserves edge-case behavior for users of `Diffable for Sheet`, but it reintroduces workbook-ish logic at the sheet layer.

**Phase-6-aligned recommendation:** implement leaf semantics for `engine::diff_sheets`, and if you keep `Diffable for Sheet`, decide separately whether it should preserve compatibility semantics or simply become a thin wrapper around the new leaf API.

---

### Workstream C: Wire exports cleanly (`engine/mod.rs` + `lib.rs::advanced`)

#### C1) Export from `engine/mod.rs`

Right now, `engine/mod.rs` only reexports:

* workbook diff entry points, and
* database-mode grid functions.

Phase 6 requires adding the new leaf exports:

* `pub use grid_diff::{diff_grids, try_diff_grids_streaming, ...};`
* `pub use sheet_diff::{diff_sheets, try_diff_sheets_streaming, ...};`

This is the “official” engine-level API surface that Phase 6 asked for.

#### C2) Reexport from `excel_diff::advanced`

Your crate already uses `advanced` to expose “with_pool” style entry points (`diff_workbooks_with_pool`, `try_diff_workbooks_with_pool`, etc.). 

Add leaf entries there too, so callers can do:

* `excel_diff::advanced::diff_grids_with_pool(...)`
* `excel_diff::advanced::try_diff_grids_streaming(...)`
* `excel_diff::advanced::diff_sheets_with_pool(...)`
* etc.

This keeps the public “recommended” surface (`WorkbookPackage`) unchanged, while giving power users/test code a clean direct route.

---

### Workstream D: Re-scope `Diffable` to stop cloning (and decide its future)

This is the second bullet of Phase 6. 

#### D1) Remove clone-heavy implementations immediately

Replace the two implementations shown below: 

* `impl Diffable for Sheet { ... self.clone() ... other.clone() ... }`
* `impl Diffable for Grid { ... self.clone() ... other.clone() ... }`

With versions that call your new leaf APIs.

This single change is where the “grounded” performance win comes from: it completely removes cloning for grid/sheet diffs.

#### D2) Decide “narrow” vs “remove”

Phase 6 text explicitly allows either. 

A pragmatic approach that minimizes churn:

* **Phase 6 outcome:** keep `Diffable` but make `Sheet` and `Grid` impls non-clone-heavy by delegating to leaf APIs.
* **Follow-on (later phase):** optionally deprecate `Diffable for Sheet/Grid` if you want to force explicit API selection (workbook vs leaf).

This matches how you already handle “legacy API” paths: leave them available but route users to the preferred entry points.

---

## Test and validation plan (must be as strong as workbook streaming tests)

You already have high-quality streaming contract tests for:

* workbook streaming finish-once, finish-on-error, limit errors
* database-mode streaming finish-once, finish-on-emit-error 
* string table header correctness and “no out-of-range StringId” in JSONL output

Phase 6 should add the same rigor for the new leaf APIs.

### T1) Streaming lifecycle tests for grid leaf streaming

Add to `core/tests/streaming_contract_tests.rs` alongside existing ones:

* `engine_grid_streaming_calls_finish_once`
* `engine_grid_streaming_finishes_on_emit_error`
* `engine_grid_streaming_finishes_on_limit_error` (configure `LimitBehavior::ReturnError` and low alignment limits, matching workbook test pattern)

These should use the same helper sinks already present (`StrictLifecycleSink`, `FailAfterNSink`).

### T2) Streaming lifecycle tests for sheet leaf streaming

Same trio for `try_diff_sheets_streaming`:

* finish once on success
* finish on sink emit error
* finish on limit error

### T3) “No new strings after begin” tests

You already have the “frozen pool” sink in test code.

Add:

* `grid_leaf_streaming_does_not_intern_after_begin`

  * Ensure `"<grid>"` is interned *before* begin in the leaf API implementation.
* `sheet_leaf_streaming_does_not_intern_after_begin`

  * No interning should occur at all in normal cases.

This protects the JSON Lines header contract implied by `JsonLinesSink::begin`. 

### T4) Output equivalence tests vs the old “single-sheet workbook wrapper”

Before you remove/alter the old path, add regression tests proving equivalence:

For grids:

* Build `grid_a`, `grid_b`
* Compare:

  * `engine::diff_grids(&grid_a, &grid_b, ...)`
  * vs `engine::diff_workbooks(&single_sheet_workbook("<grid>", grid_a.clone()), &single_sheet_workbook("<grid>", grid_b.clone()), ...)`
* Assert ops identical and summary flags consistent.

For sheets:

* Same pattern using `Sheet { name: "...", kind: Worksheet, grid: ... }` wrappers.

This anchors correctness on the known-good engine behavior and ensures Phase 6 is a pure refactor for typical cases.

### T5) Determinism tests under `parallel`

There is already a determinism test that diffs two grids using `Diffable` (`a.diff(&b, &mut ctx)`) and compares outputs across thread counts.

After Phase 6, add one of:

* If you keep `Diffable for Grid`: existing test should keep passing (but now it’s actually exercising your new leaf API indirectly).
* If you remove `Diffable for Grid`: rewrite this test to call `engine::diff_grids` directly (and still compare outputs across pools/threads).

Either way, keep the determinism guarantee explicit.

---

## Documentation updates (keep it practical and codebase-aligned)

### D1) `excel_diff::advanced` rustdoc

Add a new section “Leaf diffs” showing how to diff:

* two `Grid`s directly (with `<grid>` naming)
* two `Sheet`s directly (using `sheet.name`)

Also explicitly mention the streaming contract implication:

* “All strings referenced by emitted ops must be interned before `begin()` is called,” and point to the streaming contract doc (you already cite it in engine docs).

### D2) Internal docs: `docs/streaming_contract.md`

Optionally add leaf APIs as additional “producers” that must follow the same begin/emit/finish lifecycle. You already centralize that contract; this phase should ensure leaf diffs are documented as first-class contract participants.

---

## Sequencing (the safest order to implement without breaking anything)

1. **Add grid leaf streaming + non-streaming APIs** (`engine/grid_diff.rs`), with tests.
2. **Add sheet leaf streaming + non-streaming APIs** (new file or module), with tests.
3. **Export via `engine/mod.rs` and `excel_diff::advanced`**.
4. **Refactor `Diffable for Sheet/Grid` to delegate to leaf APIs** (no clones).
5. **Run/extend determinism tests** (`parallel_determinism_tests.rs`) and keep the streaming contract suite green.
6. **Optional: deprecate or gate `Diffable for Sheet/Grid`** if you want to narrow the abstraction further (but do this only once the leaf APIs are stable and well-tested).

---

## Definition of done (Phase 6 “exit criteria”)

Phase 6 is complete when all of the following are true:

* `engine::diff_grids` and `engine::diff_sheets` exist (plus streaming forms) and they directly invoke the grid diff pipeline (`try_diff_grids`) rather than constructing temporary workbooks.
* `Diffable for Sheet` and `Diffable for Grid` no longer clone grids/sheets into synthetic workbooks (or those impls are removed and tests updated accordingly).
* New streaming leaf APIs obey the same sink lifecycle guarantees as workbook/database-mode streaming (finish called exactly once; finish called even on emit/limit error).
* Leaf streaming does not mutate the string pool after `begin()`; JSONL headers remain valid (no out-of-range IDs).
* Determinism across thread counts remains intact (either through `Diffable` wrappers or direct leaf API tests).

