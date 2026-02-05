I dug through the reports and the hot paths in the parser. Two things stand out:

1. **The “regressions” are almost entirely in `parse_time_ms`, and your `cycle_delta.md` shows very large run‑to‑run swings even with the same commit.** That strongly suggests the e2e parse metric is being influenced by **I/O and zip reads** (and/or CPU frequency / thermal state), not just pure parsing CPU.

2. Independently of the noise, there are a couple of **low-risk changes** that reduce work in the parse path without touching the diff logic that produced the big wins (e.g., `perf_50k_alignment_block_move`).

Below are fixes in the order I’d do them.

---

## Fix 1 (benchmark stability): take disk I/O out of the timed “parse” metric

Right now, the e2e harness opens fixtures as a `File` and hands it to `WorkbookPackage::open_with_limits(...)`. That means `parse_time_ms` includes whatever the OS decides to do for file I/O and seeks inside the zip container.

If you want e2e parse numbers to be stable and reflect *parsing/decompression* rather than disk behavior, read the file into memory first and pass a `Cursor<Vec<u8>>`.

### `core/tests/e2e_perf_workbook_open.rs`

**Replace this code:**

```rust
fn open_fixture_with_size(name: &str) -> (WorkbookPackage, u64) {
    let path = fixture_path(name);
    let bytes = std::fs::metadata(&path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });
    let limits = ContainerLimits {
        max_entries: 10_000,
        max_part_uncompressed_bytes: 512 * 1024 * 1024,
        max_total_uncompressed_bytes: 1024 * 1024 * 1024,
    };
    let pkg = WorkbookPackage::open_with_limits(file, limits).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    });
    (pkg, bytes)
}
```

**With this code:**

```rust
fn open_fixture_with_size(name: &str) -> (WorkbookPackage, u64) {
    let path = fixture_path(name);
    let bytes = std::fs::metadata(&path)
        .map(|meta| meta.len())
        .unwrap_or(0);

    let mut file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });

    let mut data = Vec::with_capacity(bytes.min(usize::MAX as u64) as usize);
    use std::io::Read;
    file.read_to_end(&mut data).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", path.display());
    });

    let limits = ContainerLimits {
        max_entries: 10_000,
        max_part_uncompressed_bytes: 512 * 1024 * 1024,
        max_total_uncompressed_bytes: 1024 * 1024 * 1024,
    };

    let cursor = std::io::Cursor::new(data);
    let pkg = WorkbookPackage::open_with_limits(cursor, limits).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    });

    (pkg, bytes)
}
```

**Why this helps:** it removes disk variability from `parse_time_ms` and `total_time_ms` in the emitted PERF_METRIC line, which should make “regressions” stop flapping. (You’ll likely need to refresh e2e baselines after doing this because times will drop.)

---

## Fix 2 (real parse speed): stop paying “mutation invalidation” costs during sheet construction

While parsing XML, you build a brand-new `Grid` and then insert millions of cells. Going through `Grid::insert_cell` for that path adds extra work that doesn’t buy you anything during initial construction.

Specifically:

* `insert_cell` clears signature caches every call
* `insert_cell` calls `maybe_upgrade_to_dense()` every call

But in `build_grid(...)` you *already* chose dense vs sparse up front. And a new grid has no cached signatures to invalidate.

### `core/src/grid_parser.rs`

**Replace this code:**

```rust
fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, GridParseError> {
    let filled = cells.len();
    let mut grid = if Grid::should_use_dense(nrows, ncols, filled) {
        Grid::new_dense(nrows, ncols)
    } else {
        Grid::new(nrows, ncols)
    };

    for parsed in cells {
        grid.insert_cell(parsed.row, parsed.col, parsed.value, parsed.formula);
    }

    Ok(grid)
}
```

**With this code:**

```rust
fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, GridParseError> {
    let filled = cells.len();
    let mut grid = if Grid::should_use_dense(nrows, ncols, filled) {
        Grid::new_dense(nrows, ncols)
    } else {
        Grid::new(nrows, ncols)
    };

    for parsed in cells {
        if parsed.value.is_none() && parsed.formula.is_none() {
            continue;
        }

        debug_assert!(parsed.row < nrows && parsed.col < ncols);

        grid.cells.insert(
            parsed.row,
            parsed.col,
            crate::workbook::CellContent {
                value: parsed.value,
                formula: parsed.formula,
            },
        );
    }

    Ok(grid)
}
```

**Why this helps:** it removes per-cell overhead in the hottest parse loop without touching alignment/diff logic at all.

---

## Fix 3 (real parse speed): make signature invalidation conditional

Even outside the parser, clearing caches unconditionally on `get_mut`/`insert_cell` is unnecessary work in the common case where signatures were never computed.

This doesn’t change correctness and is low risk.

### `core/src/workbook.rs`

**Replace this code:**

```rust
pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
    self.row_signatures = None;
    self.col_signatures = None;
    self.cells.get_mut(row, col)
}

pub fn insert_cell(
    &mut self,
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
) {
    debug_assert!(
        row < self.nrows && col < self.ncols,
        "insert_cell out of bounds: ({row},{col}) for grid {}x{}",
        self.nrows,
        self.ncols
    );

    self.row_signatures = None;
    self.col_signatures = None;

    self.cells.insert(
        row,
        col,
        CellContent {
            value,
            formula,
        },
    );
    self.maybe_upgrade_to_dense();
}
```

**With this code:**

```rust
pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
    let out = self.cells.get_mut(row, col);
    if out.is_some() && (self.row_signatures.is_some() || self.col_signatures.is_some()) {
        self.row_signatures = None;
        self.col_signatures = None;
    }
    out
}

pub fn insert_cell(
    &mut self,
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
) {
    debug_assert!(
        row < self.nrows && col < self.ncols,
        "insert_cell out of bounds: ({row},{col}) for grid {}x{}",
        self.nrows,
        self.ncols
    );

    if self.row_signatures.is_some() || self.col_signatures.is_some() {
        self.row_signatures = None;
        self.col_signatures = None;
    }

    self.cells.insert(
        row,
        col,
        CellContent {
            value,
            formula,
        },
    );
    self.maybe_upgrade_to_dense();
}
```

---

## Fix 4 (real parse speed, higher impact than it looks): optimize A1 address parsing

Every `<c r="A1">` hits `address_to_index`. Your current implementation iterates `chars()`, which does UTF‑8 decoding overhead you don’t need (cell addresses are ASCII).

This is pure parsing CPU and will show up most in numeric-heavy fixtures like `e2e_p2_noise`.

### `core/src/addressing.rs`

**Replace this code:**

```rust
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    if a1.is_empty() {
        return None;
    }

    let mut col: u32 = 0;
    let mut row: u32 = 0;
    let mut saw_letter = false;
    let mut saw_digit = false;

    for ch in a1.chars() {
        if ch.is_ascii_alphabetic() {
            saw_letter = true;
            if saw_digit {
                // Letters after digits are not allowed.
                return None;
            }
            let upper = ch.to_ascii_uppercase() as u8;
            if !upper.is_ascii_uppercase() {
                return None;
            }
            col = col
                .checked_mul(26)?
                .checked_add((upper - b'A' + 1) as u32)?;
        } else if ch.is_ascii_digit() {
            saw_digit = true;
            row = row.checked_mul(10)?.checked_add((ch as u8 - b'0') as u32)?;
        } else {
            return None;
        }
    }

    if !saw_letter || !saw_digit || row == 0 || col == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}
```

**With this code:**

```rust
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    let bytes = a1.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut i: usize = 0;
    let mut col: u32 = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_alphabetic() {
            break;
        }
        let upper = b.to_ascii_uppercase();
        col = col
            .checked_mul(26)?
            .checked_add((upper - b'A' + 1) as u32)?;
        i += 1;
    }

    if i == 0 || i >= bytes.len() || col == 0 {
        return None;
    }

    let mut row: u32 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            return None;
        }
        row = row.checked_mul(10)?.checked_add((b - b'0') as u32)?;
        i += 1;
    }

    if row == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}
```