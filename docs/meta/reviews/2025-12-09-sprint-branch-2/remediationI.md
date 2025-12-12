## core/src/engine.rs

### 1) Add a whole-grid equality helper (used for the identical fast-path)

Replace this:

```rust
fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), None) | (None, Some(cell_a)) => {
            cell_a.value.is_none() && cell_a.formula.is_none()
        }
        (Some(cell_a), Some(cell_b)) => cell_a.value == cell_b.value && cell_a.formula == cell_b.formula,
    }
}
```

With this:

```rust
fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), None) | (None, Some(cell_a)) => {
            cell_a.value.is_none() && cell_a.formula.is_none()
        }
        (Some(cell_a), Some(cell_b)) => cell_a.value == cell_b.value && cell_a.formula == cell_b.formula,
    }
}

fn grids_content_equal(old: &Grid, new: &Grid) -> bool {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return false;
    }

    for ((row, col), cell_old) in &old.cells {
        let cell_new = new.get(*row, *col);
        if !cells_content_equal(Some(cell_old), cell_new) {
            return false;
        }
    }

    for ((row, col), cell_new) in &new.cells {
        let cell_old = old.get(*row, *col);
        if !cells_content_equal(cell_old, Some(cell_new)) {
            return false;
        }
    }

    true
}
```

---

### 2) Identical-grid fast-path before move detection, alignment, etc.

Replace this block at the top of `diff_grids_core`:

```rust
) {
    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
```

With this:

```rust
) {
    if grids_content_equal(old, new) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
            m.end_phase(Phase::MoveDetection);
        }
        return;
    }

    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
```

---

### 3) Fix `row_signature_counts` so it does not do O(nrows * total_cells)

Replace this:

```rust
fn row_signature_counts(grid: &Grid) -> HashMap<RowSignature, u32> {
    let mut counts = HashMap::new();
    for row in 0..grid.nrows {
        if let Some(sig) = row_signature_at(grid, row) {
            *counts.entry(sig).or_insert(0) += 1;
        }
    }
    counts
}
```

With this:

```rust
fn row_signature_counts(grid: &Grid) -> HashMap<RowSignature, u32> {
    if let Some(rows) = grid.row_signatures.as_ref() {
        let mut counts = HashMap::new();
        for &sig in rows {
            *counts.entry(sig).or_insert(0) += 1;
        }
        return counts;
    }

    use crate::hashing::hash_row_content_128;

    let nrows = grid.nrows as usize;
    let mut rows: Vec<Vec<(u32, &Cell)>> = vec![Vec::new(); nrows];

    for ((row, col), cell) in &grid.cells {
        rows[*row as usize].push((*col, cell));
    }

    let mut counts = HashMap::new();
    for mut row_cells in rows {
        row_cells.sort_unstable_by_key(|(col, _)| *col);
        let sig = RowSignature {
            hash: hash_row_content_128(&row_cells),
        };
        *counts.entry(sig).or_insert(0) += 1;
    }

    counts
}
```

---

### 4) Don’t compute `has_row_edits` unless there are structural rows

Replace this:

```rust
        let has_row_edits = alignment
            .matched
            .iter()
            .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));
        if has_structural_rows && has_row_edits {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            positional_diff(sheet_id, old, new, ops);
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.add_cells_compared(cells_in_overlap(old, new));
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return;
        }
```

With this:

```rust
        if has_structural_rows {
            let has_row_edits = alignment
                .matched
                .iter()
                .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));

            if has_row_edits {
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.start_phase(Phase::CellDiff);
                }
                positional_diff(sheet_id, old, new, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                    m.end_phase(Phase::CellDiff);
                }
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.end_phase(Phase::Alignment);
                }
                return;
            }
        }
```

---

## core/src/workbook.rs

### 5) Make `compute_row_signature` and `compute_col_signature` stop scanning the entire grid every call

Replace this:

```rust
pub fn compute_row_signature(&self, row: u32) -> RowSignature {
    let mut row_cells: Vec<_> = self
        .cells
        .values()
        .filter(|cell| cell.row == row)
        .map(|cell| (cell.col, cell))
        .collect();
    row_cells.sort_by_key(|(col, _)| *col);
    let hash = hash_row_content_128(&row_cells);
    RowSignature { hash }
}

pub fn compute_col_signature(&self, col: u32) -> ColSignature {
    let mut col_cells: Vec<_> = self
        .cells
        .values()
        .filter(|cell| cell.col == col)
        .collect();
    col_cells.sort_by_key(|c| c.row);
    let hash = hash_col_content_128(&col_cells);
    ColSignature { hash }
}
```

With this:

```rust
pub fn compute_row_signature(&self, row: u32) -> RowSignature {
    let nrows = self.nrows as usize;
    let ncols = self.ncols as usize;
    let total_cells = self.cells.len();

    let avg_cells_per_row = if nrows == 0 { 0 } else { total_cells / nrows };
    let scan_cols = ncols <= 256 || avg_cells_per_row.saturating_mul(4) >= ncols;

    let hash = if scan_cols {
        let mut row_cells: Vec<(u32, &Cell)> = Vec::with_capacity(avg_cells_per_row.min(ncols));
        for col in 0..self.ncols {
            if let Some(cell) = self.get(row, col) {
                row_cells.push((col, cell));
            }
        }
        hash_row_content_128(&row_cells)
    } else {
        let mut row_cells: Vec<_> = self
            .cells
            .values()
            .filter(|cell| cell.row == row)
            .map(|cell| (cell.col, cell))
            .collect();
        row_cells.sort_unstable_by_key(|(col, _)| *col);
        hash_row_content_128(&row_cells)
    };

    RowSignature { hash }
}

pub fn compute_col_signature(&self, col: u32) -> ColSignature {
    let nrows = self.nrows as usize;
    let ncols = self.ncols as usize;
    let total_cells = self.cells.len();

    let avg_cells_per_col = if ncols == 0 { 0 } else { total_cells / ncols };
    let scan_rows = nrows <= 1024 || avg_cells_per_col.saturating_mul(4) >= nrows;

    let hash = if scan_rows {
        let mut col_cells: Vec<&Cell> = Vec::with_capacity(avg_cells_per_col.min(nrows));
        for row in 0..self.nrows {
            if let Some(cell) = self.get(row, col) {
                col_cells.push(cell);
            }
        }
        hash_col_content_128(&col_cells)
    } else {
        let mut col_cells: Vec<_> = self.cells.values().filter(|cell| cell.col == col).collect();
        col_cells.sort_unstable_by_key(|c| c.row);
        hash_col_content_128(&col_cells)
    };

    ColSignature { hash }
}
```

---

### 6) Use unstable sorts in `compute_all_signatures` hot path

Replace this:

```rust
for row_cells in &mut rows {
    row_cells.sort_by_key(|(col, _)| *col);
}
for col_cells in &mut cols {
    col_cells.sort_by_key(|c| c.row);
}
```

With this:

```rust
for row_cells in &mut rows {
    row_cells.sort_unstable_by_key(|(col, _)| *col);
}
for col_cells in &mut cols {
    col_cells.sort_unstable_by_key(|c| c.row);
}
```

---

## core/src/grid_view.rs

### 7) Use unstable sorts in GridView construction

Replace this:

```rust
for row_view in rows.iter_mut() {
    row_view.cells.sort_by_key(|(col, _)| *col);
}
```

With this:

```rust
for row_view in rows.iter_mut() {
    row_view.cells.sort_unstable_by_key(|(col, _)| *col);
}
```

And replace this:

```rust
cells.sort_by_key(|c| c.row);
```

With this:

```rust
cells.sort_unstable_by_key(|c| c.row);
```
