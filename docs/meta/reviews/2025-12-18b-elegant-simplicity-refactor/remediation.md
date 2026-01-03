The updated codebase is in good shape structurally: the `alignment/` subsystem is thoughtfully documented and test-backed, `GridView` has become a real “center of gravity” for sparse metadata, and the engine’s move/alignment pipeline is much more readable than a typical spreadsheet diff core. With all tests passing, you’re in the ideal phase to finish the “elegant simplicity” refactor: remove the remaining *transitional* complexity (dual types / dual emit paths / duplicated heuristics), and tighten dependency direction so the architecture “reads” correctly.

Below is a remediation plan that focuses on what’s still *accidentally complex* in the current codebase, and how to eliminate it with minimal behavioral risk.

---

# What still feels incomplete for “Elegant Simplicity”

## 1) A subtle dependency inversion

`grid_view.rs` currently imports `alignment::row_metadata::classify_row_frequencies` and re-exports `RowMeta/FrequencyClass` from `alignment::row_metadata`. That makes the conceptual layering feel “inside out”: a view/metadata layer shouldn’t depend on an algorithm folder.

This doesn’t break correctness, but it breaks *inevitability*: the reader asks, “why does the view depend on alignment?”

## 2) Two row-alignment worlds still exist

The engine still carries:

* `LegacyRowAlignment` / `LegacyRowBlockMove` (from `row_alignment.rs`)
* `AmrAlignment` / `AmrRowBlockMove` (from `alignment`)

…and it needs two nearly identical emitters: `emit_aligned_diffs` and `emit_amr_aligned_diffs`.

That’s classic refactor tail: correctness is fine, but the system still pays a cognitive tax every time someone touches the row-alignment path.

## 3) Heuristics are duplicated across detectors

Functions like:

* `is_within_size_bounds`
* `has_heavy_repetition`
* `blank_dominated`
* `low_info_dominated`
* “unique-to-a/b” checks

exist in multiple places (`row_alignment.rs`, `column_alignment.rs`, `rect_block_move.rs`). When heuristics are duplicated, the system slowly becomes inconsistent over time and harder to reason about.

## 4) `RowMeta` currently stores the same thing twice

`RowMeta` has both `signature: RowSignature` and `hash: RowSignature`, and `GridView` sets them equal.

That’s a very loud smell: it signals “two eras of the code are still cohabiting.” It’s one of the biggest remaining sources of accidental complexity because it forces the reader to wonder whether they can diverge (even if they never do).

## 5) Hidden state convenience APIs are still mixed into the “core story”

`WorkbookPackage::open/diff/diff_streaming` always goes through `with_default_session`, so the codebase’s “main path” still subtly relies on thread-local state. This is convenient, but it’s not the simplest mental model.

You can keep the convenience, but the simplest architecture makes the “explicit” path primary and the “implicit” path a thin wrapper.

---

# Remediation plan to complete the refactor

## Step 1: Move row metadata out of `alignment/` and fix dependency direction

### Goal

Make `GridView` depend on a metadata module, and make `alignment` depend on `GridView` + metadata (not the other way around).

### Create a new module: `core/src/grid_metadata.rs`

Create this new file with the content below (it’s your existing `alignment/row_metadata.rs`, adjusted to become the canonical home and to remove the redundant `hash` field):

```rust
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

### Update `core/src/lib.rs` to include the module

Replace this block:

```rust
mod grid_parser;
mod grid_view;
pub(crate) mod hashing;
```

With this:

```rust
mod grid_parser;
mod grid_metadata;
mod grid_view;
pub(crate) mod hashing;
```

### Update `core/src/grid_view.rs` to import/re-export from `grid_metadata`

Replace these lines:

```rust
use crate::alignment::row_metadata::classify_row_frequencies;
pub use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
```

With these:

```rust
use crate::grid_metadata::classify_row_frequencies;
pub use crate::grid_metadata::{FrequencyClass, RowMeta};
```

### Update `GridView` construction to stop writing `RowMeta.hash`

In `grid_view.rs`, replace this `RowMeta` init:

```rust
RowMeta {
    row_idx: idx as u32,
    signature,
    hash: signature,
    non_blank_count,
    first_non_blank_col,
    frequency_class,
    is_low_info,
}
```

With:

```rust
RowMeta {
    row_idx: idx as u32,
    signature,
    non_blank_count,
    first_non_blank_col,
    frequency_class,
    is_low_info,
}
```

### Update alignment modules to import from `crate::grid_metadata`

Example replacement pattern:

Old:

```rust
use crate::alignment::row_metadata::RowMeta;
```

New:

```rust
use crate::grid_metadata::RowMeta;
```

Do the same for `FrequencyClass`.

### Remove `core/src/alignment/row_metadata.rs` and drop it from `alignment/mod.rs`

In `core/src/alignment/mod.rs`, remove:

```rust
pub(crate) mod row_metadata;
```

---

## Step 2: Unify RowAlignment and RowBlockMove types (remove “Legacy vs AMR” duplication)

### Goal

There should be exactly one `RowAlignment` type (with `moves: Vec<RowBlockMove>` always present). Legacy code can return `moves: Vec::new()`.

### Update `core/src/row_alignment.rs` to stop defining its own RowAlignment/RowBlockMove

Replace this block:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowAlignment {
    pub matched: Vec<(u32, u32)>,
    pub inserted: Vec<u32>,
    pub deleted: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowBlockMove {
    pub src_start_row: u32,
    pub row_count: u32,
    pub dst_start_row: u32,
}
```

With:

```rust
use crate::alignment::{RowAlignment, RowBlockMove};
```

### Ensure legacy constructors set `moves: Vec::new()`

For example, replace:

```rust
let alignment = RowAlignment {
    matched,
    inserted,
    deleted,
};
```

With:

```rust
let alignment = RowAlignment {
    matched,
    inserted,
    deleted,
    moves: Vec::new(),
};
```

### Replace all `meta.hash` uses in `row_alignment.rs` (RowMeta no longer has it)

Example change inside the exact move detector:

Old:

```rust
if meta_a.hash == meta_b.hash {
```

New:

```rust
if meta_a.signature == meta_b.signature {
```

And update the uniqueness checks:

Old:

```rust
if !is_unique_to_b(meta.hash, stats) {
```

New (after Step 4 adds methods):

```rust
if !stats.is_unique_to_b(meta.signature) {
```

---

## Step 3: Collapse `emit_aligned_diffs` and `emit_amr_aligned_diffs` into one function

### Goal

One row-alignment emission path.

Replace the two functions:

```rust
fn emit_aligned_diffs( ... alignment: &LegacyRowAlignment, ... ) -> Result<u64, DiffError> { ... }

fn emit_amr_aligned_diffs( ... alignment: &AmrAlignment, ... ) -> Result<u64, DiffError> { ... }
```

With a single unified function:

```rust
fn emit_row_alignment_diffs(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &RowAlignment,
    pool: &StringPool,
    formula_cache: &mut FormulaParseCache,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    let mut compared = 0u64;

    for (row_a, row_b) in &alignment.matched {
        if let (Some(old_row), Some(new_row)) = (
            old_view.rows.get(*row_a as usize),
            new_view.rows.get(*row_b as usize),
        ) {
            compared = compared.saturating_add(diff_row_pair_sparse(
                sheet_id,
                pool,
                formula_cache,
                *row_a,
                *row_b,
                overlap_cols,
                &old_row.cells,
                &new_row.cells,
                sink,
                op_count,
                config,
            )?);
        }
    }

    for row_idx in &alignment.inserted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_added(sheet_id.clone(), *row_idx, None),
        )?;
    }

    for row_idx in &alignment.deleted {
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id.clone(), *row_idx, None),
        )?;
    }

    for mv in &alignment.moves {
        emit_op(
            sink,
            op_count,
            DiffOp::BlockMovedRows {
                sheet: sheet_id.clone(),
                src_start_row: mv.src_start_row,
                row_count: mv.row_count,
                dst_start_row: mv.dst_start_row,
                block_hash: None,
            },
        )?;
    }

    if new_view.source.ncols > old_view.source.ncols {
        for col_idx in old_view.source.ncols..new_view.source.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_added(sheet_id.clone(), col_idx, None),
            )?;
        }
    } else if old_view.source.ncols > new_view.source.ncols {
        for col_idx in new_view.source.ncols..old_view.source.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_removed(sheet_id.clone(), col_idx, None),
            )?;
        }
    }

    Ok(compared)
}
```

Then update the two call sites in `engine.rs`:

Old (AMR path):

```rust
let compared = emit_amr_aligned_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    &mut op_count,
    config,
)?;
```

New:

```rust
let compared = emit_row_alignment_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    &mut op_count,
    config,
)?;
```

Old (legacy path):

```rust
let compared = emit_aligned_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    &mut op_count,
    config,
)?;
```

New:

```rust
let compared = emit_row_alignment_diffs(
    sheet_id,
    &old_view,
    &new_view,
    &alignment,
    pool,
    &mut ctx.formula_cache,
    sink,
    &mut op_count,
    config,
)?;
```

---

## Step 4: Centralize “quality heuristics” into `GridView` and `HashStats`

### Goal

Delete duplicated helper functions across modules and make the heuristics read like domain language.

### Add these methods to `GridView` (in `grid_view.rs`)

Add to the `impl<'a> GridView<'a>` block:

```rust
pub fn is_low_info_dominated(&self) -> bool {
    if self.row_meta.is_empty() {
        return false;
    }
    let low = self.row_meta.iter().filter(|m| m.is_low_info()).count();
    low * 2 > self.row_meta.len()
}

pub fn is_blank_dominated(&self) -> bool {
    if self.col_meta.is_empty() {
        return false;
    }
    let blank = self.col_meta.iter().filter(|m| m.non_blank_count == 0).count();
    blank * 2 > self.col_meta.len()
}
```

### Add these methods to `HashStats` (in `grid_view.rs`)

Replace the generic impl block:

```rust
impl<H> HashStats<H>
where
    H: Eq + Hash + Copy,
{
    pub fn is_unique(&self, hash: H) -> bool { ... }
    pub fn is_rare(&self, hash: H, threshold: u32) -> bool { ... }
    pub fn is_common(&self, hash: H, threshold: u32) -> bool { ... }
    pub fn appears_in_both(&self, hash: H) -> bool { ... }
}
```

With:

```rust
impl<H> HashStats<H>
where
    H: Eq + Hash + Copy,
{
    pub fn is_unique(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 1
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 1
    }

    pub fn is_unique_to_a(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 1
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 0
    }

    pub fn is_unique_to_b(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 0
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 1
    }

    pub fn max_frequency(&self) -> u32 {
        self.freq_a
            .values()
            .chain(self.freq_b.values())
            .copied()
            .max()
            .unwrap_or(0)
    }

    pub fn has_heavy_repetition(&self, max_repeat: u32) -> bool {
        self.max_frequency() > max_repeat
    }

    pub fn is_rare(&self, hash: H, threshold: u32) -> bool {
        let freq_a = self.freq_a.get(&hash).copied().unwrap_or(0);
        let freq_b = self.freq_b.get(&hash).copied().unwrap_or(0);

        if freq_a == 0 || freq_b == 0 || self.is_unique(hash) {
            return false;
        }

        freq_a <= threshold && freq_b <= threshold
    }

    pub fn is_common(&self, hash: H, threshold: u32) -> bool {
        let freq_a = self.freq_a.get(&hash).copied().unwrap_or(0);
        let freq_b = self.freq_b.get(&hash).copied().unwrap_or(0);

        if freq_a == 0 && freq_b == 0 {
            return false;
        }

        freq_a > threshold || freq_b > threshold
    }

    pub fn appears_in_both(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) > 0
            && self.freq_b.get(&hash).copied().unwrap_or(0) > 0
    }
}
```

### Replace duplicated heuristics in detectors

Examples:

In `row_alignment.rs`:

Old:

```rust
if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
    return None;
}
...
if has_heavy_repetition(&stats, config) {
    return None;
}
```

New:

```rust
if view_a.is_low_info_dominated() || view_b.is_low_info_dominated() {
    return None;
}
...
if stats.has_heavy_repetition(config.max_hash_repeat) {
    return None;
}
```

In `column_alignment.rs`:

Old:

```rust
if has_heavy_repetition(&stats, config) {
    return None;
}
if blank_dominated(&view_a) || blank_dominated(&view_b) {
    return None;
}
```

New:

```rust
if stats.has_heavy_repetition(config.max_hash_repeat) {
    return None;
}
if view_a.is_blank_dominated() || view_b.is_blank_dominated() {
    return None;
}
```

In `rect_block_move.rs`:

Old:

```rust
if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
    return None;
}

if blank_dominated(&view_a) || blank_dominated(&view_b) {
    return None;
}

if has_heavy_repetition(&row_stats, config) || has_heavy_repetition(&col_stats, config) {
    return None;
}
```

New:

```rust
if view_a.is_low_info_dominated() || view_b.is_low_info_dominated() {
    return None;
}

if view_a.is_blank_dominated() || view_b.is_blank_dominated() {
    return None;
}

if row_stats.has_heavy_repetition(config.max_hash_repeat)
    || col_stats.has_heavy_repetition(config.max_hash_repeat)
{
    return None;
}
```

After this, you can delete the duplicated free functions in those modules.

---

## Step 5: Update tests that referenced `RowMeta.hash`

Example in `grid_view_hashstats_tests`:

Replace:

```rust
for (idx, meta) in view.row_meta.iter().enumerate() {
    assert_eq!(meta.hash, row_signatures[idx]);
}
```

With:

```rust
for (idx, meta) in view.row_meta.iter().enumerate() {
    assert_eq!(meta.signature, row_signatures[idx]);
}
```

Similarly, any test comparing `row_meta[...].hash` should switch to `.signature`.

---

## Step 6: Shrink production surface by moving test-only wrappers under `#[cfg(test)]`

This one is optional, but it’s “free” simplicity: less production API, fewer names to hold in your head.

### In `row_alignment.rs`, wrap `align_row_changes`

Replace:

```rust
#[allow(dead_code)]
pub(crate) fn align_row_changes(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignment> {
```

With:

```rust
#[cfg(test)]
pub(crate) fn align_row_changes(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignment> {
```

Do the same in `column_alignment.rs` for `align_single_column_change`.

---

## Step 7: Make `WorkbookPackage`’s explicit pool path the “real” implementation

This is the cleanest way to keep the convenience while making the architecture simple.

### In `package.rs`, add pool-explicit methods and make existing methods wrappers

Replace the current `diff` method:

```rust
pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
    crate::with_default_session(|session| {
        let mut report = crate::engine::diff_workbooks(
            &self.workbook,
            &other.workbook,
            &mut session.strings,
            config,
        );

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            &mut session.strings,
            config,
        );

        report.ops.extend(m_ops);
        report.strings = session.strings.strings().to_vec();
        report
    })
}
```

With:

```rust
pub fn diff_with_pool(
    &self,
    other: &Self,
    pool: &mut crate::string_pool::StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut report = crate::engine::diff_workbooks(&self.workbook, &other.workbook, pool, config);

    let m_ops = crate::m_diff::diff_m_ops_for_packages(
        &self.data_mashup,
        &other.data_mashup,
        pool,
        config,
    );

    report.ops.extend(m_ops);
    report.strings = pool.strings().to_vec();
    report
}

pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
    crate::with_default_session(|session| self.diff_with_pool(other, &mut session.strings, config))
}
```

Do the same for `diff_streaming` (add `diff_streaming_with_pool`).

This change makes the “real” system explicit and keeps thread-local state as a small convenience wrapper.

---

## Step 8: Optional “final polish”: introduce an `EmitCtx` to reduce parameter noise

This is the one change I’d call *pure elegance*: it doesn’t change behavior, it just makes code read like prose.

Today the emitter-related functions pass `pool, formula_cache, sink, op_count, config` everywhere.

A tiny struct makes intent explicit:

* “I am emitting ops for this sheet”
* “I hold the sink, op counter, config, and formula cache”

If you want to do this, start with just `emit_op` and `emit_cell_edit` and then gradually move `diff_row_pair_sparse` into it.

Example direction:

* Replace free functions with methods on `EmitCtx`.
* Convert call sites one function at a time.
* Keep old free functions as thin wrappers temporarily (then delete them once all call sites move).

I didn’t include the full rewrite here because it touches a lot of lines, but the payoff is large: engine code becomes narrative instead of plumbing.

---

# What “done” looks like after these steps

When the refactor is complete, you should see:

* `GridView` and `grid_metadata` are self-contained and do not import `alignment`.
* There is exactly one `RowAlignment` type and one row-alignment emission function.
* “Quality heuristics” live in one place (methods on `GridView` / `HashStats`), and detectors read like domain logic.
* `RowMeta` has a single field for its row signature (no duplicates).
* `WorkbookPackage` has explicit pool-based APIs as the real implementation, with thread-local convenience wrappers on top.

If you want, I can also produce a “mechanical checklist” of all exact symbol replacements (search patterns + replacements) to make the refactor quick and low-risk.
