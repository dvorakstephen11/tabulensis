I reviewed the updated `codebase_context.md`. The big structural pieces from the maintainability / “elegant simplicity” plan look in place:

* `core/src/engine/` split is present (`context`, `grid_diff`, `move_mask`, `amr`, `workbook_diff`).
* Emission is centralized via `EmitCtx` + `emit_op`.
* Streaming sinks now have `begin()` / `finish()`, and `NoFinishSink` exists to prevent double-finishing in package streaming.
* Package streaming precomputes M-diff ops so the JSONL header string table includes them.
* The new tests around “finish gets called on error” and “M strings are in the header” are present.

What I’d correct / tighten to fully match the intended posture (clean boundaries, less incidental complexity, fewer hidden dependencies):

---

## 1) Make `SheetId` a value inside `EmitCtx` (remove clones + drop a lifetime)

Right now `EmitCtx` holds `sheet_id: &SheetId`, and many call sites do `ctx.sheet_id.clone()`. But `SheetId` is `StringId`, which is `Copy`, so this is pure noise and adds lifetime friction.

### Change `EmitCtx` + `DiffContext` visibility, and store `sheet_id: SheetId`

**Replace this code in `core/src/engine/context.rs`:**

```rust
#[derive(Debug, Default)]
pub(crate) struct DiffContext {
    pub(crate) warnings: Vec<String>,
    pub(crate) formula_cache: FormulaParseCache,
}

pub(crate) fn emit_op<S: DiffSink>(
    sink: &mut S,
    op_count: &mut usize,
    op: DiffOp,
) -> Result<(), DiffError> {
    sink.emit(op)?;
    *op_count = op_count.saturating_add(1);
    Ok(())
}

pub(crate) struct EmitCtx<'a, S: DiffSink> {
    pub(crate) sheet_id: &'a SheetId,
    pub(crate) pool: &'a StringPool,
    pub(crate) config: &'a DiffConfig,
    pub(crate) cache: &'a mut FormulaParseCache,
    pub(crate) sink: &'a mut S,
    pub(crate) op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    pub(crate) fn new(
        sheet_id: &'a SheetId,
        pool: &'a StringPool,
        config: &'a DiffConfig,
        cache: &'a mut FormulaParseCache,
        sink: &'a mut S,
        op_count: &'a mut usize,
    ) -> Self {
        Self {
            sheet_id,
            pool,
            config,
            cache,
            sink,
            op_count,
        }
    }

    pub(crate) fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}
```

**With this:**

```rust
#[derive(Debug, Default)]
pub(super) struct DiffContext {
    pub(super) warnings: Vec<String>,
    pub(super) formula_cache: FormulaParseCache,
}

pub(super) fn emit_op<S: DiffSink>(
    sink: &mut S,
    op_count: &mut usize,
    op: DiffOp,
) -> Result<(), DiffError> {
    sink.emit(op)?;
    *op_count = op_count.saturating_add(1);
    Ok(())
}

pub(super) struct EmitCtx<'a, S: DiffSink> {
    pub(super) sheet_id: SheetId,
    pub(super) pool: &'a StringPool,
    pub(super) config: &'a DiffConfig,
    pub(super) cache: &'a mut FormulaParseCache,
    pub(super) sink: &'a mut S,
    pub(super) op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    pub(super) fn new(
        sheet_id: SheetId,
        pool: &'a StringPool,
        config: &'a DiffConfig,
        cache: &'a mut FormulaParseCache,
        sink: &'a mut S,
        op_count: &'a mut usize,
    ) -> Self {
        Self {
            sheet_id,
            pool,
            config,
            cache,
            sink,
            op_count,
        }
    }

    pub(super) fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}
```

Notes:

* This also tightens visibility (`pub(super)`), which is the next item, but it pairs well here.
* After this, anything using `ctx.sheet_id.clone()` should become `ctx.sheet_id`.

---

## 2) Propagate `SheetId`-by-value through the grid pipeline

Once `EmitCtx::new` takes `SheetId` by value, you’ll want the rest of the engine sheet-diff path to stop threading `&SheetId`.

### Update `SheetGridDiffer::new` to take `SheetId` by value

**Replace this in `core/src/engine/move_mask.rs`:**

```rust
pub(crate) fn new(
    sheet_id: &'a SheetId,
    old: &'b Grid,
    new: &'b Grid,
    config: &'a DiffConfig,
    pool: &'a StringPool,
    cache: &'a mut FormulaParseCache,
    sink: &'a mut S,
    op_count: &'a mut usize,
    #[cfg(feature = "perf-metrics")]
    metrics: Option<&'a mut DiffMetrics>,
) -> Self {
    let emit_ctx = EmitCtx::new(sheet_id, pool, config, cache, sink, op_count);
    Self {
        emit_ctx,
        old,
        new,
        old_view: GridView::new(old),
        new_view: GridView::new(new),
        masks: BTreeMap::new(),
        #[cfg(feature = "perf-metrics")]
        metrics,
    }
}
```

**With this:**

```rust
pub(crate) fn new(
    sheet_id: SheetId,
    old: &'b Grid,
    new: &'b Grid,
    config: &'a DiffConfig,
    pool: &'a StringPool,
    cache: &'a mut FormulaParseCache,
    sink: &'a mut S,
    op_count: &'a mut usize,
    #[cfg(feature = "perf-metrics")]
    metrics: Option<&'a mut DiffMetrics>,
) -> Self {
    let emit_ctx = EmitCtx::new(sheet_id, pool, config, cache, sink, op_count);
    Self {
        emit_ctx,
        old,
        new,
        old_view: GridView::new(old),
        new_view: GridView::new(new),
        masks: BTreeMap::new(),
        #[cfg(feature = "perf-metrics")]
        metrics,
    }
}
```

### Update `try_diff_grids` + `diff_grids_core` signatures

**Replace this in `core/src/engine/grid_diff.rs`:**

```rust
pub(crate) fn try_diff_grids<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    ...
}
```

**With this:**

```rust
pub(super) fn try_diff_grids<S: DiffSink>(
    sheet_id: SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    ...
}
```

And similarly, change:

**Replace:**

```rust
fn diff_grids_core<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    ...
}
```

**With:**

```rust
fn diff_grids_core<S: DiffSink>(
    sheet_id: SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    ...
}
```

Then update the construction:

**Replace:**

```rust
let mut differ = SheetGridDiffer::new(
    sheet_id,
    old,
    new,
    config,
    pool,
    &mut ctx.formula_cache,
    sink,
    op_count,
    metrics,
);
```

**With:**

```rust
let mut differ = SheetGridDiffer::new(
    sheet_id,
    old,
    new,
    config,
    pool,
    &mut ctx.formula_cache,
    sink,
    op_count,
    metrics,
);
```

That call site doesn’t change textually, but now `sheet_id` is a value everywhere.

Also fix any warning formatting that does `*sheet_id`:

**Replace:**

```rust
pool.resolve(*sheet_id).unwrap_or("<unknown>")
```

**With:**

```rust
pool.resolve(sheet_id).unwrap_or("<unknown>")
```

### Update the workbook-level call site

**Replace this in `core/src/engine/workbook_diff.rs`:**

```rust
try_diff_grids(
    &sheet_id,
    old_sheet.grid,
    new_sheet.grid,
    config,
    pool,
    sink,
    op_count,
    ctx,
    #[cfg(feature = "perf-metrics")]
    metrics,
)?;
```

**With this:**

```rust
try_diff_grids(
    sheet_id,
    old_sheet.grid,
    new_sheet.grid,
    config,
    pool,
    sink,
    op_count,
    ctx,
    #[cfg(feature = "perf-metrics")]
    metrics,
)?;
```

### Mechanical cleanup: remove all `sheet_id.clone()` on `EmitCtx`

Once `EmitCtx.sheet_id` is a `SheetId`, the right pattern is just `ctx.sheet_id`.

Example replacement:

**Replace:**

```rust
ctx.emit(crate::diff::DiffOp::row_added(ctx.sheet_id.clone(), row_idx, new.row_height(row_idx)))?;
```

**With:**

```rust
ctx.emit(crate::diff::DiffOp::row_added(ctx.sheet_id, row_idx, new.row_height(row_idx)))?;
```

Do this for every occurrence in:

* `core/src/engine/grid_diff.rs`
* `core/src/engine/move_mask.rs`

---

## 3) Tighten internal visibility to avoid “engine leakage”

Right now a lot of engine plumbing is `pub(crate)`, which makes it easy for unrelated modules to start depending on internals over time.

Policy that matches the plan:

* Inside `core/src/engine/*`, default to private.
* Use `pub(super)` for helpers needed across `engine::...` submodules.
* Only re-export the true API surface from `engine/mod.rs`.

You already have the re-export layer in `engine/mod.rs`, so the main work is:

* `context.rs`: already handled above.
* `grid_diff.rs`: change helper fns that are only used inside engine from `pub(crate)` to `pub(super)`.

This is a mechanical pass: after changing visibility, run a compile and only bump visibility back up where a real cross-module usage exists.

---

## 4) Simplify perf-metrics instrumentation using `PhaseGuard`

You already have `DiffMetrics::phase_guard()` and `PhaseGuard` in `perf.rs`, but a lot of code still does explicit `start_phase()` / `end_phase()` pairs.

This is exactly the kind of “paperwork” that hurts readability over time.

### Example: replace start/end pairs in `diff_grids_core`

**Replace:**

```rust
#[cfg(feature = "perf-metrics")]
if let Some(m) = differ.metrics.as_mut() {
    m.start_phase(Phase::MoveDetection);
}

differ.detect_moves()?;

#[cfg(feature = "perf-metrics")]
if let Some(m) = differ.metrics.as_mut() {
    m.end_phase(Phase::MoveDetection);
}
```

**With:**

```rust
#[cfg(feature = "perf-metrics")]
let _phase = differ
    .metrics
    .as_deref_mut()
    .map(|m| m.phase_guard(Phase::MoveDetection));

differ.detect_moves()?;
```

### Example: `run_positional_diff_with_metrics`

**Replace:**

```rust
#[cfg(feature = "perf-metrics")]
if let Some(m) = metrics.as_mut() {
    m.start_phase(Phase::CellDiff);
}

let compared = positional_diff(ctx, old, new)?;

#[cfg(feature = "perf-metrics")]
if let Some(m) = metrics.as_mut() {
    m.add_cells_compared(compared);
    m.end_phase(Phase::CellDiff);
}

Ok(())
```

**With:**

```rust
#[cfg(feature = "perf-metrics")]
let _phase = metrics.as_deref_mut().map(|m| m.phase_guard(Phase::CellDiff));

let compared = positional_diff(ctx, old, new)?;

#[cfg(feature = "perf-metrics")]
if let Some(m) = metrics.as_deref_mut() {
    m.add_cells_compared(compared);
}

Ok(())
```

Apply the same pattern to:

* row-alignment phases
* single-column-alignment phases
* any other paired start/end blocks

---

## 5) The one architectural gap: `move_mask.rs` imports `grid_diff.rs` internals

This is the main remaining maintainability smell introduced by the split: `move_mask` pulls in helpers like `emit_cell_edit`, `try_row_alignment_internal`, `run_positional_diff_with_metrics`, etc.

That creates a “soft cycle” in the design:

* `grid_diff` depends on `SheetGridDiffer` (in `move_mask`)
* `move_mask` depends on `grid_diff` helpers

It compiles, but it makes the boundaries fuzzy and increases the mental load of future changes.

### Recommended fix: extract the shared helpers into a third internal module

Create a module like:

* `core/src/engine/grid_primitives.rs` (or `grid_helpers.rs`)

Move the shared helpers there (no behavior change), and then both `grid_diff.rs` and `move_mask.rs` import from that module instead of each other.

At minimum, the “shared set” is the exact list `move_mask.rs` currently imports from `grid_diff.rs`:

* `cells_content_equal`
* `emit_cell_edit`
* `emit_row_block_move`
* `emit_column_block_move`
* `emit_rect_block_move`
* `emit_moved_row_block_edits`
* `run_positional_diff_with_metrics`
* `try_row_alignment_internal`
* `try_single_column_alignment_internal`

#### Update the import in `move_mask.rs`

**Replace:**

```rust
use super::grid_diff::{
    cells_content_equal, emit_cell_edit, emit_column_block_move, emit_moved_row_block_edits,
    emit_rect_block_move, emit_row_block_move, run_positional_diff_with_metrics,
    try_row_alignment_internal, try_single_column_alignment_internal,
};
```

**With:**

```rust
use super::grid_primitives::{
    cells_content_equal, emit_cell_edit, emit_column_block_move, emit_moved_row_block_edits,
    emit_rect_block_move, emit_row_block_move, run_positional_diff_with_metrics,
    try_row_alignment_internal, try_single_column_alignment_internal,
};
```

#### Add the module to `engine/mod.rs`

Create the new file and then:

**Replace in `core/src/engine/mod.rs`:**

```rust
mod grid_diff;
mod move_mask;
```

**With:**

```rust
mod grid_diff;
mod grid_primitives;
mod move_mask;
```

In `grid_diff.rs`, import what it needs from `grid_primitives` (or keep call sites unchanged by re-exporting them from `grid_diff`).

This is the last “big” cleanup that makes the engine split feel truly clean instead of just “files moved around”.

---

## Verification checklist (so you catch regressions immediately)

Run these after each step:

* `cargo fmt`
* `cargo test -p core`
* `cargo test -p core --features perf-metrics` (if that’s a supported workflow)
* `cargo clippy -p core -- -D warnings` (at least once at the end)

Pay special attention to:

* `SheetId` changes (signature mismatches are easy to spot)
* any places that accidentally kept `ctx.sheet_id.clone()`
* any visibility change that leaks outside `engine/`

---

If you apply the five items above in order, you’ll end up with:

* cleaner types (less lifetime noise)
* tighter internal boundaries
* less metrics boilerplate
* and (most importantly) no cross-module “soft cycles” after the refactor.
