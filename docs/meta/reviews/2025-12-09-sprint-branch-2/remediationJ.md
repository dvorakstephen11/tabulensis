### `core/src/workbook.rs` (invalidate cached signatures on mutation)

```rust
pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut Cell> {
        self.cells.get_mut(&(row, col))
    }

    pub fn insert(&mut self, cell: Cell) {
        debug_assert!(
            cell.row < self.nrows && cell.col < self.ncols,
            "cell coordinates must lie within the grid bounds"
        );
        self.cells.insert((cell.row, cell.col), cell);
    }
```

```rust
pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut Cell> {
        self.row_signatures = None;
        self.col_signatures = None;
        self.cells.get_mut(&(row, col))
    }

    pub fn insert(&mut self, cell: Cell) {
        debug_assert!(
            cell.row < self.nrows && cell.col < self.ncols,
            "cell coordinates must lie within the grid bounds"
        );
        self.row_signatures = None;
        self.col_signatures = None;
        self.cells.insert((cell.row, cell.col), cell);
    }
```

---

### `core/src/workbook.rs` (make row/col signature computation O(axis) instead of O(all_cells))

```rust
pub fn compute_row_signature(&self, row: u32) -> RowSignature {
        use crate::hashing::hash_row_content_128;
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
        use crate::hashing::hash_col_content_128;
        let mut col_cells: Vec<_> = self.cells.values().filter(|cell| cell.col == col).collect();
        col_cells.sort_by_key(|cell| cell.row);

        let hash = hash_col_content_128(&col_cells);
        ColSignature { hash }
    }
```

```rust
pub fn compute_row_signature(&self, row: u32) -> RowSignature {
        use crate::hashing::hash_cell_value;
        use std::hash::Hash;
        use xxhash_rust::xxh3::Xxh3;

        let mut hasher = Xxh3::new();

        if (self.ncols as usize) <= self.cells.len() {
            for col in 0..self.ncols {
                if let Some(cell) = self.cells.get(&(row, col)) {
                    if cell.value.is_none() && cell.formula.is_none() {
                        continue;
                    }
                    hash_cell_value(&cell.value, &mut hasher);
                    cell.formula.hash(&mut hasher);
                }
            }
        } else {
            let mut row_cells: Vec<&Cell> = self.cells.values().filter(|c| c.row == row).collect();
            row_cells.sort_by_key(|c| c.col);
            for cell in row_cells {
                if cell.value.is_none() && cell.formula.is_none() {
                    continue;
                }
                hash_cell_value(&cell.value, &mut hasher);
                cell.formula.hash(&mut hasher);
            }
        }

        RowSignature {
            hash: hasher.digest128(),
        }
    }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature {
        use crate::hashing::hash_cell_value;
        use std::hash::Hash;
        use xxhash_rust::xxh3::Xxh3;

        let mut hasher = Xxh3::new();

        if (self.nrows as usize) <= self.cells.len() {
            for row in 0..self.nrows {
                if let Some(cell) = self.cells.get(&(row, col)) {
                    if cell.value.is_none() && cell.formula.is_none() {
                        continue;
                    }
                    hash_cell_value(&cell.value, &mut hasher);
                    cell.formula.hash(&mut hasher);
                }
            }
        } else {
            let mut col_cells: Vec<&Cell> = self.cells.values().filter(|c| c.col == col).collect();
            col_cells.sort_by_key(|c| c.row);
            for cell in col_cells {
                if cell.value.is_none() && cell.formula.is_none() {
                    continue;
                }
                hash_cell_value(&cell.value, &mut hasher);
                cell.formula.hash(&mut hasher);
            }
        }

        ColSignature {
            hash: hasher.digest128(),
        }
    }
```

---

### `core/src/engine.rs` (avoid HashMap-count multiset compare for row signatures)

```rust
fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

fn row_signature_counts(grid: &Grid) -> HashMap<RowSignature, u32> {
    let mut counts = HashMap::new();
    for row in 0..grid.nrows {
        if let Some(sig) = row_signature_at(grid, row) {
            *counts.entry(sig).or_insert(0) += 1;
        }
    }
    counts
}

fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }
    row_signature_counts(a) == row_signature_counts(b)
}
```

```rust
fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }

    let mut a_sigs: Vec<RowSignature> = (0..a.nrows)
        .filter_map(|row| row_signature_at(a, row))
        .collect();
    let mut b_sigs: Vec<RowSignature> = (0..b.nrows)
        .filter_map(|row| row_signature_at(b, row))
        .collect();

    a_sigs.sort_unstable_by_key(|s| s.hash);
    b_sigs.sort_unstable_by_key(|s| s.hash);

    a_sigs == b_sigs
}
```

---

### `core/src/engine.rs` (add fast identical-grid check helper)

```rust
fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
        (Some(cell_a), None) => cell_a.value.is_none() && cell_a.formula.is_none(),
        (None, Some(cell_b)) => cell_b.value.is_none() && cell_b.formula.is_none(),
    }
}
```

```rust
fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
        (Some(cell_a), None) => cell_a.value.is_none() && cell_a.formula.is_none(),
        (None, Some(cell_b)) => cell_b.value.is_none() && cell_b.formula.is_none(),
    }
}

fn grids_non_blank_cells_equal(old: &Grid, new: &Grid) -> bool {
    if old.cells.len() != new.cells.len() {
        return false;
    }

    for (coord, cell_a) in old.cells.iter() {
        let Some(cell_b) = new.cells.get(coord) else {
            return false;
        };
        if cell_a.value != cell_b.value || cell_a.formula != cell_b.formula {
            return false;
        }
    }

    true
}
```

---

### `core/src/engine.rs` (skip move detection + alignment entirely for identical grids)

```rust
fn diff_grids_core(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
    _ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) {
    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
    let move_detection_enabled = old.nrows.max(new.nrows) <= config.recursive_align_threshold
        && old.ncols.max(new.ncols) <= 256;
    let mut iteration = 0;
```

```rust
fn diff_grids_core(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
    _ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) {
    if old.nrows == new.nrows && old.ncols == new.ncols && grids_non_blank_cells_equal(old, new) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
        }
        return;
    }

    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
    let move_detection_enabled = old.nrows.max(new.nrows) <= config.recursive_align_threshold
        && old.ncols.max(new.ncols) <= 256;
    let mut iteration = 0;
```
