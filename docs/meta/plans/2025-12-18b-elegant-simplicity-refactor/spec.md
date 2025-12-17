Below is a concrete, implementation-oriented plan to deliver the **Elegant Simplicity** refactors I recommended: reduce duplication, make control-flow read like the algorithmic narrative (select strategy → align → emit structure ops → emit cell edits), and shrink “parameter sprawl” so functions explain themselves. The plan is intentionally staged so you can keep the build green after each step and avoid a risky big-bang rewrite.

---

# Elegant Simplicity Refactor Plan

## Goals

1. **Delete accidental duplication** (especially “legacy vs AMR” parallel types and nearly-identical emit paths). 
2. **Make the grid diff driver read like the spec’s phases** (alignment strategy choice and fallbacks become explicit helpers). 
3. **Reduce signature noise** by bundling the stable context (`sheet_id`, `pool`, `config`, `cache`, `sink`, `op_count`) into a small struct so leaf functions become readable. 
4. Preserve correctness invariants: **DiffOp ordering is stable** (tests assume list order is meaningful). 

---

## Phase 1: Unify RowAlignment/RowBlockMove types (remove “two worlds”)

### Why this is worth doing

Right now the code has **two `RowAlignment` types**: one in `alignment` (AMR) and one in `row_alignment` (legacy). The engine therefore carries aliases (`AmrAlignment`, `LegacyRowAlignment`) and separate emit functions (`emit_aligned_diffs`, `emit_amr_aligned_diffs`). That’s classic accidental complexity: the domain concept is “row alignment”, but the code forces you to keep two mental models.

### Step 1.1: In `core/src/row_alignment.rs`, delete the local struct definitions and re-use AMR’s types

**Replace this code** in `core/src/row_alignment.rs`: 

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowAlignment {
    pub matched: Vec<(u32, u32)>, // (row_idx_a, row_idx_b)
    pub inserted: Vec<u32>,       // row indices in B
    pub deleted: Vec<u32>,        // row indices in A
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}
```

**With this code**:

```rust
pub(crate) use crate::alignment::{RowAlignment, RowBlockMove};
```

This keeps the legacy algorithms intact while **collapsing the type universe** to a single alignment output type.

### Step 1.2: Patch the legacy constructors to populate the `moves` field

Because AMR’s `RowAlignment` includes a `moves: Vec<RowBlockMove>`, any struct literals must fill it.

**Replace this code** (first occurrence): 

```rust
    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
    };
```

**With this code**:

```rust
    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    };
```

**Replace this code** (second occurrence): 

```rust
    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
    };
```

**With this code**:

```rust
    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    };
```

### Step 1.3: Keep API stability for call sites

No other code needs to “know” that legacy alignments now carry `moves: vec![]`; they still use `matched/inserted/deleted` exactly as before.

---

## Phase 2: Collapse the duplicated “row-aligned emit” paths in `engine.rs`

### Why this is worth doing

`emit_aligned_diffs` and `emit_amr_aligned_diffs` are the same story with minor differences:

* AMR emits `moves` and column add/remove ops.
* legacy emits matched/inserted/deleted rows.

Once legacy uses the same `RowAlignment` type, this becomes one function with conditional behavior based on data (moves empty → emits nothing). 

### Step 2.1: Simplify imports and remove the “legacy vs AMR” aliases

**Replace this import block** in `core/src/engine.rs`: 

```rust
use crate::alignment::move_extraction::moves_from_matched_pairs;
use crate::alignment::{RowAlignment as AmrAlignment, align_rows_amr_with_signatures_from_views};
use crate::row_alignment::{
    RowAlignment as LegacyRowAlignment, RowBlockMove as LegacyRowBlockMove,
    align_row_changes_from_views, detect_exact_row_block_move, detect_fuzzy_row_block_move,
};
```

**With this import block**:

```rust
use crate::alignment::move_extraction::moves_from_matched_pairs;
use crate::alignment::{RowAlignment, RowBlockMove, align_rows_amr_with_signatures_from_views};
use crate::row_alignment::{align_row_changes_from_views, detect_exact_row_block_move, detect_fuzzy_row_block_move};
```

### Step 2.2: Update `emit_row_block_move` to accept the unified type

**Replace this function signature** in `engine.rs`: 

```rust
fn emit_row_block_move(
    sheet_id: &SheetId,
    mv: LegacyRowBlockMove,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
```

**With this signature**:

```rust
fn emit_row_block_move(
    sheet_id: &SheetId,
    mv: RowBlockMove,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
```

No body change required (fields match).

### Step 2.3: Replace both emit functions with one `emit_row_aligned_diffs`

**Replace these two functions** in `engine.rs`: 

```rust
fn emit_aligned_diffs(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &LegacyRowAlignment,
    pool: &StringPool,
    cache: &mut FormulaParseCache,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    for &(old_row, new_row) in &alignment.matched {
        diff_row_pair(
            sheet_id,
            &old_view.source,
            &new_view.source,
            old_row,
            new_row,
            old_view.source.ncols.min(new_view.source.ncols),
            pool,
            config,
            cache,
            sink,
            op_count,
        )?;
    }
    for &new_row in &alignment.inserted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_added(sheet_id.clone(), new_row, None),
        )?;
    }
    for &old_row in &alignment.deleted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id.clone(), old_row, None),
        )?;
    }
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols) as u64;
    Ok(overlap_cols.saturating_mul(alignment.matched.len() as u64))
}

fn emit_amr_aligned_diffs(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &AmrAlignment,
    pool: &StringPool,
    cache: &mut FormulaParseCache,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    for mv in &alignment.moves {
        emit_row_block_move(sheet_id, *mv, sink, op_count)?;
    }

    if old_view.source.ncols < new_view.source.ncols {
        for col_idx in old_view.source.ncols..new_view.source.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_added(sheet_id.clone(), col_idx, None),
            )?;
        }
    }
    if old_view.source.ncols > new_view.source.ncols {
        for col_idx in new_view.source.ncols..old_view.source.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_removed(sheet_id.clone(), col_idx, None),
            )?;
        }
    }

    for &(old_row, new_row) in &alignment.matched {
        diff_row_pair(
            sheet_id,
            &old_view.source,
            &new_view.source,
            old_row,
            new_row,
            old_view.source.ncols.min(new_view.source.ncols),
            pool,
            config,
            cache,
            sink,
            op_count,
        )?;
    }
    for &new_row in &alignment.inserted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_added(sheet_id.clone(), new_row, None),
        )?;
    }
    for &old_row in &alignment.deleted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id.clone(), old_row, None),
        )?;
    }
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols) as u64;
    Ok(overlap_cols.saturating_mul(alignment.matched.len() as u64))
}
```

**With this single function**:

```rust
fn emit_row_aligned_diffs(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &RowAlignment,
    pool: &StringPool,
    cache: &mut FormulaParseCache,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    for mv in &alignment.moves {
        emit_row_block_move(sheet_id, *mv, sink, op_count)?;
    }

    if old_view.source.ncols < new_view.source.ncols {
        for col_idx in old_view.source.ncols..new_view.source.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_added(sheet_id.clone(), col_idx, None),
            )?;
        }
    }
    if old_view.source.ncols > new_view.source.ncols {
        for col_idx in new_view.source.ncols..old_view.source.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_removed(sheet_id.clone(), col_idx, None),
            )?;
        }
    }

    for &(old_row, new_row) in &alignment.matched {
        diff_row_pair(
            sheet_id,
            &old_view.source,
            &new_view.source,
            old_row,
            new_row,
            old_view.source.ncols.min(new_view.source.ncols),
            pool,
            config,
            cache,
            sink,
            op_count,
        )?;
    }

    for &new_row in &alignment.inserted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_added(sheet_id.clone(), new_row, None),
        )?;
    }

    for &old_row in &alignment.deleted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id.clone(), old_row, None),
        )?;
    }

    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols) as u64;
    Ok(overlap_cols.saturating_mul(alignment.matched.len() as u64))
}
```

### Step 2.4: Update call sites in `diff_grids_core`

**Replace this call** in the AMR path: 

```rust
emit_amr_aligned_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    op_count,
    config,
)?;
```

**With this**:

```rust
emit_row_aligned_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    op_count,
    config,
)?;
```

**Replace this call** in the legacy row alignment path: 

```rust
emit_aligned_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    op_count,
    config,
)?;
```

**With this**:

```rust
emit_row_aligned_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    op_count,
    config,
)?;
```

At this point, you can delete the old functions entirely.

---

## Phase 3: Reduce repetition in the AMR “fallback to positional” path

### Why this is worth doing

The AMR branch contains multiple cut-and-paste blocks:

* end alignment metrics
* start cell-diff metrics
* run positional diff
* record cells compared
* end cell-diff metrics
* return

That repetition makes the code hard to scan: the “why” is buried in boilerplate.

### Step 3.1: Add a single helper for “positional diff with metrics”

Add this helper near `positional_diff` (same module/file):

```rust
fn run_positional_diff_with_metrics<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    pool: &StringPool,
    config: &DiffConfig,
    cache: &mut FormulaParseCache,
    sink: &mut S,
    op_count: &mut usize,
    #[cfg(feature = "perf-metrics")] metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }

    positional_diff(sheet_id, old, new, pool, config, cache, sink, op_count)?;

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.add_cells_compared(cells_in_overlap(old, new));
        m.end_phase(Phase::CellDiff);
    }

    Ok(())
}
```

### Step 3.2: Replace one repeated fallback block (and then apply the same replacement to the rest)

Here’s one representative fallback (triggered by “row signatures not preserved” in AMR). **Replace this code**: 

```rust
#[cfg(feature = "perf-metrics")]
if let Some(m) = metrics.as_mut() {
    m.end_phase(Phase::Alignment);
}

positional_diff(
    sheet_id,
    old,
    new,
    pool,
    config,
    &mut ctx.formula_cache,
    sink,
    op_count,
)?;

#[cfg(feature = "perf-metrics")]
if let Some(m) = metrics.as_mut() {
    m.start_phase(Phase::CellDiff);
    m.add_cells_compared(cells_in_overlap(old, new));
    m.end_phase(Phase::CellDiff);
}

return Ok(());
```

**With this code**:

```rust
#[cfg(feature = "perf-metrics")]
if let Some(m) = metrics.as_mut() {
    m.end_phase(Phase::Alignment);
}

run_positional_diff_with_metrics(
    sheet_id,
    old,
    new,
    pool,
    config,
    &mut ctx.formula_cache,
    sink,
    op_count,
    metrics,
)?;

return Ok(());
```

Apply the same pattern to the other AMR early-exit points. The effect is that AMR logic reads as:

* “Check condition; if violated, fall back to positional”
  rather than “check condition; then a wall of metrics boilerplate”.

This directly supports the spec’s “hybrid pipeline with fallbacks” description by making the fallback explicit and uniform. 

---

## Phase 4: Bundle “stable context” to shrink function signatures (parameter sprawl → one object)

### Why this is worth doing

Leaf functions like `emit_cell_edit` currently take 10 parameters, mostly the same across every call site (`sheet_id`, `pool`, `config`, `cache`, `sink`, `op_count`). That’s not domain complexity; it’s wiring complexity. 

### Step 4.1: Introduce an `EmitCtx` and convert `emit_cell_edit` first (thin vertical slice)

Add this struct near the top of `engine.rs` (or in a new `emit.rs` module later):

```rust
struct EmitCtx<'a, S: DiffSink> {
    sheet_id: &'a SheetId,
    pool: &'a StringPool,
    config: &'a DiffConfig,
    cache: &'a mut FormulaParseCache,
    sink: &'a mut S,
    op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}
```

Now convert `emit_cell_edit`.

**Replace this function**: 

```rust
fn emit_cell_edit(
    sheet_id: &SheetId,
    addr: CellAddress,
    old_cell: Option<&Cell>,
    new_cell: Option<&Cell>,
    row_shift: i32,
    col_shift: i32,
    pool: &StringPool,
    config: &DiffConfig,
    cache: &mut FormulaParseCache,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    let from = snapshot_with_addr(old_cell, addr);
    let to = snapshot_with_addr(new_cell, addr);
    let formula_diff =
        compute_formula_diff(pool, cache, old_cell, new_cell, row_shift, col_shift, config);
    emit_op(
        sink,
        op_count,
        DiffOp::cell_edited(sheet_id.clone(), addr, from, to, formula_diff),
    )
}
```

**With this version**:

```rust
fn emit_cell_edit<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    addr: CellAddress,
    old_cell: Option<&Cell>,
    new_cell: Option<&Cell>,
    row_shift: i32,
    col_shift: i32,
) -> Result<(), DiffError> {
    let from = snapshot_with_addr(old_cell, addr);
    let to = snapshot_with_addr(new_cell, addr);
    let formula_diff = compute_formula_diff(
        ctx.pool,
        ctx.cache,
        old_cell,
        new_cell,
        row_shift,
        col_shift,
        ctx.config,
    );
    ctx.emit(DiffOp::cell_edited(ctx.sheet_id.clone(), addr, from, to, formula_diff))
}
```

### Step 4.2: Propagate the pattern outward

After the above compiles, convert outward in this order (low risk to high risk):

1. `diff_row_pair(...)` → `diff_row_pair(ctx, old, new, old_row, new_row, overlap_cols)`
2. `positional_diff(...)` constructs `EmitCtx` once and loops rows calling `diff_row_pair(ctx, ...)`.
3. All `emit_*` helpers (`RowAdded`, `RowRemoved`, etc.) become `ctx.emit(DiffOp::...)`.

This is the mechanical part that buys a lot of readability: functions stop being “wiring diagrams”.

---

## Phase 5: Optional but high-leverage: isolate “grid strategy selection” into a named helper

### Why this is worth doing

`diff_grids_core` currently interleaves:

* “try moves/masking”
* “start metrics”
* “try AMR alignment”
* “special-case column-only changes”
* “try legacy row gap”
* “try column alignment”
* “fallback positional”

Even after Phase 1–3, it’s still a lot in one function. Extracting “choose and run a strategy” makes it read like the algorithm spec’s phases. 

### Suggested shape (new helper)

Create:

```rust
fn try_diff_with_amr<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    pool: &StringPool,
    config: &DiffConfig,
    cache: &mut FormulaParseCache,
    sink: &mut S,
    op_count: &mut usize,
    #[cfg(feature = "perf-metrics")] metrics: Option<&mut DiffMetrics>,
) -> Result<bool, DiffError> {
    let Some(amr_result) = align_rows_amr_with_signatures_from_views(old_view, new_view, config)
    else {
        return Ok(false);
    };

    let mut alignment = amr_result.alignment;

    if config.max_move_iterations > 0 {
        inject_moves_from_insert_delete(
            old,
            new,
            &mut alignment,
            &amr_result.row_signatures_a,
            &amr_result.row_signatures_b,
            config,
        );
    }

    if !row_signature_multiset_equal(&amr_result.row_signatures_a, &amr_result.row_signatures_b) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }

        run_positional_diff_with_metrics(
            sheet_id, old, new, pool, config, cache, sink, op_count, metrics,
        )?;
        return Ok(true);
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.end_phase(Phase::Alignment);
        m.start_phase(Phase::CellDiff);
    }

    let compared = emit_row_aligned_diffs(
        sheet_id,
        old_view,
        new_view,
        &alignment,
        pool,
        cache,
        sink,
        op_count,
        config,
    )?;

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.add_cells_compared(compared);
        m.end_phase(Phase::CellDiff);
    }

    Ok(true)
}
```

Then `diff_grids_core` becomes “story-shaped”:

* if exact equal → done
* if masking moves apply → recurse
* start alignment phase
* if try_amr() handled → done
* if legacy row alignment → emit_row_aligned_diffs → done
* if column alignment → emit_col_aligned_diffs → done
* fallback positional

That’s the simplest form of the algorithmic narrative.

---

## Test and safety checklist (do this after each phase)

1. Run unit + integration suites. The ordering contract matters. 
2. Specifically re-run move/alignment milestones (G11–G13) to ensure:

   * moved blocks still emit one `BlockMoved*` instead of delete+insert spam 
3. Re-run snapshot / JSON report tests to ensure op ordering and schema are preserved. 

---

# What you get after these refactors

* One `RowAlignment` concept throughout the engine (legacy and AMR become “different producers of the same alignment type”).
* One “emit row-aligned diffs” implementation, so future changes happen in one place.
* A reusable helper for positional fallback, eliminating metrics boilerplate repetition.
* A path to dramatically simpler signatures by bundling stable context (`EmitCtx`), without changing core algorithms.
* A much clearer diff driver that reflects the spec’s staged pipeline. 
