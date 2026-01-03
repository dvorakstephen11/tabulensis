## What section #3 is asking us to ship

Section **#3** in the sprint outline targets “cheap, high‑impact” parallelism, specifically:

1. **Parallel signature building** (row/col hashes) because it dominates runtime in the “50k dense single edit” benchmark. 
2. **Parallel cell diff for aligned row pairs** because each matched row pair is independent. 
3. Optionally, **parallelize scoring inside move detection** (after the above). 

In the current implementation, signature building happens when `try_diff_grids_with_metrics` enters `Phase::SignatureBuild` and constructs two `GridView`s via `GridView::from_grid_with_config(...)`. 
`GridView::from_grid_with_config` currently does per-row sort + per-row hashing **sequentially**. 

Aligned-row cell diff is done in `emit_row_aligned_diffs`, which walks `alignment.matched` sequentially and calls `diff_row_pair_sparse(...)` per pair. 

The plan below implements #3 while keeping **deterministic output** across thread counts (a requirement called out in the sprint outline). 

---

## 3.1 Add a native-only `parallel` feature (Rayon), enabled by CLI, disabled by wasm

### Files to change

* `core/Cargo.toml` (add optional `rayon`, add `parallel` feature) 
* `cli/Cargo.toml` (enable `parallel` feature on `excel_diff`) 
* `wasm/Cargo.toml` (leave as-is, keep `parallel` disabled) 
* `core/src/lib.rs` (guard: compile error if `parallel` enabled on wasm)

### Core crate feature wiring

Replace this block in `core/Cargo.toml`:

```toml
[features]
default = ["excel-open-xml", "std-fs", "vba"]
legacy-api = []
perf-metrics = []
model-diff = []
dev-apis = []
excel-open-xml = ["dep:quick-xml", "dep:zip"]
std-fs = []
vba = ["dep:base64", "dep:sha1", "dep:sha2"]

[dependencies]
# ...
xxhash-rust = { version = "0.8.10", features = ["xxh3"] }
zip = { version = "0.6.6", optional = true, default-features = false, features = ["deflate"] }
```

With:

```toml
[features]
default = ["excel-open-xml", "std-fs", "vba"]
legacy-api = []
perf-metrics = []
model-diff = []
dev-apis = []
parallel = ["dep:rayon"]
excel-open-xml = ["dep:quick-xml", "dep:zip"]
std-fs = []
vba = ["dep:base64", "dep:sha1", "dep:sha2"]

[dependencies]
# ...
rayon = { version = "1.10.0", optional = true }
xxhash-rust = { version = "0.8.10", features = ["xxh3"] }
zip = { version = "0.6.6", optional = true, default-features = false, features = ["deflate"] }
```

(Exact Rayon version can be adjusted to match repo policy; this is the wiring pattern.)

### CLI enables `parallel`

Replace this in `cli/Cargo.toml`:

```toml
excel_diff = { path = "../core", features = ["model-diff"] }
```

With:

```toml
excel_diff = { path = "../core", features = ["model-diff", "parallel"] }
```

### wasm stays single-threaded

No change needed; wasm already uses `default-features = false` and explicitly lists features; do not add `parallel`. 

### Prevent accidental wasm+parallel builds

In `core/src/lib.rs`, add a compile-time guard near the top-level module declarations.

Replace something like:

```rust
pub mod engine;
pub mod workbook;
// ...
```

With:

```rust
#[cfg(all(feature = "parallel", target_arch = "wasm32"))]
compile_error!("feature \"parallel\" is not supported on wasm32");

pub mod engine;
pub mod workbook;
// ...
```

This makes the intended “native-only” contract explicit. 

---

## 3.2 Parallel signature building (focus on row signatures in `GridView`)

### Why here

`GridView::from_grid_with_config` is used during `Phase::SignatureBuild` for both old/new grids. 
It currently:

* pushes all cells into per-row vectors,
* sorts each row’s cells,
* hashes each row sequentially. 

This is exactly the “pure CPU” hotspot #3 calls out. 

### What we’ll parallelize (cheap + safe)

* **Sort each row’s cell vector** in parallel (`rows.par_iter_mut()`).
* **Compute `RowMeta.signature`** in parallel (`rows.par_iter().enumerate()`).

We keep:

* the single pass that builds `col_hashers` sequential (hard to parallelize without changing hashing semantics). 

### Implementation approach

Add small helper functions in `core/src/grid_view.rs` to keep `from_grid_with_config` readable and to allow build-time gating.

#### Modify `GridView::from_grid_with_config`

Inside `GridView::from_grid_with_config`, you currently have:

* sequential per-row sort 
* sequential `row_meta` construction 

Replace the “sort rows” + “build row_meta” region with a parallel-capable version.

**Replace this block** (excerpt):

```rust
for row_view in rows.iter_mut() {
    row_view.cells.sort_unstable_by_key(|(col, _)| *col);
}

let mut row_meta: Vec<RowMeta> = rows
    .iter()
    .enumerate()
    .map(|(idx, row_view)| {
        let count = row_counts.get(idx).copied().unwrap_or(0);
        let non_blank_count = to_u16(count);
        let first_non_blank_col = row_first_non_blank
            .get(idx)
            .and_then(|c| c.map(to_u16))
            .unwrap_or(0);
        let is_low_info = compute_is_low_info(non_blank_count, row_view);

        let signature = RowSignature {
            hash: hash_row_content_128(&row_view.cells),
        };

        let frequency_class = if is_low_info {
            FrequencyClass::LowInfo
        } else {
            FrequencyClass::Common
        };

        RowMeta {
            row_idx: idx as u32,
            signature,
            non_blank_count,
            first_non_blank_col,
            frequency_class,
            is_low_info,
        }
    })
    .collect();

classify_row_frequencies(&mut row_meta, config);
```

**With this block**:

```rust
let total_cells = total_cells;

sort_row_cells(&mut rows, total_cells);

let mut row_meta =
    build_row_meta(&rows, &row_counts, &row_first_non_blank, config, total_cells);

classify_row_frequencies(&mut row_meta, config);
```

Then add these helpers in the same module (near other private helpers):

```rust
const PAR_MIN_ROWS: usize = 2048;
const PAR_MIN_CELLS: usize = 200_000;

fn should_parallelize_rows(row_len: usize, total_cells: usize) -> bool {
    row_len >= PAR_MIN_ROWS && total_cells >= PAR_MIN_CELLS
}

fn sort_row_cells(rows: &mut [RowView<'_>], total_cells: usize) {
    #[cfg(feature = "parallel")]
    {
        if should_parallelize_rows(rows.len(), total_cells) {
            use rayon::prelude::*;
            rows.par_iter_mut()
                .for_each(|r| r.cells.sort_unstable_by_key(|(c, _)| *c));
            return;
        }
    }

    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    total_cells: usize,
) -> Vec<RowMeta> {
    #[cfg(feature = "parallel")]
    {
        if should_parallelize_rows(rows.len(), total_cells) {
            use rayon::prelude::*;
            return rows
                .par_iter()
                .enumerate()
                .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
                .collect();
        }
    }

    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}

fn row_meta_for_row<'a>(
    idx: usize,
    row_view: &RowView<'a>,
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
) -> RowMeta {
    let count = row_counts.get(idx).copied().unwrap_or(0);
    let non_blank_count = to_u16(count);
    let first_non_blank_col = row_first_non_blank
        .get(idx)
        .and_then(|c| c.map(to_u16))
        .unwrap_or(0);

    let is_low_info = compute_is_low_info(non_blank_count, row_view);

    let signature = RowSignature {
        hash: hash_row_content_128(&row_view.cells),
    };

    let frequency_class = if is_low_info {
        FrequencyClass::LowInfo
    } else {
        FrequencyClass::Common
    };

    RowMeta {
        row_idx: idx as u32,
        signature,
        non_blank_count,
        first_non_blank_col,
        frequency_class,
        is_low_info,
    }
}
```

#### Also track `total_cells` during the `iter_cells` loop

Right now the loop pushes cells but does not count them. 
Add `let mut total_cells: usize = 0;` before the loop and increment inside it.

(That’s just a tiny local change; no snippet needed beyond “increment `total_cells += 1` when `rows[r].cells.push(...)` happens”.)

### Determinism guarantee

* We do not reorder rows: `par_iter().enumerate().collect::<Vec<_>>()` on an indexed slice preserves order.
* We still sort each row by `col`, so per-row signature hashing sees the same sequence. 
* Frequency classification remains a single-threaded post-pass over `row_meta`, so it’s deterministic given the same signatures. 

### Optional: parallelize other obvious per-item hashing loops

If/when you want more wins without risk, `unordered_col_hashes` (used in column alignment) hashes each column independently and can be safely parallelized by column using the same `#[cfg(feature="parallel")]` pattern. 

---

## 3.3 Parallel cell diff for aligned row pairs (stable merge, deterministic output)

### Where to parallelize

Aligned row pairs live in `RowAlignment.matched` and are processed in `emit_row_aligned_diffs`. 
Each matched pair is independent for the expensive part: **scan the two row cell lists and decide which cells changed** via `diff_row_pair_sparse`. 

### Key constraint: don’t emit from worker threads

`EmitCtx` holds:

* the sink,
* op_count,
* warnings,
* the hardening controller,
* and the mutable formula parse cache. 

So the safe model is:

1. **Parallel planning**: compute “what changed” per row pair (a pure function).
2. **Sequential emission**: in deterministic row order, turn planned edits into `DiffOp`s and call `ctx.emit(...)`.

This also matches the sprint outline’s “stable merge” requirement. 

### Step 3.3.1: Split “plan” from “emit” for row diffs

Right now, `diff_row_pair_sparse` takes `&mut EmitCtx` mostly to read `ctx.config` (and `include_unchanged_cells` / dense threshold). 

Add a pure planner function that takes only `&DiffConfig` and data slices.

In `core/src/engine/grid_primitives.rs`:

**Replace this function signature + body** (the existing `diff_row_pair_sparse`) with a wrapper that delegates to a pure planner.

Old (current pattern) :

```rust
pub(super) fn diff_row_pair_sparse<'a, S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> Result<RowDiffResult<'a>, DiffError> {
    // ...
}
```

New:

```rust
fn diff_row_pair_sparse_plan<'a>(
    config: &DiffConfig,
    overlap_cols: u32,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> RowDiffResult<'a> {
    let Some(threshold) = dense_row_replace_threshold(config, overlap_cols) else {
        return diff_row_pair_sparse_thresholdless(config, old_cells, new_cells);
    };

    let mut compared = 0u64;
    let mut changed_cells = 0u32;
    let mut pending: Vec<PendingCell<'a>> = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_cells.len() || j < new_cells.len() {
        let col = match (old_cells.get(i), new_cells.get(j)) {
            (Some((ca, _)), Some((cb, _))) => (*ca).min(*cb),
            (Some((ca, _)), None) => *ca,
            (None, Some((cb, _))) => *cb,
            (None, None) => break,
        };

        let mut old_cell = None;
        let mut new_cell = None;

        if let Some((c, cell)) = old_cells.get(i) {
            if *c == col {
                old_cell = Some(*cell);
                i += 1;
            }
        }
        if let Some((c, cell)) = new_cells.get(j) {
            if *c == col {
                new_cell = Some(*cell);
                j += 1;
            }
        }

        compared = compared.saturating_add(1);

        if cells_content_equal(old_cell, new_cell) {
            if config.include_unchanged_cells {
                pending.push(PendingCell { col, old_cell, new_cell });
            }
            continue;
        }

        changed_cells = changed_cells.saturating_add(1);
        if changed_cells >= threshold {
            return RowDiffResult {
                compared,
                replaced: true,
                pending: Vec::new(),
            };
        }

        pending.push(PendingCell { col, old_cell, new_cell });
    }

    RowDiffResult {
        compared,
        replaced: false,
        pending,
    }
}

fn diff_row_pair_sparse_thresholdless<'a>(
    config: &DiffConfig,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> RowDiffResult<'a> {
    let mut compared = 0u64;
    let mut pending: Vec<PendingCell<'a>> = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_cells.len() || j < new_cells.len() {
        let col = match (old_cells.get(i), new_cells.get(j)) {
            (Some((ca, _)), Some((cb, _))) => (*ca).min(*cb),
            (Some((ca, _)), None) => *ca,
            (None, Some((cb, _))) => *cb,
            (None, None) => break,
        };

        let mut old_cell = None;
        let mut new_cell = None;

        if let Some((c, cell)) = old_cells.get(i) {
            if *c == col {
                old_cell = Some(*cell);
                i += 1;
            }
        }
        if let Some((c, cell)) = new_cells.get(j) {
            if *c == col {
                new_cell = Some(*cell);
                j += 1;
            }
        }

        compared = compared.saturating_add(1);

        if config.include_unchanged_cells || !cells_content_equal(old_cell, new_cell) {
            pending.push(PendingCell { col, old_cell, new_cell });
        }
    }

    RowDiffResult {
        compared,
        replaced: false,
        pending,
    }
}

pub(super) fn diff_row_pair_sparse<'a, S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    _row_a: u32,
    _row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> Result<RowDiffResult<'a>, DiffError> {
    Ok(diff_row_pair_sparse_plan(ctx.config, overlap_cols, old_cells, new_cells))
}
```

This keeps all existing call sites working (still returns `Result<...>`), but unlocks the planner for parallel use.

### Step 3.3.2: Add a chunked parallel planner for `alignment.matched`

We want to avoid “buffer the entire sheet worth of pending edits”, especially in streaming mode; chunking keeps memory bounded while still leveraging parallelism.

Add an internal result struct:

```rust
struct RowPairPlan<'a> {
    row_a: u32,
    row_b: u32,
    skipped: bool,
    replaced: bool,
    compared: u64,
    pending: Vec<PendingCell<'a>>,
}
```

Then a helper that computes plans for a chunk, using Rayon when available:

```rust
fn plan_row_pair_chunk<'a>(
    old_view: &'a GridView<'a>,
    new_view: &'a GridView<'a>,
    chunk: &[(u32, u32)],
    overlap_cols: u32,
    config: &DiffConfig,
) -> Vec<RowPairPlan<'a>> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        return chunk
            .par_iter()
            .map(|(row_a, row_b)| plan_one_row_pair(old_view, new_view, *row_a, *row_b, overlap_cols, config))
            .collect();
    }

    chunk
        .iter()
        .map(|(row_a, row_b)| plan_one_row_pair(old_view, new_view, *row_a, *row_b, overlap_cols, config))
        .collect()
}

fn plan_one_row_pair<'a>(
    old_view: &'a GridView<'a>,
    new_view: &'a GridView<'a>,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    config: &DiffConfig,
) -> RowPairPlan<'a> {
    let Some(row_view_a) = old_view.rows.get(row_a as usize) else {
        return RowPairPlan { row_a, row_b, skipped: true, replaced: false, compared: 0, pending: Vec::new() };
    };
    let Some(row_view_b) = new_view.rows.get(row_b as usize) else {
        return RowPairPlan { row_a, row_b, skipped: true, replaced: false, compared: 0, pending: Vec::new() };
    };

    let sig_a = old_view.row_meta[row_a as usize].signature;
    let sig_b = new_view.row_meta[row_b as usize].signature;

    if sig_a == sig_b && !config.include_unchanged_cells {
        return RowPairPlan { row_a, row_b, skipped: true, replaced: false, compared: 0, pending: Vec::new() };
    }

    let r = diff_row_pair_sparse_plan(config, overlap_cols, &row_view_a.cells, &row_view_b.cells);

    RowPairPlan {
        row_a,
        row_b,
        skipped: false,
        replaced: r.replaced,
        compared: r.compared,
        pending: r.pending,
    }
}
```

### Step 3.3.3: Use the planner inside `emit_row_aligned_diffs`, then emit in deterministic order

`emit_row_aligned_diffs` currently loops `alignment.matched` and does work inline. 
We’ll change it to:

* iterate in chunks,
* plan chunk in parallel,
* emit chunk sequentially in chunk order.

This preserves:

* existing `pending_rect` coalescing behavior (because we emit in the same row order),
* deterministic order independent of thread count. 

You don’t need to rewrite the entire function; replace the “matched loop” portion.

**Replace this part** (conceptually the `for (row_a, row_b) in alignment.matched.iter()` loop) 
**With a chunked loop** like:

```rust
let mut compared: u64 = 0;

let mut pending_rect: Option<PendingRect> = None;

let matched = &alignment.matched;
let mut idx = 0usize;

while idx < matched.len() {
    if emit_ctx.hardening.should_abort() {
        break;
    }

    let chunk_len = cell_diff_chunk_len(overlap_cols);
    let end = (idx + chunk_len).min(matched.len());
    let chunk = &matched[idx..end];

    let plans = plan_row_pair_chunk(old_view, new_view, chunk, overlap_cols, emit_ctx.config);

    for plan in plans {
        if plan.skipped {
            flush_pending_rect(emit_ctx, &mut pending_rect, overlap_cols)?;
            continue;
        }

        compared = compared.saturating_add(plan.compared);

        if plan.replaced {
            if let Some(rect) = pending_rect.as_mut() {
                let expected_old = rect.start_old.saturating_add(rect.row_count);
                let expected_new = rect.start_new.saturating_add(rect.row_count);
                if plan.row_a == expected_old && plan.row_b == expected_new {
                    rect.row_count = rect.row_count.saturating_add(1);
                } else {
                    flush_pending_rect(emit_ctx, &mut pending_rect, overlap_cols)?;
                    pending_rect = Some(PendingRect {
                        start_old: plan.row_a,
                        start_new: plan.row_b,
                        row_count: 1,
                    });
                }
            } else {
                pending_rect = Some(PendingRect {
                    start_old: plan.row_a,
                    start_new: plan.row_b,
                    row_count: 1,
                });
            }
            continue;
        }

        flush_pending_rect(emit_ctx, &mut pending_rect, overlap_cols)?;
        emit_pending_cells(emit_ctx, plan.row_a, plan.row_b, plan.pending)?;
    }

    idx = end;

    if emit_ctx.hardening.check_timeout("cell diff", emit_ctx.warnings) {
        break;
    }
}

flush_pending_rect(emit_ctx, &mut pending_rect, overlap_cols)?;
```

Then keep the remainder of `emit_row_aligned_diffs` (inserted/deleted rows, moves, column ops) as-is, since those are already linear and deterministic when iterated in a stable order. 

Add a tiny helper to adapt chunk size based on column width (memory control):

```rust
fn cell_diff_chunk_len(overlap_cols: u32) -> usize {
    if overlap_cols >= 2048 {
        64
    } else if overlap_cols >= 512 {
        256
    } else {
        1024
    }
}
```

### Step 3.3.4: Apply the same pattern to `emit_moved_row_block_edits` (optional but cheap)

Moved-row block edits are also per-row-pair independent. The function currently loops offsets and calls `diff_row_pair_sparse` sequentially. 

You can reuse the same planner idea with a vector of `(row_a, row_b)` pairs derived from `src_start`, `dst_start`, and `row_count`, then emit results in offset order.

This is a follow-on once the aligned-row path is stable.

### Determinism checks

This implementation stays deterministic because:

* planning is pure and does not emit,
* we always emit chunk-by-chunk and plan-by-plan in the original alignment order,
* within a row pair, `PendingCell` order is based on column order from the merge walk. 

This satisfies “same inputs, different thread counts → identical `DiffReport.ops` ordering” from the sprint outline. 

---

## 3.4 Streaming + stable ordering considerations (JsonLinesSink)

`JsonLinesSink` writes each op as a JSON line immediately on `emit`.
To keep streaming deterministic with parallel planning, the critical rule is: **only one thread calls `emit`, and it does so in a deterministic order**.

The plan above accomplishes that by keeping emission strictly sequential and keyed off the stable `alignment.matched` order. 

If you still want an extra “belt and suspenders” layer for streaming, you can add an opt-in wrapper sink that buffers per sheet and sorts ops by a stable key, but I would treat that as optional because it undermines the point of JSONL streaming for large diffs.

---

## 3.5 Determinism + thread-count test plan

### Add determinism tests gated by `parallel`

Create a new test module (integration test or `#[cfg(test)]` unit test) that:

* runs the same diff twice under different Rayon thread pools,
* asserts that `DiffReport.ops` are byte-for-byte equal.

Example test skeleton (new file `core/tests/parallel_determinism_tests.rs`):

```rust
#[cfg(feature = "parallel")]
use excel_diff::{DiffConfig, Grid, CellValue, with_default_session};
#[cfg(feature = "parallel")]
use rayon::ThreadPoolBuilder;

#[cfg(feature = "parallel")]
fn run_in_pool<T>(threads: usize, f: impl FnOnce() -> T + Send) -> T
where
    T: Send,
{
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("build pool");
    pool.install(f)
}

#[cfg(feature = "parallel")]
#[test]
fn ops_are_identical_across_thread_counts() {
    let mut a = Grid::new_dense(10_000, 50);
    let mut b = Grid::new_dense(10_000, 50);

    for r in 0..10_000u32 {
        for c in 0..50u32 {
            a.insert_cell(r, c, Some(CellValue::Number((r * 100 + c) as f64)), None);
            b.insert_cell(r, c, Some(CellValue::Number((r * 100 + c) as f64)), None);
        }
    }

    b.insert_cell(5000, 25, Some(CellValue::Number(123456.0)), None);

    let config = DiffConfig::default();

    let ops_1 = run_in_pool(1, || {
        with_default_session(|session| {
            let report = a.diff(&b, &mut excel_diff::DiffContext::new(&mut session.strings, &config));
            report.ops
        })
    });

    let ops_4 = run_in_pool(4, || {
        with_default_session(|session| {
            let report = a.diff(&b, &mut excel_diff::DiffContext::new(&mut session.strings, &config));
            report.ops
        })
    });

    assert_eq!(ops_1, ops_4);
}
```

(You may want to adjust this to avoid allocating huge ops vectors if you have configs that emit unchanged cells.)

### CI hook

Because the `parallel` feature is not a default feature, add a CI step that runs:

* `cargo test -p excel_diff --features parallel`

That ensures determinism tests actually execute.

This directly matches the sprint outline’s requirement to validate determinism across thread counts. 

---

## 3.6 Optional: parallelize move-detection scoring (do after 3.2 + 3.3)

Move detection’s row-block similarity is computed by `block_similarity(...)` over candidate ranges. 
This can be parallelized safely because each candidate’s score is independent.

Low-risk approach:

* keep candidate enumeration and dedup (`seen`) sequential,
* compute the score for each candidate in parallel (map),
* then stable-sort candidates by `(score desc, src_start asc, dst_start asc, len desc)` to keep deterministic tie-breaking.

Because the similarity calculation is simple counting + division, results are deterministic across threads (no reductions). 

I’d only do this after signature build + cell diff parallelism land, because those two are explicitly called “cheapest high-impact targets” in #3. 

---

## Acceptance checklist for section #3

* `parallel` feature exists, enabled in CLI, disabled in wasm builds.
* GridView signature building uses parallel per-row sort + per-row hashing on native builds, and remains identical on wasm/single-thread.
* `emit_row_aligned_diffs` uses parallel planning + sequential emission with chunking; output ops are identical across thread counts.
* Determinism tests cover at least one large sheet scenario under different Rayon thread pools. 
* Perf validation: `signature_build_time_ms` meaningfully drops on `perf_50k_dense_single_edit` when running native CLI with multiple threads (the explicit motivation in #3). 
