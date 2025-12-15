I reviewed the updated `codebase_context.md` plus the test run and the benchmark/perf outputs you attached.

## What looks complete and correct

* **Unit tests are green** (141 passed, 0 failed). 
* **Branch-3 config plumbing appears in place** (builder validation, serde roundtrip + alias tests, presets). 

## What is still wrong or unfinished

### 1) Full-scale performance is failing key thresholds and regressed vs baseline

From the current full-scale benchmark JSON:

* `perf_50k_dense_single_edit`: **12,257 ms** (max 5,000 ms)
* `perf_50k_completely_different`: **12,022 ms** (max 10,000 ms)
* `perf_50k_identical`: **2,870 ms** (max 200 ms)



Compared to the earlier baseline run you attached (`8bec00c3...`), these are materially slower (e.g. dense single edit **8,070 ms → 12,257 ms**). 

### 2) Root cause A: GridView column hashing still sorts entire columns by row

In `core/src/grid_view.rs`, `GridView::from_grid_with_config` builds `col_cells` and then does a **per-column sort by row** before hashing. For a 50k x 100 dense sheet, that is 100 sorts of 50k elements per grid (and you build views multiple times in the pipeline). 

### 3) Root cause B: AMR and row-change aligners rebuild GridView repeatedly

Both:

* `align_rows_amr_with_signatures` 
* `align_row_changes` 

construct fresh `GridView`s internally, so the diff pipeline pays view construction costs multiple times per sheet diff.

### 4) Root cause C: “identical grids” fast-path uses millions of slow HashMap lookups

In `try_diff_grids_with_config`, identical grids are short-circuited via `grids_non_blank_cells_equal(old, new)` (after checking `nrows/ncols`), and that function does `new.cells.get(coord)` for every cell in `old`. On a 5M-cell grid this is **5M hash lookups** and dominates the “identical” case time.  

Your `Grid.cells` is currently a `std::collections::HashMap` (SipHash), which is secure but slow. 

### 5) Root cause D: per-row diff still scans every column and does HashMap gets per cell

`diff_row_pair` loops `for col in 0..overlap_cols` and does `old.get` + `new.get` for each column, i.e. millions of HashMap lookups for dense sheets and (worse) wasted work for sparse sheets. 

This also conflicts with the design goal in the spec that diffs should iterate **non-empty cells**, not the full column range. 

### 6) Performance threshold script is inconsistent with the documented table

`scripts/check_perf_thresholds.py` prints a table with tighter thresholds but uses much looser internal constants (`30s`, `60s`, etc.), so CI gating won’t catch regressions like the ones above. 

---

# Implementation plan with concrete patches

Below are the highest-impact, lowest-risk changes first. If you apply **Patches 1–3 + Patch 5**, you should see the biggest immediate reductions on fullscale. Patch 4 is targeted specifically at the “identical” regression.

---

## Patch 1: Remove per-column sorting in GridView by hashing columns in row order

### Why

You already sort each row’s cells by column; you can compute per-column hashes by scanning rows in increasing row index and updating a per-column hasher. That preserves the exact same semantics as “collect column cells + sort by row + hash”.

This removes the expensive `cells.sort_unstable_by_key(|c| c.row)` in GridView construction. 

### 1A) Update imports in `core/src/grid_view.rs`

Replace this:

```rust
use crate::hashing::{hash_col_content_128, hash_row_content_128};
```

With this:

```rust
use crate::hashing::{hash_cell_value, hash_row_content_128};
use xxhash_rust::xxh3::Xxh3;
```

### 1B) Remove `col_cells` allocation

Replace this block:

```rust
        let mut row_counts: Vec<u32> = vec![0; nrows as usize];
        let mut col_counts: Vec<u32> = vec![0; ncols as usize];
        let mut row_first_non_blank: Vec<Option<u32>> = vec![None; nrows as usize];
        let mut col_first_non_blank: Vec<Option<u32>> = vec![None; ncols as usize];
        let mut col_cells: Vec<Vec<&'a Cell>> = vec![Vec::new(); ncols as usize];
```

With this:

```rust
        let mut row_counts: Vec<u32> = vec![0; nrows as usize];
        let mut col_counts: Vec<u32> = vec![0; ncols as usize];
        let mut row_first_non_blank: Vec<Option<u32>> = vec![None; nrows as usize];
        let mut col_first_non_blank: Vec<Option<u32>> = vec![None; ncols as usize];
```

### 1C) Remove the `col_cells[c].push(cell)` write

Replace this:

```rust
            if is_non_blank(cell) {
                row_counts[r] += 1;
                col_counts[c] += 1;
                if row_first_non_blank[r].map_or(true, |x| c as u32 < x) {
                    row_first_non_blank[r] = Some(c as u32);
                }
                if col_first_non_blank[c].map_or(true, |x| r as u32 < x) {
                    col_first_non_blank[c] = Some(r as u32);
                }
            }

            col_cells[c].push(cell);
```

With this:

```rust
            if is_non_blank(cell) {
                row_counts[r] += 1;
                col_counts[c] += 1;
                if row_first_non_blank[r].map_or(true, |x| c as u32 < x) {
                    row_first_non_blank[r] = Some(c as u32);
                }
                if col_first_non_blank[c].map_or(true, |x| r as u32 < x) {
                    col_first_non_blank[c] = Some(r as u32);
                }
            }
```

### 1D) Replace the `col_meta` computation

Replace this:

```rust
        let col_meta = col_cells
            .into_iter()
            .enumerate()
            .map(|(idx, mut cells)| {
                cells.sort_unstable_by_key(|c| c.row);
                let hash = hash_col_content_128(&cells);

                ColMeta {
                    col_idx: idx as u32,
                    hash,
                    non_blank_count: to_u16(col_counts.get(idx).copied().unwrap_or(0)),
                    first_non_blank_row: col_first_non_blank
                        .get(idx)
                        .and_then(|r| r.map(to_u16))
                        .unwrap_or(0),
                }
            })
            .collect();
```

With this:

```rust
        let mut col_hashers: Vec<Xxh3> = (0..ncols).map(|_| Xxh3::new()).collect();

        for row_view in rows.iter() {
            for (col, cell) in row_view.cells.iter() {
                let idx = *col as usize;
                if idx >= col_hashers.len() {
                    continue;
                }
                hash_cell_value(&cell.value, &mut col_hashers[idx]);
                cell.formula.hash(&mut col_hashers[idx]);
            }
        }

        let col_meta: Vec<ColMeta> = (0..ncols as usize)
            .map(|idx| ColMeta {
                col_idx: idx as u32,
                hash: col_hashers[idx].digest128(),
                non_blank_count: to_u16(col_counts[idx]),
                first_non_blank_row: col_first_non_blank[idx].map(to_u16).unwrap_or(0),
            })
            .collect();
```

---

## Patch 2: Remove per-column sorting in `Grid::compute_all_signatures`

### Why

You currently do the same expensive per-column “collect + sort-by-row + hash” in `compute_all_signatures`. 
Even if not used in the hot path, this must stay consistent with `GridView` (and it’s a correctness expectation in tests that compare signatures vs view meta).

Replace this in `core/src/workbook.rs`:

```rust
    pub fn compute_all_signatures(&mut self) {
        let mut row_cells: Vec<Vec<(u32, &Cell)>> = vec![Vec::new(); self.nrows as usize];
        let mut col_cells: Vec<Vec<&Cell>> = vec![Vec::new(); self.ncols as usize];

        for cell in self.cells.values() {
            let row_idx = cell.row as usize;
            let col_idx = cell.col as usize;
            row_cells[row_idx].push((cell.col, cell));
            col_cells[col_idx].push(cell);
        }

        self.row_signatures = Some(
            row_cells
                .into_iter()
                .map(|mut row| {
                    row.sort_by_key(|(col, _)| *col);
                    RowSignature {
                        hash: hash_row_content_128(&row),
                    }
                })
                .collect(),
        );

        self.col_signatures = Some(
            col_cells
                .into_iter()
                .map(|mut col| {
                    col.sort_by_key(|cell| cell.row);
                    ColSignature {
                        hash: hash_col_content_128(&col),
                    }
                })
                .collect(),
        );
    }
```

With this:

```rust
    pub fn compute_all_signatures(&mut self) {
        use crate::hashing::hash_cell_value;
        use xxhash_rust::xxh3::Xxh3;

        let mut row_cells: Vec<Vec<(u32, &Cell)>> = vec![Vec::new(); self.nrows as usize];

        for cell in self.cells.values() {
            let row_idx = cell.row as usize;
            row_cells[row_idx].push((cell.col, cell));
        }

        for row in row_cells.iter_mut() {
            row.sort_by_key(|(col, _)| *col);
        }

        let row_signatures: Vec<RowSignature> = row_cells
            .iter()
            .map(|row| RowSignature {
                hash: hash_row_content_128(row),
            })
            .collect();

        let mut col_hashers: Vec<Xxh3> = (0..self.ncols).map(|_| Xxh3::new()).collect();
        for row in row_cells.iter() {
            for (col, cell) in row.iter() {
                let idx = *col as usize;
                if idx >= col_hashers.len() {
                    continue;
                }
                hash_cell_value(&cell.value, &mut col_hashers[idx]);
                cell.formula.hash(&mut col_hashers[idx]);
            }
        }

        let col_signatures: Vec<ColSignature> = col_hashers
            .into_iter()
            .map(|h| ColSignature { hash: h.digest128() })
            .collect();

        self.row_signatures = Some(row_signatures);
        self.col_signatures = Some(col_signatures);
    }
```

---

## Patch 3: Make unordered column hashes stop sorting by row

### Why

`unordered_col_hashes` sorts by row before computing an unordered hash. That sort is redundant. 

In `core/src/column_alignment.rs`, replace:

```rust
    let hashes: Vec<u128> = col_cells
        .iter_mut()
        .map(|cells| {
            cells.sort_unstable_by_key(|c| c.row);
            hash_col_content_unordered_128(cells)
        })
        .collect();
```

With:

```rust
    let hashes: Vec<u128> = col_cells
        .iter()
        .map(|cells| hash_col_content_unordered_128(cells))
        .collect();
```

---

## Patch 4: Fix the “identical grids” regression by switching Grid.cells to a fast hasher

### Why

The identical fast-path is dominated by HashMap lookups in `grids_non_blank_cells_equal`.
Switching `Grid.cells` away from SipHash is the simplest way to speed:

* the identical early equality check
* all `Grid::get` / `Grid::get_mut` lookups
* the current `diff_row_pair` implementation (until Patch 5 lands)

### 4A) Add dependency

In `core/Cargo.toml`, replace:

```toml
[dependencies]
quick-xml = "0.32"
thiserror = "1.0"
zip = { version = "0.6", default-features = false, features = ["deflate"] }
base64 = "0.22"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
xxhash-rust = { version = "0.8", features = ["xxh64", "xxh3"] }
```

With:

```toml
[dependencies]
quick-xml = "0.32"
thiserror = "1.0"
zip = { version = "0.6", default-features = false, features = ["deflate"] }
base64 = "0.22"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
xxhash-rust = { version = "0.8", features = ["xxh64", "xxh3"] }
rustc-hash = "1.1"
```

(Your current `core/Cargo.toml` is shown in the context here. )

### 4B) Change Grid.cells type

In `core/src/workbook.rs`, replace the `Grid` struct field:

```rust
pub cells: HashMap<(u32, u32), Cell>,
```

With:

```rust
pub cells: rustc_hash::FxHashMap<(u32, u32), Cell>,
```

And update `Grid::new` accordingly. Replace:

```rust
cells: HashMap::new(),
```

With:

```rust
cells: rustc_hash::FxHashMap::default(),
```

This will cascade compile errors anywhere you explicitly name the `HashMap<(u32,u32),Cell>` type (most callsites just use `.cells.*` and should work unchanged).

---

## Patch 5: Replace per-column scanning in `diff_row_pair` with non-empty cell iteration

### Why

This is both a performance and an algorithmic-completeness fix: the spec expectation is to iterate only non-empty cells rather than scanning full column ranges. 
The current code does:

* scan `0..overlap_cols`
* for each col: `old.get` and `new.get` (HashMap lookups) 

### Approach

1. Build `GridView` for each grid **once** at the start of the sheet diff (see Patch 6 for the caching piece).
2. Implement a `diff_row_pair_sparse` that merges the two sorted per-row cell lists:

   * emits diffs for columns present in either row
   * treats missing as blank
   * never iterates blank-blank cells

### 5A) Add a sparse row diff helper

Add this new function near `diff_row_pair` in `core/src/diff.rs`:

```rust
fn diff_row_pair_sparse(
    sheet: &str,
    a_row: u32,
    b_row: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
    edits: &mut Vec<DiffOp>,
    config: &DiffConfig,
) {
    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_cells.len() || j < new_cells.len() {
        let col_a = old_cells.get(i).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col_b = new_cells.get(j).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col = col_a.min(col_b);

        if col >= overlap_cols {
            break;
        }

        let old_cell = if col_a == col {
            let (_, cell) = old_cells[i];
            i += 1;
            Some(cell)
        } else {
            None
        };

        let new_cell = if col_b == col {
            let (_, cell) = new_cells[j];
            j += 1;
            Some(cell)
        } else {
            None
        };

        let addr = CellAddress::from_indices(b_row, col);

        let from = old_cell.map(|c| CellSnapshot::from_cell_at(a_row, col, c));
        let to = new_cell.map(|c| CellSnapshot::from_cell_at(b_row, col, c));

        let changed = match (from.as_ref(), to.as_ref()) {
            (Some(f), Some(t)) => !cells_content_equal(&f.value, &f.formula, &t.value, &t.formula),
            (Some(_), None) => true,
            (None, Some(_)) => true,
            (None, None) => false,
        };

        if changed || config.include_unchanged_cells {
            edits.push(DiffOp::CellEdited {
                sheet: sheet.to_string(),
                addr,
                from,
                to,
            });
        }
    }
}
```

### 5B) Wire it into aligned diff emission

This requires having row cell lists available. The cleanest way is to pass `GridView`s into `emit_amr_aligned_diffs` and use `view.rows[row_idx].cells`.

You’ll need to change `emit_amr_aligned_diffs` signature and body. Replace:

```rust
fn emit_amr_aligned_diffs(
    sheet: &str,
    old: &Grid,
    new: &Grid,
    row_alignment: &RowAlignment,
    edits: &mut Vec<DiffOp>,
    config: &DiffConfig,
    metrics: &mut DiffMetrics,
) {
    let overlap_cols = old.ncols.min(new.ncols);
    let overlap_rows = row_alignment.matched.len() as u32;
    metrics.add_cells_compared(overlap_rows * overlap_cols);

    let start = Instant::now();
    for (old_row, new_row) in &row_alignment.matched {
        diff_row_pair(
            sheet,
            *old_row,
            *new_row,
            overlap_cols,
            old,
            new,
            edits,
            config,
            metrics,
        );
    }
    metrics.add_cell_diff_time(start.elapsed());
}
```

With:

```rust
fn emit_amr_aligned_diffs(
    sheet: &str,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    row_alignment: &RowAlignment,
    edits: &mut Vec<DiffOp>,
    config: &DiffConfig,
    metrics: &mut DiffMetrics,
) {
    let overlap_cols = old.ncols.min(new.ncols);

    let start = Instant::now();
    for (old_row, new_row) in &row_alignment.matched {
        let old_cells = &old_view.rows[*old_row as usize].cells;
        let new_cells = &new_view.rows[*new_row as usize].cells;

        diff_row_pair_sparse(
            sheet,
            *old_row,
            *new_row,
            overlap_cols,
            old_cells,
            new_cells,
            edits,
            config,
        );
    }
    metrics.add_cell_diff_time(start.elapsed());
}
```

Note: once you do this, the existing `metrics.add_cells_compared(overlap_rows * overlap_cols)` becomes misleading for sparse iteration. Either:

* remove it, or
* increment by `old_cells.len() + new_cells.len()` per row, or
* treat it as a “logical worst-case compared” counter and keep it.

---

## Patch 6: Cache GridView once per sheet diff and reuse across alignment strategies

### Why

You currently build `GridView` inside multiple alignment routines (AMR, row-change, single-column-change, move detection).  
Even after Patch 1, rebuilding views is still wasted work.

### Implementation pattern

1. In `try_diff_grids_with_config`, after the “identical” fast-path check, build:

```rust
let old_view = GridView::from_grid_with_config(old, config);
let new_view = GridView::from_grid_with_config(new, config);
```

2. Add `*_from_views` variants for the aligners (keeps API compatibility):

* `align_rows_amr_with_signatures_from_views(&old_view, &new_view, config)`
* `align_row_changes_from_views(&old_view, &new_view, config)`
* etc.

3. Update the pipeline to call the view-based variants, and pass the same views into diff emission (Patch 5).

This is the part that prevents “view build N times” behavior and is very likely part of what regressed branch-3 runtime.

---

## Patch 7: Make perf gating match the benchmark table

This won’t speed anything up, but it prevents future regressions.

In `scripts/check_perf_thresholds.py`, the printed table implies much tighter targets than the actual `THRESHOLDS` dict. 

Two options:

* Change `THRESHOLDS` to match the table.
* Or read `max_time_ms` directly from your `benchmark_results.json` and enforce those.

---

# Suggested order of execution

1. Patch 1 (GridView col hashing)
2. Patch 2 (Grid signatures col hashing)
3. Patch 4 (FxHashMap for Grid.cells)
4. Patch 6 (cache GridView + add view-based aligners)
5. Patch 5 (sparse per-row diff using row views)
6. Patch 3 (small cleanup)
7. Patch 7 (perf gating)

This ordering targets:

* biggest algorithmic hot spots first (column sorts, repeated view builds)
* then the “identical” regression (HashMap lookups)
* then the still-expensive cell diff loop

---

# What I’d expect to improve immediately

* **Dense single edit / completely different**: large reduction from removing column sorts and avoiding repeated GridView construction.
* **Identical**: should drop sharply once `Grid.cells` uses a fast hasher (because the early equality check is lookup-heavy).

If you apply the patches above and re-run the same fullscale harness, you should see whether “identical” is still dominated by equality checks or if it shifts elsewhere (at which point we can tighten further, e.g., by adding a cheap per-grid fingerprint cache, or a dense-grid representation fast path).
