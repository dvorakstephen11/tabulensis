## Branch 7 implementation plan

Branch 7 has five deliverables. I’m going to treat them as one cohesive “perf + observability + release-readiness” tranche, because the code paths and tooling overlap heavily.

---

# 0. What exists today (so we leverage it, not fight it)

### Diff phases + metrics today

* There is already a `Phase::Parse` variant in `core/src/perf.rs`, but **it is intentionally a no-op** right now (see the unit test `metrics_parse_phase_is_no_op`).
* We already track:

  * `total_time_ms`, `move_detection_time_ms`, `alignment_time_ms`, `cell_diff_time_ms`
  * `rows_processed`, `cells_compared`, `anchors_found`, `moves_detected`
* The perf test harness emits one-line metrics in `PERF_METRIC ... key=value ...` format (`core/tests/perf_large_grid_tests.rs`), and CI parses those lines (`scripts/check_perf_thresholds.py`).

### The specific performance problem

In `core/src/engine/grid_diff.rs`, when preflight decides to short-circuit (`ShortCircuitNearIdentical` or `ShortCircuitDissimilar`), we do this:

* Build `GridView`s anyway (old and new)
* Then call `run_positional_diff_with_metrics(&mut emit_ctx, old, new, ...)`
* `run_positional_diff_with_metrics` calls `positional_diff`, which does **dense per-cell scanning with hash-map lookups** (`old.get(row,col)` / `new.get(row,col)`)

That’s exactly why `perf_50k_dense_single_edit` is slow: it still scans the entire overlap cell-by-cell even though only 1 row differs.

---

# 1. Speed up dense near-equal diffs (row-signature fast-skip)

## 1.1 Design goal

When we are doing positional (identity) row alignment (i.e., comparing row `i` to row `i`), we should do:

* If row signatures match: **skip cell-by-cell diff for that row entirely**
* Only run per-cell diff on rows whose signatures differ
* Preserve correctness and existing behavior when `include_unchanged_cells=true`

This should bring `perf_50k_dense_single_edit` down to “GridView build + preflight + emit 1 cell edit”, instead of “GridView build + full-grid cell scan”.

## 1.2 Implementation steps

### Step A: Introduce a positional diff that uses `GridView` rows and signatures

Add a new internal function that:

* Iterates row indices (positional)
* Checks `old_view.row_meta[row].signature == new_view.row_meta[row].signature`
* Skips unchanged rows (unless `include_unchanged_cells`)
* For changed rows, uses an efficient sparse row merge (the existing `diff_row_pair_sparse` logic)

This needs to live somewhere both `grid_diff.rs` and `move_mask.rs` can call. The cleanest place is `core/src/engine/grid_primitives.rs` (where `positional_diff` already lives).

#### Move `diff_row_pair_sparse` into `grid_primitives.rs`

Today it’s implemented in `core/src/engine/grid_diff.rs`. We want to reuse it in the new positional-with-views diff. So:

* Move the function into `grid_primitives.rs` as `pub(super) fn diff_row_pair_sparse(...) -> Result<u64, DiffError>`
* Update `grid_diff.rs` to call the moved function

This is low-risk because it’s a purely internal helper, and it only depends on other primitives already in `grid_primitives.rs` (`snapshot_with_addr`, `compute_formula_diff`, `cells_content_equal`).

### Step B: Add `run_positional_diff_from_views_with_metrics`

Parallel to the existing `run_positional_diff_with_metrics`, add a version that:

* Wraps Phase::CellDiff timing (when perf-metrics enabled)
* Uses the new `positional_diff_from_views`
* Adds an accurate `cells_compared` count (sum of per-row comparisons, not `rows*cols`)

### Step C: Use the new view-based positional diff anywhere we already have views

Update three key call sites:

1. **Preflight short-circuit path** in `diff_grids_core` (the big one)
2. **SheetGridDiffer.positional()** in `core/src/engine/move_mask.rs` (fallback path after alignment fails)
3. **AMR fallback paths** in `try_diff_with_amr` (when it falls back to positional after already building views)

This ensures we don’t “throw away” the expensive `GridView` we already built and then do slow hash-map scanning.

## 1.3 Concrete code changes

### 1.3.1 `core/src/engine/grid_diff.rs` — preflight short-circuit uses views

**Code to replace (current preflight short-circuit):**

```rust
if matches!(
    preflight,
    PreflightDecision::ShortCircuitNearIdentical | PreflightDecision::ShortCircuitDissimilar
) {
    let mut emit_ctx = EmitCtx::new(
        sheet_id,
        pool,
        config,
        &mut ctx.formula_cache,
        sink,
        op_count,
        &mut ctx.warnings,
        hardening,
    );
    #[cfg(feature = "perf-metrics")]
    run_positional_diff_with_metrics(&mut emit_ctx, old, new, metrics.as_deref_mut())?;
    #[cfg(not(feature = "perf-metrics"))]
    run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;
    return Ok(());
}
```

**Replace with (use view-based positional diff):**

```rust
if matches!(
    preflight,
    PreflightDecision::ShortCircuitNearIdentical | PreflightDecision::ShortCircuitDissimilar
) {
    let mut emit_ctx = EmitCtx::new(
        sheet_id,
        pool,
        config,
        &mut ctx.formula_cache,
        sink,
        op_count,
        &mut ctx.warnings,
        hardening,
    );

    #[cfg(feature = "perf-metrics")]
    run_positional_diff_from_views_with_metrics(
        &mut emit_ctx,
        old,
        new,
        &old_view,
        &new_view,
        metrics.as_deref_mut(),
    )?;
    #[cfg(not(feature = "perf-metrics"))]
    run_positional_diff_from_views_with_metrics(
        &mut emit_ctx,
        old,
        new,
        &old_view,
        &new_view,
    )?;

    return Ok(());
}
```

You’ll also update the `use super::grid_primitives::{ ... }` import list to include `run_positional_diff_from_views_with_metrics`.

### 1.3.2 `core/src/engine/move_mask.rs` — SheetGridDiffer positional uses views

**Code to replace:**

```rust
#[cfg(feature = "perf-metrics")]
run_positional_diff_with_metrics(
    &mut self.emit_ctx,
    self.old,
    self.new,
    self.metrics.as_deref_mut(),
)?;
#[cfg(not(feature = "perf-metrics"))]
run_positional_diff_with_metrics(&mut self.emit_ctx, self.old, self.new)?;
Ok(())
```

**Replace with:**

```rust
#[cfg(feature = "perf-metrics")]
run_positional_diff_from_views_with_metrics(
    &mut self.emit_ctx,
    self.old,
    self.new,
    &self.old_view,
    &self.new_view,
    self.metrics.as_deref_mut(),
)?;
#[cfg(not(feature = "perf-metrics"))]
run_positional_diff_from_views_with_metrics(
    &mut self.emit_ctx,
    self.old,
    self.new,
    &self.old_view,
    &self.new_view,
)?;
Ok(())
```

### 1.3.3 `core/src/engine/grid_primitives.rs` — add new functions

Add new functions near the existing positional helpers:

```rust
use crate::grid_view::GridView;

pub(super) fn positional_diff_from_views<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<u64, DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    ctx.hardening.progress("cell_diff", 0.0);

    let mut compared: u64 = 0;

    for row in 0..overlap_rows {
        if ctx.hardening.check_timeout(ctx.warnings) {
            break;
        }
        if overlap_rows > 0 {
            ctx.hardening
                .progress("cell_diff", (row as f64) / (overlap_rows as f64));
        }

        if !ctx.config.include_unchanged_cells {
            let old_sig = old_view.row_meta.get(row as usize).map(|m| m.signature);
            let new_sig = new_view.row_meta.get(row as usize).map(|m| m.signature);
            if let (Some(a), Some(b)) = (old_sig, new_sig) {
                if a == b {
                    continue;
                }
            }
        }

        let old_cells = old_view
            .rows
            .get(row as usize)
            .map(|r| r.cells.as_slice())
            .unwrap_or(&[]);
        let new_cells = new_view
            .rows
            .get(row as usize)
            .map(|r| r.cells.as_slice())
            .unwrap_or(&[]);

        compared = compared.saturating_add(diff_row_pair_sparse(
            ctx,
            row,
            row,
            overlap_cols,
            old_cells,
            new_cells,
        )?);
    }

    if old.nrows > new.nrows {
        for row in new.nrows..old.nrows {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::row_removed(ctx.sheet_id, row, None))?;
        }
    } else if new.nrows > old.nrows {
        for row in old.nrows..new.nrows {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::row_added(ctx.sheet_id, row, None))?;
        }
    }

    if old.ncols > new.ncols {
        for col in new.ncols..old.ncols {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::col_removed(ctx.sheet_id, col, None))?;
        }
    } else if new.ncols > old.ncols {
        for col in old.ncols..new.ncols {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::col_added(ctx.sheet_id, col, None))?;
        }
    }

    ctx.hardening.progress("cell_diff", 1.0);

    Ok(compared)
}

#[cfg(feature = "perf-metrics")]
pub(super) fn run_positional_diff_from_views_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    let _guard = metrics.as_deref_mut().map(|m| m.phase_guard(Phase::CellDiff));
    let compared = positional_diff_from_views(ctx, old, new, old_view, new_view)?;
    if let Some(m) = metrics.as_deref_mut() {
        m.add_cells_compared(compared);
    }
    Ok(())
}

#[cfg(not(feature = "perf-metrics"))]
pub(super) fn run_positional_diff_from_views_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<(), DiffError> {
    let _ = positional_diff_from_views(ctx, old, new, old_view, new_view)?;
    Ok(())
}
```

Then move `diff_row_pair_sparse` here (unchanged logic) and update `grid_diff.rs` to call it from `grid_primitives`.

## 1.4 Optional but recommended: skip unchanged matched rows in aligned diff too

In `emit_row_aligned_diffs` (currently diffs every matched pair), add:

* If `!include_unchanged_cells` and `old_view.row_meta[row_a].signature == new_view.row_meta[row_b].signature`, skip

This helps non-preflight large diffs too.

## 1.5 Validation for this deliverable

* Run:

  * `cargo test --release --features perf-metrics perf_50k_dense_single_edit -- --ignored --nocapture --test-threads=1`
* Expect:

  * `cell_diff_time_ms` collapses dramatically
  * total drops under the target
  * output still includes the correct `CellEdited` op(s)

---

# 2. Instrument parse vs diff time (parse_time_ms + diff_time_ms)

## 2.1 Design goal

Expose a stable split:

* `parse_time_ms`: time spent in “setup” (the code already labels this as `"parse"` progress)
* `diff_time_ms`: `total_time_ms - parse_time_ms`

Also ensure this shows up in:

* CLI JSON output (already serializes `DiffReport`)
* Perf line output (`PERF_METRIC ...`), so scripts can gate and compare

## 2.2 Implementation steps

### Step A: Extend DiffMetrics

Add:

* `parse_time_ms: u64`
* `diff_time_ms: u64` (computed when total ends)

Update `end_phase`:

* `Phase::Parse` accumulates into `parse_time_ms` (no longer a no-op)
* `Phase::Total` sets `diff_time_ms = total_time_ms.saturating_sub(parse_time_ms)`

### Step B: Actually start/end the parse phase in the real code paths

There are two places where parse timing matters:

1. Workbook-level “parse” section in `try_diff_workbooks_streaming_impl`
   Wrap the existing `hardening.progress("parse", ...)` region.

2. Sheet-level GridView build + preflight in `diff_grids_core`
   Without this, a bunch of setup work still ends up in total but in no phase buckets.

## 2.3 Concrete code changes

### 2.3.1 `core/src/perf.rs`

**Code to replace (current DiffMetrics struct header):**

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffMetrics {
    pub move_detection_time_ms: u64,
    pub alignment_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
    #[serde(skip)]
    phase_start: HashMap<Phase, Instant>,
}
```

**Replace with:**

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DiffMetrics {
    pub parse_time_ms: u64,
    pub move_detection_time_ms: u64,
    pub alignment_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub diff_time_ms: u64,
    pub peak_memory_bytes: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
    #[serde(skip)]
    phase_start: HashMap<Phase, Instant>,
}
```

Then update `end_phase` match arms. The key change is:

* Make `Phase::Parse` real.
* Compute diff_time at end of total.

---

# 3. Add peak memory tracking (peak_memory_bytes)

## 3.1 Design goal

Provide **measured** peak heap allocation during the diff, not just estimated memory guards.

We’ll implement a lightweight heap allocation counter:

* Tracks current allocated bytes and peak bytes (max current)
* Feature-gated (only in perf-metrics builds)
* Plumbed into DiffMetrics as `peak_memory_bytes`

## 3.2 Implementation steps

### Step A: Add a counting allocator module

Create `core/src/memory_metrics.rs` (internal).

It provides:

* `CountingAllocator<System>`
* Atomics for current and peak
* `reset_peak_to_current()` and `peak_bytes()`

### Step B: Install it as the global allocator (perf builds only)

In `core/src/lib.rs`, add:

* `#[cfg(feature = "perf-metrics")] mod memory_metrics;`
* `#[global_allocator]` static using the counting allocator (guarded so it doesn’t affect wasm)

### Step C: Reset and record around Phase::Total

* At `DiffMetrics::start_phase(Phase::Total)` → reset peak
* At `DiffMetrics::end_phase(Phase::Total)` → read and store peak

### Step D: Make perf runs single-threaded

Because this is a **process-global allocator counter**, parallel tests will mix peaks.
So:

* CI should run perf tests with `--test-threads=1`
* Scripts should enforce this

## 3.3 Concrete code snippet: new memory allocator module

Add `core/src/memory_metrics.rs`:

```rust
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicU64, Ordering};

static CURRENT: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);

pub struct CountingAllocator<A> {
    inner: A,
}

impl<A> CountingAllocator<A> {
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }
}

fn update_peak(new_current: u64) {
    let mut peak = PEAK.load(Ordering::Relaxed);
    while new_current > peak {
        match PEAK.compare_exchange_weak(
            peak,
            new_current,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(p) => peak = p,
        }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);
        if !ptr.is_null() {
            let size = layout.size() as u64;
            let new_current = CURRENT.fetch_add(size, Ordering::Relaxed).saturating_add(size);
            update_peak(new_current);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout);
        let size = layout.size() as u64;
        let mut cur = CURRENT.load(Ordering::Relaxed);
        loop {
            let next = cur.saturating_sub(size);
            match CURRENT.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => cur = actual,
            }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size() as u64;
        let ptr2 = self.inner.realloc(ptr, layout, new_size);
        if !ptr2.is_null() {
            let new_size_u = new_size as u64;
            if new_size_u >= old_size {
                let delta = new_size_u - old_size;
                let new_current = CURRENT.fetch_add(delta, Ordering::Relaxed).saturating_add(delta);
                update_peak(new_current);
            } else {
                let delta = old_size - new_size_u;
                let mut cur = CURRENT.load(Ordering::Relaxed);
                loop {
                    let next = cur.saturating_sub(delta);
                    match CURRENT.compare_exchange_weak(
                        cur,
                        next,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(actual) => cur = actual,
                    }
                }
            }
        }
        ptr2
    }
}

pub fn reset_peak_to_current() {
    let cur = CURRENT.load(Ordering::Relaxed);
    PEAK.store(cur, Ordering::Relaxed);
}

pub fn peak_bytes() -> u64 {
    PEAK.load(Ordering::Relaxed)
}
```

And in `core/src/lib.rs`, add (near the module list):

```rust
#[cfg(feature = "perf-metrics")]
mod memory_metrics;

#[cfg(all(feature = "perf-metrics", not(target_arch = "wasm32")))]
#[global_allocator]
static GLOBAL_ALLOC: memory_metrics::CountingAllocator<std::alloc::System> =
    memory_metrics::CountingAllocator::new(std::alloc::System);
```

Then in `DiffMetrics::start_phase` / `end_phase`, call `reset_peak_to_current()` and read `peak_bytes()` (guarded on the same cfg).

---

# 4. Tighten perf CI gates (baseline + absolute caps) + add scheduled full-scale job

## 4.1 What’s broken/incomplete today

* `scripts/check_perf_thresholds.py` only enforces absolute max seconds on a small set of quick tests.
* `--full-scale` mode exists, but the script’s thresholds map does not match the ignored test names, so it’s not usable for gating full-scale today.
* The perf workflow runs `cargo test` **twice** (once directly, then again inside the python script).

## 4.2 What we’ll implement

### Step A: Extend `scripts/check_perf_thresholds.py`

1. Support two suites:

* Quick suite (default): non-ignored `perf_p1_*`, `perf_p2_*`, `perf_p3_*`
* Full-scale suite (`--full-scale`): ignored `perf_50k_*`, `perf_100k_*`, `perf_many_sheets`, etc.

2. Add baseline regression checks:

* Load baseline metrics from the repo (recommended: latest JSON in `benchmarks/results/` matching suite)
* Fail if:

  * `total_time_ms` regresses beyond a slack factor (e.g., +10% quick, +15% full-scale)
  * `peak_memory_bytes` regresses beyond slack
* Still enforce absolute caps for selected tests

3. Export results from the same run:

* Add `--export-json path` (writes the same schema used by `export_perf_metrics.py`)
* Add `--export-csv path` (the current CSV functionality already exists)

4. Enforce single-threaded test execution:

* add `-- --test-threads=1` to the cargo test invocation

### Step B: Update `.github/workflows/perf.yml`

* Remove the redundant “Run perf test suite” step
* Let the python script run tests once
* Set `RUST_TEST_THREADS=1` or pass `--test-threads=1` (script-level is better, because it’s explicit)
* Upload the CSV/JSON outputs as artifacts (useful when a regression happens)

### Step C: Add `.github/workflows/perf_fullscale.yml`

* `on: schedule:` (and optionally `workflow_dispatch`)
* Run `python scripts/check_perf_thresholds.py --full-scale --export-json ... --export-csv ...`
* Upload artifacts
* Optionally: commit the JSON to `benchmarks/results/` on main if the repo already does this (guard to avoid infinite loop)

## 4.3 Concrete code snippet: perf workflow fix

### `.github/workflows/perf.yml`

**Code to replace (current relevant steps):**

```yaml
      - name: Run perf test suite
        run: cargo test --release --features perf-metrics perf_ -- --nocapture
        working-directory: core

      - name: Check perf thresholds
        run: python scripts/check_perf_thresholds.py
```

**Replace with:**

```yaml
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Check perf thresholds (quick suite)
        run: python scripts/check_perf_thresholds.py --export-csv benchmarks/latest_quick.csv
```

(And add an artifact upload step for `benchmarks/latest_quick.csv`.)

## 4.4 Full-scale scheduled workflow (new file)

Create `.github/workflows/perf_fullscale.yml`:

```yaml
name: Performance Full Scale

on:
  schedule:
    - cron: "0 3 * * 1"
  workflow_dispatch: {}

jobs:
  perf-fullscale:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Run full-scale perf suite + gates
        run: python scripts/check_perf_thresholds.py --full-scale --export-csv benchmarks/latest_fullscale.csv

      - name: Upload perf artifacts
        uses: actions/upload-artifact@v4
        with:
          name: perf-fullscale
          path: benchmarks/latest_fullscale.csv
```

If you want the “repo stores history” behavior, extend this workflow to write JSON into `benchmarks/results/<timestamp>.json` and commit it, but only if that’s already an accepted pattern in this repo.

---

# 5. Update release readiness checklist + user-facing docs

## 5.1 What must be documented (per branch 7)

1. **PBIX support limits**

   * Only legacy DataMashup-based extraction is supported.
   * If the PBIX has no DataMashup (Tabular model), we error with `NoDataMashupUseTabularModel` and error code `EXDIFF_PKG_010`.

2. **Semantic M diff behavior and how to enable it**

   * `DiffConfig.enable_m_semantic_diff` is `true` by default.
   * The CLI’s `--fast` preset sets it to `false` (see `build_config`).
   * Therefore:

     * “Enable semantic M diff” = don’t use `--fast` (or use `--precise` / default)

3. **Resource ceilings knobs**

   * `DiffConfig.max_memory_mb` (memory guard; can force fallback to positional)
   * `DiffConfig.timeout_seconds`
   * Also mention alignment limits and `on_limit_exceeded` behavior for completeness.

## 5.2 Where to put this

* Add a dedicated doc file, e.g. `docs/release_readiness.md` (or `docs/limits.md`)
* Add a short “Limits and knobs” section in `README.md`
* Ensure the CLI help mentions the knobs (it already wires `--max-memory` and `--timeout` into config)

## 5.3 Suggested checklist content (example)

Add a checklist section like:

* [ ] PBIX/PBIT: verified legacy DataMashup extraction works; verified Tabular-only PBIX returns `EXDIFF_PKG_010`
* [ ] Semantic M diff: default mode includes semantic detail in QueryDefinitionChanged; `--fast` disables it
* [ ] Resource ceilings:

  * [ ] `--timeout` aborts cleanly with partial report and warnings
  * [ ] `--max-memory` triggers memory guard and falls back to positional diff with warning
* [ ] Perf gates: quick suite baseline + caps passing; full-scale scheduled job green

---

# 6. Tests and tooling updates you should include in the branch

## 6.1 Update perf test log output to include new fields

Update `log_perf_metric` in `core/tests/perf_large_grid_tests.rs` to print:

* `parse_time_ms`
* `diff_time_ms`
* `peak_memory_bytes`

This keeps the parsing model stable (`key=value` pairs), and lets scripts gate memory and parse/diff splits.

## 6.2 Update unit tests in `core/tests/metrics_unit_tests.rs`

* Replace `metrics_parse_phase_is_no_op` with a test that asserts parse time increases when Phase::Parse is started and ended.
* Add a test that:

  * starts parse and total
  * ends parse
  * ends total
  * asserts `diff_time_ms == total_time_ms - parse_time_ms`

## 6.3 Add a small regression test for row-signature skipping

Add a test (perf-metrics only) that:

* Forces preflight to trigger on a small grid by setting `preflight_min_rows = 0` and lowering `preflight_in_order_match_ratio_min`
* Changes exactly one row
* Asserts `metrics.cells_compared` is roughly `ncols`, not `nrows*ncols`

This confirms we’re truly skipping unchanged rows.

---

# 7. Branch 7 “Definition of Done” checklist (mapped to sprint plan)

* **Dense near-equal speedup**

  * Preflight short-circuit uses view-based positional diff
  * Unchanged rows are skipped via row signatures when alignment is identity
  * `perf_50k_dense_single_edit` is materially faster (goal threshold enforced in full-scale gates)

* **Parse vs diff instrumentation**

  * `DiffMetrics` includes `parse_time_ms` and `diff_time_ms`
  * Parse time is actually measured (not no-op)
  * Values appear in JSON and perf log output

* **Peak memory tracking**

  * Feature-gated counting allocator added
  * `peak_memory_bytes` is populated in metrics
  * CI runs perf with `--test-threads=1` so peaks are meaningful

* **Perf CI gates**

  * Quick suite: baseline regression + absolute caps
  * Full-scale scheduled suite added and produces artifacts (and optionally updates benchmark history)

* **Release readiness docs**

  * PBIX DataMashup-only limitation documented (with error code)
  * Semantic M diff behavior and enabling documented
  * Resource ceilings knobs documented (`--timeout`, `--max-memory`, and fallback behavior)