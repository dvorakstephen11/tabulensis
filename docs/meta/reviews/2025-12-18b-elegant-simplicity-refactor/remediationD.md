Below is my best-effort “post‑refactor” review based on the artifacts you attached (`codebase_context.md`, `cycle_summary.txt`, and `benchmark_results.json`). One important caveat: I **don’t have the literal text of my original Elegant Simplicity plan/recommendations in this chat** (it was produced in a delegated run), so I can’t do a line‑by‑line checklist diff against that exact plan. What I *can* do—and I’m doing here—is:

1. assess how the code reads and “hangs together” now (where the refactor clearly succeeded), and
2. call out the remaining *simplicity leaks / coherence gaps* that would prevent calling the refactor “complete”, with a concrete remediation plan + snippets.

---

# How the updated codebase looks

## What’s noticeably improved

### The AMR alignment subsystem now reads like a narrative

The `core/src/alignment/` split is strong: each submodule has one job, and `alignment/mod.rs` documents the pipeline as “metadata → anchors → chain → gaps → assembly”, including explicit “intentional spec deviations”. That’s exactly the kind of honesty that makes future maintenance simple.

* `gap_strategy.rs` is especially clean: crisp enum, single `select_gap_strategy` gate, and a tiny helper for matching unique signatures.
* `anchor_chain.rs` is small and correct: “sort by new_row → LIS on old_row” reads naturally.
* `assembly.rs` is still big (it has to be), but it’s now *structured* rather than “mystery meat”.

### Config centralization is real (and testable)

`DiffConfig` consolidates thresholds and introduces `validate()`, presets (`fastest`, `balanced`, `most_precise`), and a builder. This is a significant simplicity win: fewer scattered constants, more explicit tuning surface.

### Streaming diff is now first‑class

`DiffSink`, `VecSink`, `CallbackSink`, and the `try_diff_workbooks_streaming` path are a big architectural simplification for callers and for future “instant diff” performance work.

---

## Where “elegant simplicity” is *not yet fully closed*

These are the main remaining “complexity smells” I see that would keep this from being a fully-complete simplicity refactor:

1. **Layering leak around row metadata**
   In `codebase_context.md`, `GridView` imports `crate::alignment::row_metadata::classify_row_frequencies` and re‑exports `RowMeta/FrequencyClass` from `alignment::row_metadata`. That creates an awkward conceptual dependency direction: a “view/metadata” layer depends on an “alignment algorithm” module.

   *However:* your `cycle_summary.txt` strongly suggests you already introduced a top-level `core/src/grid_metadata.rs` (and even `core/tests/grid_metadata_tests.rs`). If that is true in the actual repo, then the remaining work is: **finish the migration and delete the old coupling / shims** (or regenerate `codebase_context.md` so it reflects reality).

2. **`RowMeta` carries both `signature` and `hash` (same value)**
   `RowMeta { signature, hash: signature, ... }` is a classic “transitional alias field” that keeps code compiling but increases cognitive load everywhere. This is exactly the kind of accidental complexity that “Elegant Simplicity” should erase or at least quarantine behind deprecation.

3. **`assembly.rs` still contains several orthogonal algorithms in one file**
   It’s *structured* now, but it’s still doing: fast paths, gap slicing, DP LCS, Myers, hash/LIS fallback, recursive anchoring, and opportunistic move detection. That is a lot for one module to hold in working memory.

   The refactor is close—you just need one more pass to extract a couple of “units of thought” so the control flow reads like prose.

4. **A lot of `#[allow(dead_code)]` wrapper APIs remain**
   Examples:

   * `discover_anchors(GridView, GridView)` wrapper
   * `align_rows_amr(Grid, Grid)` wrapper
   * `align_single_column_change(Grid, Grid)` wrapper

   These are minor, but they’re the kind of rough edges that prevent the system from feeling “inevitable”: they suggest the public surface is still undecided.

5. **`engine.rs` is still monolithic**
   You did the right “small refactor” steps (streaming sink, clean IR, config, AMR subsystem), but the workbook/grid diff orchestration in `engine.rs` remains the big “knot”. Elegant simplicity usually isn’t complete until that knot is either:

   * encapsulated in a stateful `struct` that owns context, or
   * split into a few intentionally-named modules.

---

# Detailed remediation plan to close all simplicity gaps

I’m going to give you a plan that is safe, incremental, and keeps tests green at each step. You can stop after any phase and still have a coherent system.

## Phase 1 — Fix the metadata layering leak (highest ROI)

### Goal

Make “grid metadata” a first-class concept that **does not live inside** the alignment algorithm namespace, and ensure `GridView` depends only on grid/metadata utilities—not alignment.

### Target end state

* `core/src/grid_metadata.rs` defines:

  * `FrequencyClass`
  * `RowMeta`
  * `classify_row_frequencies` (+ helpers)
* `GridView` uses `crate::grid_metadata::*`
* `alignment::*` uses `crate::grid_metadata::*`
* `alignment/row_metadata.rs` is either deleted or turned into a temporary compatibility shim (then deleted)

### Step 1.1 — Introduce `grid_metadata.rs` (or verify it is the canonical one)

Create `core/src/grid_metadata.rs` (if it doesn’t already exist) by moving the contents of `core/src/alignment/row_metadata.rs`.

```rust
// core/src/grid_metadata.rs
//! Grid row metadata and frequency classification.
//!
//! Shared by GridView construction and alignment algorithms.

use std::collections::HashMap;

use crate::config::DiffConfig;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrequencyClass {
    Unique,
    Rare,
    Common,
    LowInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RowMeta {
    pub row_idx: u32,
    pub signature: RowSignature,
    // (see Phase 2 for hash/signature cleanup)
    pub hash: RowSignature,
    pub non_blank_count: u16,
    pub first_non_blank_col: u16,
    pub frequency_class: FrequencyClass,
    pub is_low_info: bool,
}

impl RowMeta {
    pub fn is_low_info(&self) -> bool {
        self.is_low_info || matches!(self.frequency_class, FrequencyClass::LowInfo)
    }
}

pub fn frequency_map(row_meta: &[RowMeta]) -> HashMap<RowSignature, u32> {
    let mut map = HashMap::new();
    for meta in row_meta {
        *map.entry(meta.signature).or_insert(0) += 1;
    }
    map
}

pub fn classify_row_frequencies(row_meta: &mut [RowMeta], config: &DiffConfig) {
    let freq_map = frequency_map(row_meta);
    for meta in row_meta.iter_mut() {
        if meta.frequency_class == FrequencyClass::LowInfo {
            continue;
        }

        let count = freq_map.get(&meta.signature).copied().unwrap_or(0);
        let mut class = match count {
            1 => FrequencyClass::Unique,
            0 => FrequencyClass::Common,
            c if c <= config.rare_threshold => FrequencyClass::Rare,
            _ => FrequencyClass::Common,
        };

        if (meta.non_blank_count as u32) < config.low_info_threshold || meta.is_low_info {
            class = FrequencyClass::LowInfo;
            meta.is_low_info = true;
        }

        meta.frequency_class = class;
    }
}
```

### Step 1.2 — Update `grid_view.rs` to depend on `grid_metadata`

Change:

```rust
use crate::alignment::row_metadata::classify_row_frequencies;
pub use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
```

to:

```rust
use crate::grid_metadata::classify_row_frequencies;
pub use crate::grid_metadata::{FrequencyClass, RowMeta};
```

…and keep the rest unchanged.

### Step 1.3 — Update alignment imports everywhere

Mechanical replace in `core/src/alignment/*.rs`:

```rust
use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
```

→

```rust
use crate::grid_metadata::{FrequencyClass, RowMeta};
```

### Step 1.4 — Add module wiring in `lib.rs`

Add:

```rust
mod grid_metadata;
```

and update re-exports (you can keep exporting RowMeta via `grid_view` if you prefer, but it’s simpler and more honest to export from `grid_metadata`):

```rust
pub use grid_metadata::{FrequencyClass, RowMeta};
```

If you keep existing exports from `grid_view`, at least ensure `grid_view` itself re-exports from `grid_metadata` so there’s only one source of truth.

### Step 1.5 — Transitional shim (optional, but keeps churn minimal)

If you want to avoid changing a bunch of imports in one go, you can keep a short-lived compatibility shim:

```rust
// core/src/alignment/row_metadata.rs (temporary)
pub use crate::grid_metadata::*;
```

But **do not keep `GridView` importing from this shim**—that’s the layering inversion. Use it only to ease alignment-module migrations.

---

## Phase 2 — Remove the `RowMeta.signature` vs `RowMeta.hash` ambiguity

### Goal

Make the “row identity” vocabulary consistent and eliminate dual fields that mean the same thing.

### Best-practice approach (non-breaking)

Since `RowMeta` is part of your public surface (re-exported), removing fields is semver breaking. The cleanest path is:

1. keep both fields temporarily,
2. **deprecate the alias**,
3. update all internal code to use exactly one name,
4. remove the deprecated field at a major version bump (or when you’re ready).

### Step 2.1 — Deprecate `hash` field

In `RowMeta`:

```rust
pub struct RowMeta {
    pub row_idx: u32,
    pub signature: RowSignature,

    #[deprecated(note = "use `signature` (hash is an alias kept for compatibility)")]
    pub hash: RowSignature,

    // ...
}
```

Rust supports `#[deprecated]` on fields; this will guide downstream users too.

### Step 2.2 — Make internal code stop using `meta.hash` for rows

Mechanical pass in core:

* replace uses of `meta.hash` (where it refers to row signature) with `meta.signature`

Examples to fix (based on patterns I saw in your extracted context):

* hash stats / uniqueness checks
* anchor discovery / local anchors
* any move detection that keys on row signature

This is the *single biggest “reads like prose” improvement* you can make with almost no algorithmic risk.

### Step 2.3 — Stop populating both fields (optional, if you’re willing to break)

If you *are* okay with breaking now (early stage), the “true simplicity” move is:

* delete `hash` from `RowMeta`
* update constructors/tests
* remove `RowHash` alias if it’s just `RowSignature`

If you want that version, the diff is:

```rust
pub struct RowMeta {
    pub row_idx: u32,
    pub signature: RowSignature,
    // pub hash: RowSignature,  // removed
    // ...
}
```

and in `GridView::from_grid_with_config` remove `hash: signature`.

---

## Phase 3 — Make `assembly.rs` feel “inevitable” (final simplicity pass)

`assembly.rs` is already much better structured, but it still requires mental stack juggling. The goal here is not to change algorithms—it’s to make the code *compose*.

### Step 3.1 — Introduce a `GapCtx` to shrink parameter lists

Right now `fill_gap` takes 6 parameters and re-slices internally. Create a small struct:

```rust
struct GapCtx<'a> {
    old_range: Range<u32>,
    new_range: Range<u32>,
    old_slice: &'a [RowMeta],
    new_slice: &'a [RowMeta],
}

fn make_gap<'a>(
    old_meta: &'a [RowMeta],
    new_meta: &'a [RowMeta],
    old_range: Range<u32>,
    new_range: Range<u32>,
) -> GapCtx<'a> {
    let old_slice = slice_by_range(old_meta, &old_range);
    let new_slice = slice_by_range(new_meta, &new_range);
    GapCtx { old_range, new_range, old_slice, new_slice }
}
```

Now `fill_gap` becomes:

```rust
fn fill_gap(ctx: GapCtx<'_>, config: &DiffConfig, depth: u32) -> GapAlignmentResult {
    let has_recursed = depth >= config.max_recursion_depth;
    let strategy = select_gap_strategy(ctx.old_slice, ctx.new_slice, config, has_recursed);

    match strategy {
        GapStrategy::Empty => GapAlignmentResult::default(),
        GapStrategy::InsertAll => GapAlignmentResult {
            inserted: (ctx.new_range.start..ctx.new_range.end).collect(),
            ..Default::default()
        },
        GapStrategy::DeleteAll => GapAlignmentResult {
            deleted: (ctx.old_range.start..ctx.old_range.end).collect(),
            ..Default::default()
        },
        GapStrategy::SmallEdit => align_gap_default(ctx.old_slice, ctx.new_slice, config),
        GapStrategy::HashFallback => align_gap_hash(ctx.old_slice, ctx.new_slice),
        GapStrategy::MoveCandidate => align_gap_with_moves(ctx.old_slice, ctx.new_slice, config),
        GapStrategy::RecursiveAlign => align_gap_recursive(ctx.old_slice, ctx.new_slice, config, depth),
    }
}
```

### Step 3.2 — Extract “default gap alignment” and “hash gap alignment”

These two patterns show up repeatedly:

```rust
fn align_gap_default(old_slice: &[RowMeta], new_slice: &[RowMeta], config: &DiffConfig) -> GapAlignmentResult {
    if old_slice.len() as u32 > config.max_lcs_gap_size || new_slice.len() as u32 > config.max_lcs_gap_size {
        return align_gap_via_hash(old_slice, new_slice);
    }
    align_small_gap(old_slice, new_slice, config)
}

fn align_gap_hash(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let mut result = align_gap_via_hash(old_slice, new_slice);
    result.moves.extend(moves_from_matched_pairs(&result.matched));
    result
}
```

Now your `match` arms get tiny and uniform.

### Step 3.3 — Extract recursive anchor selection into one function

Right now the `RecursiveAlign` branch is correct but dense. Extract the “choose anchors” logic:

```rust
fn recursive_anchor_candidates(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    depth: u32,
    config: &DiffConfig,
) -> Vec<Anchor> {
    if depth == 0 {
        return discover_anchors_from_meta(old_slice, new_slice);
    }

    let k1 = config.context_anchor_k1 as usize;
    let k2 = config.context_anchor_k2 as usize;

    let mut anchors = discover_local_anchors(old_slice, new_slice);
    if anchors.is_empty() {
        anchors = discover_context_anchors(old_slice, new_slice, k1);
        if anchors.is_empty() {
            anchors = discover_context_anchors(old_slice, new_slice, k2);
        }
        return anchors;
    }

    if anchors.len() < k1 {
        let mut ctx = discover_context_anchors(old_slice, new_slice, k1);
        anchors.append(&mut ctx);
    }

    anchors
}
```

Then `RecursiveAlign` becomes a simple “if no anchors → fallback else recurse”.

That’s the “reads like prose” threshold.

---

## Phase 4 — Remove `#[allow(dead_code)]` clutter by deciding what’s real API

### Goal

Either:

* make these wrappers real `pub` API (and use them), or
* demote them to test-only/dev-only helpers.

### Recommended tactic: dev-only feature flag

If you want to keep convenience entry points without polluting production builds:

1. add to `core/Cargo.toml`:

```toml
[features]
dev-apis = []
```

2. annotate wrappers:

```rust
#[cfg(any(test, feature = "dev-apis"))]
pub fn align_rows_amr(/* ... */) -> Option<RowAlignment> { /* ... */ }
```

Repeat for:

* `discover_anchors(old: &GridView, new: &GridView)`
* `align_single_column_change(old: &Grid, new: &Grid, ...)`

Or delete them and update tests to call the underlying `*_from_views` functions.

---

## Phase 5 — Finish the “engine knot” (optional, but this is what makes it *complete*)

If the goal is truly “Elegant Simplicity refactor is complete”, `engine.rs` is the last major place where complexity still “leaks”.

### Step 5.1 — Introduce an internal `SheetGridDiffer` struct

This keeps state in one place and turns long parameter lists into fields.

```rust
struct SheetGridDiffer<'a, S: DiffSink> {
    sheet_id: &'a SheetId,
    old: &'a Grid,
    new: &'a Grid,
    old_view: GridView<'a>,
    new_view: GridView<'a>,
    pool: &'a mut StringPool,
    config: &'a DiffConfig,
    sink: &'a mut S,
    op_count: &'a mut usize,
    ctx: &'a mut DiffContext,
    #[cfg(feature = "perf-metrics")]
    metrics: Option<&'a mut crate::perf::DiffMetrics>,
}

impl<'a, S: DiffSink> SheetGridDiffer<'a, S> {
    fn run(&mut self) -> Result<(), DiffError> {
        self.detect_masked_moves()?;
        self.align_and_diff_cells()
    }

    fn detect_masked_moves(&mut self) -> Result<(), DiffError> {
        // move detection phase as-is, but using fields
        Ok(())
    }

    fn align_and_diff_cells(&mut self) -> Result<(), DiffError> {
        // alignment + cell diff phase as-is, but using fields
        Ok(())
    }
}
```

Then your existing free function becomes:

```rust
fn diff_sheet_grid<S: DiffSink>(/* existing args */) -> Result<(), DiffError> {
    let old_view = GridView::from_grid_with_config(old, config);
    let new_view = GridView::from_grid_with_config(new, config);

    let mut differ = SheetGridDiffer {
        sheet_id,
        old,
        new,
        old_view,
        new_view,
        pool,
        config,
        sink,
        op_count,
        ctx,
        #[cfg(feature="perf-metrics")]
        metrics: metrics.as_mut(),
    };

    differ.run()
}
```

This is a pure refactor: no behavior change, but huge readability gain.

### Step 5.2 — Add a perf phase guard to remove repetitive cfg blocks

If perf metrics are enabled, the current code has a lot of “start/end phase” scaffolding. An RAII guard makes it disappear:

```rust
#[cfg(feature = "perf-metrics")]
struct PhaseGuard<'a> {
    metrics: &'a mut crate::perf::DiffMetrics,
    phase: crate::perf::Phase,
}

#[cfg(feature = "perf-metrics")]
impl<'a> PhaseGuard<'a> {
    fn new(metrics: &'a mut crate::perf::DiffMetrics, phase: crate::perf::Phase) -> Self {
        metrics.start_phase(phase);
        Self { metrics, phase }
    }
}

#[cfg(feature = "perf-metrics")]
impl Drop for PhaseGuard<'_> {
    fn drop(&mut self) {
        self.metrics.end_phase(self.phase);
    }
}
```

Usage:

```rust
#[cfg(feature="perf-metrics")]
let _guard = PhaseGuard::new(metrics, Phase::Alignment);
```

This makes the engine logic read like actual logic, not instrumentation.

---

# “Definition of Done” checklist for Elegant Simplicity

I’d call the refactor *complete* when all of these are true:

* [ ] `GridView` and grid metadata do **not** depend on `alignment::*` modules
* [ ] There is **one canonical row identity name** internally (`signature`), and any alias (`hash`) is deprecated or gone
* [ ] `assembly.rs` reads like “select strategy → dispatch → return” with recursive logic extracted into helpers
* [ ] No production build needs `#[allow(dead_code)]` for wrappers (either feature-gated or removed)
* [ ] `engine.rs` orchestration is encapsulated (state struct) or split into a few small modules
* [ ] The review artifacts (`codebase_context.md`, benchmarks) reflect the actual branch/commit you’re evaluating

---

# Small but important note on your attached benchmarks

`benchmark_results.json` reports `git_branch: development` and commit `2d52dca` (timestamp 2025‑12‑17). If your elegant-simplicity refactor lives on a different branch, you’ll want to re-run benchmarks and update the artifact so architectural evaluation and performance evidence refer to the same code revision.

