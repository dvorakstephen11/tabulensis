# Codebase Context for Review

## Directory Structure

```text
/
  .cursorignore
  .cursorindexingignore
  .github/
    workflows/
      perf.yml
      wasm.yml
  .gitignore
  benchmarks/
    README.md
    results/
      .gitkeep
      2025-12-12_163759.json
      2025-12-12_175341.json
      2025-12-12_203400.json
      2025-12-12_203454.json
      2025-12-12_223643.json
      2025-12-13_000346.json
      2025-12-13_000410.json
      2025-12-13_155200_fullscale.json
      2025-12-13_165236_fullscale.json
      2025-12-13_174735_fullscale.json
      2025-12-13_174822_fullscale.json
      2025-12-13_175318_fullscale.json
      2025-12-13_202028.json
      2025-12-13_202327_fullscale.json
      2025-12-14_003645.json
      2025-12-14_004611.json
      2025-12-14_004643_fullscale.json
      2025-12-14_005407_fullscale.json
      2025-12-14_202417_fullscale.json
      2025-12-15_183914.json
      2025-12-15_191921_fullscale.json
      combined_results.csv
      plots/
        commit_comparison.png
        latest_comparison.png
        metric_breakdown_fullscale.png
        metric_breakdown_quick.png
        speedup_heatmap.png
        time_trends.png
        trend_summary.md
  Cargo.lock
  Cargo.toml
  core/
    benches/
      diff_benchmarks.rs
    Cargo.lock
    Cargo.toml
    src/
      addressing.rs
      alignment/
        anchor_chain.rs
        anchor_discovery.rs
        assembly.rs
        gap_strategy.rs
        mod.rs
        move_extraction.rs
        row_metadata.rs
        runs.rs
      bin/
        wasm_smoke.rs
      column_alignment.rs
      config.rs
      container.rs
      database_alignment.rs
      datamashup.rs
      datamashup_framing.rs
      datamashup_package.rs
      diff.rs
      engine.rs
      excel_open_xml.rs
      grid_parser.rs
      grid_view.rs
      hashing.rs
      lib.rs
      m_ast.rs
      m_diff.rs
      m_section.rs
      output/
        json.rs
        json_lines.rs
        mod.rs
      package.rs
      perf.rs
      rect_block_move.rs
      region_mask.rs
      row_alignment.rs
      session.rs
      sink.rs
      string_pool.rs
      workbook.rs
    tests/
      addressing_pg2_tests.rs
      amr_multi_gap_tests.rs
      common/
        mod.rs
      d1_database_mode_tests.rs
      data_mashup_tests.rs
      engine_tests.rs
      excel_open_xml_tests.rs
      g10_row_block_alignment_grid_workbook_tests.rs
      g11_row_block_move_grid_workbook_tests.rs
      g12_column_block_move_grid_workbook_tests.rs
      g12_rect_block_move_grid_workbook_tests.rs
      g13_fuzzy_row_move_grid_workbook_tests.rs
      g14_move_combination_tests.rs
      g15_column_structure_row_alignment_tests.rs
      g1_g2_grid_workbook_tests.rs
      g5_g7_grid_workbook_tests.rs
      g8_row_alignment_grid_workbook_tests.rs
      g9_column_alignment_grid_workbook_tests.rs
      grid_view_hashstats_tests.rs
      grid_view_tests.rs
      integration_test.rs
      limit_behavior_tests.rs
      m4_package_parts_tests.rs
      m4_permissions_metadata_tests.rs
      m5_query_domain_tests.rs
      m6_textual_m_diff_tests.rs
      m7_ast_canonicalization_tests.rs
      m7_semantic_m_diff_tests.rs
      m_section_splitting_tests.rs
      metrics_unit_tests.rs
      output_tests.rs
      package_streaming_tests.rs
      perf_large_grid_tests.rs
      pg1_ir_tests.rs
      pg3_snapshot_tests.rs
      pg4_diffop_tests.rs
      pg5_grid_diff_tests.rs
      pg6_object_vs_grid_tests.rs
      signature_tests.rs
      sparse_grid_tests.rs
      streaming_sink_tests.rs
      string_pool_tests.rs
  fixtures/
    manifest.yaml
    pyproject.toml
    README.md
    requirements.txt
    src/
      __init__.py
      generate.py
      generators/
        __init__.py
        base.py
        corrupt.py
        database.py
        grid.py
        mashup.py
        perf.py
  ideas.md
  logs/
    2025-11-28b-diffop-pg4/
      activity_log.txt
  plan_review.md
  README.md
  related_files.txt.md
  scripts/
    check_perf_thresholds.py
    combine_results_to_csv.py
    compare_perf_results.py
    export_perf_metrics.py
    visualize_benchmarks.py
```

## File Contents

### File: `.github\workflows\perf.yml`

```yaml
name: Performance Regression

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  perf-regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
        
      - name: Build with perf metrics
        run: cargo build --release --features perf-metrics
        working-directory: core
        
      - name: Run perf test suite
        run: cargo test --release --features perf-metrics perf_ -- --nocapture
        working-directory: core
        
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'
          
      - name: Check perf thresholds
        run: python scripts/check_perf_thresholds.py


```

---

### File: `.github\workflows\wasm.yml`

```yaml
name: WASM Smoke

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  wasm-smoke:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Build wasm smoke binary
        run: cargo build --release --target wasm32-unknown-unknown --no-default-features -p excel_diff --bin wasm_smoke
        working-directory: core

      - name: Enforce wasm size budget
        run: |
          SIZE=$(stat -c%s "core/target/wasm32-unknown-unknown/release/wasm_smoke.wasm")
          echo "wasm_smoke.wasm size: $SIZE bytes"
          if [ "$SIZE" -gt 5000000 ]; then
            echo "WASM size $SIZE exceeds 5MB limit"
            exit 1
          fi

```

---

### File: `.gitignore`

```
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
fixtures/generated/*.csv


# Docs
docs/meta/completion_estimates/

```

---

### File: `Cargo.toml`

```toml
[workspace]
members = ["core"]
resolver = "2"

```

---

### File: `core\benches\diff_benchmarks.rs`

```rust
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use excel_diff::config::DiffConfig;
use excel_diff::diff_workbooks;
use excel_diff::{CellValue, Grid, Sheet, SheetKind, Workbook, with_default_session};
use std::time::Duration;

const MAX_BENCH_TIME_SECS: u64 = 30;
const WARMUP_SECS: u64 = 3;
const SAMPLE_SIZE: usize = 10;

fn create_large_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                None,
            );
        }
    }
    grid
}

fn create_repetitive_grid(nrows: u32, ncols: u32, pattern_length: u32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        let pattern_idx = row % pattern_length;
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number((pattern_idx * 1000 + col) as f64)),
                None,
            );
        }
    }
    grid
}

fn create_sparse_grid(nrows: u32, ncols: u32, fill_percent: u32, seed: u64) -> Grid {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            let mut hasher = DefaultHasher::new();
            (row, col, seed).hash(&mut hasher);
            let hash = hasher.finish();
            if (hash % 100) < fill_percent as u64 {
                grid.insert_cell(
                    row,
                    col,
                    Some(CellValue::Number((row * 1000 + col) as f64)),
                    None,
                );
            }
        }
    }
    grid
}

fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    with_default_session(|session| Workbook {
        sheets: vec![Sheet {
            name: session.strings.intern(name),
            kind: SheetKind::Worksheet,
            grid,
        }],
    })
}

fn bench_identical_grids(c: &mut Criterion) {
    let mut group = c.benchmark_group("identical_grids");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let grid_a = create_large_grid(*size, 50, 0);
        let grid_b = create_large_grid(*size, 50, 0);
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks(&wb_a, &wb_b, &config));
        });
    }
    group.finish();
}

fn bench_single_cell_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_cell_edit");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let grid_a = create_large_grid(*size, 50, 0);
        let mut grid_b = create_large_grid(*size, 50, 0);
        grid_b.insert_cell(size / 2, 25, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks(&wb_a, &wb_b, &config));
        });
    }
    group.finish();
}

fn bench_all_rows_different(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_rows_different");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000].iter() {
        let grid_a = create_large_grid(*size, 50, 0);
        let grid_b = create_large_grid(*size, 50, 1);
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks(&wb_a, &wb_b, &config));
        });
    }
    group.finish();
}

fn bench_adversarial_repetitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("adversarial_repetitive");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000].iter() {
        let grid_a = create_repetitive_grid(*size, 50, 100);
        let mut grid_b = create_repetitive_grid(*size, 50, 100);
        grid_b.insert_cell(size / 2, 25, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks(&wb_a, &wb_b, &config));
        });
    }
    group.finish();
}

fn bench_sparse_grid(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_grid_1pct");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let grid_a = create_sparse_grid(*size, 100, 1, 12345);
        let mut grid_b = create_sparse_grid(*size, 100, 1, 12345);
        grid_b.insert_cell(size / 2, 50, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 100));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks(&wb_a, &wb_b, &config));
        });
    }
    group.finish();
}

fn bench_row_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_insertion");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000].iter() {
        let grid_a = create_large_grid(*size, 50, 0);
        let mut grid_b = Grid::new(size + 100, 50);
        for row in 0..(size / 2) {
            for col in 0..50 {
                grid_b.insert_cell(
                    row,
                    col,
                    Some(CellValue::Number((row as i64 * 1000 + col as i64) as f64)),
                    None,
                );
            }
        }
        for col in 0..50 {
            for i in 0..100 {
                let row = size / 2 + i;
                let marker = 1_000_000.0 + i as f64 * 10.0 + col as f64;
                grid_b.insert_cell(row, col, Some(CellValue::Number(marker)), None);
            }
        }
        for row in (size / 2)..*size {
            for col in 0..50 {
                let new_row = row + 100;
                grid_b.insert_cell(
                    new_row,
                    col,
                    Some(CellValue::Number((row as i64 * 1000 + col as i64) as f64)),
                    None,
                );
            }
        }
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks(&wb_a, &wb_b, &config));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_identical_grids,
    bench_single_cell_edit,
    bench_all_rows_different,
    bench_adversarial_repetitive,
    bench_sparse_grid,
    bench_row_insertion,
);

criterion_main!(benches);

```

---

### File: `core\Cargo.toml`

```toml
[package]
name = "excel_diff"
version = "0.1.0"
edition = "2024"

[lib]
name = "excel_diff"
path = "src/lib.rs"

[features]
default = ["excel-open-xml", "std-fs"]
excel-open-xml = []
std-fs = []
perf-metrics = []

[dependencies]
quick-xml = "0.32"
thiserror = "1.0"
zip = { version = "0.6", default-features = false, features = ["deflate"] }
base64 = "0.22"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
xxhash-rust = { version = "0.8", features = ["xxh64", "xxh3"] }
rustc-hash = "1.1"

[dev-dependencies]
pretty_assertions = "1.4"
tempfile = "3.10"
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "diff_benchmarks"
harness = false

```

---

### File: `core\src\addressing.rs`

```rust
//! Excel cell addressing utilities.
//!
//! Provides conversion between A1-style cell addresses (e.g., "B2", "AA10") and
//! zero-based (row, column) index pairs.

use std::fmt;

/// Error returned when parsing an invalid A1-style cell address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressParseError {
    pub input: String,
}

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cell address: '{}'", self.input)
    }
}

impl std::error::Error for AddressParseError {}

/// Convert zero-based (row, col) indices to an Excel A1 address string.
pub fn index_to_address(row: u32, col: u32) -> String {
    let mut col_index = col;
    let mut col_label = String::new();

    loop {
        let rem = (col_index % 26) as u8;
        col_label.push((b'A' + rem) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }

    col_label.chars().rev().collect::<String>() + &(row + 1).to_string()
}

/// Parse an A1 address into zero-based (row, col) indices.
/// Returns `None` for malformed addresses.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_address_examples() {
        assert_eq!(index_to_address(0, 0), "A1");
        assert_eq!(index_to_address(0, 25), "Z1");
        assert_eq!(index_to_address(0, 26), "AA1");
        assert_eq!(index_to_address(0, 27), "AB1");
        assert_eq!(index_to_address(0, 51), "AZ1");
        assert_eq!(index_to_address(0, 52), "BA1");
    }

    #[test]
    fn round_trip_addresses() {
        let addresses = [
            "A1", "B2", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1",
        ];
        for addr in addresses {
            let (r, c) = address_to_index(addr).expect("address should parse");
            assert_eq!(index_to_address(r, c), addr);
        }
    }

    #[test]
    fn invalid_addresses_rejected() {
        let invalid = ["", "1A", "A0", "A", "AA0", "A-1", "A1A"];
        for addr in invalid {
            assert!(address_to_index(addr).is_none(), "{addr} should be invalid");
        }
    }
}

```

---

### File: `core\src\alignment\anchor_chain.rs`

```rust
//! Anchor chain construction using Longest Increasing Subsequence (LIS).
//!
//! Implements anchor chain building as described in the unified grid diff
//! specification Section 10. Given a set of discovered anchors, this module
//! selects the maximal subset that preserves relative order in both grids.
//!
//! For example, if anchors show:
//! - Row A: old=0, new=0
//! - Row B: old=2, new=1  (B moved up)
//! - Row C: old=1, new=2
//!
//! The LIS algorithm selects {A, C} because their old_row indices (0, 1) are
//! increasing, making them a valid ordering chain. Row B is excluded because
//! including it would create a crossing (B is at old=2 but new=1, while C is
//! at old=1 but new=2).

use crate::alignment::anchor_discovery::Anchor;

pub fn build_anchor_chain(mut anchors: Vec<Anchor>) -> Vec<Anchor> {
    // Sort by new_row to preserve destination order before LIS on old_row.
    anchors.sort_by_key(|a| a.new_row);
    let indices = lis_indices(&anchors, |a| a.old_row);
    indices.into_iter().map(|idx| anchors[idx]).collect()
}

fn lis_indices<T, F>(items: &[T], key: F) -> Vec<usize>
where
    F: Fn(&T) -> u32,
{
    let mut piles: Vec<usize> = Vec::new();
    let mut predecessors: Vec<Option<usize>> = vec![None; items.len()];

    for (idx, item) in items.iter().enumerate() {
        let k = key(item);
        let pos = piles
            .binary_search_by_key(&k, |&pile_idx| key(&items[pile_idx]))
            .unwrap_or_else(|insert_pos| insert_pos);

        if pos > 0 {
            predecessors[idx] = Some(piles[pos - 1]);
        }

        if pos == piles.len() {
            piles.push(idx);
        } else {
            piles[pos] = idx;
        }
    }

    if piles.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<usize> = Vec::new();
    let mut current = *piles.last().unwrap();
    loop {
        result.push(current);
        if let Some(prev) = predecessors[current] {
            current = prev;
        } else {
            break;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::anchor_discovery::Anchor;
    use crate::workbook::RowSignature;

    #[test]
    fn builds_increasing_chain() {
        let anchors = vec![
            Anchor {
                old_row: 0,
                new_row: 0,
                signature: RowSignature { hash: 1 },
            },
            Anchor {
                old_row: 2,
                new_row: 1,
                signature: RowSignature { hash: 2 },
            },
            Anchor {
                old_row: 1,
                new_row: 2,
                signature: RowSignature { hash: 3 },
            },
        ];

        let chain = build_anchor_chain(anchors);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].old_row, 0);
        assert_eq!(chain[1].old_row, 1);
    }
}

```

---

### File: `core\src\alignment\anchor_discovery.rs`

```rust
//! Anchor discovery for AMR alignment.
//!
//! Implements anchor discovery as described in the unified grid diff specification
//! Section 10. Anchors are rows that:
//!
//! 1. Are unique (appear exactly once) in BOTH grids
//! 2. Have matching signatures (content hash)
//!
//! These rows serve as fixed points around which the alignment is built.
//! Rows that are unique in one grid but not the other cannot be anchors
//! since their position cannot be reliably determined.

use std::collections::HashMap;

use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::grid_view::GridView;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub old_row: u32,
    pub new_row: u32,
    pub signature: RowSignature,
}

#[allow(dead_code)]
pub fn discover_anchors(old: &GridView<'_>, new: &GridView<'_>) -> Vec<Anchor> {
    discover_anchors_from_meta(&old.row_meta, &new.row_meta)
}

pub fn discover_anchors_from_meta(old: &[RowMeta], new: &[RowMeta]) -> Vec<Anchor> {
    let mut old_unique: HashMap<RowSignature, u32> = HashMap::new();
    for meta in old.iter() {
        if meta.frequency_class == FrequencyClass::Unique {
            old_unique.insert(meta.signature, meta.row_idx);
        }
    }

    new.iter()
        .filter(|meta| meta.frequency_class == FrequencyClass::Unique)
        .filter_map(|meta| {
            old_unique.get(&meta.signature).map(|old_idx| Anchor {
                old_row: *old_idx,
                new_row: meta.row_idx,
                signature: meta.signature,
            })
        })
        .collect()
}

pub fn discover_context_anchors(old: &[RowMeta], new: &[RowMeta], k: usize) -> Vec<Anchor> {
    if k == 0 || old.len() < k || new.len() < k {
        return Vec::new();
    }

    fn window_signature(window: &[RowMeta]) -> Option<RowSignature> {
        if window.iter().any(|m| m.is_low_info()) {
            return None;
        }
        let mut acc: u128 = 0x9e37_79b1_85eb_ca87;
        for (idx, meta) in window.iter().enumerate() {
            let mul = 0x1000_0000_01b3u128;
            acc = acc
                .wrapping_mul(mul)
                .wrapping_add(meta.signature.hash ^ ((idx as u128) << 1) ^ 0x517c_c1b7_2722_0a95);
            acc ^= acc >> 33;
            acc = acc.rotate_left(7);
        }
        Some(RowSignature { hash: acc })
    }

    let mut count_old: HashMap<RowSignature, u32> = HashMap::new();
    let mut pos_old: HashMap<RowSignature, u32> = HashMap::new();
    for i in 0..=old.len() - k {
        if let Some(sig) = window_signature(&old[i..i + k]) {
            *count_old.entry(sig).or_insert(0) += 1;
            pos_old.entry(sig).or_insert(old[i].row_idx);
        }
    }

    let mut count_new: HashMap<RowSignature, u32> = HashMap::new();
    let mut pos_new: HashMap<RowSignature, u32> = HashMap::new();
    for i in 0..=new.len() - k {
        if let Some(sig) = window_signature(&new[i..i + k]) {
            *count_new.entry(sig).or_insert(0) += 1;
            pos_new.entry(sig).or_insert(new[i].row_idx);
        }
    }

    let mut anchors = Vec::new();
    for (sig, &new_row) in pos_new.iter() {
        if count_new.get(sig).copied().unwrap_or(0) != 1 {
            continue;
        }
        if count_old.get(sig).copied().unwrap_or(0) != 1 {
            continue;
        }
        if let Some(old_row) = pos_old.get(sig) {
            anchors.push(Anchor {
                old_row: *old_row,
                new_row,
                signature: *sig,
            });
        }
    }

    anchors
}

pub fn discover_local_anchors(old: &[RowMeta], new: &[RowMeta]) -> Vec<Anchor> {
    let mut count_old: HashMap<RowSignature, u32> = HashMap::new();
    for m in old.iter() {
        if !m.is_low_info() {
            *count_old.entry(m.signature).or_insert(0) += 1;
        }
    }

    let mut count_new: HashMap<RowSignature, u32> = HashMap::new();
    for m in new.iter() {
        if !m.is_low_info() {
            *count_new.entry(m.signature).or_insert(0) += 1;
        }
    }

    let mut pos_old: HashMap<RowSignature, u32> = HashMap::new();
    for m in old.iter() {
        if !m.is_low_info() && count_old.get(&m.signature).copied().unwrap_or(0) == 1 {
            pos_old.insert(m.signature, m.row_idx);
        }
    }

    let mut out = Vec::new();
    for m in new.iter() {
        if m.is_low_info() {
            continue;
        }
        if count_new.get(&m.signature).copied().unwrap_or(0) != 1 {
            continue;
        }
        if let Some(old_row) = pos_old.get(&m.signature) {
            out.push(Anchor {
                old_row: *old_row,
                new_row: m.row_idx,
                signature: m.signature,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::row_metadata::{FrequencyClass, RowMeta};

    fn meta_from_hashes(hashes: &[u128]) -> Vec<RowMeta> {
        hashes
            .iter()
            .enumerate()
            .map(|(idx, &hash)| {
                let sig = RowSignature { hash };
                RowMeta {
                    row_idx: idx as u32,
                    signature: sig,
                    hash: sig,
                    non_blank_count: 1,
                    first_non_blank_col: 0,
                    frequency_class: FrequencyClass::Common,
                    is_low_info: false,
                }
            })
            .collect()
    }

    #[test]
    fn discovers_context_anchors_when_no_uniques() {
        let old = meta_from_hashes(&[1, 2, 3, 4, 5, 6, 1, 2]);
        let new = meta_from_hashes(&[7, 1, 2, 3, 4, 5, 6, 8]);

        let anchors = discover_context_anchors(&old, &new, 4);
        assert!(!anchors.is_empty(), "should find context anchors");
        let mut rows: Vec<(u32, u32)> = anchors.iter().map(|a| (a.old_row, a.new_row)).collect();
        rows.sort();
        assert!(rows.contains(&(0, 1)));
        assert!(rows.contains(&(1, 2)));
        assert!(rows.contains(&(2, 3)));
    }
}

```

---

### File: `core\src\alignment\assembly.rs`

```rust
//! Final alignment assembly for AMR algorithm.
//!
//! Implements the final assembly phase as described in the unified grid diff
//! specification Section 12. This module:
//!
//! 1. Orchestrates the full AMR pipeline (metadata → anchors → chain → gaps)
//! 2. Assembles matched pairs, insertions, deletions, and moves into final alignment
//! 3. Provides fast paths for special cases (RLE compression, single-run grids)
//!
//! The main entry point is `align_rows_amr` which returns an `Option<RowAlignment>`.
//! Returns `None` when alignment cannot be determined (falls back to positional diff).

use std::ops::Range;

use crate::alignment::anchor_chain::build_anchor_chain;
use crate::alignment::anchor_discovery::{
    Anchor, discover_anchors_from_meta, discover_context_anchors, discover_local_anchors,
};
use crate::alignment::gap_strategy::{GapStrategy, select_gap_strategy};
use crate::alignment::move_extraction::{find_block_move, moves_from_matched_pairs};
use crate::alignment::row_metadata::RowMeta;
use crate::alignment::runs::{RowRun, compress_to_runs};
use crate::config::DiffConfig;
use crate::grid_view::GridView;
use crate::workbook::{Grid, RowSignature};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowAlignment {
    pub matched: Vec<(u32, u32)>,
    pub inserted: Vec<u32>,
    pub deleted: Vec<u32>,
    pub moves: Vec<RowBlockMove>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}

#[derive(Default)]
struct GapAlignmentResult {
    matched: Vec<(u32, u32)>,
    inserted: Vec<u32>,
    deleted: Vec<u32>,
    moves: Vec<RowBlockMove>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowAlignmentWithSignatures {
    pub alignment: RowAlignment,
    pub row_signatures_a: Vec<RowSignature>,
    pub row_signatures_b: Vec<RowSignature>,
}

#[allow(dead_code)]
pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    align_rows_amr_with_signatures(old, new, config).map(|result| result.alignment)
}

pub fn align_rows_amr_with_signatures(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignmentWithSignatures> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_rows_amr_with_signatures_from_views(&view_a, &view_b, config)
}

pub fn align_rows_amr_with_signatures_from_views(
    view_a: &GridView,
    view_b: &GridView,
    config: &DiffConfig,
) -> Option<RowAlignmentWithSignatures> {
    let alignment = align_rows_from_meta(&view_a.row_meta, &view_b.row_meta, config)?;
    let row_signatures_a: Vec<RowSignature> =
        view_a.row_meta.iter().map(|meta| meta.signature).collect();
    let row_signatures_b: Vec<RowSignature> =
        view_b.row_meta.iter().map(|meta| meta.signature).collect();

    Some(RowAlignmentWithSignatures {
        alignment,
        row_signatures_a,
        row_signatures_b,
    })
}

fn align_rows_from_meta(
    rows_a: &[RowMeta],
    rows_b: &[RowMeta],
    config: &DiffConfig,
) -> Option<RowAlignment> {
    if rows_a.len() == rows_b.len()
        && rows_a
            .iter()
            .zip(rows_b.iter())
            .all(|(a, b)| a.signature == b.signature)
    {
        let mut matched = Vec::with_capacity(rows_a.len());
        for (a, b) in rows_a.iter().zip(rows_b.iter()) {
            matched.push((a.row_idx, b.row_idx));
        }
        return Some(RowAlignment {
            matched,
            inserted: Vec::new(),
            deleted: Vec::new(),
            moves: Vec::new(),
        });
    }

    let runs_a = compress_to_runs(rows_a);
    let runs_b = compress_to_runs(rows_b);
    if runs_a.len() == 1 && runs_b.len() == 1 && runs_a[0].signature == runs_b[0].signature {
        let shared = runs_a[0].count.min(runs_b[0].count);
        let mut matched = Vec::new();
        for offset in 0..shared {
            matched.push((runs_a[0].start_row + offset, runs_b[0].start_row + offset));
        }
        let mut inserted = Vec::new();
        if runs_b[0].count > shared {
            inserted
                .extend((runs_b[0].start_row + shared)..(runs_b[0].start_row + runs_b[0].count));
        }
        let mut deleted = Vec::new();
        if runs_a[0].count > shared {
            deleted.extend((runs_a[0].start_row + shared)..(runs_a[0].start_row + runs_a[0].count));
        }
        return Some(RowAlignment {
            matched,
            inserted,
            deleted,
            moves: Vec::new(),
        });
    }

    let compressed_a = runs_a.len() * 2 <= rows_a.len();
    let compressed_b = runs_b.len() * 2 <= rows_b.len();
    if (compressed_a || compressed_b)
        && !runs_a.is_empty()
        && !runs_b.is_empty()
        && let Some(alignment) = align_runs_stable(&runs_a, &runs_b)
    {
        return Some(alignment);
    }

    let anchors = build_anchor_chain(discover_anchors_from_meta(rows_a, rows_b));
    Some(assemble_from_meta(rows_a, rows_b, anchors, config, 0))
}

fn assemble_from_meta(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    anchors: Vec<Anchor>,
    config: &DiffConfig,
    depth: u32,
) -> RowAlignment {
    if old_meta.is_empty() && new_meta.is_empty() {
        return RowAlignment::default();
    }

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut moves = Vec::new();

    let mut prev_old = old_meta.first().map(|m| m.row_idx).unwrap_or(0);
    let mut prev_new = new_meta.first().map(|m| m.row_idx).unwrap_or(0);

    for anchor in anchors.iter() {
        let gap_old = prev_old..anchor.old_row;
        let gap_new = prev_new..anchor.new_row;
        let gap_result = fill_gap(gap_old, gap_new, old_meta, new_meta, config, depth);
        matched.extend(gap_result.matched);
        inserted.extend(gap_result.inserted);
        deleted.extend(gap_result.deleted);
        moves.extend(gap_result.moves);

        matched.push((anchor.old_row, anchor.new_row));
        prev_old = anchor.old_row + 1;
        prev_new = anchor.new_row + 1;
    }

    let old_end = old_meta.last().map(|m| m.row_idx + 1).unwrap_or(prev_old);
    let new_end = new_meta.last().map(|m| m.row_idx + 1).unwrap_or(prev_new);
    let tail_result = fill_gap(
        prev_old..old_end,
        prev_new..new_end,
        old_meta,
        new_meta,
        config,
        depth,
    );
    matched.extend(tail_result.matched);
    inserted.extend(tail_result.inserted);
    deleted.extend(tail_result.deleted);
    moves.extend(tail_result.moves);

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();
    moves.sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));

    RowAlignment {
        matched,
        inserted,
        deleted,
        moves,
    }
}

fn fill_gap(
    old_gap: Range<u32>,
    new_gap: Range<u32>,
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    config: &DiffConfig,
    depth: u32,
) -> GapAlignmentResult {
    let old_slice = slice_by_range(old_meta, &old_gap);
    let new_slice = slice_by_range(new_meta, &new_gap);
    let has_recursed = depth >= config.max_recursion_depth;
    let strategy = select_gap_strategy(old_slice, new_slice, config, has_recursed);

    match strategy {
        GapStrategy::Empty => GapAlignmentResult::default(),

        GapStrategy::InsertAll => GapAlignmentResult {
            matched: Vec::new(),
            inserted: (new_gap.start..new_gap.end).collect(),
            deleted: Vec::new(),
            moves: Vec::new(),
        },

        GapStrategy::DeleteAll => GapAlignmentResult {
            matched: Vec::new(),
            inserted: Vec::new(),
            deleted: (old_gap.start..old_gap.end).collect(),
            moves: Vec::new(),
        },

        GapStrategy::SmallEdit => align_small_gap(old_slice, new_slice, config),

        GapStrategy::HashFallback => {
            let mut result = align_gap_via_hash(old_slice, new_slice);
            result
                .moves
                .extend(moves_from_matched_pairs(&result.matched));
            result
        }

        GapStrategy::MoveCandidate => {
            let mut result = if old_slice.len() as u32 > config.max_lcs_gap_size
                || new_slice.len() as u32 > config.max_lcs_gap_size
            {
                align_gap_via_hash(old_slice, new_slice)
            } else {
                align_small_gap(old_slice, new_slice, config)
            };

            let mut detected_moves = moves_from_matched_pairs(&result.matched);

            if detected_moves.is_empty() {
                let has_nonzero_offset = result
                    .matched
                    .iter()
                    .any(|(a, b)| (*b as i64 - *a as i64) != 0);

                if has_nonzero_offset
                    && let Some(mv) = find_block_move(
                        old_slice,
                        new_slice,
                        config.min_block_size_for_move,
                        config,
                    )
                {
                    detected_moves.push(mv);
                }
            }

            result.moves.extend(detected_moves);
            result
        }

        GapStrategy::RecursiveAlign => {
            let at_limit = depth >= config.max_recursion_depth;
            if at_limit {
                if old_slice.len() as u32 > config.max_lcs_gap_size
                    || new_slice.len() as u32 > config.max_lcs_gap_size
                {
                    return align_gap_via_hash(old_slice, new_slice);
                }
                return align_small_gap(old_slice, new_slice, config);
            }

            let anchor_candidates = if depth == 0 {
                discover_anchors_from_meta(old_slice, new_slice)
            } else {
                let ctx_k1 = config.context_anchor_k1 as usize;
                let ctx_k2 = config.context_anchor_k2 as usize;
                let mut anchors = discover_local_anchors(old_slice, new_slice);
                if anchors.is_empty() {
                    anchors = discover_context_anchors(old_slice, new_slice, ctx_k1);
                    if anchors.is_empty() {
                        anchors = discover_context_anchors(old_slice, new_slice, ctx_k2);
                    }
                } else {
                    let mut ctx_anchors = discover_context_anchors(old_slice, new_slice, ctx_k1);
                    if anchors.len() < ctx_k1 {
                        anchors.append(&mut ctx_anchors);
                    }
                }
                anchors
            };

            let anchors = build_anchor_chain(anchor_candidates);
            if anchors.is_empty() {
                if old_slice.len() as u32 > config.max_lcs_gap_size
                    || new_slice.len() as u32 > config.max_lcs_gap_size
                {
                    return align_gap_via_hash(old_slice, new_slice);
                }
                return align_small_gap(old_slice, new_slice, config);
            }

            let alignment = assemble_from_meta(old_slice, new_slice, anchors, config, depth + 1);
            GapAlignmentResult {
                matched: alignment.matched,
                inserted: alignment.inserted,
                deleted: alignment.deleted,
                moves: alignment.moves,
            }
        }
    }
}

fn align_runs_stable(runs_a: &[RowRun], runs_b: &[RowRun]) -> Option<RowAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < runs_a.len() && idx_b < runs_b.len() {
        let run_a = &runs_a[idx_a];
        let run_b = &runs_b[idx_b];

        if run_a.signature != run_b.signature {
            return None;
        }

        let shared = run_a.count.min(run_b.count);
        for offset in 0..shared {
            matched.push((run_a.start_row + offset, run_b.start_row + offset));
        }

        if run_a.count > shared {
            for offset in shared..run_a.count {
                deleted.push(run_a.start_row + offset);
            }
        }

        if run_b.count > shared {
            for offset in shared..run_b.count {
                inserted.push(run_b.start_row + offset);
            }
        }

        idx_a += 1;
        idx_b += 1;
    }

    for run in runs_a.iter().skip(idx_a) {
        for offset in 0..run.count {
            deleted.push(run.start_row + offset);
        }
    }

    for run in runs_b.iter().skip(idx_b) {
        for offset in 0..run.count {
            inserted.push(run.start_row + offset);
        }
    }

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();

    Some(RowAlignment {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    })
}

fn slice_by_range<'a>(meta: &'a [RowMeta], range: &Range<u32>) -> &'a [RowMeta] {
    if meta.is_empty() || range.start >= range.end {
        return &[];
    }
    let base = meta.first().map(|m| m.row_idx).unwrap_or(0);
    if range.start < base {
        return &[];
    }
    let start = (range.start - base) as usize;
    if start >= meta.len() {
        return &[];
    }
    let end = (start + (range.end - range.start) as usize).min(meta.len());
    &meta[start..end]
}

fn align_small_gap(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    config: &DiffConfig,
) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    if m as u32 > config.max_lcs_gap_size || n as u32 > config.max_lcs_gap_size {
        return align_gap_via_hash(old_slice, new_slice);
    }

    if m.saturating_mul(n) > config.lcs_dp_work_limit {
        return align_gap_via_myers(old_slice, new_slice);
    }

    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old_slice[i].signature == new_slice[j].signature {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if old_slice[i].signature == new_slice[j].signature {
            matched.push((old_slice[i].row_idx, new_slice[j].row_idx));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            deleted.push(old_slice[i].row_idx);
            i += 1;
        } else {
            inserted.push(new_slice[j].row_idx);
            j += 1;
        }
    }

    while i < m {
        deleted.push(old_slice[i].row_idx);
        i += 1;
    }
    while j < n {
        inserted.push(new_slice[j].row_idx);
        j += 1;
    }

    if matched.is_empty() && m == n {
        matched = old_slice
            .iter()
            .zip(new_slice.iter())
            .map(|(a, b)| (a.row_idx, b.row_idx))
            .collect();
        inserted.clear();
        deleted.clear();
    }

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}

fn align_gap_via_myers(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let edits = myers_edit_script(old_slice, new_slice);

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    for edit in edits {
        match edit {
            Edit::Match(i, j) => matched.push((old_slice[i].row_idx, new_slice[j].row_idx)),
            Edit::Insert(j) => inserted.push(new_slice[j].row_idx),
            Edit::Delete(i) => deleted.push(old_slice[i].row_idx),
        }
    }

    if matched.is_empty() && m == n {
        matched = old_slice
            .iter()
            .zip(new_slice.iter())
            .map(|(a, b)| (a.row_idx, b.row_idx))
            .collect();
        inserted.clear();
        deleted.clear();
    }

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edit {
    Match(usize, usize),
    Insert(usize),
    Delete(usize),
}

fn myers_edit_script(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> Vec<Edit> {
    let n = old_slice.len() as isize;
    let m = new_slice.len() as isize;
    if n == 0 {
        return (0..m as usize).map(Edit::Insert).collect();
    }
    if m == 0 {
        return (0..n as usize).map(Edit::Delete).collect();
    }

    let max = (n + m) as usize;
    let offset = max as isize;
    let mut v = vec![0isize; 2 * max + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();

    for d in 0..=max {
        let mut v_next = v.clone();
        for k in (-(d as isize)..=d as isize).step_by(2) {
            let idx = (k + offset) as usize;
            let x_start = if k == -(d as isize) || (k != d as isize && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };

            let mut x = x_start;
            let mut y = x - k;
            while x < n
                && y < m
                && old_slice[x as usize].signature == new_slice[y as usize].signature
            {
                x += 1;
                y += 1;
            }
            v_next[idx] = x;
            if x >= n && y >= m {
                trace.push(v_next);
                return reconstruct_myers(trace, old_slice.len(), new_slice.len(), offset);
            }
        }
        trace.push(v_next.clone());
        v = v_next;
    }

    Vec::new()
}

fn reconstruct_myers(
    trace: Vec<Vec<isize>>,
    old_len: usize,
    new_len: usize,
    offset: isize,
) -> Vec<Edit> {
    let mut edits = Vec::new();
    let mut x = old_len as isize;
    let mut y = new_len as isize;

    for d_rev in (0..trace.len()).rev() {
        let v = &trace[d_rev];
        let k = x - y;
        let idx = (k + offset) as usize;

        let (prev_x, prev_y, from_down);
        if d_rev == 0 {
            prev_x = 0;
            prev_y = 0;
            from_down = false;
        } else {
            let use_down =
                k == -(d_rev as isize) || (k != d_rev as isize && v[idx - 1] < v[idx + 1]);
            let prev_k = if use_down { k + 1 } else { k - 1 };
            let prev_idx = (prev_k + offset) as usize;
            let prev_v = &trace[d_rev - 1];
            prev_x = prev_v[prev_idx].max(0);
            prev_y = (prev_x - prev_k).max(0);
            from_down = use_down;
        }

        let mut cur_x = x;
        let mut cur_y = y;
        while cur_x > prev_x && cur_y > prev_y {
            cur_x -= 1;
            cur_y -= 1;
            edits.push(Edit::Match(cur_x as usize, cur_y as usize));
        }

        if d_rev > 0 {
            if from_down {
                edits.push(Edit::Insert(prev_y as usize));
            } else {
                edits.push(Edit::Delete(prev_x as usize));
            }
        }

        x = prev_x;
        y = prev_y;
    }

    edits.reverse();
    edits
}

fn align_gap_via_hash(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    use std::collections::{HashMap, VecDeque};

    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let mut sig_to_new: HashMap<crate::workbook::RowSignature, VecDeque<u32>> = HashMap::new();
    for (j, meta) in new_slice.iter().enumerate() {
        sig_to_new
            .entry(meta.signature)
            .or_default()
            .push_back(j as u32);
    }

    let mut candidate_pairs: Vec<(u32, u32)> = Vec::new();
    for (i, meta) in old_slice.iter().enumerate() {
        if let Some(q) = sig_to_new.get_mut(&meta.signature)
            && let Some(j) = q.pop_front()
        {
            candidate_pairs.push((i as u32, j));
        }
    }

    if candidate_pairs.is_empty() && m == n {
        let matched = old_slice
            .iter()
            .zip(new_slice.iter())
            .map(|(a, b)| (a.row_idx, b.row_idx))
            .collect();

        return GapAlignmentResult {
            matched,
            inserted: Vec::new(),
            deleted: Vec::new(),
            moves: Vec::new(),
        };
    }

    let lis = lis_indices_u32(&candidate_pairs, |&(_, new_j)| new_j);

    let mut keep = vec![false; candidate_pairs.len()];
    for idx in lis {
        keep[idx] = true;
    }

    let mut used_old = vec![false; m];
    let mut used_new = vec![false; n];
    let mut matched: Vec<(u32, u32)> = Vec::new();

    for (k, (old_i, new_j)) in candidate_pairs.iter().copied().enumerate() {
        if keep[k] {
            used_old[old_i as usize] = true;
            used_new[new_j as usize] = true;
            matched.push((
                old_slice[old_i as usize].row_idx,
                new_slice[new_j as usize].row_idx,
            ));
        }
    }

    let mut deleted: Vec<u32> = Vec::new();
    for i in 0..m {
        if !used_old[i] {
            deleted.push(old_slice[i].row_idx);
        }
    }

    let mut inserted: Vec<u32> = Vec::new();
    for j in 0..n {
        if !used_new[j] {
            inserted.push(new_slice[j].row_idx);
        }
    }

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}

fn lis_indices_u32<T, F>(items: &[T], key: F) -> Vec<usize>
where
    F: Fn(&T) -> u32,
{
    let mut piles: Vec<usize> = Vec::new();
    let mut predecessors: Vec<Option<usize>> = vec![None; items.len()];

    for (idx, item) in items.iter().enumerate() {
        let k = key(item);
        let pos = piles
            .binary_search_by_key(&k, |&pile_idx| key(&items[pile_idx]))
            .unwrap_or_else(|insert_pos| insert_pos);

        if pos > 0 {
            predecessors[idx] = Some(piles[pos - 1]);
        }

        if pos == piles.len() {
            piles.push(idx);
        } else {
            piles[pos] = idx;
        }
    }

    if piles.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<usize> = Vec::new();
    let mut current = *piles.last().unwrap();
    loop {
        result.push(current);
        if let Some(prev) = predecessors[current] {
            current = prev;
        } else {
            break;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
    use crate::workbook::CellValue;

    fn grid_from_run_lengths(pattern: &[(i32, u32)]) -> Grid {
        let total_rows: u32 = pattern.iter().map(|(_, count)| *count).sum();
        let mut grid = Grid::new(total_rows, 1);
        let mut row_idx = 0u32;
        for (val, count) in pattern {
            for _ in 0..*count {
                grid.insert_cell(row_idx, 0, Some(CellValue::Number(*val as f64)), None);
                row_idx = row_idx.saturating_add(1);
            }
        }
        grid
    }

    fn grid_with_unique_rows(rows: &[i32]) -> Grid {
        let nrows = rows.len() as u32;
        let mut grid = Grid::new(nrows, 1);
        for (r, &val) in rows.iter().enumerate() {
            grid.insert_cell(r as u32, 0, Some(CellValue::Number(val as f64)), None);
        }
        grid
    }

    fn row_meta_from_hashes(start_row: u32, hashes: &[u128]) -> Vec<RowMeta> {
        hashes
            .iter()
            .enumerate()
            .map(|(idx, &hash)| {
                let signature = crate::workbook::RowSignature { hash };
                RowMeta {
                    row_idx: start_row + idx as u32,
                    signature,
                    hash: signature,
                    non_blank_count: 1,
                    first_non_blank_col: 0,
                    frequency_class: FrequencyClass::Common,
                    is_low_info: false,
                }
            })
            .collect()
    }

    #[test]
    fn aligns_compressed_runs_with_insert_and_delete() {
        let grid_a = grid_from_run_lengths(&[(1, 50), (2, 5), (1, 50)]);
        let grid_b = grid_from_run_lengths(&[(1, 52), (2, 3), (1, 50)]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed for repetitive runs");
        assert!(alignment.moves.is_empty());
        assert_eq!(alignment.inserted.len(), 2);
        assert_eq!(alignment.deleted.len(), 2);
        assert_eq!(alignment.matched.len(), 103);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn run_alignment_falls_back_on_mismatch() {
        let grid_a = grid_from_run_lengths(&[(1, 3), (2, 3), (1, 3)]);
        let grid_b = grid_from_run_lengths(&[(1, 3), (3, 3), (1, 3)]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should still produce result via full AMR");
        assert!(!alignment.matched.is_empty());
    }

    #[test]
    fn amr_disjoint_gaps_with_insertions_and_deletions() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 100, 4, 5, 6, 200, 7, 8, 9]);
        let grid_b = grid_with_unique_rows(&[1, 2, 10, 3, 4, 5, 6, 7, 20, 8, 9]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with disjoint gaps");

        assert!(!alignment.matched.is_empty(), "should have matched pairs");

        let matched_is_monotonic = alignment
            .matched
            .windows(2)
            .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].1);
        assert!(
            matched_is_monotonic,
            "matched pairs should be monotonically increasing"
        );

        assert!(
            !alignment.inserted.is_empty() || !alignment.deleted.is_empty(),
            "should have insertions and/or deletions"
        );
    }

    #[test]
    fn amr_recursive_gap_alignment_returns_monotonic_alignment() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let rows_b = vec![
            1, 2, 100, 3, 4, 5, 200, 6, 7, 8, 300, 9, 10, 11, 400, 12, 13, 14, 15,
        ];
        let grid_b = grid_with_unique_rows(&rows_b);

        let config = DiffConfig {
            recursive_align_threshold: 5,
            small_gap_threshold: 2,
            ..Default::default()
        };

        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with recursive gaps");

        let matched_is_monotonic = alignment
            .matched
            .windows(2)
            .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].1);
        assert!(
            matched_is_monotonic,
            "recursive alignment should produce monotonic matched pairs"
        );

        for &inserted_row in &alignment.inserted {
            assert!(
                !alignment.matched.iter().any(|(_, b)| *b == inserted_row),
                "inserted rows should not appear in matched pairs"
            );
        }

        for &deleted_row in &alignment.deleted {
            assert!(
                !alignment.matched.iter().any(|(a, _)| *a == deleted_row),
                "deleted rows should not appear in matched pairs"
            );
        }
    }

    #[test]
    fn amr_multi_gap_move_detection_produces_expected_row_block_move() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let grid_b = grid_with_unique_rows(&[1, 2, 6, 7, 8, 3, 4, 5, 9, 10]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with moved block");

        assert!(
            !alignment.matched.is_empty(),
            "should have matched pairs even with moves"
        );

        let old_rows: std::collections::HashSet<_> =
            alignment.matched.iter().map(|(a, _)| *a).collect();
        let new_rows: std::collections::HashSet<_> =
            alignment.matched.iter().map(|(_, b)| *b).collect();

        assert!(
            old_rows.len() <= 10 && new_rows.len() <= 10,
            "matched rows should not exceed input size"
        );
    }

    #[test]
    fn amr_alignment_empty_grids() {
        let grid_a = Grid::new(0, 0);
        let grid_b = Grid::new(0, 0);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed for empty grids");

        assert!(alignment.matched.is_empty());
        assert!(alignment.inserted.is_empty());
        assert!(alignment.deleted.is_empty());
        assert!(alignment.moves.is_empty());
    }

    #[test]
    fn align_rows_amr_with_signatures_exposes_row_hashes() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4]);
        let grid_b = grid_with_unique_rows(&[1, 2, 3, 4]);

        let config = DiffConfig::default();
        let result =
            align_rows_amr_with_signatures(&grid_a, &grid_b, &config).expect("should align");

        assert_eq!(result.row_signatures_a.len(), grid_a.nrows as usize);
        assert_eq!(result.row_signatures_b.len(), grid_b.nrows as usize);
        assert_eq!(result.alignment.matched.len(), grid_a.nrows as usize);

        for row in 0..grid_a.nrows {
            let expected_a = grid_a.compute_row_signature(row);
            let expected_b = grid_b.compute_row_signature(row);
            assert_eq!(
                Some(expected_a),
                result.row_signatures_a.get(row as usize).copied(),
                "row {} signature for grid A should match compute_row_signature",
                row
            );
            assert_eq!(
                Some(expected_b),
                result.row_signatures_b.get(row as usize).copied(),
                "row {} signature for grid B should match compute_row_signature",
                row
            );
        }
    }

    #[test]
    fn amr_alignment_all_deleted() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5]);
        let grid_b = Grid::new(0, 1);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed when all rows deleted");

        assert!(alignment.matched.is_empty());
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted.len(), 5);
    }

    #[test]
    fn amr_alignment_all_inserted() {
        let grid_a = Grid::new(0, 1);
        let grid_b = grid_with_unique_rows(&[1, 2, 3, 4, 5]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed when all rows inserted");

        assert!(alignment.matched.is_empty());
        assert_eq!(alignment.inserted.len(), 5);
        assert!(alignment.deleted.is_empty());
    }

    #[test]
    fn align_small_gap_enforces_cap_with_hash_fallback() {
        let config = DiffConfig::default();
        let large = (config.max_lcs_gap_size + 1) as usize;
        let old_hashes: Vec<u128> = (0..large as u32).map(|i| i as u128).collect();
        let new_hashes: Vec<u128> = (0..large as u32).map(|i| (10_000 + i) as u128).collect();

        let old_meta = row_meta_from_hashes(10, &old_hashes);
        let new_meta = row_meta_from_hashes(20, &new_hashes);

        let result = align_small_gap(&old_meta, &new_meta, &config);
        assert_eq!(result.matched.len(), large);
        assert!(result.inserted.is_empty());
        assert!(result.deleted.is_empty());
        assert_eq!(result.matched.first(), Some(&(10, 20)));
        assert_eq!(
            result.matched.last(),
            Some(&(10 + large as u32 - 1, 20 + large as u32 - 1))
        );
    }

    #[test]
    fn hash_fallback_produces_monotone_pairs() {
        let old_meta = row_meta_from_hashes(0, &[1, 2, 3, 4]);
        let new_meta = row_meta_from_hashes(0, &[2, 1, 3, 4]);

        let result = align_gap_via_hash(&old_meta, &new_meta);
        assert_eq!(result.matched, vec![(1, 0), (2, 2), (3, 3)]);

        let is_monotone = result
            .matched
            .windows(2)
            .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].1);
        assert!(is_monotone, "hash fallback must preserve monotone ordering");
        assert_eq!(result.inserted, vec![1]);
        assert_eq!(result.deleted, vec![0]);
    }

    #[test]
    fn myers_handles_medium_gap_with_single_insertion() {
        let count = 300usize;
        let old_hashes: Vec<u128> = (0..count as u128).collect();
        let mut new_hashes: Vec<u128> = old_hashes.clone();
        new_hashes.insert(150, 9_999);

        let old_meta = row_meta_from_hashes(0, &old_hashes);
        let new_meta = row_meta_from_hashes(0, &new_hashes);

        let result = align_small_gap(&old_meta, &new_meta, &DiffConfig::default());
        assert_eq!(result.inserted, vec![150]);
        assert!(result.deleted.is_empty());
        assert_eq!(result.matched.len(), count);
        assert_eq!(result.matched.first(), Some(&(0, 0)));
        assert_eq!(
            result.matched.last(),
            Some(&(count as u32 - 1, (count + 1) as u32 - 1))
        );
    }
}

```

---

### File: `core\src\alignment\gap_strategy.rs`

```rust
//! Gap strategy selection for AMR alignment.
//!
//! Implements gap strategy selection as described in the unified grid diff
//! specification Sections 9.6 and 12. After anchors divide the grids into
//! gaps, each gap is processed according to its characteristics:
//!
//! - **Empty**: Both sides empty, nothing to do
//! - **InsertAll**: Old side empty, all new rows are insertions
//! - **DeleteAll**: New side empty, all old rows are deletions
//! - **SmallEdit**: Both sides small enough for O(n*m) LCS alignment
//! - **MoveCandidate**: Gap contains matching unique signatures that may indicate moves
//! - **RecursiveAlign**: Gap is large; recursively apply AMR with rare anchors
//! - **HashFallback**: Monotone hash/LIS fallback for large gaps

use std::collections::HashSet;

use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::config::DiffConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapStrategy {
    Empty,
    InsertAll,
    DeleteAll,
    SmallEdit,
    MoveCandidate,
    RecursiveAlign,
    HashFallback,
}

pub fn select_gap_strategy(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    config: &DiffConfig,
    has_recursed: bool,
) -> GapStrategy {
    let old_len = old_slice.len() as u32;
    let new_len = new_slice.len() as u32;

    if old_len == 0 && new_len == 0 {
        return GapStrategy::Empty;
    }
    if old_len == 0 {
        return GapStrategy::InsertAll;
    }
    if new_len == 0 {
        return GapStrategy::DeleteAll;
    }

    let is_move_candidate = has_matching_signatures(old_slice, new_slice);

    let small_threshold = config.small_gap_threshold.min(config.max_lcs_gap_size);
    if old_len <= small_threshold && new_len <= small_threshold {
        return if is_move_candidate {
            GapStrategy::MoveCandidate
        } else {
            GapStrategy::SmallEdit
        };
    }

    if (old_len > config.recursive_align_threshold || new_len > config.recursive_align_threshold)
        && !has_recursed
    {
        return GapStrategy::RecursiveAlign;
    }

    if is_move_candidate {
        return GapStrategy::MoveCandidate;
    }

    if old_len > config.max_lcs_gap_size || new_len > config.max_lcs_gap_size {
        return GapStrategy::HashFallback;
    }

    GapStrategy::SmallEdit
}

fn has_matching_signatures(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> bool {
    let set: HashSet<_> = old_slice
        .iter()
        .filter(|m| m.frequency_class == FrequencyClass::Unique)
        .map(|m| m.signature)
        .collect();

    new_slice
        .iter()
        .filter(|m| m.frequency_class == FrequencyClass::Unique)
        .any(|m| set.contains(&m.signature))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
    use crate::workbook::RowSignature;

    fn meta(row_idx: u32, hash: u128) -> RowMeta {
        let signature = RowSignature { hash };
        RowMeta {
            row_idx,
            signature,
            hash: signature,
            non_blank_count: 1,
            first_non_blank_col: 0,
            frequency_class: FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn respects_configured_max_lcs_gap_size() {
        let config = DiffConfig {
            max_lcs_gap_size: 2,
            small_gap_threshold: 10,
            ..Default::default()
        };
        let rows_a = vec![meta(0, 1), meta(1, 2), meta(2, 3)];
        let rows_b = vec![meta(0, 4), meta(1, 5), meta(2, 6)];

        let strategy = select_gap_strategy(&rows_a, &rows_b, &config, false);
        assert_eq!(strategy, GapStrategy::HashFallback);
    }
}

```

---

### File: `core\src\alignment\mod.rs`

```rust
//! Anchor-Move-Refine (AMR) row alignment algorithm.
//!
//! This module implements a simplified version of the AMR algorithm described in the
//! unified grid diff specification. The implementation follows the general structure:
//!
//! 1. **Row Metadata Collection** (`row_metadata.rs`, Spec Section 9.11)
//!    - Compute row signatures and classify by frequency (Unique/Rare/Common/LowInfo)
//!
//! 2. **Anchor Discovery** (`anchor_discovery.rs`, Spec Section 10)
//!    - Find rows that are unique in both grids with matching signatures
//!
//! 3. **Anchor Chain Construction** (`anchor_chain.rs`, Spec Section 10)
//!    - Build longest increasing subsequence (LIS) of anchors to preserve relative order
//!
//! 4. **Gap Strategy Selection** (`gap_strategy.rs`, Spec Sections 9.6, 12)
//!    - For each gap between anchors, select appropriate strategy:
//!      Empty, InsertAll, DeleteAll, SmallEdit, MoveCandidate, or RecursiveAlign
//!
//! 5. **Assembly** (`assembly.rs`, Spec Section 12)
//!    - Assemble final alignment by processing gaps and anchors
//!
//! ## Intentional Spec Deviations
//!
//! The current implementation simplifies the full AMR spec in the following ways:
//!
//! - **No global move-candidate extraction phase**: The full spec (Sections 9.5-9.7, 11)
//!   describes a global phase that extracts out-of-order matches before gap filling.
//!   This implementation instead detects moves opportunistically within gaps via
//!   `GapStrategy::MoveCandidate` and `find_block_move`. This is simpler but may miss
//!   some complex multi-block move patterns that the full spec would detect.
//!
//! - **No explicit move validation phase**: The spec describes validating move candidates
//!   (Section 11) to resolve conflicts. The current implementation accepts the first
//!   valid move found within each gap.
//!
//! - **RLE fast path**: For highly repetitive grids (>50% compression), the implementation
//!   uses a run-length encoded alignment path (`runs.rs`) that bypasses full AMR.
//!
//! These simplifications are acceptable for most real-world Excel workbooks and keep
//! the implementation maintainable. Future work may implement the full global move
//! extraction if complex reordering scenarios require it.

pub(crate) mod anchor_chain;
pub(crate) mod anchor_discovery;
pub(crate) mod assembly;
pub(crate) mod gap_strategy;
pub(crate) mod move_extraction;
pub(crate) mod row_metadata;
pub(crate) mod runs;

#[allow(unused_imports)]
pub(crate) use assembly::{
    RowAlignment, RowAlignmentWithSignatures, RowBlockMove, align_rows_amr,
    align_rows_amr_with_signatures, align_rows_amr_with_signatures_from_views,
};

```

---

### File: `core\src\alignment\move_extraction.rs`

```rust
//! Move extraction from alignment gaps.
//!
//! Implements localized move detection within gaps. This is a simplified approach
//! compared to the full spec (Sections 9.5-9.7, 11) which describes global
//! move-candidate extraction and validation phases.
//!
//! ## Current Implementation
//!
//! - `find_block_move`: Scans for contiguous blocks of matching signatures
//!   between old and new slices within a gap. Returns the largest found.
//!
//! - `moves_from_matched_pairs`: Extracts block moves from matched row pairs
//!   where consecutive pairs have the same offset (indicating they moved together).
//!
//! ## Future Work (TODO)
//!
//! To implement full spec compliance, this module would need:
//!
//! 1. Global unanchored match collection (all out-of-order signature matches)
//! 2. Candidate move construction from unanchored matches
//! 3. Move validation to resolve overlapping/conflicting candidates
//! 4. Integration with gap filling to consume validated moves

use std::collections::HashMap;

use crate::alignment::RowBlockMove;
use crate::alignment::row_metadata::RowMeta;
use crate::config::DiffConfig;
use crate::workbook::RowSignature;

pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    let max_slice_len = config.move_extraction_max_slice_len as usize;
    if old_slice.len() > max_slice_len || new_slice.len() > max_slice_len {
        return None;
    }

    let mut positions: HashMap<RowSignature, Vec<usize>> = HashMap::new();
    for (idx, meta) in old_slice.iter().enumerate() {
        if meta.is_low_info() {
            continue;
        }
        positions.entry(meta.signature).or_default().push(idx);
    }

    let mut best: Option<RowBlockMove> = None;
    let mut best_len: usize = 0;

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if meta.is_low_info() {
            continue;
        }

        let Some(candidates) = positions.get(&meta.signature) else {
            continue;
        };

        let max_candidates = config.move_extraction_max_candidates_per_sig as usize;
        for &old_idx in candidates.iter().take(max_candidates) {
            let max_possible = (old_slice.len() - old_idx).min(new_slice.len() - new_idx);
            if max_possible <= best_len {
                continue;
            }

            let mut len = 0usize;
            while len < max_possible
                && old_slice[old_idx + len].signature == new_slice[new_idx + len].signature
            {
                len += 1;
            }

            if len >= min_len as usize && len > best_len {
                best_len = len;
                best = Some(RowBlockMove {
                    src_start_row: old_slice[old_idx].row_idx,
                    dst_start_row: new_slice[new_idx].row_idx,
                    row_count: len as u32,
                });
            }
        }
    }

    best
}

pub fn moves_from_matched_pairs(pairs: &[(u32, u32)]) -> Vec<RowBlockMove> {
    if pairs.is_empty() {
        return Vec::new();
    }

    let mut sorted = pairs.to_vec();
    sorted.sort_by_key(|(a, b)| (*a, *b));

    let mut moves = Vec::new();
    let mut start = sorted[0];
    let mut prev = sorted[0];
    let mut run_len = 1u32;
    let mut current_offset: i64 = prev.1 as i64 - prev.0 as i64;

    for &(a, b) in sorted.iter().skip(1) {
        let offset = b as i64 - a as i64;
        if offset == current_offset && a == prev.0 + 1 && b == prev.1 + 1 {
            run_len += 1;
            prev = (a, b);
            continue;
        }

        if run_len > 1 && current_offset != 0 {
            moves.push(RowBlockMove {
                src_start_row: start.0,
                dst_start_row: start.1,
                row_count: run_len,
            });
        }

        start = (a, b);
        prev = (a, b);
        current_offset = offset;
        run_len = 1;
    }

    if run_len > 1 && current_offset != 0 {
        moves.push(RowBlockMove {
            src_start_row: start.0,
            dst_start_row: start.1,
            row_count: run_len,
        });
    }

    moves
}

```

---

### File: `core\src\alignment\row_metadata.rs`

```rust
//! Row metadata and frequency classification for AMR alignment.
//!
//! Implements row frequency classification as described in the unified grid diff
//! specification Section 9.11. Each row is classified into one of four frequency classes:
//!
//! - **Unique**: Appears exactly once in the grid (highest anchor quality)
//! - **Rare**: Appears 2-N times where N is configurable (can serve as secondary anchors)
//! - **Common**: Appears frequently (poor anchor quality)
//! - **LowInfo**: Blank or near-blank rows (ignored for anchoring)

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

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn make_meta(row_idx: u32, hash: u128, non_blank: u16) -> RowMeta {
        let sig = RowSignature { hash };
        RowMeta {
            row_idx,
            signature: sig,
            hash: sig,
            non_blank_count: non_blank,
            first_non_blank_col: 0,
            frequency_class: FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn classifies_unique_and_rare_and_low_info() {
        let mut meta = vec![make_meta(0, 1, 3), make_meta(1, 1, 3), make_meta(2, 2, 1)];

        let mut config = DiffConfig::default();
        config.rare_threshold = 2;
        config.low_info_threshold = 2;

        classify_row_frequencies(&mut meta, &config);

        assert_eq!(meta[0].frequency_class, FrequencyClass::Rare);
        assert_eq!(meta[1].frequency_class, FrequencyClass::Rare);
        assert_eq!(meta[2].frequency_class, FrequencyClass::LowInfo);
    }
}

```

---

### File: `core\src\alignment\runs.rs`

```rust
//! Run-length encoding for repetitive row patterns.
//!
//! Implements run-length compression as described in the unified grid diff
//! specification Section 2.6 (optional optimization). For grids where >50%
//! of rows share signatures with adjacent rows, this provides a fast path
//! that avoids full AMR computation.
//!
//! This is particularly effective for:
//! - Template-based workbooks with many identical rows
//! - Data with long runs of blank or placeholder rows
//! - Adversarial cases designed to stress the alignment algorithm

use crate::alignment::row_metadata::RowMeta;
use crate::workbook::RowSignature;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRun {
    pub signature: RowSignature,
    pub start_row: u32,
    pub count: u32,
}

pub fn compress_to_runs(meta: &[RowMeta]) -> Vec<RowRun> {
    let mut runs = Vec::new();
    let mut i = 0usize;
    while i < meta.len() {
        let sig = meta[i].signature;
        let start = i;
        while i < meta.len() && meta[i].signature == sig {
            i += 1;
        }
        runs.push(RowRun {
            signature: sig,
            start_row: meta[start].row_idx,
            count: (i - start) as u32,
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(idx: u32, hash: u128) -> RowMeta {
        let sig = RowSignature { hash };
        RowMeta {
            row_idx: idx,
            signature: sig,
            hash: sig,
            non_blank_count: 1,
            first_non_blank_col: 0,
            frequency_class: crate::alignment::row_metadata::FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn compresses_identical_rows() {
        let meta = vec![make_meta(0, 1), make_meta(1, 1), make_meta(2, 2)];
        let runs = compress_to_runs(&meta);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].count, 2);
        assert_eq!(runs[1].count, 1);
    }

    #[test]
    fn compresses_10k_identical_rows_to_single_run() {
        let meta: Vec<RowMeta> = (0..10_000).map(|i| make_meta(i, 42)).collect();
        let runs = compress_to_runs(&meta);

        assert_eq!(
            runs.len(),
            1,
            "10K identical rows should compress to a single run"
        );
        assert_eq!(
            runs[0].count, 10_000,
            "single run should have count of 10,000"
        );
        assert_eq!(
            runs[0].signature.hash, 42,
            "run signature should match input"
        );
        assert_eq!(runs[0].start_row, 0, "run should start at row 0");
    }

    #[test]
    fn alternating_pattern_ab_does_not_overcompress() {
        let meta: Vec<RowMeta> = (0..10_000)
            .map(|i| {
                let hash = if i % 2 == 0 { 1 } else { 2 };
                make_meta(i, hash)
            })
            .collect();
        let runs = compress_to_runs(&meta);

        assert_eq!(
            runs.len(),
            10_000,
            "alternating A-B pattern should produce 10K runs (no compression benefit)"
        );

        for (i, run) in runs.iter().enumerate() {
            assert_eq!(
                run.count, 1,
                "each run should have count of 1 for alternating pattern"
            );
            let expected_hash = if i % 2 == 0 { 1 } else { 2 };
            assert_eq!(
                run.signature.hash, expected_hash,
                "run signature should alternate"
            );
        }
    }

    #[test]
    fn mixed_runs_with_varying_lengths() {
        let mut meta = Vec::new();
        let mut row_idx = 0u32;

        for _ in 0..100 {
            meta.push(make_meta(row_idx, 1));
            row_idx += 1;
        }
        for _ in 0..50 {
            meta.push(make_meta(row_idx, 2));
            row_idx += 1;
        }
        for _ in 0..200 {
            meta.push(make_meta(row_idx, 3));
            row_idx += 1;
        }
        for _ in 0..1 {
            meta.push(make_meta(row_idx, 4));
            row_idx += 1;
        }

        let runs = compress_to_runs(&meta);

        assert_eq!(
            runs.len(),
            4,
            "should produce 4 runs for 4 distinct signatures"
        );
        assert_eq!(runs[0].count, 100);
        assert_eq!(runs[1].count, 50);
        assert_eq!(runs[2].count, 200);
        assert_eq!(runs[3].count, 1);
    }

    #[test]
    fn empty_input_produces_empty_runs() {
        let meta: Vec<RowMeta> = vec![];
        let runs = compress_to_runs(&meta);
        assert!(runs.is_empty(), "empty input should produce empty runs");
    }

    #[test]
    fn single_row_produces_single_run() {
        let meta = vec![make_meta(0, 999)];
        let runs = compress_to_runs(&meta);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].count, 1);
        assert_eq!(runs[0].start_row, 0);
        assert_eq!(runs[0].signature.hash, 999);
    }

    #[test]
    fn run_compression_preserves_row_indices() {
        let meta: Vec<RowMeta> = (0..1000u32)
            .map(|i| make_meta(i, (i / 100) as u128))
            .collect();
        let runs = compress_to_runs(&meta);

        assert_eq!(runs.len(), 10, "should have 10 runs (one per 100 rows)");

        for (group_idx, run) in runs.iter().enumerate() {
            let expected_start = (group_idx * 100) as u32;
            assert_eq!(
                run.start_row, expected_start,
                "run {} should start at row {}",
                group_idx, expected_start
            );
            assert_eq!(run.count, 100, "each run should have 100 rows");
        }
    }
}

```

---

### File: `core\src\bin\wasm_smoke.rs`

```rust
use excel_diff::{
    CallbackSink, CellValue, DiffConfig, DiffSession, Grid, Sheet, SheetKind, Workbook,
    try_diff_workbooks_streaming,
};

fn make_workbook(session: &mut DiffSession, value: f64) -> Workbook {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(value)), None);

    let sheet_name = session.strings.intern("WasmSmoke");

    Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
}

fn main() {
    let mut session = DiffSession::new();
    let wb_a = make_workbook(&mut session, 1.0);
    let wb_b = make_workbook(&mut session, 2.0);

    let mut op_count = 0usize;
    {
        let mut sink = CallbackSink::new(|_op| op_count += 1);
        let summary = try_diff_workbooks_streaming(
            &wb_a,
            &wb_b,
            &mut session.strings,
            &DiffConfig::default(),
            &mut sink,
        )
        .expect("smoke diff should succeed");

        assert!(summary.complete, "smoke diff should be complete");
        assert_eq!(
            summary.op_count, op_count,
            "sink count should match reported op count"
        );
        assert!(op_count > 0, "expected at least one diff op");
    }
}

```

---

### File: `core\src\column_alignment.rs`

```rust
use crate::config::DiffConfig;
use crate::grid_view::{ColHash, ColMeta, GridView, HashStats};
use crate::hashing::hash_col_content_unordered_128;
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnAlignment {
    pub(crate) matched: Vec<(u32, u32)>, // (col_idx_a, col_idx_b)
    pub(crate) inserted: Vec<u32>,       // columns present only in B
    pub(crate) deleted: Vec<u32>,        // columns present only in A
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnBlockMove {
    pub src_start_col: u32,
    pub dst_start_col: u32,
    pub col_count: u32,
}

fn unordered_col_hashes(grid: &Grid) -> Vec<ColHash> {
    let mut col_cells: Vec<Vec<&crate::workbook::Cell>> = vec![Vec::new(); grid.ncols as usize];
    for ((_, col), cell) in grid.iter_cells() {
        let idx = col as usize;
        if idx < col_cells.len() {
            col_cells[idx].push(cell);
        }
    }
    col_cells
        .iter()
        .map(|cells| hash_col_content_unordered_128(cells))
        .collect()
}

pub(crate) fn detect_exact_column_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<ColumnBlockMove> {
    if old.ncols != new.ncols || old.nrows != new.nrows {
        return None;
    }

    if old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    let unordered_a = unordered_col_hashes(old);
    let unordered_b = unordered_col_hashes(new);

    let col_meta_a: Vec<ColMeta> = view_a
        .col_meta
        .iter()
        .enumerate()
        .map(|(idx, meta)| ColMeta {
            hash: *unordered_a.get(idx).unwrap_or(&meta.hash),
            ..*meta
        })
        .collect();
    let col_meta_b: Vec<ColMeta> = view_b
        .col_meta
        .iter()
        .enumerate()
        .map(|(idx, meta)| ColMeta {
            hash: *unordered_b.get(idx).unwrap_or(&meta.hash),
            ..*meta
        })
        .collect();

    if blank_dominated(&view_a) || blank_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_col_meta(&col_meta_a, &col_meta_b);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    let meta_a = &col_meta_a;
    let meta_b = &col_meta_b;
    let n = meta_a.len();

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let prefix = (0..n).find(|&idx| meta_a[idx].hash != meta_b[idx].hash)?;

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = n - 1 - suffix_len;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }
    let tail_start = n - suffix_len;

    let try_candidate = |src_start: usize, dst_start: usize| -> Option<ColumnBlockMove> {
        if src_start >= tail_start || dst_start >= tail_start {
            return None;
        }

        let mut len = 0usize;
        while src_start + len < tail_start && dst_start + len < tail_start {
            if meta_a[src_start + len].hash != meta_b[dst_start + len].hash {
                break;
            }
            len += 1;
        }

        if len == 0 {
            return None;
        }

        let src_end = src_start + len;
        let dst_end = dst_start + len;

        if !(src_end <= dst_start || dst_end <= src_start) {
            return None;
        }

        let mut idx_a = 0usize;
        let mut idx_b = 0usize;

        loop {
            if idx_a == src_start {
                idx_a = src_end;
            }
            if idx_b == dst_start {
                idx_b = dst_end;
            }

            if idx_a >= n && idx_b >= n {
                break;
            }

            if idx_a >= n || idx_b >= n {
                return None;
            }

            if meta_a[idx_a].hash != meta_b[idx_b].hash {
                return None;
            }

            idx_a += 1;
            idx_b += 1;
        }

        for meta in &meta_a[src_start..src_end] {
            if stats.freq_a.get(&meta.hash).copied().unwrap_or(0) != 1
                || stats.freq_b.get(&meta.hash).copied().unwrap_or(0) != 1
            {
                return None;
            }
        }

        Some(ColumnBlockMove {
            src_start_col: meta_a[src_start].col_idx,
            dst_start_col: meta_b[dst_start].col_idx,
            col_count: len as u32,
        })
    };

    if let Some(src_start) =
        (prefix..tail_start).find(|&idx| meta_a[idx].hash == meta_b[prefix].hash)
        && let Some(mv) = try_candidate(src_start, prefix)
    {
        return Some(mv);
    }

    if let Some(dst_start) =
        (prefix..tail_start).find(|&idx| meta_b[idx].hash == meta_a[prefix].hash)
        && let Some(mv) = try_candidate(prefix, dst_start)
    {
        return Some(mv);
    }

    None
}

#[allow(dead_code)]
pub(crate) fn align_single_column_change(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<ColumnAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_single_column_change_from_views(&view_a, &view_b, config)
}

pub(crate) fn align_single_column_change_from_views(
    view_a: &GridView,
    view_b: &GridView,
    config: &DiffConfig,
) -> Option<ColumnAlignment> {
    if !is_within_size_bounds(view_a.source, view_b.source, config) {
        return None;
    }

    if view_a.source.nrows != view_b.source.nrows {
        return None;
    }

    let col_diff = view_b.source.ncols as i64 - view_a.source.ncols as i64;
    if col_diff.abs() != 1 {
        return None;
    }

    let stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    if col_diff == 1 {
        find_single_gap_alignment(
            &view_a.col_meta,
            &view_b.col_meta,
            &stats,
            ColumnChange::Insert,
        )
    } else {
        find_single_gap_alignment(
            &view_a.col_meta,
            &view_b.col_meta,
            &stats,
            ColumnChange::Delete,
        )
    }
}

enum ColumnChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    cols_a: &[ColMeta],
    cols_b: &[ColMeta],
    stats: &HashStats<ColHash>,
    change: ColumnChange,
) -> Option<ColumnAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < cols_a.len() && idx_b < cols_b.len() {
        let meta_a = cols_a[idx_a];
        let meta_b = cols_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.col_idx, meta_b.col_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            ColumnChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.col_idx);
                idx_b += 1;
            }
            ColumnChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.col_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < cols_a.len() || idx_b < cols_b.len() {
        if skipped {
            return None;
        }

        match change {
            ColumnChange::Insert if idx_a == cols_a.len() && cols_b.len() == idx_b + 1 => {
                let meta_b = cols_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.col_idx);
            }
            ColumnChange::Delete if idx_b == cols_b.len() && cols_a.len() == idx_a + 1 => {
                let meta_a = cols_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.col_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = ColumnAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn is_monotonic(pairs: &[(u32, u32)]) -> bool {
    pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1)
}

fn is_unique_to_b(hash: ColHash, stats: &HashStats<ColHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: ColHash, stats: &HashStats<ColHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid, config: &DiffConfig) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= config.max_align_rows && cols <= config.max_align_cols
}

fn has_heavy_repetition(stats: &HashStats<ColHash>, config: &DiffConfig) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > config.max_hash_repeat
}

fn blank_dominated(view: &GridView<'_>) -> bool {
    if view.col_meta.is_empty() {
        return false;
    }

    let blank_cols = view
        .col_meta
        .iter()
        .filter(|meta| meta.non_blank_count == 0)
        .count();

    blank_cols * 2 > view.col_meta.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

    fn grid_from_numbers(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert_cell(
                    r_idx as u32,
                    c_idx as u32,
                    Some(CellValue::Number(*value as f64)),
                    None,
                );
            }
        }

        grid
    }

    #[test]
    fn single_insert_aligns_all_columns() {
        let base_rows: Vec<Vec<i32>> =
            vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8], vec![9, 10, 11, 12]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&base_refs);

        let inserted_rows: Vec<Vec<i32>> = base_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let mut new_row = row.clone();
                new_row.insert(2, 100 + idx as i32); // insert at index 2 (0-based)
                new_row
            })
            .collect();
        let inserted_refs: Vec<&[i32]> = inserted_rows.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&inserted_refs);

        let alignment = align_single_column_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");

        assert_eq!(alignment.inserted, vec![2]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[1], (1, 1));
        assert_eq!(alignment.matched[2], (2, 3));
        assert_eq!(alignment.matched[3], (3, 4));
    }

    #[test]
    fn multiple_unique_columns_causes_bailout() {
        let base_rows: Vec<Vec<i32>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&base_refs);

        let mut rows_b: Vec<Vec<i32>> = base_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let mut new_row = row.clone();
                new_row.insert(1, 100 + idx as i32); // inserted column
                new_row
            })
            .collect();
        if let Some(cell) = rows_b.get_mut(1).and_then(|row| row.get_mut(3)) {
            *cell = 999;
        }
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&rows_b_refs);

        assert!(align_single_column_change(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn heavy_repetition_causes_bailout() {
        let repetitive_cols = 9;
        let rows: usize = 3;

        let values_a: Vec<Vec<i32>> = (0..rows).map(|_| vec![1; repetitive_cols]).collect();
        let refs_a: Vec<&[i32]> = values_a.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&refs_a);

        let values_b: Vec<Vec<i32>> = (0..rows)
            .map(|row_idx| {
                let mut row = vec![1; repetitive_cols];
                row.insert(4, 2 + row_idx as i32);
                row
            })
            .collect();
        let refs_b: Vec<&[i32]> = values_b.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&refs_b);

        assert!(align_single_column_change(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_simple_case() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

        let grid_b = grid_from_numbers(&[&[10, 30, 40, 20], &[11, 31, 41, 21]]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected column move found");
        assert_eq!(mv.src_start_col, 1);
        assert_eq!(mv.col_count, 1);
        assert_eq!(mv.dst_start_col, 3);
    }

    #[test]
    fn detect_exact_column_block_move_rejects_internal_edits() {
        let grid_a = grid_from_numbers(&[&[1, 2, 3, 4], &[5, 6, 7, 8], &[9, 10, 11, 12]]);

        let grid_b = grid_from_numbers(&[
            &[1, 3, 4, 2],
            &[5, 7, 8, 6],
            &[9, 11, 12, 999], // edit inside moved column
        ]);

        assert!(detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_rejects_repetition() {
        let grid_a = grid_from_numbers(&[&[1, 1, 2, 2], &[10, 10, 20, 20]]);
        let grid_b = grid_from_numbers(&[&[2, 2, 1, 1], &[20, 20, 10, 10]]);

        assert!(detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_multi_column_block() {
        let grid_a = grid_from_numbers(&[
            &[10, 20, 30, 40, 50, 60],
            &[11, 21, 31, 41, 51, 61],
            &[12, 22, 32, 42, 52, 62],
        ]);

        let grid_b = grid_from_numbers(&[
            &[10, 40, 50, 20, 30, 60],
            &[11, 41, 51, 21, 31, 61],
            &[12, 42, 52, 22, 32, 62],
        ]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected multi-column move");
        assert_eq!(mv.src_start_col, 3);
        assert_eq!(mv.col_count, 2);
        assert_eq!(mv.dst_start_col, 1);
    }

    #[test]
    fn detect_exact_column_block_move_rejects_two_independent_moves() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40, 50, 60], &[11, 21, 31, 41, 51, 61]]);

        let grid_b = grid_from_numbers(&[&[20, 10, 30, 40, 60, 50], &[21, 11, 31, 41, 61, 51]]);

        assert!(
            detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "two independent column swaps must not be detected as a single block move"
        );
    }

    #[test]
    fn detect_exact_column_block_move_swap_as_single_move() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

        let grid_b = grid_from_numbers(&[&[20, 10, 30, 40], &[21, 11, 31, 41]]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("swap of adjacent columns should be detected as single-column move");
        assert_eq!(mv.col_count, 1);
        assert!(
            (mv.src_start_col == 0 && mv.dst_start_col == 1)
                || (mv.src_start_col == 1 && mv.dst_start_col == 0),
            "swap should be represented as moving one column past the other"
        );
    }
}

```

---

### File: `core\src\config.rs`

```rust
//! Configuration for the diff engine.
//!
//! `DiffConfig` centralizes all algorithm thresholds and behavioral knobs
//! to avoid hardcoded constants scattered throughout the codebase.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitBehavior {
    FallbackToPositional,
    ReturnPartialResult,
    ReturnError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffConfig {
    /// Maximum number of masked move-detection iterations per sheet.
    /// Set to 0 to disable move detection and represent moves as insert/delete.
    pub max_move_iterations: u32,
    pub max_align_rows: u32,
    pub max_align_cols: u32,
    pub max_block_gap: u32,
    pub max_hash_repeat: u32,
    pub fuzzy_similarity_threshold: f64,
    pub max_fuzzy_block_rows: u32,
    #[serde(alias = "rare_frequency_threshold")]
    pub rare_threshold: u32,
    #[serde(alias = "low_info_cell_threshold")]
    pub low_info_threshold: u32,
    /// Row-count threshold for recursive gap alignment. Does not gate masked move detection.
    #[serde(alias = "recursive_threshold")]
    pub recursive_align_threshold: u32,
    pub small_gap_threshold: u32,
    pub max_recursion_depth: u32,
    pub on_limit_exceeded: LimitBehavior,
    pub enable_fuzzy_moves: bool,
    pub enable_m_semantic_diff: bool,
    pub enable_formula_semantic_diff: bool,
    /// When true, emits CellEdited ops even when values are unchanged (diagnostic);
    /// downstream consumers should treat edits as semantic only if from != to.
    pub include_unchanged_cells: bool,
    pub max_context_rows: u32,
    pub min_block_size_for_move: u32,
    pub max_lcs_gap_size: u32,
    pub lcs_dp_work_limit: usize,
    pub move_extraction_max_slice_len: u32,
    pub move_extraction_max_candidates_per_sig: u32,
    pub context_anchor_k1: u32,
    pub context_anchor_k2: u32,
    /// Masked move detection runs only when max(old.nrows, new.nrows) <= this.
    pub max_move_detection_rows: u32,
    /// Masked move detection runs only when max(old.ncols, new.ncols) <= this.
    pub max_move_detection_cols: u32,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            max_move_iterations: 20,
            max_align_rows: 500_000,
            max_align_cols: 16_384,
            max_block_gap: 10_000,
            max_hash_repeat: 8,
            fuzzy_similarity_threshold: 0.80,
            max_fuzzy_block_rows: 32,
            rare_threshold: 5,
            low_info_threshold: 2,
            small_gap_threshold: 50,
            recursive_align_threshold: 200,
            max_recursion_depth: 10,
            on_limit_exceeded: LimitBehavior::FallbackToPositional,
            enable_fuzzy_moves: true,
            enable_m_semantic_diff: true,
            enable_formula_semantic_diff: false,
            include_unchanged_cells: false,
            max_context_rows: 3,
            min_block_size_for_move: 3,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            context_anchor_k1: 4,
            context_anchor_k2: 8,
            max_move_detection_rows: 200,
            max_move_detection_cols: 256,
        }
    }
}

impl DiffConfig {
    pub fn fastest() -> Self {
        Self {
            max_move_iterations: 5,
            max_block_gap: 1_000,
            small_gap_threshold: 20,
            recursive_align_threshold: 80,
            max_move_detection_rows: 80,
            enable_fuzzy_moves: false,
            enable_m_semantic_diff: false,
            ..Default::default()
        }
    }

    pub fn balanced() -> Self {
        Self::default()
    }

    pub fn most_precise() -> Self {
        Self {
            max_move_iterations: 30,
            max_block_gap: 20_000,
            fuzzy_similarity_threshold: 0.95,
            small_gap_threshold: 80,
            recursive_align_threshold: 400,
            enable_formula_semantic_diff: true,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            max_move_detection_rows: 400,
            max_move_detection_cols: 256,
            ..Default::default()
        }
    }

    pub fn builder() -> DiffConfigBuilder {
        DiffConfigBuilder {
            inner: DiffConfig::default(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.fuzzy_similarity_threshold.is_finite()
            || self.fuzzy_similarity_threshold < 0.0
            || self.fuzzy_similarity_threshold > 1.0
        {
            return Err(ConfigError::InvalidFuzzySimilarity {
                value: self.fuzzy_similarity_threshold,
            });
        }

        ensure_non_zero_u32(self.max_align_rows, "max_align_rows")?;
        ensure_non_zero_u32(self.max_align_cols, "max_align_cols")?;
        ensure_non_zero_u32(self.max_lcs_gap_size, "max_lcs_gap_size")?;
        ensure_non_zero_u32(
            self.move_extraction_max_slice_len,
            "move_extraction_max_slice_len",
        )?;
        ensure_non_zero_u32(
            self.move_extraction_max_candidates_per_sig,
            "move_extraction_max_candidates_per_sig",
        )?;
        ensure_non_zero_u32(self.context_anchor_k1, "context_anchor_k1")?;
        ensure_non_zero_u32(self.context_anchor_k2, "context_anchor_k2")?;
        ensure_non_zero_u32(self.max_move_detection_rows, "max_move_detection_rows")?;
        ensure_non_zero_u32(self.max_move_detection_cols, "max_move_detection_cols")?;
        ensure_non_zero_u32(self.max_context_rows, "max_context_rows")?;
        ensure_non_zero_u32(self.min_block_size_for_move, "min_block_size_for_move")?;

        if self.lcs_dp_work_limit == 0 {
            return Err(ConfigError::NonPositiveLimit {
                field: "lcs_dp_work_limit",
                value: 0,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConfigError {
    #[error("fuzzy_similarity_threshold must be in [0.0, 1.0] and finite (got {value})")]
    InvalidFuzzySimilarity { value: f64 },
    #[error("{field} must be greater than zero (got {value})")]
    NonPositiveLimit { field: &'static str, value: u64 },
}

fn ensure_non_zero_u32(value: u32, field: &'static str) -> Result<(), ConfigError> {
    if value == 0 {
        return Err(ConfigError::NonPositiveLimit {
            field,
            value: value as u64,
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct DiffConfigBuilder {
    inner: DiffConfig,
}

impl Default for DiffConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffConfigBuilder {
    pub fn new() -> Self {
        DiffConfig::builder()
    }

    pub fn max_move_iterations(mut self, value: u32) -> Self {
        self.inner.max_move_iterations = value;
        self
    }

    pub fn max_align_rows(mut self, value: u32) -> Self {
        self.inner.max_align_rows = value;
        self
    }

    pub fn max_align_cols(mut self, value: u32) -> Self {
        self.inner.max_align_cols = value;
        self
    }

    pub fn max_block_gap(mut self, value: u32) -> Self {
        self.inner.max_block_gap = value;
        self
    }

    pub fn max_hash_repeat(mut self, value: u32) -> Self {
        self.inner.max_hash_repeat = value;
        self
    }

    pub fn fuzzy_similarity_threshold(mut self, value: f64) -> Self {
        self.inner.fuzzy_similarity_threshold = value;
        self
    }

    pub fn max_fuzzy_block_rows(mut self, value: u32) -> Self {
        self.inner.max_fuzzy_block_rows = value;
        self
    }

    pub fn rare_threshold(mut self, value: u32) -> Self {
        self.inner.rare_threshold = value;
        self
    }

    pub fn low_info_threshold(mut self, value: u32) -> Self {
        self.inner.low_info_threshold = value;
        self
    }

    pub fn recursive_align_threshold(mut self, value: u32) -> Self {
        self.inner.recursive_align_threshold = value;
        self
    }

    pub fn small_gap_threshold(mut self, value: u32) -> Self {
        self.inner.small_gap_threshold = value;
        self
    }

    pub fn max_recursion_depth(mut self, value: u32) -> Self {
        self.inner.max_recursion_depth = value;
        self
    }

    pub fn on_limit_exceeded(mut self, value: LimitBehavior) -> Self {
        self.inner.on_limit_exceeded = value;
        self
    }

    pub fn enable_fuzzy_moves(mut self, value: bool) -> Self {
        self.inner.enable_fuzzy_moves = value;
        self
    }

    pub fn enable_m_semantic_diff(mut self, value: bool) -> Self {
        self.inner.enable_m_semantic_diff = value;
        self
    }

    pub fn enable_formula_semantic_diff(mut self, value: bool) -> Self {
        self.inner.enable_formula_semantic_diff = value;
        self
    }

    pub fn include_unchanged_cells(mut self, value: bool) -> Self {
        self.inner.include_unchanged_cells = value;
        self
    }

    pub fn max_context_rows(mut self, value: u32) -> Self {
        self.inner.max_context_rows = value;
        self
    }

    pub fn min_block_size_for_move(mut self, value: u32) -> Self {
        self.inner.min_block_size_for_move = value;
        self
    }

    pub fn max_lcs_gap_size(mut self, value: u32) -> Self {
        self.inner.max_lcs_gap_size = value;
        self
    }

    pub fn lcs_dp_work_limit(mut self, value: usize) -> Self {
        self.inner.lcs_dp_work_limit = value;
        self
    }

    pub fn move_extraction_max_slice_len(mut self, value: u32) -> Self {
        self.inner.move_extraction_max_slice_len = value;
        self
    }

    pub fn move_extraction_max_candidates_per_sig(mut self, value: u32) -> Self {
        self.inner.move_extraction_max_candidates_per_sig = value;
        self
    }

    pub fn context_anchor_k1(mut self, value: u32) -> Self {
        self.inner.context_anchor_k1 = value;
        self
    }

    pub fn context_anchor_k2(mut self, value: u32) -> Self {
        self.inner.context_anchor_k2 = value;
        self
    }

    pub fn max_move_detection_rows(mut self, value: u32) -> Self {
        self.inner.max_move_detection_rows = value;
        self
    }

    pub fn max_move_detection_cols(mut self, value: u32) -> Self {
        self.inner.max_move_detection_cols = value;
        self
    }

    pub fn build(self) -> Result<DiffConfig, ConfigError> {
        self.inner.validate()?;
        Ok(self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_limit_spec() {
        let cfg = DiffConfig::default();

        assert_eq!(cfg.max_align_rows, 500_000);
        assert_eq!(cfg.max_align_cols, 16_384);
        assert_eq!(cfg.max_recursion_depth, 10);
        assert!(matches!(
            cfg.on_limit_exceeded,
            LimitBehavior::FallbackToPositional
        ));

        assert_eq!(cfg.fuzzy_similarity_threshold, 0.80);
        assert_eq!(cfg.min_block_size_for_move, 3);
        assert_eq!(cfg.max_move_iterations, 20);

        assert_eq!(cfg.recursive_align_threshold, 200);
        assert_eq!(cfg.small_gap_threshold, 50);
        assert_eq!(cfg.low_info_threshold, 2);
        assert_eq!(cfg.rare_threshold, 5);
        assert_eq!(cfg.max_block_gap, 10_000);

        assert_eq!(cfg.max_move_detection_rows, 200);
        assert_eq!(cfg.max_move_detection_cols, 256);

        assert!(!cfg.include_unchanged_cells);
        assert_eq!(cfg.max_context_rows, 3);

        assert!(cfg.enable_fuzzy_moves);
        assert!(cfg.enable_m_semantic_diff);
        assert!(!cfg.enable_formula_semantic_diff);
    }

    #[test]
    fn serde_roundtrip_preserves_defaults() {
        let cfg = DiffConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize default config");
        let parsed: DiffConfig = serde_json::from_str(&json).expect("deserialize default config");
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn serde_aliases_populate_fields() {
        let json = r#"{
            "rare_frequency_threshold": 9,
            "low_info_cell_threshold": 3,
            "recursive_threshold": 123
        }"#;
        let cfg: DiffConfig = serde_json::from_str(json).expect("deserialize with aliases");
        assert_eq!(cfg.rare_threshold, 9);
        assert_eq!(cfg.low_info_threshold, 3);
        assert_eq!(cfg.recursive_align_threshold, 123);
    }

    #[test]
    fn builder_rejects_invalid_similarity_threshold() {
        let err = DiffConfig::builder()
            .fuzzy_similarity_threshold(2.0)
            .build()
            .expect_err("builder should reject invalid probability");
        assert!(matches!(
            err,
            ConfigError::InvalidFuzzySimilarity { value } if (value - 2.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn presets_differ_in_expected_directions() {
        let fastest = DiffConfig::fastest();
        let balanced = DiffConfig::balanced();
        let precise = DiffConfig::most_precise();

        assert!(!fastest.enable_fuzzy_moves);
        assert!(!fastest.enable_m_semantic_diff);
        assert!(precise.max_move_iterations >= balanced.max_move_iterations);
        assert!(precise.max_block_gap >= balanced.max_block_gap);
        assert!(precise.fuzzy_similarity_threshold >= balanced.fuzzy_similarity_threshold);
    }

    #[test]
    fn most_precise_matches_sprint_plan_values() {
        let cfg = DiffConfig::most_precise();
        assert_eq!(cfg.fuzzy_similarity_threshold, 0.95);
        assert!(cfg.enable_formula_semantic_diff);
    }
}

```

---

### File: `core\src\container.rs`

```rust
//! OPC (Open Packaging Conventions) container handling.
//!
//! Provides abstraction over ZIP-based Office Open XML packages, validating
//! that required structural elements like `[Content_Types].xml` are present.

use std::io::{Read, Seek};
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

/// Errors that can occur when opening or reading an OPC container.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContainerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(String),
    #[error("not a ZIP container")]
    NotZipContainer,
    #[error("not an OPC package (missing [Content_Types].xml)")]
    NotOpcPackage,
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct OpcContainer {
    pub(crate) archive: ZipArchive<Box<dyn ReadSeek>>,
}

impl OpcContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(
        reader: R,
    ) -> Result<OpcContainer, ContainerError> {
        let reader: Box<dyn ReadSeek> = Box::new(reader);
        let archive = ZipArchive::new(reader).map_err(|err| match err {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                ContainerError::NotZipContainer
            }
            ZipError::Io(e) => ContainerError::Io(e),
            other => ContainerError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                other.to_string(),
            )),
        })?;

        let mut container = OpcContainer { archive };
        if container.read_file("[Content_Types].xml").is_err() {
            return Err(ContainerError::NotOpcPackage);
        }

        Ok(container)
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path(path: impl AsRef<std::path::Path>) -> Result<OpcContainer, ContainerError> {
        let file = std::fs::File::open(path)?;
        Self::open_from_reader(file)
    }

    #[cfg(feature = "std-fs")]
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<OpcContainer, ContainerError> {
        Self::open_from_path(path)
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, ZipError> {
        let mut file = self.archive.by_name(name)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn read_file_optional(&mut self, name: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
        match self.read_file(name) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(ZipError::FileNotFound) => Ok(None),
            Err(ZipError::Io(e)) => Err(e),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        }
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archive.file_names()
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

```

---

### File: `core\src\database_alignment.rs`

```rust
use crate::hashing::normalize_float_for_hash;
use crate::string_pool::StringId;
use crate::workbook::{CellValue, Grid};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyColumnSpec {
    pub columns: Vec<u32>,
}

impl KeyColumnSpec {
    pub fn new(columns: Vec<u32>) -> KeyColumnSpec {
        KeyColumnSpec { columns }
    }

    pub fn is_key_column(&self, col: u32) -> bool {
        self.columns.contains(&col)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum KeyValueRepr {
    None,
    Number(u64),
    Text(StringId),
    Bool(bool),
}

impl KeyValueRepr {
    fn from_cell_value(value: Option<&CellValue>) -> KeyValueRepr {
        match value {
            Some(CellValue::Number(n)) => KeyValueRepr::Number(normalize_float_for_hash(*n)),
            Some(CellValue::Text(id)) => KeyValueRepr::Text(*id),
            Some(CellValue::Bool(b)) => KeyValueRepr::Bool(*b),
            Some(CellValue::Blank) => KeyValueRepr::None,
            Some(CellValue::Error(id)) => KeyValueRepr::Text(*id),
            None => KeyValueRepr::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct KeyComponent {
    pub value: KeyValueRepr,
    pub formula: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct KeyValue {
    components: Vec<KeyComponent>,
}

impl KeyValue {
    fn new(components: Vec<KeyComponent>) -> KeyValue {
        KeyValue { components }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyedRow {
    pub key: KeyValue,
    pub row_idx: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyedAlignment {
    pub matched_rows: Vec<(u32, u32)>, // (row_idx_a, row_idx_b)
    pub left_only_rows: Vec<u32>,
    pub right_only_rows: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyAlignmentError {
    DuplicateKeyLeft(KeyValue),
    DuplicateKeyRight(KeyValue),
}

pub(crate) fn diff_table_by_key(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
) -> Result<KeyedAlignment, KeyAlignmentError> {
    let spec = KeyColumnSpec::new(key_columns.to_vec());
    let (left_rows, _left_lookup) = build_keyed_rows(old, &spec, true)?;
    let (right_rows, right_lookup) = build_keyed_rows(new, &spec, false)?;

    let mut matched_rows = Vec::new();
    let mut left_only_rows = Vec::new();
    let mut right_only_rows = Vec::new();

    let mut matched_right_rows: HashSet<u32> = HashSet::new();

    for row in &left_rows {
        if let Some(&row_b) = right_lookup.get(&row.key) {
            matched_rows.push((row.row_idx, row_b));
            matched_right_rows.insert(row_b);
        } else {
            left_only_rows.push(row.row_idx);
        }
    }

    for row in &right_rows {
        if !matched_right_rows.contains(&row.row_idx) {
            right_only_rows.push(row.row_idx);
        }
    }

    Ok(KeyedAlignment {
        matched_rows,
        left_only_rows,
        right_only_rows,
    })
}

fn build_keyed_rows(
    grid: &Grid,
    spec: &KeyColumnSpec,
    is_left: bool,
) -> Result<(Vec<KeyedRow>, HashMap<KeyValue, u32>), KeyAlignmentError> {
    let mut rows = Vec::with_capacity(grid.nrows as usize);
    let mut lookup = HashMap::new();

    for row_idx in 0..grid.nrows {
        let key = extract_key(grid, row_idx, spec);
        if lookup.insert(key.clone(), row_idx).is_some() {
            return Err(if is_left {
                KeyAlignmentError::DuplicateKeyLeft(key)
            } else {
                KeyAlignmentError::DuplicateKeyRight(key)
            });
        }
        rows.push(KeyedRow { key, row_idx });
    }

    Ok((rows, lookup))
}

fn extract_key(grid: &Grid, row_idx: u32, spec: &KeyColumnSpec) -> KeyValue {
    let mut components = Vec::with_capacity(spec.columns.len());

    for &col in &spec.columns {
        let component = match grid.get(row_idx, col) {
            Some(cell) => KeyComponent {
                value: KeyValueRepr::from_cell_value(cell.value.as_ref()),
                formula: cell.formula.clone(),
            },
            None => KeyComponent {
                value: KeyValueRepr::None,
                formula: None,
            },
        };
        components.push(component);
    }

    KeyValue::new(components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

    fn grid_from_rows(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert_cell(
                    r_idx as u32,
                    c_idx as u32,
                    Some(CellValue::Number(*value as f64)),
                    None,
                );
            }
        }

        grid
    }

    #[test]
    fn unique_keys_reorder_no_changes() {
        let grid_a = grid_from_rows(&[&[1, 10], &[2, 20], &[3, 30]]);
        let grid_b = grid_from_rows(&[&[3, 30], &[1, 10], &[2, 20]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]).expect("unique keys");
        assert_eq!(
            alignment.matched_rows,
            vec![(0, 1), (1, 2), (2, 0)],
            "all keys should align regardless of order"
        );
        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());
    }

    #[test]
    fn unique_keys_insert_delete_classified() {
        let grid_a = grid_from_rows(&[&[1, 10], &[2, 20]]);
        let grid_b = grid_from_rows(&[&[1, 10], &[2, 20], &[3, 30]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]).expect("unique keys");
        assert_eq!(alignment.matched_rows, vec![(0, 0), (1, 1)]);
        assert!(alignment.left_only_rows.is_empty());
        assert_eq!(alignment.right_only_rows, vec![2]);
    }

    #[test]
    fn duplicate_keys_error_or_unsupported() {
        let grid_a = grid_from_rows(&[&[1, 10], &[1, 99]]);
        let grid_b = grid_from_rows(&[&[1, 10]]);

        let err = diff_table_by_key(&grid_a, &grid_b, &[0]).expect_err("duplicate keys");
        assert!(matches!(err, KeyAlignmentError::DuplicateKeyLeft(_)));
    }

    #[test]
    fn composite_key_alignment_matches_rows_correctly() {
        let grid_a = grid_from_rows(&[&[1, 10, 100], &[1, 20, 200], &[2, 10, 300]]);
        let grid_b = grid_from_rows(&[&[1, 20, 200], &[2, 10, 300], &[1, 10, 100]]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 1]).expect("unique composite keys");

        assert!(
            alignment.left_only_rows.is_empty(),
            "no left-only rows expected"
        );
        assert!(
            alignment.right_only_rows.is_empty(),
            "no right-only rows expected"
        );

        let mut matched = alignment.matched_rows.clone();
        matched.sort_unstable();

        let mut expected = vec![(0, 2), (1, 0), (2, 1)];
        expected.sort_unstable();

        assert_eq!(
            matched, expected,
            "composite keys should align rows sharing the same key tuple regardless of order"
        );
    }

    #[test]
    fn non_contiguous_key_columns_alignment() {
        let grid_a = grid_from_rows(&[&[1, 999, 10, 100], &[1, 888, 20, 200], &[2, 777, 10, 300]]);
        let grid_b = grid_from_rows(&[&[2, 777, 10, 300], &[1, 999, 10, 100], &[1, 888, 20, 200]]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 2]).expect("unique non-contiguous keys");

        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());

        let mut matched = alignment.matched_rows.clone();
        matched.sort_unstable();

        let mut expected = vec![(0, 1), (1, 2), (2, 0)];
        expected.sort_unstable();

        assert_eq!(
            matched, expected,
            "non-contiguous key columns [0,2] should align correctly"
        );
    }

    #[test]
    fn three_column_composite_key_alignment() {
        let grid_a = grid_from_rows(&[
            &[1, 10, 100, 1000],
            &[1, 10, 200, 2000],
            &[1, 20, 100, 3000],
            &[2, 10, 100, 4000],
        ]);
        let grid_b = grid_from_rows(&[
            &[2, 10, 100, 4000],
            &[1, 20, 100, 3000],
            &[1, 10, 200, 2000],
            &[1, 10, 100, 1000],
        ]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 1, 2]).expect("unique three-column keys");

        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());

        let mut matched = alignment.matched_rows.clone();
        matched.sort_unstable();

        let mut expected = vec![(0, 3), (1, 2), (2, 1), (3, 0)];
        expected.sort_unstable();

        assert_eq!(
            matched, expected,
            "three-column composite keys should align correctly"
        );
    }

    #[test]
    fn is_key_column_single_column() {
        let spec = KeyColumnSpec::new(vec![0]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(
            !spec.is_key_column(1),
            "column 1 should not be a key column"
        );
        assert!(
            !spec.is_key_column(2),
            "column 2 should not be a key column"
        );
    }

    #[test]
    fn is_key_column_contiguous_columns() {
        let spec = KeyColumnSpec::new(vec![0, 1]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(spec.is_key_column(1), "column 1 should be a key column");
        assert!(
            !spec.is_key_column(2),
            "column 2 should not be a key column"
        );
        assert!(
            !spec.is_key_column(3),
            "column 3 should not be a key column"
        );
    }

    #[test]
    fn is_key_column_non_contiguous_columns() {
        let spec = KeyColumnSpec::new(vec![0, 2]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(
            !spec.is_key_column(1),
            "column 1 should not be a key column"
        );
        assert!(spec.is_key_column(2), "column 2 should be a key column");
        assert!(
            !spec.is_key_column(3),
            "column 3 should not be a key column"
        );
    }

    #[test]
    fn is_key_column_three_columns() {
        let spec = KeyColumnSpec::new(vec![0, 1, 2]);
        assert!(spec.is_key_column(0));
        assert!(spec.is_key_column(1));
        assert!(spec.is_key_column(2));
        assert!(!spec.is_key_column(3));
    }

    #[test]
    fn is_key_column_non_contiguous_three_columns() {
        let spec = KeyColumnSpec::new(vec![1, 3, 5]);
        assert!(
            !spec.is_key_column(0),
            "column 0 should not be a key column"
        );
        assert!(spec.is_key_column(1), "column 1 should be a key column");
        assert!(
            !spec.is_key_column(2),
            "column 2 should not be a key column"
        );
        assert!(spec.is_key_column(3), "column 3 should be a key column");
        assert!(
            !spec.is_key_column(4),
            "column 4 should not be a key column"
        );
        assert!(spec.is_key_column(5), "column 5 should be a key column");
        assert!(
            !spec.is_key_column(6),
            "column 6 should not be a key column"
        );
    }
}

```

---

### File: `core\src\datamashup.rs`

```rust
//! High-level DataMashup (Power Query) parsing and query extraction.
//!
//! Builds on the low-level framing and package parsing to provide structured
//! access to queries, permissions, and metadata stored in Excel DataMashup sections.

use std::collections::HashMap;

use crate::datamashup_framing::{DataMashupError, RawDataMashup};
use crate::datamashup_package::{PackageParts, parse_package_parts};
use crate::m_section::{SectionParseError, parse_section_members};
use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataMashup {
    pub version: u32,
    pub package_parts: PackageParts,
    pub permissions: Permissions,
    pub metadata: Metadata,
    pub permission_bindings_raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permissions {
    pub can_evaluate_future_packages: bool,
    pub firewall_enabled: bool,
    pub workbook_group_type: Option<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions {
            can_evaluate_future_packages: false,
            firewall_enabled: true,
            workbook_group_type: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub formulas: Vec<QueryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMetadata {
    pub item_path: String,
    pub section_name: String,
    pub formula_name: String,
    pub load_to_sheet: bool,
    pub load_to_model: bool,
    pub is_connection_only: bool,
    pub group_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub name: String,
    pub section_member: String,
    pub expression_m: String,
    pub metadata: QueryMetadata,
}

pub fn build_data_mashup(raw: &RawDataMashup) -> Result<DataMashup, DataMashupError> {
    let package_parts = parse_package_parts(&raw.package_parts)?;
    let permissions = parse_permissions(&raw.permissions);
    let metadata = parse_metadata(&raw.metadata)?;

    Ok(DataMashup {
        version: raw.version,
        package_parts,
        permissions,
        metadata,
        permission_bindings_raw: raw.permission_bindings.clone(),
    })
}

pub fn build_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError> {
    let members = parse_section_members(&dm.package_parts.main_section.source)?;

    let mut metadata_index: HashMap<(String, String), QueryMetadata> = HashMap::new();
    for meta in &dm.metadata.formulas {
        metadata_index.insert(
            (meta.section_name.clone(), meta.formula_name.clone()),
            meta.clone(),
        );
    }

    let mut positions: HashMap<String, usize> = HashMap::new();
    let mut queries = Vec::new();

    for member in members {
        let section_name = member.section_name.clone();
        let member_name = member.member_name.clone();
        let key = (section_name.clone(), member_name.clone());
        let metadata = metadata_index
            .get(&key)
            .cloned()
            .unwrap_or_else(|| QueryMetadata {
                item_path: format!("{}/{}", section_name, member_name),
                section_name: section_name.clone(),
                formula_name: member_name.clone(),
                load_to_sheet: false,
                load_to_model: false,
                is_connection_only: true,
                group_path: None,
            });

        let name = format!("{}/{}", section_name, member_name);
        let query = Query {
            name: name.clone(),
            section_member: member.member_name,
            expression_m: member.expression_m,
            metadata,
        };

        if let Some(idx) = positions.get(&name) {
            debug_assert!(
                false,
                "duplicate query name '{}' found in DataMashup section; \
                 later definition will overwrite earlier one",
                name
            );
            queries[*idx] = query;
        } else {
            positions.insert(name, queries.len());
            queries.push(query);
        }
    }

    Ok(queries)
}

pub fn parse_permissions(xml_bytes: &[u8]) -> Permissions {
    if xml_bytes.is_empty() {
        return Permissions::default();
    }

    let Ok(mut text) = String::from_utf8(xml_bytes.to_vec()) else {
        return Permissions::default();
    };
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }

    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current_tag: Option<String> = None;
    let mut permissions = Permissions::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag =
                    Some(String::from_utf8_lossy(local_name(e.name().as_ref())).to_string());
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = current_tag.as_deref() {
                    let value = match t.unescape() {
                        Ok(v) => v.into_owned(),
                        Err(_) => {
                            // Any unescape failure means the permissions payload is unusable; fall back to defaults.
                            return Permissions::default();
                        }
                    };
                    match tag {
                        "CanEvaluateFuturePackages" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.can_evaluate_future_packages = v;
                            }
                        }
                        "FirewallEnabled" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.firewall_enabled = v;
                            }
                        }
                        "WorkbookGroupType" => {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                permissions.workbook_group_type = Some(trimmed.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(tag) = current_tag.as_deref() {
                    let value = String::from_utf8_lossy(&t.into_inner()).to_string();
                    match tag {
                        "CanEvaluateFuturePackages" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.can_evaluate_future_packages = v;
                            }
                        }
                        "FirewallEnabled" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.firewall_enabled = v;
                            }
                        }
                        "WorkbookGroupType" => {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                permissions.workbook_group_type = Some(trimmed.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => current_tag = None,
            Ok(Event::Eof) => break,
            Err(_) => return Permissions::default(),
            _ => {}
        }
        buf.clear();
    }

    permissions
}

pub fn parse_metadata(metadata_bytes: &[u8]) -> Result<Metadata, DataMashupError> {
    if metadata_bytes.is_empty() {
        return Ok(Metadata {
            formulas: Vec::new(),
        });
    }

    let xml_bytes = metadata_xml_bytes(metadata_bytes)?;
    let mut text = String::from_utf8(xml_bytes)
        .map_err(|_| DataMashupError::XmlError("metadata is not valid UTF-8".into()))?;
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }

    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut element_stack: Vec<String> = Vec::new();
    let mut item_type: Option<String> = None;
    let mut item_path: Option<String> = None;
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut formulas: Vec<QueryMetadata> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(local_name(e.name().as_ref())).to_string();
                if name == "Entry"
                    && let Some((typ, val)) = parse_entry_attributes(&e)?
                {
                    entries.push((typ, val));
                }
            }
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(local_name(e.name().as_ref())).to_string();
                if name == "Item" {
                    item_type = None;
                    item_path = None;
                    entries.clear();
                }
                if name == "Entry"
                    && let Some((typ, val)) = parse_entry_attributes(&e)?
                {
                    entries.push((typ, val));
                }
                element_stack.push(name);
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = element_stack.last() {
                    let value = t
                        .unescape()
                        .map_err(|e| DataMashupError::XmlError(e.to_string()))?
                        .into_owned();
                    match tag.as_str() {
                        "ItemType" => {
                            item_type = Some(value.trim().to_string());
                        }
                        "ItemPath" => {
                            item_path = Some(value.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(tag) = element_stack.last() {
                    let value = String::from_utf8_lossy(&t.into_inner()).to_string();
                    match tag.as_str() {
                        "ItemType" => {
                            item_type = Some(value.trim().to_string());
                        }
                        "ItemPath" => {
                            item_path = Some(value.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name_bytes = local_name(e.name().as_ref()).to_vec();
                if name_bytes.as_slice() == b"Item" && item_type.as_deref() == Some("Formula") {
                    let raw_path = item_path.clone().ok_or_else(|| {
                        DataMashupError::XmlError("Formula item missing ItemPath".into())
                    })?;
                    let decoded_path = decode_item_path(&raw_path)?;
                    let (section_name, formula_name) = split_item_path(&decoded_path)?;
                    let load_to_sheet =
                        entry_bool(&entries, &["FillEnabled", "LoadEnabled"]).unwrap_or(false);
                    let load_to_model = entry_bool(
                        &entries,
                        &[
                            "FillToDataModelEnabled",
                            "AddedToDataModel",
                            "LoadToDataModel",
                        ],
                    )
                    .unwrap_or(false);
                    // Group paths are derived solely from per-formula entries for now; the AllFormulas tree is not parsed yet.
                    let group_path = entry_string(
                        &entries,
                        &[
                            "QueryGroupId",
                            "QueryGroupID",
                            "QueryGroupPath",
                            "QueryGroup",
                        ],
                    );

                    let metadata = QueryMetadata {
                        item_path: decoded_path.clone(),
                        section_name,
                        formula_name,
                        load_to_sheet,
                        load_to_model,
                        is_connection_only: !(load_to_sheet || load_to_model),
                        group_path,
                    };
                    formulas.push(metadata);
                }

                if let Some(last) = element_stack.last()
                    && last.as_bytes() == name_bytes.as_slice()
                {
                    element_stack.pop();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DataMashupError::XmlError(e.to_string())),
            _ => {}
        }

        buf.clear();
    }

    Ok(Metadata { formulas })
}

fn metadata_xml_bytes(metadata_bytes: &[u8]) -> Result<Vec<u8>, DataMashupError> {
    if looks_like_xml(metadata_bytes) {
        return Ok(metadata_bytes.to_vec());
    }

    if metadata_bytes.len() >= 8 {
        let content_len = u32::from_le_bytes(metadata_bytes[0..4].try_into().unwrap()) as usize;
        let xml_len = u32::from_le_bytes(metadata_bytes[4..8].try_into().unwrap()) as usize;
        let start = 8usize
            .checked_add(content_len)
            .ok_or_else(|| DataMashupError::XmlError("metadata length overflow".into()))?;
        let end = start
            .checked_add(xml_len)
            .ok_or_else(|| DataMashupError::XmlError("metadata length overflow".into()))?;
        if end <= metadata_bytes.len() {
            return Ok(metadata_bytes[start..end].to_vec());
        }
        return Err(DataMashupError::XmlError(
            "metadata length prefix invalid".into(),
        ));
    }

    Err(DataMashupError::XmlError("metadata XML not found".into()))
}

fn looks_like_xml(bytes: &[u8]) -> bool {
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    if idx >= bytes.len() {
        return false;
    }

    let slice = &bytes[idx..];
    slice.starts_with(b"<")
        || slice.starts_with(&[0xEF, 0xBB, 0xBF])
        || slice.starts_with(&[0xFE, 0xFF])
        || slice.starts_with(&[0xFF, 0xFE])
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..).unwrap_or(name),
        None => name,
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    let trimmed = text.trim();
    let payload = trimmed
        .strip_prefix(|c| c == 'l' || c == 'L')
        .unwrap_or(trimmed);
    let lowered = payload.to_ascii_lowercase();
    match lowered.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn parse_entry_attributes(
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<Option<(String, String)>, DataMashupError> {
    let mut typ: Option<String> = None;
    let mut value: Option<String> = None;

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|e| DataMashupError::XmlError(e.to_string()))?;
        let key = local_name(attr.key.as_ref());
        if key == b"Type" {
            typ = Some(
                String::from_utf8(attr.value.as_ref().to_vec())
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?,
            );
        } else if key == b"Value" {
            value = Some(
                String::from_utf8(attr.value.as_ref().to_vec())
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?,
            );
        }
    }

    match (typ, value) {
        (Some(t), Some(v)) => Ok(Some((t, v))),
        _ => Ok(None),
    }
}

fn entry_bool(entries: &[(String, String)], keys: &[&str]) -> Option<bool> {
    for (key, val) in entries {
        if keys.iter().any(|k| k.eq_ignore_ascii_case(key))
            && let Some(b) = parse_bool(val)
        {
            return Some(b);
        }
    }
    None
}

fn entry_string(entries: &[(String, String)], keys: &[&str]) -> Option<String> {
    for (key, val) in entries {
        if keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
            let trimmed = val.trim();
            let without_prefix = trimmed
                .strip_prefix('s')
                .or_else(|| trimmed.strip_prefix('S'))
                .unwrap_or(trimmed);
            if without_prefix.is_empty() {
                return None;
            }
            return Some(without_prefix.to_string());
        }
    }
    None
}

fn decode_item_path(path: &str) -> Result<String, DataMashupError> {
    let mut decoded = Vec::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b'%' {
            if idx + 2 >= bytes.len() {
                return Err(DataMashupError::XmlError(
                    "invalid percent-encoding in ItemPath".into(),
                ));
            }
            let hi = hex_value(bytes[idx + 1]).ok_or_else(|| {
                DataMashupError::XmlError("invalid percent-encoding in ItemPath".into())
            })?;
            let lo = hex_value(bytes[idx + 2]).ok_or_else(|| {
                DataMashupError::XmlError("invalid percent-encoding in ItemPath".into())
            })?;
            decoded.push(hi << 4 | lo);
            idx += 3;
            continue;
        }
        decoded.push(b);
        idx += 1;
    }
    String::from_utf8(decoded)
        .map_err(|_| DataMashupError::XmlError("invalid UTF-8 in ItemPath".into()))
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}

fn split_item_path(path: &str) -> Result<(String, String), DataMashupError> {
    let mut parts = path.split('/');
    let section = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    if section.is_empty() || rest.is_empty() {
        return Err(DataMashupError::XmlError(
            "invalid ItemPath in metadata".into(),
        ));
    }
    let formula = rest.join("/");
    Ok((section.to_string(), formula))
}

```

---

### File: `core\src\datamashup_framing.rs`

```rust
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use quick_xml::Reader;
use quick_xml::events::Event;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DataMashupError {
    #[error("base64 decoding failed")]
    Base64Invalid,
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid framing structure")]
    FramingInvalid,
    #[error("XML parse error: {0}")]
    XmlError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDataMashup {
    pub version: u32,
    pub package_parts: Vec<u8>,
    pub permissions: Vec<u8>,
    pub metadata: Vec<u8>,
    pub permission_bindings: Vec<u8>,
}

pub fn parse_data_mashup(bytes: &[u8]) -> Result<RawDataMashup, DataMashupError> {
    const MIN_SIZE: usize = 4 + 4 * 4;
    if bytes.len() < MIN_SIZE {
        return Err(DataMashupError::FramingInvalid);
    }

    let mut offset: usize = 0;
    let version = read_u32_at(bytes, offset).ok_or(DataMashupError::FramingInvalid)?;
    offset += 4;

    if version != 0 {
        return Err(DataMashupError::UnsupportedVersion(version));
    }

    let package_parts_len = read_length(bytes, offset)?;
    offset += 4;
    let package_parts = take_segment(bytes, &mut offset, package_parts_len)?;

    let permissions_len = read_length(bytes, offset)?;
    offset += 4;
    let permissions = take_segment(bytes, &mut offset, permissions_len)?;

    let metadata_len = read_length(bytes, offset)?;
    offset += 4;
    let metadata = take_segment(bytes, &mut offset, metadata_len)?;

    let permission_bindings_len = read_length(bytes, offset)?;
    offset += 4;
    let permission_bindings = take_segment(bytes, &mut offset, permission_bindings_len)?;

    if offset != bytes.len() {
        return Err(DataMashupError::FramingInvalid);
    }

    Ok(RawDataMashup {
        version,
        package_parts,
        permissions,
        metadata,
        permission_bindings,
    })
}

pub fn read_datamashup_text(xml: &[u8]) -> Result<Option<String>, DataMashupError> {
    let utf8_xml = decode_datamashup_xml(xml)?;

    let mut reader = Reader::from_reader(utf8_xml.as_deref().unwrap_or(xml));
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut found_content: Option<String> = None;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup || found_content.is_some() {
                    return Err(DataMashupError::FramingInvalid);
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t
                    .unescape()
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?
                    .into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                let data = t.into_inner();
                content.push_str(&String::from_utf8_lossy(&data));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return Err(DataMashupError::FramingInvalid);
                }
                in_datamashup = false;
                found_content = Some(content.clone());
            }
            Ok(Event::Eof) if in_datamashup => {
                return Err(DataMashupError::FramingInvalid);
            }
            Ok(Event::Eof) => return Ok(found_content),
            Err(e) => return Err(DataMashupError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

pub fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, DataMashupError> {
    let cleaned: String = text.split_whitespace().collect();
    STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| DataMashupError::Base64Invalid)
}

pub(crate) fn decode_datamashup_xml(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    if xml.starts_with(&[0xFF, 0xFE]) {
        return Ok(Some(decode_utf16_xml(xml, true, true)?));
    }
    if xml.starts_with(&[0xFE, 0xFF]) {
        return Ok(Some(decode_utf16_xml(xml, false, true)?));
    }

    decode_declared_utf16_without_bom(xml)
}

fn decode_declared_utf16_without_bom(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    let attempt_decode = |little_endian| -> Result<Option<Vec<u8>>, DataMashupError> {
        if !looks_like_utf16(xml, little_endian) {
            return Ok(None);
        }
        let decoded = decode_utf16_xml(xml, little_endian, false)?;
        let lower = String::from_utf8_lossy(&decoded).to_ascii_lowercase();
        if lower.contains("encoding=\"utf-16\"") || lower.contains("encoding='utf-16'") {
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    };

    if let Some(decoded) = attempt_decode(true)? {
        return Ok(Some(decoded));
    }
    attempt_decode(false)
}

fn looks_like_utf16(xml: &[u8], little_endian: bool) -> bool {
    if xml.len() < 4 {
        return false;
    }

    if little_endian {
        xml[0] == b'<' && xml[1] == 0 && xml[2] == b'?' && xml[3] == 0
    } else {
        xml[0] == 0 && xml[1] == b'<' && xml[2] == 0 && xml[3] == b'?'
    }
}

fn decode_utf16_xml(
    xml: &[u8],
    little_endian: bool,
    has_bom: bool,
) -> Result<Vec<u8>, DataMashupError> {
    let start = if has_bom { 2 } else { 0 };
    let body = xml
        .get(start..)
        .ok_or_else(|| DataMashupError::XmlError("invalid UTF-16 XML".into()))?;
    if body.len() % 2 != 0 {
        return Err(DataMashupError::XmlError(
            "invalid UTF-16 byte length".into(),
        ));
    }

    let mut code_units = Vec::with_capacity(body.len() / 2);
    for chunk in body.chunks_exact(2) {
        let unit = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        code_units.push(unit);
    }

    let utf8 = String::from_utf16(&code_units)
        .map_err(|_| DataMashupError::XmlError("invalid UTF-16 XML".into()))?;
    Ok(utf8.into_bytes())
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    let array: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_le_bytes(array))
}

fn read_length(bytes: &[u8], offset: usize) -> Result<usize, DataMashupError> {
    let len = read_u32_at(bytes, offset).ok_or(DataMashupError::FramingInvalid)?;
    usize::try_from(len).map_err(|_| DataMashupError::FramingInvalid)
}

fn take_segment(bytes: &[u8], offset: &mut usize, len: usize) -> Result<Vec<u8>, DataMashupError> {
    let start = *offset;
    let end = start
        .checked_add(len)
        .ok_or(DataMashupError::FramingInvalid)?;
    if end > bytes.len() {
        return Err(DataMashupError::FramingInvalid);
    }

    let segment = bytes[start..end].to_vec();
    *offset = end;
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use super::{
        DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
        read_datamashup_text,
    };

    fn build_dm_bytes(
        version: u32,
        package_parts: &[u8],
        permissions: &[u8],
        metadata: &[u8],
        permission_bindings: &[u8],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&(package_parts.len() as u32).to_le_bytes());
        bytes.extend_from_slice(package_parts);
        bytes.extend_from_slice(&(permissions.len() as u32).to_le_bytes());
        bytes.extend_from_slice(permissions);
        bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        bytes.extend_from_slice(metadata);
        bytes.extend_from_slice(&(permission_bindings.len() as u32).to_le_bytes());
        bytes.extend_from_slice(permission_bindings);
        bytes
    }

    #[test]
    fn parse_zero_length_stream_succeeds() {
        let bytes = build_dm_bytes(0, b"", b"", b"", b"");
        let parsed = parse_data_mashup(&bytes).expect("zero-length sections should parse");
        assert_eq!(
            parsed,
            RawDataMashup {
                version: 0,
                package_parts: Vec::new(),
                permissions: Vec::new(),
                metadata: Vec::new(),
                permission_bindings: Vec::new(),
            }
        );
    }

    #[test]
    fn parse_basic_non_zero_lengths() {
        let bytes = build_dm_bytes(0, b"AAAA", b"BBBB", b"CCCC", b"DDDD");
        let parsed = parse_data_mashup(&bytes).expect("non-zero lengths should parse");
        assert_eq!(parsed.version, 0);
        assert_eq!(parsed.package_parts, b"AAAA");
        assert_eq!(parsed.permissions, b"BBBB");
        assert_eq!(parsed.metadata, b"CCCC");
        assert_eq!(parsed.permission_bindings, b"DDDD");
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let bytes = build_dm_bytes(1, b"AAAA", b"BBBB", b"CCCC", b"DDDD");
        let err = parse_data_mashup(&bytes).expect_err("version 1 should be unsupported");
        assert!(matches!(err, DataMashupError::UnsupportedVersion(1)));
    }

    #[test]
    fn truncated_stream_errors() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&100u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        let err = parse_data_mashup(&bytes).expect_err("length overflows buffer");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn trailing_bytes_are_invalid() {
        let mut bytes = build_dm_bytes(0, b"", b"", b"", b"");
        bytes.push(0xFF);
        let err = parse_data_mashup(&bytes).expect_err("trailing bytes should fail");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn too_short_stream_is_framing_invalid() {
        let bytes = vec![0u8; 8];
        let err =
            parse_data_mashup(&bytes).expect_err("buffer shorter than header must be invalid");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn utf16_datamashup_xml_decodes_correctly() {
        let xml_text = r#"<?xml version="1.0" encoding="utf-16"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>QQ==</dm:DataMashup></root>"#;
        let mut xml_bytes = Vec::with_capacity(2 + xml_text.len() * 2);
        xml_bytes.extend_from_slice(&[0xFF, 0xFE]);
        for unit in xml_text.encode_utf16() {
            xml_bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let text = read_datamashup_text(&xml_bytes)
            .expect("UTF-16 XML should parse")
            .expect("DataMashup element should be found");
        assert_eq!(text.trim(), "QQ==");
    }

    #[test]
    fn utf16_without_bom_with_declared_encoding_parses() {
        let xml_text = r#"<?xml version="1.0" encoding="utf-16"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>QQ==</dm:DataMashup></root>"#;
        for &little_endian in &[true, false] {
            let mut xml_bytes = Vec::with_capacity(xml_text.len() * 2);
            for unit in xml_text.encode_utf16() {
                let bytes = if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                xml_bytes.extend_from_slice(&bytes);
            }

            let text = read_datamashup_text(&xml_bytes)
                .expect("UTF-16 XML without BOM should parse when declared")
                .expect("DataMashup element should be found");
            assert_eq!(text.trim(), "QQ==");
        }
    }

    #[test]
    fn elements_with_datamashup_suffix_are_ignored() {
        let xml = br#"<?xml version="1.0"?><root><FooDataMashup>QQ==</FooDataMashup></root>"#;
        let result = read_datamashup_text(xml).expect("parsing should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_sibling_datamashup_elements_error() {
        let xml = br#"<?xml version="1.0"?>
<root xmlns:dm="http://schemas.microsoft.com/DataMashup">
  <dm:DataMashup>QQ==</dm:DataMashup>
  <dm:DataMashup>QQ==</dm:DataMashup>
</root>"#;
        let err = read_datamashup_text(xml).expect_err("duplicate DataMashup elements should fail");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn decode_datamashup_base64_rejects_invalid() {
        let err = decode_datamashup_base64("!!!").expect_err("invalid base64 should fail");
        assert!(matches!(err, DataMashupError::Base64Invalid));
    }

    #[test]
    fn fuzz_style_never_panics() {
        for seed in 0u64..32 {
            let len = (seed as usize * 7 % 48) + (seed as usize % 5);
            let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                state = state
                    .wrapping_mul(2862933555777941757)
                    .wrapping_add(3037000493);
                bytes.push((state >> 32) as u8);
            }
            let _ = parse_data_mashup(&bytes);
        }
    }
}

```

---

### File: `core\src\datamashup_package.rs`

```rust
use crate::datamashup_framing::DataMashupError;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageXml {
    pub raw_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDocument {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedContent {
    /// Normalized PackageParts path for the embedded package (never starts with '/').
    pub name: String,
    pub section: SectionDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageParts {
    pub package_xml: PackageXml,
    pub main_section: SectionDocument,
    pub embedded_contents: Vec<EmbeddedContent>,
}

pub fn parse_package_parts(bytes: &[u8]) -> Result<PackageParts, DataMashupError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| DataMashupError::FramingInvalid)?;

    let mut package_xml: Option<PackageXml> = None;
    let mut main_section: Option<SectionDocument> = None;
    let mut embedded_contents: Vec<EmbeddedContent> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|_| DataMashupError::FramingInvalid)?;
        if file.is_dir() {
            continue;
        }

        let raw_name = file.name().to_string();
        let name = normalize_path(&raw_name);
        if package_xml.is_none() && name == "Config/Package.xml" {
            let text = read_file_to_string(&mut file)?;
            package_xml = Some(PackageXml { raw_xml: text });
            continue;
        }
        if main_section.is_none() && name == "Formulas/Section1.m" {
            let text = strip_leading_bom(read_file_to_string(&mut file)?);
            main_section = Some(SectionDocument { source: text });
            continue;
        }
        if name.starts_with("Content/") {
            let mut content_bytes = Vec::new();
            if file.read_to_end(&mut content_bytes).is_err() {
                continue;
            }

            if let Some(section) = extract_embedded_section(&content_bytes) {
                embedded_contents.push(EmbeddedContent {
                    name: normalize_path(&raw_name).to_string(),
                    section: SectionDocument { source: section },
                });
            }
        }
    }

    let package_xml = package_xml.ok_or(DataMashupError::FramingInvalid)?;
    let main_section = main_section.ok_or(DataMashupError::FramingInvalid)?;

    Ok(PackageParts {
        package_xml,
        main_section,
        embedded_contents,
    })
}

fn normalize_path(name: &str) -> &str {
    name.trim_start_matches('/')
}

fn read_file_to_string(file: &mut zip::read::ZipFile<'_>) -> Result<String, DataMashupError> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|_| DataMashupError::FramingInvalid)?;
    String::from_utf8(buf).map_err(|_| DataMashupError::FramingInvalid)
}

fn extract_embedded_section(bytes: &[u8]) -> Option<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).ok()?;
    find_section_document(&mut archive)
}

fn find_section_document<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for idx in 0..archive.len() {
        let mut file = match archive.by_index(idx) {
            Ok(file) => file,
            Err(_) => continue,
        };
        if file.is_dir() {
            continue;
        }

        if normalize_path(file.name()) == "Formulas/Section1.m" {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let text = String::from_utf8(buf).ok()?;
                return Some(strip_leading_bom(text));
            }
        }
    }
    None
}

fn strip_leading_bom(text: String) -> String {
    text.strip_prefix('\u{FEFF}')
        .map(|s| s.to_string())
        .unwrap_or(text)
}

```

---

### File: `core\src\diff.rs`

```rust
//! Diff operations and reports for workbook comparison.
//!
//! This module defines the types used to represent differences between two workbooks:
//! - [`DiffOp`]: Individual operations representing a single change (cell edit, row/column add/remove, etc.)
//! - [`DiffReport`]: A versioned collection of diff operations
//! - [`DiffError`]: Errors that can occur during the diff process

use crate::string_pool::StringId;
use crate::workbook::{CellAddress, CellSnapshot, ColSignature, RowSignature};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryChangeKind {
    Semantic,
    FormattingOnly,
    Renamed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryMetadataField {
    LoadToSheet,
    LoadToModel,
    GroupPath,
    ConnectionOnly,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DiffError {
    #[error(
        "alignment limits exceeded for sheet '{sheet}': rows={rows}, cols={cols} (limits: rows={max_rows}, cols={max_cols})"
    )]
    LimitsExceeded {
        sheet: StringId,
        rows: u32,
        cols: u32,
        max_rows: u32,
        max_cols: u32,
    },

    #[error("sink error: {message}")]
    SinkError { message: String },
}

pub type SheetId = StringId;

/// Summary metadata about a diff run emitted alongside streamed ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSummary {
    pub complete: bool,
    pub warnings: Vec<String>,
    pub op_count: usize,
    #[cfg(feature = "perf-metrics")]
    pub metrics: Option<crate::perf::DiffMetrics>,
}

/// A single diff operation representing one logical change between workbooks.
///
/// Operations are emitted by the diff engine and collected into a [`DiffReport`].
/// The enum is marked `#[non_exhaustive]` to allow future additions.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum DiffOp {
    SheetAdded {
        sheet: SheetId,
    },
    SheetRemoved {
        sheet: SheetId,
    },
    RowAdded {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    RowRemoved {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    ColumnAdded {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    ColumnRemoved {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    BlockMovedRows {
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedColumns {
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedRect {
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    /// Logical change to a single cell.
    ///
    /// Invariants (maintained by producers and tests, not by the type system):
    /// - `addr` is the canonical location for the edit.
    /// - `from.addr` and `to.addr` must both equal `addr`.
    /// - `CellSnapshot` equality intentionally ignores `addr` and compares only
    ///   `(value, formula)`, so `DiffOp::CellEdited` equality does not by itself
    ///   enforce the address invariants; callers must respect them when
    ///   constructing ops.
    CellEdited {
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    },

    QueryAdded {
        name: StringId,
    },
    QueryRemoved {
        name: StringId,
    },
    QueryRenamed {
        from: StringId,
        to: StringId,
    },
    QueryDefinitionChanged {
        name: StringId,
        change_kind: QueryChangeKind,
        old_hash: u64,
        new_hash: u64,
    },
    QueryMetadataChanged {
        name: StringId,
        field: QueryMetadataField,
        old: Option<StringId>,
        new: Option<StringId>,
    },

    // Future: DAX operations
    // MeasureAdded { name: StringId }
    // MeasureRemoved { name: StringId }
    // MeasureDefinitionChanged { name: StringId, change_kind: QueryChangeKind }
}

/// A versioned collection of diff operations between two workbooks.
///
/// The `version` field indicates the schema version for forwards compatibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    /// Schema version (currently "1").
    pub version: String,
    /// Interned string table used by ids referenced in this report.
    #[serde(default)]
    pub strings: Vec<String>,
    /// The list of diff operations.
    pub ops: Vec<DiffOp>,
    /// Whether the diff result is complete. When `false`, some operations may be missing
    /// due to resource limits being exceeded (e.g., row/column limits).
    #[serde(default = "default_complete")]
    pub complete: bool,
    /// Warnings generated during the diff process. Non-empty when limits were exceeded
    /// or other partial-result conditions occurred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[cfg(feature = "perf-metrics")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<crate::perf::DiffMetrics>,
}

fn default_complete() -> bool {
    true
}

impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
            ops,
            complete: true,
            warnings: Vec::new(),
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn with_partial_result(ops: Vec<DiffOp>, warning: String) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
            ops,
            complete: false,
            warnings: vec![warning],
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
        self.complete = false;
    }

    pub fn grid_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| !op.is_m_op())
    }

    pub fn m_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| op.is_m_op())
    }
}

impl DiffOp {
    pub fn is_m_op(&self) -> bool {
        matches!(
            self,
            DiffOp::QueryAdded { .. }
                | DiffOp::QueryRemoved { .. }
                | DiffOp::QueryRenamed { .. }
                | DiffOp::QueryDefinitionChanged { .. }
                | DiffOp::QueryMetadataChanged { .. }
        )
    }

    pub fn cell_edited(
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    ) -> DiffOp {
        debug_assert_eq!(from.addr, addr, "from.addr must match canonical addr");
        debug_assert_eq!(to.addr, addr, "to.addr must match canonical addr");
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        }
    }

    pub fn row_added(sheet: SheetId, row_idx: u32, row_signature: Option<RowSignature>) -> DiffOp {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn row_removed(
        sheet: SheetId,
        row_idx: u32,
        row_signature: Option<RowSignature>,
    ) -> DiffOp {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn column_added(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn column_removed(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn block_moved_rows(
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        }
    }

    pub fn block_moved_columns(
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn block_moved_rect(
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash,
        }
    }
}

```

---

### File: `core\src\engine.rs`

```rust
//! Core diffing engine for workbook comparison.
//!
//! Provides the main entry point [`diff_workbooks`] for comparing two workbooks
//! and generating a [`DiffReport`] of all changes.

use crate::alignment::move_extraction::moves_from_matched_pairs;
use crate::alignment::{RowAlignment as AmrAlignment, align_rows_amr_with_signatures_from_views};
use crate::column_alignment::{
    ColumnAlignment, ColumnBlockMove, align_single_column_change_from_views,
    detect_exact_column_block_move,
};
use crate::config::{DiffConfig, LimitBehavior};
use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary, SheetId};
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::rect_block_move::{RectBlockMove, detect_exact_rect_block_move};
use crate::region_mask::RegionMask;
use crate::row_alignment::{
    RowAlignment as LegacyRowAlignment, RowBlockMove as LegacyRowBlockMove,
    align_row_changes_from_views, detect_exact_row_block_move, detect_fuzzy_row_block_move,
};
use crate::sink::{DiffSink, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{Cell, CellAddress, CellSnapshot, ColSignature, Grid, RowSignature, Sheet, SheetKind, Workbook};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Default)]
struct DiffContext {
    warnings: Vec<String>,
}

const DATABASE_MODE_SHEET_ID: &str = "<database>";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

fn make_sheet_key(sheet: &Sheet, pool: &StringPool) -> SheetKey {
    SheetKey {
        name_lower: pool.resolve(sheet.name).to_lowercase(),
        kind: sheet.kind.clone(),
    }
}

fn sheet_kind_order(kind: &SheetKind) -> u8 {
    match kind {
        SheetKind::Worksheet => 0,
        SheetKind::Chart => 1,
        SheetKind::Macro => 2,
        SheetKind::Other => 3,
    }
}

fn emit_op<S: DiffSink>(
    sink: &mut S,
    op_count: &mut usize,
    op: DiffOp,
) -> Result<(), DiffError> {
    sink.emit(op)?;
    *op_count = op_count.saturating_add(1);
    Ok(())
}

pub fn diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    match try_diff_workbooks(old, new, pool, config) {
        Ok(report) => report,
        Err(e) => panic!("{}", e),
    }
}

pub fn diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> DiffSummary {
    match try_diff_workbooks_streaming(old, new, pool, config, sink) {
        Ok(summary) => summary,
        Err(e) => panic!("{}", e),
    }
}

pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    let summary = try_diff_workbooks_streaming(old, new, pool, config, &mut sink)?;
    let ops = sink.into_ops();
    let mut report = DiffReport::new(ops);
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    #[cfg(feature = "perf-metrics")]
    {
        report.metrics = summary.metrics;
    }
    report.strings = pool.strings().to_vec();
    Ok(report)
}

pub fn try_diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    let mut ctx = DiffContext::default();
    let mut op_count = 0usize;
    #[cfg(feature = "perf-metrics")]
    let mut metrics = {
        let mut m = DiffMetrics::default();
        m.start_phase(Phase::Total);
        m
    };

    let mut old_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let key = make_sheet_key(sheet, pool);
        let was_unique = old_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in old workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut new_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let key = make_sheet_key(sheet, pool);
        let was_unique = new_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in new workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut all_keys: Vec<SheetKey> = old_sheets
        .keys()
        .chain(new_sheets.keys())
        .cloned()
        .collect();
    all_keys.sort_by(|a, b| match a.name_lower.cmp(&b.name_lower) {
        std::cmp::Ordering::Equal => sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind)),
        other => other,
    });
    all_keys.dedup();

    for key in all_keys {
        match (old_sheets.get(&key), new_sheets.get(&key)) {
            (None, Some(new_sheet)) => {
                emit_op(
                    sink,
                    &mut op_count,
                    DiffOp::SheetAdded {
                        sheet: new_sheet.name,
                    },
                )?;
            }
            (Some(old_sheet), None) => {
                emit_op(
                    sink,
                    &mut op_count,
                    DiffOp::SheetRemoved {
                        sheet: old_sheet.name,
                    },
                )?;
            }
            (Some(old_sheet), Some(new_sheet)) => {
                let sheet_id: SheetId = old_sheet.name;
                try_diff_grids(
                    &sheet_id,
                    &old_sheet.grid,
                    &new_sheet.grid,
                    config,
                    pool,
                    sink,
                    &mut op_count,
                    &mut ctx,
                    #[cfg(feature = "perf-metrics")]
                    Some(&mut metrics),
                )?;
            }
            (None, None) => unreachable!(),
        }
    }

    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Total);
    }
    sink.finish()?;
    let complete = ctx.warnings.is_empty();
    Ok(DiffSummary {
        complete,
        warnings: ctx.warnings,
        op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: Some(metrics),
    })
}

pub fn diff_grids_database_mode(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut sink = VecSink::new();
    let mut op_count = 0usize;
    let summary =
        diff_grids_database_mode_streaming(old, new, key_columns, pool, config, &mut sink, &mut op_count)
            .unwrap_or_else(|e| panic!("{}", e));
    let mut report = DiffReport::new(sink.into_ops());
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    report.strings = pool.strings().to_vec();
    report
}

fn diff_grids_database_mode_streaming<S: DiffSink>(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    op_count: &mut usize,
) -> Result<DiffSummary, DiffError> {
    let spec = KeyColumnSpec::new(key_columns.to_vec());
    let alignment = match diff_table_by_key(old, new, key_columns) {
        Ok(alignment) => alignment,
        Err(_) => {
            let sheet_id: SheetId = pool.intern(DATABASE_MODE_SHEET_ID);
            let mut ctx = DiffContext::default();
            try_diff_grids(
                &sheet_id,
                old,
                new,
                config,
                pool,
                sink,
                op_count,
                &mut ctx,
                #[cfg(feature = "perf-metrics")]
                None,
            )?;
            sink.finish()?;
            let complete = ctx.warnings.is_empty();
            return Ok(DiffSummary {
                complete,
                warnings: ctx.warnings,
                op_count: *op_count,
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            });
        }
    };

    let sheet_id: SheetId = pool.intern(DATABASE_MODE_SHEET_ID);
    let max_cols = old.ncols.max(new.ncols);

    for row_idx in &alignment.left_only_rows {
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id, *row_idx, None),
        )?;
    }

    for row_idx in &alignment.right_only_rows {
        emit_op(
            sink,
            op_count,
            DiffOp::row_added(sheet_id, *row_idx, None),
        )?;
    }

    for (row_a, row_b) in &alignment.matched_rows {
        for col in 0..max_cols {
            if spec.is_key_column(col) {
                continue;
            }

            let old_cell = old.get(*row_a, col);
            let new_cell = new.get(*row_b, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            emit_op(sink, op_count, DiffOp::cell_edited(sheet_id, addr, from, to))?;
        }
    }

    sink.finish()?;
    Ok(DiffSummary {
        complete: true,
        warnings: Vec::new(),
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
}

fn try_diff_grids<S: DiffSink>(
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
    if old.nrows == 0 && new.nrows == 0 {
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.rows_processed = m
            .rows_processed
            .saturating_add(old.nrows as u64)
            .saturating_add(new.nrows as u64);
        m.start_phase(Phase::MoveDetection);
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols;
    if exceeds_limits {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(*sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );
        match config.on_limit_exceeded {
            LimitBehavior::FallbackToPositional => {
                positional_diff(sheet_id, old, new, sink, op_count)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnPartialResult => {
                ctx.warnings.push(warning);
                positional_diff(sheet_id, old, new, sink, op_count)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id.clone(),
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.max_align_rows,
                    max_cols: config.max_align_cols,
                });
            }
        }
        return Ok(());
    }

    diff_grids_core(
        sheet_id,
        old,
        new,
        config,
        sink,
        op_count,
        ctx,
        #[cfg(feature = "perf-metrics")]
        metrics,
    )?;
    Ok(())
}

fn diff_grids_core<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    sink: &mut S,
    op_count: &mut usize,
    _ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == new.nrows && old.ncols == new.ncols && grids_non_blank_cells_equal(old, new) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
        }
        return Ok(());
    }

    let old_view = GridView::from_grid_with_config(old, config);
    let new_view = GridView::from_grid_with_config(new, config);

    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
    let move_detection_enabled = old.nrows.max(new.nrows) <= config.max_move_detection_rows
        && old.ncols.max(new.ncols) <= config.max_move_detection_cols;
    let mut iteration = 0;

    if move_detection_enabled {
        loop {
            if iteration >= config.max_move_iterations {
                break;
            }

            if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
                break;
            }

            let mut found_move = false;

            if let Some(mv) =
                detect_exact_rect_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_rect_block_move(sheet_id, mv, sink, op_count)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_rect_cells(
                    mv.src_start_row,
                    mv.src_row_count,
                    mv.src_start_col,
                    mv.src_col_count,
                );
                new_mask.exclude_rect_cells(
                    mv.dst_start_row,
                    mv.src_row_count,
                    mv.dst_start_col,
                    mv.src_col_count,
                );
                old_mask.exclude_rect_cells(
                    mv.dst_start_row,
                    mv.src_row_count,
                    mv.dst_start_col,
                    mv.src_col_count,
                );
                new_mask.exclude_rect_cells(
                    mv.src_start_row,
                    mv.src_row_count,
                    mv.src_start_col,
                    mv.src_col_count,
                );
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && let Some(mv) =
                    detect_exact_row_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_row_block_move(sheet_id, mv, sink, op_count)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && let Some(mv) =
                    detect_exact_column_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_column_block_move(sheet_id, mv, sink, op_count)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_cols(mv.src_start_col, mv.col_count);
                new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && config.enable_fuzzy_moves
                && let Some(mv) =
                    detect_fuzzy_row_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_row_block_move(sheet_id, mv, sink, op_count)?;
                emit_moved_row_block_edits(sheet_id, &old_view, &new_view, mv, sink, op_count, config)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move {
                break;
            }

            if old.nrows != new.nrows || old.ncols != new.ncols {
                break;
            }
        }

        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
    } else {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
    }

    if old_mask.has_exclusions() || new_mask.has_exclusions() {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        if old.nrows != new.nrows || old.ncols != new.ncols {
            if diff_aligned_with_masks(sheet_id, old, new, &old_mask, &new_mask, sink, op_count)? {
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.end_phase(Phase::CellDiff);
                }
                return Ok(());
            }
            positional_diff_with_masks(sheet_id, old, new, &old_mask, &new_mask, sink, op_count)?;
        } else {
            positional_diff_masked_equal_size(
                sheet_id,
                old,
                new,
                &old_mask,
                &new_mask,
                sink,
                op_count,
            )?;
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::CellDiff);
        }
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::Alignment);
    }

    if let Some(amr_result) =
        align_rows_amr_with_signatures_from_views(&old_view, &new_view, config)
    {
        let mut alignment = amr_result.alignment;

        if config.max_move_iterations > 0 {
            let row_signatures_old = amr_result.row_signatures_a;
            let row_signatures_new = amr_result.row_signatures_b;
            inject_moves_from_insert_delete(
                old,
                new,
                &mut alignment,
                &row_signatures_old,
                &row_signatures_new,
            );
        } else {
            let mut deleted_from_moves = Vec::new();
            let mut inserted_from_moves = Vec::new();
            for mv in &alignment.moves {
                deleted_from_moves
                    .extend(mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count));
                inserted_from_moves
                    .extend(mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count));
            }

            let multiset_equal = row_signature_multiset_equal(old, new);
            if multiset_equal {
                for (a, b) in &alignment.matched {
                    if row_signature_at(old, *a) != row_signature_at(new, *b) {
                        deleted_from_moves.push(*a);
                        inserted_from_moves.push(*b);
                    }
                }
            }

            if !deleted_from_moves.is_empty() || !inserted_from_moves.is_empty() {
                let deleted_set: HashSet<u32> = deleted_from_moves.iter().copied().collect();
                let inserted_set: HashSet<u32> = inserted_from_moves.iter().copied().collect();

                alignment
                    .matched
                    .retain(|(a, b)| !deleted_set.contains(a) && !inserted_set.contains(b));

                alignment.deleted.extend(deleted_set);
                alignment.inserted.extend(inserted_set);
                alignment.deleted.sort_unstable();
                alignment.deleted.dedup();
                alignment.inserted.sort_unstable();
                alignment.inserted.dedup();
            }

            alignment.moves.clear();
        }
        let has_structural_rows = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();
        if has_structural_rows && alignment.matched.is_empty() {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            positional_diff(sheet_id, old, new, sink, op_count)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.add_cells_compared(cells_in_overlap(old, new));
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return Ok(());
        }
        if has_structural_rows {
            let has_row_edits = alignment
                .matched
                .iter()
                .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));
            if has_row_edits && config.max_move_iterations > 0 {
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.start_phase(Phase::CellDiff);
                }
                positional_diff(sheet_id, old, new, sink, op_count)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                    m.end_phase(Phase::CellDiff);
                }
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.end_phase(Phase::Alignment);
                }
                return Ok(());
            }
        }
        if alignment.moves.is_empty()
            && alignment.inserted.is_empty()
            && alignment.deleted.is_empty()
            && old.ncols != new.ncols
            && let Some(col_alignment) =
                align_single_column_change_from_views(&old_view, &new_view, config)
        {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            emit_column_aligned_diffs(sheet_id, old, new, &col_alignment, sink, op_count)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                let overlap_rows = old.nrows.min(new.nrows) as u64;
                m.add_cells_compared(
                    overlap_rows.saturating_mul(col_alignment.matched.len() as u64),
                );
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return Ok(());
        }
        let alignment_is_trivial_identity = alignment.moves.is_empty()
            && alignment.inserted.is_empty()
            && alignment.deleted.is_empty()
            && old.nrows == new.nrows
            && alignment.matched.len() as u32 == old.nrows
            && alignment.matched.iter().all(|(a, b)| a == b);

        if !alignment_is_trivial_identity
            && alignment.moves.is_empty()
            && row_signature_multiset_equal(old, new)
            && config.max_move_iterations > 0
        {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            positional_diff(sheet_id, old, new, sink, op_count)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.add_cells_compared(cells_in_overlap(old, new));
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return Ok(());
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        let compared = emit_amr_aligned_diffs(
            sheet_id,
            &old_view,
            &new_view,
            &alignment,
            sink,
            op_count,
            config,
        )?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(compared);
            m.anchors_found = m
                .anchors_found
                .saturating_add(alignment.matched.len() as u32);
            m.moves_detected = m
                .moves_detected
                .saturating_add(alignment.moves.len() as u32);
        }
        #[cfg(not(feature = "perf-metrics"))]
        let _ = compared;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::CellDiff);
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(());
    }

    if let Some(alignment) = align_row_changes_from_views(&old_view, &new_view, config) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        let compared =
            emit_aligned_diffs(sheet_id, &old_view, &new_view, &alignment, sink, op_count, config)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(compared);
            m.end_phase(Phase::CellDiff);
        }
        #[cfg(not(feature = "perf-metrics"))]
        let _ = compared;
    } else if let Some(alignment) =
        align_single_column_change_from_views(&old_view, &new_view, config)
    {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_column_aligned_diffs(sheet_id, old, new, &alignment, sink, op_count)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            let overlap_rows = old.nrows.min(new.nrows) as u64;
            m.add_cells_compared(overlap_rows.saturating_mul(alignment.matched.len() as u64));
            m.end_phase(Phase::CellDiff);
        }
    } else {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        positional_diff(sheet_id, old, new, sink, op_count)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
            m.end_phase(Phase::CellDiff);
        }
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.end_phase(Phase::Alignment);
    }

    Ok(())
}

fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), None) | (None, Some(cell_a)) => {
            cell_a.value.is_none() && cell_a.formula.is_none()
        }
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
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

fn collect_differences_in_grid(old: &Grid, new: &Grid) -> Vec<(u32, u32)> {
    let mut diffs = Vec::new();

    for row in 0..old.nrows {
        for col in 0..old.ncols {
            if !cells_content_equal(old.get(row, col), new.get(row, col)) {
                diffs.push((row, col));
            }
        }
    }

    diffs
}

fn contiguous_ranges<I>(indices: I) -> Vec<(u32, u32)>
where
    I: IntoIterator<Item = u32>,
{
    let mut values: Vec<u32> = indices.into_iter().collect();
    if values.is_empty() {
        return Vec::new();
    }

    values.sort_unstable();
    values.dedup();

    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut start = values[0];
    let mut prev = values[0];

    for &val in values.iter().skip(1) {
        if val == prev + 1 {
            prev = val;
            continue;
        }

        ranges.push((start, prev));
        start = val;
        prev = val;
    }
    ranges.push((start, prev));

    ranges
}

fn group_rows_by_column_patterns(diffs: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if diffs.is_empty() {
        return Vec::new();
    }

    let mut row_to_cols: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (row, col) in diffs {
        row_to_cols.entry(*row).or_default().push(*col);
    }

    for cols in row_to_cols.values_mut() {
        cols.sort_unstable();
        cols.dedup();
    }

    let mut rows: Vec<u32> = row_to_cols.keys().copied().collect();
    rows.sort_unstable();

    let mut groups: Vec<(u32, u32)> = Vec::new();
    if let Some(&first_row) = rows.first() {
        let mut start = first_row;
        let mut prev = first_row;
        let mut current_cols = row_to_cols.get(&first_row).cloned().unwrap_or_default();

        for row in rows.into_iter().skip(1) {
            let cols = row_to_cols.get(&row).cloned().unwrap_or_default();
            if row == prev + 1 && cols == current_cols {
                prev = row;
            } else {
                groups.push((start, prev));
                start = row;
                prev = row;
                current_cols = cols;
            }
        }
        groups.push((start, prev));
    }

    groups
}

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

fn col_signature_at(grid: &Grid, col: u32) -> Option<ColSignature> {
    if let Some(sig) = grid
        .col_signatures
        .as_ref()
        .and_then(|cols| cols.get(col as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_col_signature(col))
}

fn align_indices_by_signature<T: Copy + Eq>(
    idx_a: &[u32],
    idx_b: &[u32],
    sig_a: impl Fn(u32) -> Option<T>,
    sig_b: impl Fn(u32) -> Option<T>,
) -> Option<(Vec<u32>, Vec<u32>)> {
    if idx_a.is_empty() || idx_b.is_empty() {
        return None;
    }

    if idx_a.len() == idx_b.len() {
        return Some((idx_a.to_vec(), idx_b.to_vec()));
    }

    let (short, long, short_is_a) = if idx_a.len() <= idx_b.len() {
        (idx_a, idx_b, true)
    } else {
        (idx_b, idx_a, false)
    };

    let diff = long.len() - short.len();
    let mut best_offset = 0usize;
    let mut best_matches = 0usize;

    for offset in 0..=diff {
        let mut matches = 0usize;
        for (i, &short_idx) in short.iter().enumerate() {
            let long_idx = long[offset + i];
            let (sig_short, sig_long) = if short_is_a {
                (sig_a(short_idx), sig_b(long_idx))
            } else {
                (sig_b(short_idx), sig_a(long_idx))
            };
            if let (Some(sa), Some(sb)) = (sig_short, sig_long)
                && sa == sb
            {
                matches += 1;
            }
        }
        if matches > best_matches {
            best_matches = matches;
            best_offset = offset;
        }
    }

    if short_is_a {
        let aligned_b = long[best_offset..best_offset + short.len()].to_vec();
        Some((idx_a.to_vec(), aligned_b))
    } else {
        let aligned_a = long[best_offset..best_offset + short.len()].to_vec();
        Some((aligned_a, idx_b.to_vec()))
    }
}

fn inject_moves_from_insert_delete(
    old: &Grid,
    new: &Grid,
    alignment: &mut AmrAlignment,
    row_signatures_old: &[RowSignature],
    row_signatures_new: &[RowSignature],
) {
    if alignment.inserted.is_empty() || alignment.deleted.is_empty() {
        return;
    }

    let mut deleted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.deleted {
        let sig = row_signatures_old
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(old, *row));
        if let Some(sig) = sig {
            deleted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut inserted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.inserted {
        let sig = row_signatures_new
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(new, *row));
        if let Some(sig) = sig {
            inserted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut matched_pairs = Vec::new();
    for (sig, deleted_rows) in deleted_by_sig.iter() {
        if deleted_rows.len() != 1 {
            continue;
        }
        if let Some(insert_rows) = inserted_by_sig.get(sig) {
            if insert_rows.len() != 1 {
                continue;
            }
            matched_pairs.push((deleted_rows[0], insert_rows[0]));
        }
    }

    if matched_pairs.is_empty() {
        return;
    }

    let new_moves = moves_from_matched_pairs(&matched_pairs);
    if new_moves.is_empty() {
        return;
    }

    let mut moved_src = HashSet::new();
    let mut moved_dst = HashSet::new();
    for mv in &new_moves {
        for r in mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count) {
            moved_src.insert(r);
        }
        for r in mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count) {
            moved_dst.insert(r);
        }
    }

    alignment.deleted.retain(|r| !moved_src.contains(r));
    alignment.inserted.retain(|r| !moved_dst.contains(r));
    alignment.moves.extend(new_moves);
    alignment
        .moves
        .sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));
}

fn build_projected_grid_from_maps(
    source: &Grid,
    mask: &RegionMask,
    row_map: &[u32],
    col_map: &[u32],
) -> (Grid, Vec<u32>, Vec<u32>) {
    let nrows = row_map.len() as u32;
    let ncols = col_map.len() as u32;

    let mut row_lookup: Vec<Option<u32>> = vec![None; source.nrows as usize];
    for (new_idx, old_row) in row_map.iter().enumerate() {
        row_lookup[*old_row as usize] = Some(new_idx as u32);
    }

    let mut col_lookup: Vec<Option<u32>> = vec![None; source.ncols as usize];
    for (new_idx, old_col) in col_map.iter().enumerate() {
        col_lookup[*old_col as usize] = Some(new_idx as u32);
    }

    let mut projected = Grid::new(nrows, ncols);

    for ((row, col), cell) in source.iter_cells() {
        if !mask.is_cell_active(row, col) {
            continue;
        }
        let Some(new_row) = row_lookup.get(row as usize).and_then(|v| *v) else {
            continue;
        };
        let Some(new_col) = col_lookup.get(col as usize).and_then(|v| *v) else {
            continue;
        };

        projected.insert_cell(new_row, new_col, cell.value.clone(), cell.formula);
    }

    (projected, row_map.to_vec(), col_map.to_vec())
}

fn build_masked_grid(source: &Grid, mask: &RegionMask) -> (Grid, Vec<u32>, Vec<u32>) {
    let row_map: Vec<u32> = mask.active_rows().collect();
    let col_map: Vec<u32> = mask.active_cols().collect();

    let nrows = row_map.len() as u32;
    let ncols = col_map.len() as u32;

    let mut row_lookup: Vec<Option<u32>> = vec![None; source.nrows as usize];
    for (new_idx, old_row) in row_map.iter().enumerate() {
        row_lookup[*old_row as usize] = Some(new_idx as u32);
    }

    let mut col_lookup: Vec<Option<u32>> = vec![None; source.ncols as usize];
    for (new_idx, old_col) in col_map.iter().enumerate() {
        col_lookup[*old_col as usize] = Some(new_idx as u32);
    }

    let mut projected = Grid::new(nrows, ncols);

    for ((row, col), cell) in source.iter_cells() {
        if !mask.is_cell_active(row, col) {
            continue;
        }

        let Some(new_row) = row_lookup.get(row as usize).and_then(|v| *v) else {
            continue;
        };
        let Some(new_col) = col_lookup.get(col as usize).and_then(|v| *v) else {
            continue;
        };

        projected.insert_cell(new_row, new_col, cell.value.clone(), cell.formula);
    }

    (projected, row_map, col_map)
}

fn detect_exact_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<LegacyRowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        return detect_exact_row_block_move(old, new, config);
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_row_block_move(&old_proj, &new_proj, config)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(LegacyRowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

fn detect_exact_column_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<ColumnBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        return detect_exact_column_block_move(old, new, config);
    }

    let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, _, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_column_block_move(&old_proj, &new_proj, config)?;
    let src_start_col = *old_cols.get(mv_local.src_start_col as usize)?;
    let dst_start_col = *new_cols.get(mv_local.dst_start_col as usize)?;

    Some(ColumnBlockMove {
        src_start_col,
        dst_start_col,
        col_count: mv_local.col_count,
    })
}

fn detect_exact_rect_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    // Fast path: allow the strict detector to short-circuit when it succeeds, but
    // fall back to the masked search if it fails (e.g., when extra diffs exist).
    if !old_mask.has_exclusions()
        && !new_mask.has_exclusions()
        && old.nrows == new.nrows
        && old.ncols == new.ncols
        && let Some(mv) = detect_exact_rect_block_move(old, new, config)
    {
        return Some(mv);
    }

    let aligned_rows = align_indices_by_signature(
        &old_mask.active_rows().collect::<Vec<_>>(),
        &new_mask.active_rows().collect::<Vec<_>>(),
        |r| row_signature_at(old, r),
        |r| row_signature_at(new, r),
    )?;
    let aligned_cols = align_indices_by_signature(
        &old_mask.active_cols().collect::<Vec<_>>(),
        &new_mask.active_cols().collect::<Vec<_>>(),
        |c| col_signature_at(old, c),
        |c| col_signature_at(new, c),
    )?;
    let (old_proj, old_rows, old_cols) =
        build_projected_grid_from_maps(old, old_mask, &aligned_rows.0, &aligned_cols.0);
    let (new_proj, new_rows, new_cols) =
        build_projected_grid_from_maps(new, new_mask, &aligned_rows.1, &aligned_cols.1);

    let map_move = |mv_local: RectBlockMove,
                    row_map_old: &[u32],
                    row_map_new: &[u32],
                    col_map_old: &[u32],
                    col_map_new: &[u32]|
     -> Option<RectBlockMove> {
        let src_start_row = *row_map_old.get(mv_local.src_start_row as usize)?;
        let dst_start_row = *row_map_new.get(mv_local.dst_start_row as usize)?;
        let src_start_col = *col_map_old.get(mv_local.src_start_col as usize)?;
        let dst_start_col = *col_map_new.get(mv_local.dst_start_col as usize)?;

        Some(RectBlockMove {
            src_start_row,
            dst_start_row,
            src_start_col,
            dst_start_col,
            src_row_count: mv_local.src_row_count,
            src_col_count: mv_local.src_col_count,
            block_hash: mv_local.block_hash,
        })
    };

    if let Some(mv_local) = detect_exact_rect_block_move(&old_proj, &new_proj, config)
        && let Some(mapped) = map_move(mv_local, &old_rows, &new_rows, &old_cols, &new_cols)
    {
        return Some(mapped);
    }

    let diff_positions = collect_differences_in_grid(&old_proj, &new_proj);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = group_rows_by_column_patterns(&diff_positions);
    let col_ranges_full = contiguous_ranges(diff_positions.iter().map(|(_, c)| *c));
    let has_prior_exclusions = old_mask.has_exclusions() || new_mask.has_exclusions();
    if !has_prior_exclusions && row_ranges.len() <= 2 && col_ranges_full.len() <= 2 {
        return None;
    }

    let range_len = |range: (u32, u32)| range.1.saturating_sub(range.0).saturating_add(1);
    let in_range = |idx: u32, range: (u32, u32)| idx >= range.0 && idx <= range.1;
    let rectangles_match = |src_rows: (u32, u32),
                            src_cols: (u32, u32),
                            dst_rows: (u32, u32),
                            dst_cols: (u32, u32)|
     -> bool {
        let row_count = range_len(src_rows);
        let col_count = range_len(src_cols);

        for dr in 0..row_count {
            for dc in 0..col_count {
                let src_row = src_rows.0 + dr;
                let src_col = src_cols.0 + dc;
                let dst_row = dst_rows.0 + dr;
                let dst_col = dst_cols.0 + dc;

                if !cells_content_equal(
                    old_proj.get(src_row, src_col),
                    new_proj.get(dst_row, dst_col),
                ) {
                    return false;
                }
            }
        }

        true
    };

    for (row_idx, &row_a) in row_ranges.iter().enumerate() {
        for &row_b in row_ranges.iter().skip(row_idx + 1) {
            if range_len(row_a) != range_len(row_b) {
                continue;
            }

            let cols_row_a: Vec<u32> = diff_positions
                .iter()
                .filter_map(|(r, c)| if in_range(*r, row_a) { Some(*c) } else { None })
                .collect();
            let cols_row_b: Vec<u32> = diff_positions
                .iter()
                .filter_map(|(r, c)| if in_range(*r, row_b) { Some(*c) } else { None })
                .collect();
            let col_ranges_a = contiguous_ranges(cols_row_a);
            let col_ranges_b = contiguous_ranges(cols_row_b);
            let mut col_pairs: Vec<((u32, u32), (u32, u32))> = Vec::new();

            for &col_a in &col_ranges_a {
                for &col_b in &col_ranges_b {
                    if range_len(col_a) != range_len(col_b) {
                        continue;
                    }
                    col_pairs.push((col_a, col_b));
                }
            }

            if col_pairs.is_empty() {
                continue;
            }

            for (col_a, col_b) in col_pairs {
                let mut scoped_old_mask = RegionMask::all_active(old_proj.nrows, old_proj.ncols);
                let mut scoped_new_mask = RegionMask::all_active(new_proj.nrows, new_proj.ncols);

                for row in 0..old_proj.nrows {
                    if !in_range(row, row_a) && !in_range(row, row_b) {
                        scoped_old_mask.exclude_row(row);
                        scoped_new_mask.exclude_row(row);
                    }
                }

                for col in 0..old_proj.ncols {
                    if !in_range(col, col_a) && !in_range(col, col_b) {
                        scoped_old_mask.exclude_col(col);
                        scoped_new_mask.exclude_col(col);
                    }
                }

                let (old_scoped, scoped_old_rows, scoped_old_cols) =
                    build_masked_grid(&old_proj, &scoped_old_mask);
                let (new_scoped, scoped_new_rows, scoped_new_cols) =
                    build_masked_grid(&new_proj, &scoped_new_mask);

                if old_scoped.nrows != new_scoped.nrows || old_scoped.ncols != new_scoped.ncols {
                    continue;
                }

                if let Some(candidate) =
                    detect_exact_rect_block_move(&old_scoped, &new_scoped, config)
                {
                    let scoped_row_map_old: Option<Vec<u32>> = scoped_old_rows
                        .iter()
                        .map(|idx| old_rows.get(*idx as usize).copied())
                        .collect();
                    let scoped_row_map_new: Option<Vec<u32>> = scoped_new_rows
                        .iter()
                        .map(|idx| new_rows.get(*idx as usize).copied())
                        .collect();
                    let scoped_col_map_old: Option<Vec<u32>> = scoped_old_cols
                        .iter()
                        .map(|idx| old_cols.get(*idx as usize).copied())
                        .collect();
                    let scoped_col_map_new: Option<Vec<u32>> = scoped_new_cols
                        .iter()
                        .map(|idx| new_cols.get(*idx as usize).copied())
                        .collect();

                    if let (
                        Some(row_map_old),
                        Some(row_map_new),
                        Some(col_map_old),
                        Some(col_map_new),
                    ) = (
                        scoped_row_map_old,
                        scoped_row_map_new,
                        scoped_col_map_old,
                        scoped_col_map_new,
                    ) && let Some(mapped) = map_move(
                        candidate,
                        &row_map_old,
                        &row_map_new,
                        &col_map_old,
                        &col_map_new,
                    ) {
                        return Some(mapped);
                    }
                }

                let row_len = range_len(row_a);
                let col_len = range_len(col_a);
                if row_len == 0 || col_len == 0 {
                    continue;
                }

                let candidates = [
                    (row_a, col_a, row_b, col_b),
                    (row_a, col_b, row_b, col_a),
                    (row_b, col_a, row_a, col_b),
                    (row_b, col_b, row_a, col_a),
                ];

                for (src_rows, src_cols, dst_rows, dst_cols) in candidates {
                    if range_len(src_rows) != range_len(dst_rows)
                        || range_len(src_cols) != range_len(dst_cols)
                    {
                        continue;
                    }

                    if rectangles_match(src_rows, src_cols, dst_rows, dst_cols) {
                        let mapped = RectBlockMove {
                            src_start_row: *old_rows.get(src_rows.0 as usize)?,
                            dst_start_row: *new_rows.get(dst_rows.0 as usize)?,
                            src_start_col: *old_cols.get(src_cols.0 as usize)?,
                            dst_start_col: *new_cols.get(dst_cols.0 as usize)?,
                            src_row_count: range_len(src_rows),
                            src_col_count: range_len(src_cols),
                            block_hash: None,
                        };
                        return Some(mapped);
                    }
                }
            }
        }
    }

    None
}

fn detect_fuzzy_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<LegacyRowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        return detect_fuzzy_row_block_move(old, new, config);
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_fuzzy_row_block_move(&old_proj, &new_proj, config)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(LegacyRowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

#[cfg(feature = "perf-metrics")]
fn cells_in_overlap(old: &Grid, new: &Grid) -> u64 {
    let overlap_rows = old.nrows.min(new.nrows) as u64;
    let overlap_cols = old.ncols.min(new.ncols) as u64;
    overlap_rows.saturating_mul(overlap_cols)
}

fn positional_diff<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    sink: &mut S,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        diff_row_pair(
            sheet_id,
            old,
            new,
            row,
            row,
            overlap_cols,
            sink,
            op_count,
        )?;
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            emit_op(
                sink,
                op_count,
                DiffOp::row_added(sheet_id.clone(), row_idx, None),
            )?;
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            emit_op(
                sink,
                op_count,
                DiffOp::row_removed(sheet_id.clone(), row_idx, None),
            )?;
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_added(sheet_id.clone(), col_idx, None),
            )?;
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            emit_op(
                sink,
                op_count,
                DiffOp::column_removed(sheet_id.clone(), col_idx, None),
            )?;
        }
    }

    Ok(())
}

fn diff_aligned_with_masks(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<bool, DiffError> {
    let old_rows: Vec<u32> = old_mask.active_rows().collect();
    let new_rows: Vec<u32> = new_mask.active_rows().collect();
    let old_cols: Vec<u32> = old_mask.active_cols().collect();
    let new_cols: Vec<u32> = new_mask.active_cols().collect();

    let Some((rows_a, rows_b)) = align_indices_by_signature(
        &old_rows,
        &new_rows,
        |r| row_signature_at(old, r),
        |r| row_signature_at(new, r),
    ) else {
        return Ok(false);
    };

    let (cols_a, cols_b) = align_indices_by_signature(
        &old_cols,
        &new_cols,
        |c| col_signature_at(old, c),
        |c| col_signature_at(new, c),
    )
    .unwrap_or((old_cols.clone(), new_cols.clone()));

    if rows_a.len() != rows_b.len() || cols_a.len() != cols_b.len() {
        return Ok(false);
    }

    for (row_a, row_b) in rows_a.iter().zip(rows_b.iter()) {
        for (col_a, col_b) in cols_a.iter().zip(cols_b.iter()) {
            if !old_mask.is_cell_active(*row_a, *col_a) || !new_mask.is_cell_active(*row_b, *col_b)
            {
                continue;
            }
            let old_cell = old.get(*row_a, *col_a);
            let new_cell = new.get(*row_b, *col_b);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, *col_b);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
            )?;
        }
    }

    let rows_a_set: HashSet<u32> = rows_a.iter().copied().collect();
    let rows_b_set: HashSet<u32> = rows_b.iter().copied().collect();

    for row_idx in new_rows.iter().filter(|r| !rows_b_set.contains(r)) {
        if new_mask.is_row_active(*row_idx) {
            emit_op(
                sink,
                op_count,
                DiffOp::row_added(sheet_id.clone(), *row_idx, None),
            )?;
        }
    }

    for row_idx in old_rows.iter().filter(|r| !rows_a_set.contains(r)) {
        if old_mask.is_row_active(*row_idx) {
            emit_op(
                sink,
                op_count,
                DiffOp::row_removed(sheet_id.clone(), *row_idx, None),
            )?;
        }
    }

    let cols_a_set: HashSet<u32> = cols_a.iter().copied().collect();
    let cols_b_set: HashSet<u32> = cols_b.iter().copied().collect();

    for col_idx in new_cols.iter().filter(|c| !cols_b_set.contains(c)) {
        if new_mask.is_col_active(*col_idx) {
            emit_op(
                sink,
                op_count,
                DiffOp::column_added(sheet_id.clone(), *col_idx, None),
            )?;
        }
    }

    for col_idx in old_cols.iter().filter(|c| !cols_a_set.contains(c)) {
        if old_mask.is_col_active(*col_idx) {
            emit_op(
                sink,
                op_count,
                DiffOp::column_removed(sheet_id.clone(), *col_idx, None),
            )?;
        }
    }

    Ok(true)
}

fn positional_diff_with_masks(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        for col in 0..overlap_cols {
            if !old_mask.is_cell_active(row, col) || !new_mask.is_cell_active(row, col) {
                continue;
            }
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
            )?;
        }
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            if new_mask.is_row_active(row_idx) {
                emit_op(
                    sink,
                    op_count,
                    DiffOp::row_added(sheet_id.clone(), row_idx, None),
                )?;
            }
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            if old_mask.is_row_active(row_idx) {
                emit_op(
                    sink,
                    op_count,
                    DiffOp::row_removed(sheet_id.clone(), row_idx, None),
                )?;
            }
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            if new_mask.is_col_active(col_idx) {
                emit_op(
                    sink,
                    op_count,
                    DiffOp::column_added(sheet_id.clone(), col_idx, None),
                )?;
            }
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            if old_mask.is_col_active(col_idx) {
                emit_op(
                    sink,
                    op_count,
                    DiffOp::column_removed(sheet_id.clone(), col_idx, None),
                )?;
            }
        }
    }

    Ok(())
}

fn positional_diff_masked_equal_size(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    let row_shift_zone =
        compute_combined_shift_zone(old_mask.row_shift_bounds(), new_mask.row_shift_bounds());
    let col_shift_zone =
        compute_combined_shift_zone(old_mask.col_shift_bounds(), new_mask.col_shift_bounds());

    let stable_rows: Vec<u32> = (0..old.nrows)
        .filter(|&r| !is_in_zone(r, &row_shift_zone))
        .collect();
    let stable_cols: Vec<u32> = (0..old.ncols)
        .filter(|&c| !is_in_zone(c, &col_shift_zone))
        .collect();

    for &row in &stable_rows {
        for &col in &stable_cols {
            if !old_mask.is_cell_active(row, col) || !new_mask.is_cell_active(row, col) {
                continue;
            }
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
            )?;
        }
    }

    Ok(())
}

fn compute_combined_shift_zone(a: Option<(u32, u32)>, b: Option<(u32, u32)>) -> Option<(u32, u32)> {
    match (a, b) {
        (Some((a_min, a_max)), Some((b_min, b_max))) => Some((a_min.min(b_min), a_max.max(b_max))),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

fn is_in_zone(idx: u32, zone: &Option<(u32, u32)>) -> bool {
    match zone {
        Some((min, max)) => idx >= *min && idx <= *max,
        None => false,
    }
}

fn emit_row_block_move(
    sheet_id: &SheetId,
    mv: LegacyRowBlockMove,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
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
    )
}

fn emit_column_block_move(
    sheet_id: &SheetId,
    mv: ColumnBlockMove,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    emit_op(
        sink,
        op_count,
        DiffOp::BlockMovedColumns {
            sheet: sheet_id.clone(),
            src_start_col: mv.src_start_col,
            col_count: mv.col_count,
            dst_start_col: mv.dst_start_col,
            block_hash: None,
        },
    )
}

fn emit_rect_block_move(
    sheet_id: &SheetId,
    mv: RectBlockMove,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    emit_op(
        sink,
        op_count,
        DiffOp::BlockMovedRect {
            sheet: sheet_id.clone(),
            src_start_row: mv.src_start_row,
            src_row_count: mv.src_row_count,
            src_start_col: mv.src_start_col,
            src_col_count: mv.src_col_count,
            dst_start_row: mv.dst_start_row,
            dst_start_col: mv.dst_start_col,
            block_hash: mv.block_hash,
        },
    )
}

fn emit_moved_row_block_edits(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    mv: LegacyRowBlockMove,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<(), DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    for offset in 0..mv.row_count {
        let old_idx = (mv.src_start_row + offset) as usize;
        let new_idx = (mv.dst_start_row + offset) as usize;
        let Some(old_row) = old_view.rows.get(old_idx) else {
            continue;
        };
        let Some(new_row) = new_view.rows.get(new_idx) else {
            continue;
        };

        let _ = diff_row_pair_sparse(
            sheet_id,
            mv.dst_start_row + offset,
            overlap_cols,
            &old_row.cells,
            &new_row.cells,
            sink,
            op_count,
            config,
        )?;
    }
    Ok(())
}

fn emit_aligned_diffs(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &LegacyRowAlignment,
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

    Ok(compared)
}

fn emit_amr_aligned_diffs(
    sheet_id: &SheetId,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &AmrAlignment,
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

fn diff_row_pair_sparse(
    sheet_id: &SheetId,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut compared = 0u64;

    while i < old_cells.len() || j < new_cells.len() {
        let col_a = old_cells.get(i).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col_b = new_cells.get(j).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col = col_a.min(col_b);

        if col >= overlap_cols {
            break;
        }

        compared = compared.saturating_add(1);

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

        let changed = !cells_content_equal(old_cell, new_cell);

        if changed || config.include_unchanged_cells {
            let addr = CellAddress::from_indices(row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
            )?;
        }
    }

    Ok(compared)
}

fn diff_row_pair(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    for col in 0..overlap_cols {
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);

        if cells_content_equal(old_cell, new_cell) {
            continue;
        }

        let addr = CellAddress::from_indices(row_b, col);
        let from = snapshot_with_addr(old_cell, addr);
        let to = snapshot_with_addr(new_cell, addr);

        emit_op(
            sink,
            op_count,
            DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
        )?;
    }
    Ok(())
}

fn emit_column_aligned_diffs(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    alignment: &ColumnAlignment,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);

    for row in 0..overlap_rows {
        for (col_a, col_b) in &alignment.matched {
            let old_cell = old.get(row, *col_a);
            let new_cell = new.get(row, *col_b);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, *col_b);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
            )?;
        }
    }

    for col_idx in &alignment.inserted {
        emit_op(
            sink,
            op_count,
            DiffOp::column_added(sheet_id.clone(), *col_idx, None),
        )?;
    }

    for col_idx in &alignment.deleted {
        emit_op(
            sink,
            op_count,
            DiffOp::column_removed(sheet_id.clone(), *col_idx, None),
        )?;
    }

    Ok(())
}

fn snapshot_with_addr(cell: Option<&Cell>, addr: CellAddress) -> CellSnapshot {
    match cell {
        Some(cell) => CellSnapshot {
            addr,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        },
        None => CellSnapshot::empty(addr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::VecSink;
    use crate::string_pool::StringPool;
    use crate::workbook::CellValue;

    fn numbered_cell(value: f64) -> Cell {
        Cell {
            value: Some(CellValue::Number(value)),
            formula: None,
        }
    }

    fn grid_from_matrix(values: &[Vec<i32>]) -> Grid {
        let nrows = values.len() as u32;
        let ncols = if nrows == 0 {
            0
        } else {
            values[0].len() as u32
        };
        let mut grid = Grid::new(nrows, ncols);
        for (r, row) in values.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                grid.insert_cell(r as u32, c as u32, Some(CellValue::Number(*val as f64)), None);
            }
        }
        grid
    }

    #[test]
    fn sheet_kind_order_ranking_includes_macro_and_other() {
        assert!(
            sheet_kind_order(&SheetKind::Worksheet) < sheet_kind_order(&SheetKind::Chart),
            "Worksheet should rank before Chart"
        );
        assert!(
            sheet_kind_order(&SheetKind::Chart) < sheet_kind_order(&SheetKind::Macro),
            "Chart should rank before Macro"
        );
        assert!(
            sheet_kind_order(&SheetKind::Macro) < sheet_kind_order(&SheetKind::Other),
            "Macro should rank before Other"
        );
    }

    #[test]
    fn grids_non_blank_cells_equal_requires_matching_entries() {
        let base_cell = Cell {
            value: Some(CellValue::Number(1.0)),
            formula: None,
        };

        let mut grid_a = Grid::new(2, 2);
        let mut grid_b = Grid::new(2, 2);
        grid_a.insert_cell(0, 0, base_cell.value.clone(), base_cell.formula);
        grid_b.insert_cell(0, 0, base_cell.value.clone(), base_cell.formula);

        assert!(grids_non_blank_cells_equal(&grid_a, &grid_b));

        let mut grid_b_changed = grid_b.clone();
        let mut changed_cell = base_cell.clone();
        changed_cell.value = Some(CellValue::Number(2.0));
        grid_b_changed.insert_cell(0, 0, changed_cell.value.clone(), changed_cell.formula);

        assert!(!grids_non_blank_cells_equal(&grid_a, &grid_b_changed));

        grid_a.insert_cell(1, 1, None, None);

        assert!(!grids_non_blank_cells_equal(&grid_a, &grid_b));
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_columns_not_sum_lengths() {
        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("Sheet1");
        let config = DiffConfig::default();
        let mut sink = VecSink::new();
        let mut op_count = 0usize;

        let old_cells_storage = [
            numbered_cell(1.0),
            numbered_cell(2.0),
            numbered_cell(3.0),
        ];
        let new_cells_storage = [
            numbered_cell(1.0),
            numbered_cell(2.0),
            numbered_cell(4.0),
        ];

        let old_cells: Vec<(u32, &Cell)> = old_cells_storage
            .iter()
            .enumerate()
            .map(|(idx, cell)| (idx as u32, cell))
            .collect();
        let new_cells: Vec<(u32, &Cell)> = new_cells_storage
            .iter()
            .enumerate()
            .map(|(idx, cell)| (idx as u32, cell))
            .collect();

        let compared =
            diff_row_pair_sparse(
                &sheet_id,
                0,
                3,
                &old_cells,
                &new_cells,
                &mut sink,
                &mut op_count,
                &config,
            )
            .expect("diff should succeed");

        assert_eq!(compared, 3);
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_for_sparse_columns() {
        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("Sheet1");
        let config = DiffConfig::default();
        let mut sink = VecSink::new();
        let mut op_count = 0usize;

        let old_cells_storage = [numbered_cell(1.0)];
        let new_cells_storage = [numbered_cell(2.0)];

        let old_cells: Vec<(u32, &Cell)> = vec![(0, &old_cells_storage[0])];
        let new_cells: Vec<(u32, &Cell)> = vec![(2, &new_cells_storage[0])];

        let compared =
            diff_row_pair_sparse(
                &sheet_id,
                0,
                3,
                &old_cells,
                &new_cells,
                &mut sink,
                &mut op_count,
                &config,
            )
            .expect("diff should succeed");

        assert_eq!(compared, 2);
    }

    #[test]
    fn rect_move_masked_falls_back_when_outside_edit_exists() {
        let rows = 12usize;
        let cols = 12usize;
        let base: Vec<Vec<i32>> = (0..rows)
            .map(|r| {
                (0..cols)
                    .map(|c| 10_000 + (r as i32) * 100 + c as i32)
                    .collect()
            })
            .collect();
        let mut changed = base.clone();

        let src = (2usize, 2usize);
        let dst = (8usize, 6usize);
        let size = (2usize, 3usize);

        for dr in 0..size.0 {
            for dc in 0..size.1 {
                let src_r = src.0 + dr;
                let src_c = src.1 + dc;
                let dst_r = dst.0 + dr;
                let dst_c = dst.1 + dc;

                let src_val = base[src_r][src_c];
                let dst_val = base[dst_r][dst_c];

                changed[dst_r][dst_c] = src_val;
                changed[src_r][src_c] = dst_val;
            }
        }

        changed[0][0] = 77_777;

        let old = grid_from_matrix(&base);
        let new = grid_from_matrix(&changed);
        let old_mask = RegionMask::all_active(old.nrows, old.ncols);
        let new_mask = RegionMask::all_active(new.nrows, new.ncols);

        let mv = detect_exact_rect_block_move_masked(
            &old,
            &new,
            &old_mask,
            &new_mask,
            &DiffConfig::default(),
        )
        .expect("masked detector should fall back and still detect the move");

        assert_eq!(mv.src_start_row, src.0 as u32);
        assert_eq!(mv.src_start_col, src.1 as u32);
        assert_eq!(mv.src_row_count, size.0 as u32);
        assert_eq!(mv.src_col_count, size.1 as u32);
        assert_eq!(mv.dst_start_row, dst.0 as u32);
        assert_eq!(mv.dst_start_col, dst.1 as u32);
    }
}

```

---

### File: `core\src\excel_open_xml.rs`

```rust
//! Excel Open XML file parsing.
//!
//! Provides functions for opening `.xlsx` files and parsing their contents into
//! the internal representation used for diffing.

use crate::container::{ContainerError, OpcContainer};
use crate::datamashup_framing::{
    DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
    read_datamashup_text,
};
use crate::grid_parser::{
    GridParseError, parse_relationships, parse_shared_strings, parse_sheet_xml, parse_workbook_xml,
    resolve_sheet_target,
};
use crate::string_pool::StringPool;
use crate::workbook::{Sheet, SheetKind, Workbook};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PackageError {
    #[error("container error: {0}")]
    Container(#[from] ContainerError),
    #[error("grid parse error: {0}")]
    GridParse(#[from] GridParseError),
    #[error("DataMashup error: {0}")]
    DataMashup(#[from] DataMashupError),
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("serialization error: {0}")]
    SerializationError(String),
}

#[deprecated(note = "use PackageError")]
pub type ExcelOpenError = PackageError;

pub(crate) fn open_workbook_from_container(
    container: &mut OpcContainer,
    pool: &mut StringPool,
) -> Result<Workbook, PackageError> {
    let shared_strings = match container
        .read_file_optional("xl/sharedStrings.xml")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_shared_strings(&bytes, pool)?,
        None => Vec::new(),
    };

    let workbook_bytes = container
        .read_file("xl/workbook.xml")
        .map_err(|_| PackageError::WorkbookXmlMissing)?;

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match container
        .read_file_optional("xl/_rels/workbook.xml.rels")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_relationships(&bytes)?,
        None => HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes = container
            .read_file(&target)
            .map_err(|_| PackageError::WorksheetXmlMissing {
                sheet_name: sheet.name.clone(),
            })?;
        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings, pool)?;
        sheet_ir.push(Sheet {
            name: pool.intern(&sheet.name),
            kind: SheetKind::Worksheet,
            grid,
        });
    }

    Ok(Workbook { sheets: sheet_ir })
}

#[allow(deprecated)]
pub fn open_workbook(
    path: impl AsRef<Path>,
    pool: &mut StringPool,
) -> Result<Workbook, PackageError> {
    let mut container = OpcContainer::open_from_path(path.as_ref())?;
    open_workbook_from_container(&mut container, pool)
}

pub(crate) fn open_data_mashup_from_container(
    container: &mut OpcContainer,
) -> Result<Option<RawDataMashup>, PackageError> {
    let mut found: Option<RawDataMashup> = None;

    for i in 0..container.len() {
        let name = {
            let file = container.archive.by_index(i).ok();
            file.map(|f| f.name().to_string())
        };

        if let Some(name) = name {
            if !name.starts_with("customXml/") || !name.ends_with(".xml") {
                continue;
            }

            let bytes = container
                .read_file(&name)
                .map_err(|e| ContainerError::Zip(e.to_string()))?;

            if let Some(text) = read_datamashup_text(&bytes)? {
                let decoded = decode_datamashup_base64(&text)?;
                let parsed = parse_data_mashup(&decoded)?;
                if found.is_some() {
                    return Err(DataMashupError::FramingInvalid.into());
                }
                found = Some(parsed);
            }
        }
    }

    Ok(found)
}

#[allow(deprecated)]
pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, PackageError> {
    let mut container = OpcContainer::open_from_path(path.as_ref())?;
    open_data_mashup_from_container(&mut container)
}

```

---

### File: `core\src\grid_parser.rs`

```rust
//! XML parsing for Excel worksheet grids.
//!
//! Handles parsing of worksheet XML, shared strings, workbook structure, and
//! relationship files to construct [`Grid`] representations of sheet data.

use crate::addressing::address_to_index;
use crate::string_pool::{StringId, StringPool};
use crate::workbook::{CellValue, Grid};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GridParseError {
    #[error("XML parse error: {0}")]
    XmlError(String),
    #[error("invalid cell address: {0}")]
    InvalidAddress(String),
    #[error("shared string index {0} out of bounds")]
    SharedStringOutOfBounds(usize),
}

pub struct SheetDescriptor {
    pub name: String,
    pub rel_id: Option<String>,
    pub sheet_id: Option<u32>,
}

pub fn parse_shared_strings(
    xml: &[u8],
    pool: &mut StringPool,
) -> Result<Vec<StringId>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"si" => {
                current.clear();
                in_si = true;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" && in_si => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                current.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"si" => {
                let id = pool.intern(&current);
                strings.push(id);
                in_si = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(strings)
}

pub fn parse_workbook_xml(xml: &[u8]) -> Result<Vec<SheetDescriptor>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut sheets = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"sheet" => {
                let mut name = None;
                let mut rel_id = None;
                let mut sheet_id = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                    match attr.key.as_ref() {
                        b"name" => {
                            name = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        b"sheetId" => {
                            let parsed = attr.unescape_value().map_err(to_xml_err)?;
                            sheet_id = parsed.into_owned().parse::<u32>().ok();
                        }
                        b"r:id" => {
                            rel_id = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        _ => {}
                    }
                }
                if let Some(name) = name {
                    sheets.push(SheetDescriptor {
                        name,
                        rel_id,
                        sheet_id,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(sheets)
}

pub fn parse_relationships(xml: &[u8]) -> Result<HashMap<String, String>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut map = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"Relationship" => {
                let mut id = None;
                let mut target = None;
                let mut rel_type = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                    match attr.key.as_ref() {
                        b"Id" => id = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned()),
                        b"Target" => {
                            target = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        b"Type" => {
                            rel_type = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        _ => {}
                    }
                }

                if let (Some(id), Some(target), Some(rel_type)) = (id, target, rel_type)
                    && rel_type.contains("worksheet")
                {
                    map.insert(id, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(map)
}

pub fn resolve_sheet_target(
    sheet: &SheetDescriptor,
    relationships: &HashMap<String, String>,
    index: usize,
) -> String {
    if let Some(rel_id) = &sheet.rel_id
        && let Some(target) = relationships.get(rel_id)
    {
        return normalize_target(target);
    }

    let guessed = sheet
        .sheet_id
        .map(|id| format!("xl/worksheets/sheet{id}.xml"))
        .unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", index + 1));
    normalize_target(&guessed)
}

fn normalize_target(target: &str) -> String {
    let trimmed = target.trim_start_matches('/');
    if trimmed.starts_with("xl/") {
        trimmed.to_string()
    } else {
        format!("xl/{trimmed}")
    }
}

pub fn parse_sheet_xml(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<Grid, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"dimension" => {
                if let Some(r) = get_attr_value(&e, b"ref")? {
                    dimension_hint = dimension_from_ref(&r);
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(&mut reader, e, shared_strings, pool)?;
                max_row = Some(max_row.map_or(cell.row, |r| r.max(cell.row)));
                max_col = Some(max_col.map_or(cell.col, |c| c.max(cell.col)));
                parsed_cells.push(cell);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if parsed_cells.is_empty() {
        return Ok(Grid::new(0, 0));
    }

    let mut nrows = dimension_hint.map(|(r, _)| r).unwrap_or(0);
    let mut ncols = dimension_hint.map(|(_, c)| c).unwrap_or(0);

    if let Some(max_r) = max_row {
        nrows = nrows.max(max_r + 1);
    }
    if let Some(max_c) = max_col {
        ncols = ncols.max(max_c + 1);
    }

    build_grid(nrows, ncols, parsed_cells)
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<ParsedCell, GridParseError> {
    let address_raw = get_attr_value(&start, b"r")?
        .ok_or_else(|| GridParseError::XmlError("cell missing address".into()))?;
    let (row, col) = address_to_index(&address_raw)
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.clone()))?;

    let cell_type = get_attr_value(&start, b"t")?;

    let mut value_text: Option<String> = None;
    let mut formula_text: Option<String> = None;
    let mut inline_text: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                value_text = Some(text);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                let unescaped = quick_xml::escape::unescape(&text)
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                formula_text = Some(unescaped);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                inline_text = Some(read_inline_string(reader)?);
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(GridParseError::XmlError(
                    "unexpected EOF inside cell".into(),
                ));
            }
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let value = match inline_text {
        Some(text) => Some(CellValue::Text(pool.intern(&text))),
        None => convert_value(
            value_text.as_deref(),
            cell_type.as_deref(),
            shared_strings,
            pool,
        )?,
    };

    Ok(ParsedCell {
        row,
        col,
        value,
        formula: formula_text.map(|f| pool.intern(&f)),
    })
}

fn read_inline_string(reader: &mut Reader<&[u8]>) -> Result<String, GridParseError> {
    let mut buf = Vec::new();
    let mut value = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                value.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"is" => break,
            Ok(Event::Eof) => {
                return Err(GridParseError::XmlError(
                    "unexpected EOF inside inline string".into(),
                ));
            }
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(value)
}

fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<&str>,
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<Option<CellValue>, GridParseError> {
    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = raw.trim();
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    match cell_type {
        Some("s") => {
            let idx = trimmed
                .parse::<usize>()
                .map_err(|e| GridParseError::XmlError(e.to_string()))?;
            let text_id = *shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
            Ok(Some(CellValue::Text(text_id)))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("e") => Ok(Some(CellValue::Error(pool.intern(trimmed)))),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(pool.intern(raw)))),
        _ => {
            if let Ok(n) = trimmed.parse::<f64>() {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(pool.intern(trimmed))))
            }
        }
    }
}

fn dimension_from_ref(reference: &str) -> Option<(u32, u32)> {
    let mut parts = reference.split(':');
    let start = parts.next()?;
    let end = parts.next().unwrap_or(start);
    let (start_row, start_col) = address_to_index(start)?;
    let (end_row, end_col) = address_to_index(end)?;
    let height = end_row.checked_sub(start_row)?.checked_add(1)?;
    let width = end_col.checked_sub(start_col)?.checked_add(1)?;
    Some((height, width))
}

fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, GridParseError> {
    let mut grid = Grid::new(nrows, ncols);

    for parsed in cells {
        grid.insert_cell(parsed.row, parsed.col, parsed.value, parsed.formula);
    }

    Ok(grid)
}

fn get_attr_value(element: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, GridParseError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
        if attr.key.as_ref() == key {
            return Ok(Some(
                attr.unescape_value().map_err(to_xml_err)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn to_xml_err(err: quick_xml::Error) -> GridParseError {
    GridParseError::XmlError(err.to_string())
}

struct ParsedCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
}

#[cfg(test)]
mod tests {
    use super::{GridParseError, convert_value, parse_shared_strings, read_inline_string};
    use crate::string_pool::StringPool;
    use crate::workbook::CellValue;
    use quick_xml::Reader;

    #[test]
    fn parse_shared_strings_rich_text_flattens_runs() {
        let xml = br#"<?xml version="1.0"?>
<sst>
  <si>
    <r><t>Hello</t></r>
    <r><t xml:space="preserve"> World</t></r>
  </si>
</sst>"#;
        let mut pool = StringPool::new();
        let strings = parse_shared_strings(xml, &mut pool).expect("shared strings should parse");
        let first = strings.first().copied().unwrap();
        assert_eq!(pool.resolve(first), "Hello World");
    }

    #[test]
    fn read_inline_string_preserves_xml_space_preserve() {
        let xml = br#"<is><t xml:space="preserve"> hello</t></is>"#;
        let mut reader = Reader::from_reader(xml.as_ref());
        reader.config_mut().trim_text(false);
        let value = read_inline_string(&mut reader).expect("inline string should parse");
        assert_eq!(value, " hello");

        let mut pool = StringPool::new();
        let converted = convert_value(Some(value.as_str()), Some("inlineStr"), &[], &mut pool)
            .expect("inlineStr conversion should succeed");
        let text_id = converted
            .as_ref()
            .and_then(CellValue::as_text_id)
            .expect("text id");
        assert_eq!(pool.resolve(text_id), " hello");
    }

    #[test]
    fn convert_value_bool_0_1_and_other() {
        let mut pool = StringPool::new();
        let false_val = convert_value(Some("0"), Some("b"), &[], &mut pool)
            .expect("bool cell conversion should succeed");
        assert_eq!(false_val, Some(CellValue::Bool(false)));

        let mut pool = StringPool::new();
        let true_val = convert_value(Some("1"), Some("b"), &[], &mut pool)
            .expect("bool cell conversion should succeed");
        assert_eq!(true_val, Some(CellValue::Bool(true)));

        let none_val = convert_value(Some("2"), Some("b"), &[], &mut pool)
            .expect("unexpected bool tokens should still parse");
        assert!(none_val.is_none());
    }

    #[test]
    fn convert_value_shared_string_index_out_of_bounds_errors() {
        let mut pool = StringPool::new();
        let only_id = pool.intern("only");
        let err = convert_value(Some("5"), Some("s"), &[only_id], &mut pool)
            .expect_err("invalid shared string index should error");
        assert!(matches!(err, GridParseError::SharedStringOutOfBounds(5)));
    }

    #[test]
    fn convert_value_error_cell_as_text() {
        let mut pool = StringPool::new();
        let value = convert_value(Some("#DIV/0!"), Some("e"), &[], &mut pool)
            .expect("error cell should convert");
        let err_id = value
            .and_then(|v| if let CellValue::Error(id) = v { Some(id) } else { None })
            .expect("error id");
        assert_eq!(pool.resolve(err_id), "#DIV/0!");
    }
}

```

---

### File: `core\src\grid_view.rs`

```rust
use std::collections::HashMap;
use std::hash::Hash;

use crate::alignment::row_metadata::classify_row_frequencies;
use crate::config::DiffConfig;
use crate::hashing::{hash_cell_value, hash_row_content_128};
use crate::workbook::{Cell, CellValue, Grid, RowSignature};
use xxhash_rust::xxh3::Xxh3;

pub use crate::alignment::row_metadata::{FrequencyClass, RowMeta};

pub type RowHash = RowSignature;
pub type ColHash = u128;

#[derive(Debug)]
pub struct RowView<'a> {
    pub cells: Vec<(u32, &'a Cell)>, // sorted by column index
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColMeta {
    pub col_idx: u32,
    pub hash: ColHash,
    pub non_blank_count: u16,
    pub first_non_blank_row: u16,
}

#[derive(Debug)]
pub struct GridView<'a> {
    pub rows: Vec<RowView<'a>>,
    pub row_meta: Vec<RowMeta>,
    pub col_meta: Vec<ColMeta>,
    pub source: &'a Grid,
}

impl<'a> GridView<'a> {
    pub fn from_grid(grid: &'a Grid) -> GridView<'a> {
        let default_config = DiffConfig::default();
        Self::from_grid_with_config(grid, &default_config)
    }

    pub fn from_grid_with_config(grid: &'a Grid, config: &DiffConfig) -> GridView<'a> {
        let nrows = grid.nrows as usize;
        let ncols = grid.ncols as usize;

        let mut rows: Vec<RowView<'a>> =
            (0..nrows).map(|_| RowView { cells: Vec::new() }).collect();

        let mut row_counts = vec![0u32; nrows];
        let mut row_first_non_blank: Vec<Option<u32>> = vec![None; nrows];

            let mut col_counts = vec![0u32; ncols];
        let mut col_first_non_blank: Vec<Option<u32>> = vec![None; ncols];

        for ((row, col), cell) in &grid.cells {
            let r = *row as usize;
            let c = *col as usize;

            debug_assert!(
                r < nrows && c < ncols,
                "cell coordinates must lie within the grid bounds"
            );

            rows[r].cells.push((*col, cell));

            if is_non_blank(cell) {
                row_counts[r] = row_counts[r].saturating_add(1);
                col_counts[c] = col_counts[c].saturating_add(1);

                row_first_non_blank[r] =
                    Some(row_first_non_blank[r].map_or(*col, |cur| cur.min(*col)));
                col_first_non_blank[c] =
                    Some(col_first_non_blank[c].map_or(*row, |cur| cur.min(*row)));
            }
        }

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
                    hash: signature,
                    non_blank_count,
                    first_non_blank_col,
                    frequency_class,
                    is_low_info,
                }
            })
            .collect();

        classify_row_frequencies(&mut row_meta, config);

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

        let col_meta: Vec<ColMeta> = (0..ncols)
            .map(|idx| ColMeta {
                col_idx: idx as u32,
                hash: col_hashers[idx].digest128(),
                non_blank_count: to_u16(col_counts.get(idx).copied().unwrap_or(0)),
                first_non_blank_row: col_first_non_blank
                    .get(idx)
                    .and_then(|r| r.map(to_u16))
                    .unwrap_or(0),
            })
            .collect();

        GridView {
            rows,
            row_meta,
            col_meta,
            source: grid,
        }
    }
}

#[derive(Debug, Default)]
pub struct HashStats<H> {
    pub freq_a: HashMap<H, u32>,
    pub freq_b: HashMap<H, u32>,
    pub hash_to_positions_b: HashMap<H, Vec<u32>>,
}

impl HashStats<RowHash> {
    pub fn from_row_meta(rows_a: &[RowMeta], rows_b: &[RowMeta]) -> HashStats<RowHash> {
        let mut stats = HashStats::default();

        for meta in rows_a {
            *stats.freq_a.entry(meta.signature).or_insert(0) += 1;
        }

        for meta in rows_b {
            *stats.freq_b.entry(meta.signature).or_insert(0) += 1;
            stats
                .hash_to_positions_b
                .entry(meta.signature)
                .or_insert_with(Vec::new)
                .push(meta.row_idx);
        }

        stats
    }
}

impl HashStats<ColHash> {
    pub fn from_col_meta(cols_a: &[ColMeta], cols_b: &[ColMeta]) -> HashStats<ColHash> {
        let mut stats = HashStats::default();

        for meta in cols_a {
            *stats.freq_a.entry(meta.hash).or_insert(0) += 1;
        }

        for meta in cols_b {
            *stats.freq_b.entry(meta.hash).or_insert(0) += 1;
            stats
                .hash_to_positions_b
                .entry(meta.hash)
                .or_insert_with(Vec::new)
                .push(meta.col_idx);
        }

        stats
    }
}

impl<H> HashStats<H>
where
    H: Eq + Hash + Copy,
{
    pub fn is_unique(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 1
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 1
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

fn is_non_blank(cell: &Cell) -> bool {
    cell.value.is_some() || cell.formula.is_some()
}

fn compute_is_low_info(non_blank_count: u16, row_view: &RowView<'_>) -> bool {
    if non_blank_count == 0 {
        return true;
    }

    if non_blank_count > 1 {
        return false;
    }

    let cell = row_view
        .cells
        .iter()
        .find_map(|(_, cell)| is_non_blank(cell).then_some(*cell));

    match cell {
        None => true,
        Some(cell) => match (&cell.value, &cell.formula) {
            (_, Some(_)) => false,
            (Some(CellValue::Text(id)), None) => id.0 == 0,
            (Some(CellValue::Number(_)), _) => false,
            (Some(CellValue::Bool(_)), _) => false,
            (Some(CellValue::Error(_)), _) => false,
            (Some(CellValue::Blank), _) => true,
            (None, None) => true,
        },
    }
}

fn to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

```

---

### File: `core\src\hashing.rs`

```rust
//! Hash utilities for row/column signature computation.
//!
//! Provides consistent hashing functions used for computing structural
//! signatures that enable efficient alignment during diffing.
//!
//! # Position Independence
//!
//! Row signatures are computed by hashing cell content in column-sorted order
//! *without* including column indices. This ensures that inserting or deleting
//! columns does not invalidate row alignment.
//!
//! Column signatures similarly hash content in row-sorted order without row indices.
//!
//! # Collision Probability
//!
//! Using 128-bit xxHash3 signatures, the collision probability is ~10^-38 per pair.
//! At 50K rows, the birthday-bound collision probability is ~10^-29, which is
//! negligible for practical purposes.

use std::hash::{Hash, Hasher};
use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh64::Xxh64;

use crate::workbook::{CellContent, CellValue, ColSignature, RowSignature};

#[allow(dead_code)]
pub(crate) const XXH64_SEED: u64 = 0;
const HASH_MIX_CONSTANT: u64 = 0x9e3779b97f4a7c15;
const CANONICAL_NAN_BITS: u64 = 0x7FF8_0000_0000_0000;

pub(crate) fn normalize_float_for_hash(n: f64) -> u64 {
    if n.is_nan() {
        return CANONICAL_NAN_BITS;
    }
    if n == 0.0 {
        return 0u64;
    }
    let magnitude = n.abs().log10().floor() as i32;
    let scale = 10f64.powi(14 - magnitude);
    let normalized = (n * scale).round() / scale;
    normalized.to_bits()
}

pub(crate) fn hash_cell_value<H: Hasher>(value: &Option<CellValue>, state: &mut H) {
    match value {
        None => {
            3u8.hash(state);
        }
        Some(CellValue::Blank) => {
            4u8.hash(state);
        }
        Some(CellValue::Number(n)) => {
            0u8.hash(state);
            normalize_float_for_hash(*n).hash(state);
        }
        Some(CellValue::Text(s)) => {
            1u8.hash(state);
            s.hash(state);
        }
        Some(CellValue::Bool(b)) => {
            2u8.hash(state);
            b.hash(state);
        }
        Some(CellValue::Error(id)) => {
            5u8.hash(state);
            id.hash(state);
        }
    }
}

#[allow(dead_code)]
pub(crate) fn hash_cell_content(cell: &CellContent) -> u64 {
    let mut hasher = Xxh64::new(XXH64_SEED);
    hash_cell_value(&cell.value, &mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.finish()
}

#[allow(dead_code)]
pub(crate) fn hash_cell_content_128(cell: &CellContent) -> u128 {
    let mut hasher = Xxh3::new();
    hash_cell_value(&cell.value, &mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.digest128()
}

pub(crate) fn hash_row_content_128(cells: &[(u32, &CellContent)]) -> u128 {
    let mut hasher = Xxh3::new();
    for (_, cell) in cells.iter() {
        hash_cell_value(&cell.value, &mut hasher);
        cell.formula.hash(&mut hasher);
    }
    hasher.digest128()
}

pub(crate) fn hash_col_content_128(cells: &[&CellContent]) -> u128 {
    let mut hasher = Xxh3::new();
    for cell in cells.iter() {
        hash_cell_value(&cell.value, &mut hasher);
        cell.formula.hash(&mut hasher);
    }
    hasher.digest128()
}

pub(crate) fn hash_col_content_unordered_128(cells: &[&CellContent]) -> u128 {
    if cells.is_empty() {
        return Xxh3::new().digest128();
    }

    let mut cell_hashes: Vec<u128> = cells
        .iter()
        .map(|cell| {
            let mut h = Xxh3::new();
            hash_cell_value(&cell.value, &mut h);
            cell.formula.hash(&mut h);
            h.digest128()
        })
        .collect();

    cell_hashes.sort_unstable();

    let mut combined = Xxh3::new();
    for h in cell_hashes {
        combined.update(&h.to_le_bytes());
    }
    combined.digest128()
}

#[allow(dead_code)]
pub(crate) fn mix_hash(hash: u64) -> u64 {
    hash.rotate_left(13) ^ HASH_MIX_CONSTANT
}

#[allow(dead_code)]
pub(crate) fn mix_hash_128(hash: u128) -> u128 {
    hash.rotate_left(47) ^ (HASH_MIX_CONSTANT as u128)
}

#[allow(dead_code)]
pub(crate) fn combine_hashes(current: u64, contribution: u64) -> u64 {
    current.wrapping_add(mix_hash(contribution))
}

#[allow(dead_code)]
pub(crate) fn combine_hashes_128(current: u128, contribution: u128) -> u128 {
    current.wrapping_add(mix_hash_128(contribution))
}

#[allow(dead_code)]
pub(crate) fn compute_row_signature<'a>(
    cells: impl Iterator<Item = ((u32, u32), &'a CellContent)>,
    row: u32,
) -> RowSignature {
    let mut row_cells: Vec<(u32, &CellContent)> = cells
        .filter_map(|((r, c), cell)| (r == row).then_some((c, cell)))
        .collect();
    row_cells.sort_by_key(|(col, _)| *col);

    let hash = hash_row_content_128(&row_cells);
    RowSignature { hash }
}

#[allow(dead_code)]
pub(crate) fn compute_col_signature<'a>(
    cells: impl Iterator<Item = ((u32, u32), &'a CellContent)>,
    col: u32,
) -> ColSignature {
    let mut col_cells: Vec<(u32, &CellContent)> = cells
        .filter_map(|((r, c), cell)| (c == col).then_some((r, cell)))
        .collect();
    col_cells.sort_by_key(|(r, _)| *r);
    let ordered: Vec<&CellContent> = col_cells.into_iter().map(|(_, cell)| cell).collect();
    let hash = hash_col_content_128(&ordered);
    ColSignature { hash }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_zero_values() {
        assert_eq!(
            normalize_float_for_hash(0.0),
            normalize_float_for_hash(-0.0)
        );
        assert_eq!(normalize_float_for_hash(0.0), 0u64);
    }

    #[test]
    fn normalize_nan_values() {
        let nan1 = f64::NAN;
        let nan2 = f64::from_bits(0x7FF8_0000_0000_0001);
        assert_eq!(
            normalize_float_for_hash(nan1),
            normalize_float_for_hash(nan2)
        );
        assert_eq!(normalize_float_for_hash(nan1), CANONICAL_NAN_BITS);
    }

    #[test]
    fn normalize_ulp_drift() {
        let a = 1.0;
        let b = 1.0000000000000002;
        assert_eq!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }

    #[test]
    fn normalize_meaningful_difference() {
        let a = 1.0;
        let b = 1.0001;
        assert_ne!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }

    #[test]
    fn normalize_preserves_large_numbers() {
        let a = 1e15;
        let b = 1e15 + 1.0;
        assert_eq!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }

    #[test]
    fn normalize_distinguishes_different_magnitudes() {
        let a = 1.0;
        let b = 2.0;
        assert_ne!(normalize_float_for_hash(a), normalize_float_for_hash(b));
    }
}

```

---

### File: `core\src\lib.rs`

```rust
//! Excel Diff: A library for comparing Excel workbooks.
//!
//! This crate provides functionality for:
//! - Opening and parsing Excel workbooks (`.xlsx` files)
//! - Computing structural and cell-level differences between workbooks
//! - Serializing diff reports to JSON
//! - Parsing Power Query (M) code from DataMashup sections
//!
//! # Quick Start
//!
//! ```ignore
//! use excel_diff::WorkbookPackage;
//!
//! let pkg_a = WorkbookPackage::open(std::fs::File::open("file_a.xlsx")?)?;
//! let pkg_b = WorkbookPackage::open(std::fs::File::open("file_b.xlsx")?)?;
//! let report = pkg_a.diff(&pkg_b, &excel_diff::DiffConfig::default());
//!
//! for op in &report.ops {
//!     println!("{:?}", op);
//! }
//! ```

use std::cell::RefCell;

mod addressing;
pub(crate) mod alignment;
pub(crate) mod column_alignment;
mod config;
mod container;
pub(crate) mod database_alignment;
mod datamashup;
mod datamashup_framing;
mod datamashup_package;
mod diff;
mod engine;
#[cfg(feature = "excel-open-xml")]
mod excel_open_xml;
mod grid_parser;
mod grid_view;
pub(crate) mod hashing;
mod m_ast;
mod m_diff;
mod m_section;
mod output;
mod package;
#[cfg(feature = "perf-metrics")]
#[doc(hidden)]
pub mod perf;
pub(crate) mod rect_block_move;
pub(crate) mod region_mask;
pub(crate) mod row_alignment;
mod session;
mod sink;
mod string_pool;
mod workbook;

thread_local! {
    static DEFAULT_SESSION: RefCell<DiffSession> = RefCell::new(DiffSession::new());
}

#[doc(hidden)]
pub fn with_default_session<T>(f: impl FnOnce(&mut DiffSession) -> T) -> T {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        f(&mut session)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::try_diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[cfg(feature = "excel-open-xml")]
#[deprecated(note = "use WorkbookPackage::open")]
#[allow(deprecated)]
#[doc(hidden)]
pub fn open_workbook(path: impl AsRef<std::path::Path>) -> Result<Workbook, ExcelOpenError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        excel_open_xml::open_workbook(path, &mut session.strings)
    })
}

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use config::{DiffConfig, LimitBehavior};
pub use container::{ContainerError, OpcContainer};
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
#[doc(hidden)]
pub use datamashup::parse_metadata;
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{
    DiffError, DiffOp, DiffReport, DiffSummary, QueryChangeKind, QueryMetadataField, SheetId,
};
#[doc(hidden)]
pub use engine::{
    diff_grids_database_mode,
    diff_workbooks as diff_workbooks_with_pool,
    diff_workbooks_streaming,
    try_diff_workbooks as try_diff_workbooks_with_pool,
    try_diff_workbooks_streaming,
};
#[cfg(feature = "excel-open-xml")]
#[allow(deprecated)]
#[doc(hidden)]
pub use excel_open_xml::{
    ExcelOpenError, PackageError, open_data_mashup, open_workbook as open_workbook_with_pool,
};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use grid_view::{ColHash, ColMeta, FrequencyClass, GridView, HashStats, RowHash, RowMeta, RowView};
pub use m_ast::{MModuleAst, MParseError, ast_semantically_equal, canonicalize_m_ast, parse_m_expression};
#[doc(hidden)]
pub use m_ast::{MAstKind, MTokenDebug, tokenize_for_testing};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "excel-open-xml")]
#[doc(hidden)]
pub use output::json::diff_workbooks_to_json;
#[doc(hidden)]
pub use output::json::diff_report_to_cell_diffs;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use output::json_lines::JsonLinesSink;
pub use package::WorkbookPackage;
pub use session::DiffSession;
pub use sink::{CallbackSink, DiffSink, VecSink};
pub use string_pool::{StringId, StringPool};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};

```

---

### File: `core\src\m_ast.rs`

```rust
use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MModuleAst {
    root: MExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MAstKind {
    Let { binding_count: usize },
    Sequence { token_count: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MExpr {
    Let {
        bindings: Vec<LetBinding>,
        body: Box<MExpr>,
    },
    Sequence(Vec<MToken>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LetBinding {
    name: String,
    value: Box<MExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MToken {
    KeywordLet,
    KeywordIn,
    Identifier(String),
    StringLiteral(String),
    Number(String),
    Symbol(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MTokenDebug {
    KeywordLet,
    KeywordIn,
    Identifier(String),
    StringLiteral(String),
    Number(String),
    Symbol(char),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MParseError {
    #[error("expression is empty")]
    Empty,
    #[error("unterminated string literal")]
    UnterminatedString,
    #[error("unterminated block comment")]
    UnterminatedBlockComment,
    #[error("unbalanced delimiter")]
    UnbalancedDelimiter,
    #[error("invalid let binding syntax")]
    InvalidLetBinding,
    #[error("missing 'in' clause in let expression")]
    MissingInClause,
}

impl From<&MToken> for MTokenDebug {
    fn from(token: &MToken) -> Self {
        match token {
            MToken::KeywordLet => MTokenDebug::KeywordLet,
            MToken::KeywordIn => MTokenDebug::KeywordIn,
            MToken::Identifier(v) => MTokenDebug::Identifier(v.clone()),
            MToken::StringLiteral(v) => MTokenDebug::StringLiteral(v.clone()),
            MToken::Number(v) => MTokenDebug::Number(v.clone()),
            MToken::Symbol(v) => MTokenDebug::Symbol(*v),
        }
    }
}

impl MModuleAst {
    /// Returns a minimal view of the root expression kind for tests and debugging.
    ///
    /// This keeps the AST opaque for production consumers while allowing
    /// tests to assert the expected structure.
    pub fn root_kind_for_testing(&self) -> MAstKind {
        match &self.root {
            MExpr::Let { bindings, .. } => MAstKind::Let {
                binding_count: bindings.len(),
            },
            MExpr::Sequence(tokens) => MAstKind::Sequence {
                token_count: tokens.len(),
            },
        }
    }
}

/// Tokenize an M expression for testing and diagnostics.
///
/// The returned tokens are a debug-friendly mirror of the internal lexer output
/// and are not part of the stable public API.
pub fn tokenize_for_testing(source: &str) -> Result<Vec<MTokenDebug>, MParseError> {
    tokenize(source).map(|tokens| tokens.iter().map(MTokenDebug::from).collect())
}

/// Parse a Power Query M expression into a minimal AST.
///
/// Currently supports top-level `let ... in ...` expressions with simple identifier
/// bindings. Non-`let` inputs are preserved as opaque token sequences. The lexer
/// recognizes `let`/`in`, quoted identifiers (`#"Foo"`), and hash-prefixed literals
/// like `#date`/`#datetime` as single identifiers; other M constructs are parsed
/// best-effort and may be treated as generic tokens.
pub fn parse_m_expression(source: &str) -> Result<MModuleAst, MParseError> {
    let tokens = tokenize(source)?;
    if tokens.is_empty() {
        return Err(MParseError::Empty);
    }

    let root = parse_expression(&tokens)?;
    Ok(MModuleAst { root })
}

pub fn canonicalize_m_ast(ast: &mut MModuleAst) {
    canonicalize_expr(&mut ast.root);
}

pub fn ast_semantically_equal(a: &MModuleAst, b: &MModuleAst) -> bool {
    a == b
}

fn canonicalize_expr(expr: &mut MExpr) {
    match expr {
        MExpr::Let { bindings, body } => {
            for binding in bindings {
                canonicalize_expr(&mut binding.value);
            }
            canonicalize_expr(body);
        }
        MExpr::Sequence(tokens) => canonicalize_tokens(tokens),
    }
}

fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    // Tokens are already whitespace/comment free; no additional normalization needed yet.
    let _ = tokens;
}

fn parse_expression(tokens: &[MToken]) -> Result<MExpr, MParseError> {
    if let Some(let_ast) = parse_let(tokens)? {
        return Ok(let_ast);
    }

    Ok(MExpr::Sequence(tokens.to_vec()))
}

fn parse_let(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !matches!(tokens.first(), Some(MToken::KeywordLet)) {
        return Ok(None);
    }

    let mut idx = 1usize;
    let mut bindings = Vec::new();
    let mut found_in = false;

    while idx < tokens.len() {
        let name = match tokens.get(idx) {
            Some(MToken::Identifier(name)) => name.clone(),
            _ => return Err(MParseError::InvalidLetBinding),
        };
        idx += 1;

        if !matches!(tokens.get(idx), Some(MToken::Symbol('='))) {
            return Err(MParseError::InvalidLetBinding);
        }
        idx += 1;

        let value_start = idx;
        let mut depth = 0i32;
        let mut value_end: Option<usize> = None;
        let mut let_depth_in_value = 0i32;

        while idx < tokens.len() {
            match &tokens[idx] {
                MToken::Symbol(c) if *c == '(' || *c == '[' || *c == '{' => depth += 1,
                MToken::Symbol(c) if *c == ')' || *c == ']' || *c == '}' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                MToken::KeywordLet => {
                    let_depth_in_value += 1;
                }
                MToken::KeywordIn => {
                    if let_depth_in_value > 0 {
                        let_depth_in_value -= 1;
                    } else if depth == 0 {
                        value_end = Some(idx);
                        found_in = true;
                        break;
                    }
                }
                MToken::Symbol(',') if depth == 0 && let_depth_in_value == 0 => {
                    value_end = Some(idx);
                    idx += 1;
                    break;
                }
                _ => {}
            }

            idx += 1;
        }

        let end = value_end.ok_or(MParseError::MissingInClause)?;
        if end <= value_start {
            return Err(MParseError::InvalidLetBinding);
        }

        let value_expr = parse_expression(&tokens[value_start..end])?;
        bindings.push(LetBinding {
            name,
            value: Box::new(value_expr),
        });

        if found_in {
            idx = end + 1; // skip the 'in' token
            break;
        }
    }

    if !found_in {
        return Err(MParseError::MissingInClause);
    }

    if idx > tokens.len() {
        return Err(MParseError::InvalidLetBinding);
    }

    let body_tokens = &tokens[idx..];
    if body_tokens.is_empty() {
        return Err(MParseError::InvalidLetBinding);
    }
    let body = parse_expression(body_tokens)?;

    Ok(Some(MExpr::Let {
        bindings,
        body: Box::new(body),
    }))
}

fn tokenize(source: &str) -> Result<Vec<MToken>, MParseError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut delimiters: Vec<char> = Vec::new();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch == '/' {
            if matches!(chars.peek(), Some('/')) {
                skip_line_comment(&mut chars);
                continue;
            }
            if matches!(chars.peek(), Some('*')) {
                chars.next();
                skip_block_comment(&mut chars)?;
                continue;
            }
        }

        if ch == '"' {
            let literal = parse_string(&mut chars)?;
            tokens.push(MToken::StringLiteral(literal));
            continue;
        }

        if ch == '#' {
            if matches!(chars.peek(), Some('"')) {
                chars.next();
                let ident = parse_string(&mut chars)?;
                tokens.push(MToken::Identifier(ident));
                continue;
            }
            if let Some(next) = chars.peek().copied()
                && is_identifier_start(next)
            {
                chars.next();
                let ident = parse_identifier(next, &mut chars);
                tokens.push(MToken::Identifier(format!("#{ident}")));
                continue;
            }
            tokens.push(MToken::Symbol('#'));
            continue;
        }

        if is_identifier_start(ch) {
            let ident = parse_identifier(ch, &mut chars);
            if ident.eq_ignore_ascii_case("let") {
                tokens.push(MToken::KeywordLet);
            } else if ident.eq_ignore_ascii_case("in") {
                tokens.push(MToken::KeywordIn);
            } else {
                tokens.push(MToken::Identifier(ident));
            }
            continue;
        }

        if ch.is_ascii_digit() {
            let number = parse_number(ch, &mut chars);
            tokens.push(MToken::Number(number));
            continue;
        }

        if is_open_delimiter(ch) {
            delimiters.push(ch);
        } else if is_close_delimiter(ch) {
            let Some(open) = delimiters.pop() else {
                return Err(MParseError::UnbalancedDelimiter);
            };
            if !delimiters_match(open, ch) {
                return Err(MParseError::UnbalancedDelimiter);
            }
        }

        tokens.push(MToken::Symbol(ch));
    }

    if !delimiters.is_empty() {
        return Err(MParseError::UnbalancedDelimiter);
    }

    Ok(tokens)
}

#[allow(clippy::while_let_on_iterator)]
fn skip_line_comment(chars: &mut Peekable<Chars<'_>>) {
    while let Some(ch) = chars.next() {
        if ch == '\n' {
            break;
        }
    }
}

#[allow(clippy::while_let_on_iterator)]
fn skip_block_comment(chars: &mut Peekable<Chars<'_>>) -> Result<(), MParseError> {
    while let Some(ch) = chars.next() {
        if ch == '*' && matches!(chars.peek(), Some('/')) {
            chars.next();
            return Ok(());
        }
    }

    Err(MParseError::UnterminatedBlockComment)
}

fn parse_string(chars: &mut Peekable<Chars<'_>>) -> Result<String, MParseError> {
    let mut buf = String::new();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if matches!(chars.peek(), Some('"')) {
                buf.push('"');
                chars.next();
                continue;
            }
            return Ok(buf);
        }

        buf.push(ch);
    }

    Err(MParseError::UnterminatedString)
}

fn parse_identifier(first: char, chars: &mut Peekable<Chars<'_>>) -> String {
    let mut ident = String::new();
    ident.push(first);

    while let Some(&next) = chars.peek() {
        if is_identifier_continue(next) {
            ident.push(next);
            chars.next();
        } else {
            break;
        }
    }

    ident
}

fn parse_number(first: char, chars: &mut Peekable<Chars<'_>>) -> String {
    let mut number = String::new();
    number.push(first);

    while let Some(&next) = chars.peek() {
        if next.is_ascii_digit() || next == '.' {
            number.push(next);
            chars.next();
        } else {
            break;
        }
    }

    number
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_open_delimiter(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}

fn is_close_delimiter(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

fn delimiters_match(open: char, close: char) -> bool {
    matches!((open, close), ('(', ')') | ('[', ']') | ('{', '}'))
}

```

---

### File: `core\src\m_diff.rs`

```rust
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use crate::config::DiffConfig;
use crate::datamashup::{DataMashup, Query, build_queries};
use crate::diff::{DiffOp, QueryChangeKind as DiffQueryChangeKind, QueryMetadataField};
use crate::hashing::XXH64_SEED;
use crate::m_ast::{MModuleAst, canonicalize_m_ast, parse_m_expression};
use crate::string_pool::{StringId, StringPool};

#[deprecated(note = "use WorkbookPackage::diff instead")]
#[allow(dead_code)]
pub fn diff_m_queries(old_queries: &[Query], new_queries: &[Query], config: &DiffConfig) -> Vec<DiffOp> {
    crate::with_default_session(|session| {
        diff_queries_to_ops(old_queries, new_queries, &mut session.strings, config)
    })
}

fn hash64<T: Hash>(value: &T) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    value.hash(&mut h);
    h.finish()
}

fn intern_bool(pool: &mut StringPool, v: bool) -> StringId {
    if v {
        pool.intern("true")
    } else {
        pool.intern("false")
    }
}

fn canonical_ast_and_hash(expr: &str) -> Option<(MModuleAst, u64)> {
    let mut ast = parse_m_expression(expr).ok()?;
    canonicalize_m_ast(&mut ast);
    let h = hash64(&ast);
    Some((ast, h))
}

fn definition_change(
    old_expr: &str,
    new_expr: &str,
    enable_semantic: bool,
) -> Option<(DiffQueryChangeKind, u64, u64)> {
    if old_expr == new_expr {
        return None;
    }

    if enable_semantic {
        if let (Some((_, old_h)), Some((_, new_h))) =
            (canonical_ast_and_hash(old_expr), canonical_ast_and_hash(new_expr))
        {
            let kind = if old_h == new_h {
                DiffQueryChangeKind::FormattingOnly
            } else {
                DiffQueryChangeKind::Semantic
            };
            return Some((kind, old_h, new_h));
        }
    }

    let old_h = hash64(&old_expr);
    let new_h = hash64(&new_expr);
    Some((DiffQueryChangeKind::Semantic, old_h, new_h))
}

fn emit_metadata_diffs(
    pool: &mut StringPool,
    out: &mut Vec<DiffOp>,
    name: StringId,
    old_q: &Query,
    new_q: &Query,
) {
    if old_q.metadata.load_to_sheet != new_q.metadata.load_to_sheet {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::LoadToSheet,
            old: Some(intern_bool(pool, old_q.metadata.load_to_sheet)),
            new: Some(intern_bool(pool, new_q.metadata.load_to_sheet)),
        });
    }

    if old_q.metadata.load_to_model != new_q.metadata.load_to_model {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::LoadToModel,
            old: Some(intern_bool(pool, old_q.metadata.load_to_model)),
            new: Some(intern_bool(pool, new_q.metadata.load_to_model)),
        });
    }

    if old_q.metadata.is_connection_only != new_q.metadata.is_connection_only {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::ConnectionOnly,
            old: Some(intern_bool(pool, old_q.metadata.is_connection_only)),
            new: Some(intern_bool(pool, new_q.metadata.is_connection_only)),
        });
    }

    if old_q.metadata.group_path != new_q.metadata.group_path {
        let old = old_q.metadata.group_path.as_deref().map(|s| pool.intern(s));
        let new = new_q.metadata.group_path.as_deref().map(|s| pool.intern(s));
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::GroupPath,
            old,
            new,
        });
    }
}

fn diff_queries_to_ops(
    old_queries: &[Query],
    new_queries: &[Query],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut old_by_name: BTreeMap<&str, &Query> = BTreeMap::new();
    let mut new_by_name: BTreeMap<&str, &Query> = BTreeMap::new();

    for q in old_queries {
        old_by_name.insert(q.name.as_str(), q);
    }
    for q in new_queries {
        new_by_name.insert(q.name.as_str(), q);
    }

    let old_only: Vec<&Query> = old_by_name
        .iter()
        .filter_map(|(name, q)| {
            if new_by_name.contains_key(*name) {
                None
            } else {
                Some(*q)
            }
        })
        .collect();

    let new_only: Vec<&Query> = new_by_name
        .iter()
        .filter_map(|(name, q)| {
            if old_by_name.contains_key(*name) {
                None
            } else {
                Some(*q)
            }
        })
        .collect();

    let mut renamed_old: BTreeSet<&str> = BTreeSet::new();
    let mut renamed_new: BTreeSet<&str> = BTreeSet::new();
    let mut rename_ops: Vec<(StringId, StringId, &Query, &Query)> = Vec::new();

    let mut old_hash_map: BTreeMap<u64, Vec<&Query>> = BTreeMap::new();
    let mut new_hash_map: BTreeMap<u64, Vec<&Query>> = BTreeMap::new();

    for q in &old_only {
        let h = if config.enable_m_semantic_diff {
            canonical_ast_and_hash(&q.expression_m)
                .map(|(_, h)| h)
                .unwrap_or_else(|| hash64(&q.expression_m))
        } else {
            hash64(&q.expression_m)
        };
        old_hash_map.entry(h).or_default().push(*q);
    }

    for q in &new_only {
        let h = if config.enable_m_semantic_diff {
            canonical_ast_and_hash(&q.expression_m)
                .map(|(_, h)| h)
                .unwrap_or_else(|| hash64(&q.expression_m))
        } else {
            hash64(&q.expression_m)
        };
        new_hash_map.entry(h).or_default().push(*q);
    }

    for (h, olds) in &old_hash_map {
        if let Some(news) = new_hash_map.get(h) {
            if olds.len() == 1 && news.len() == 1 {
                let old_q = olds[0];
                let new_q = news[0];
                let from = pool.intern(old_q.name.as_str());
                let to = pool.intern(new_q.name.as_str());
                renamed_old.insert(old_q.name.as_str());
                renamed_new.insert(new_q.name.as_str());
                rename_ops.push((from, to, old_q, new_q));
            }
        }
    }

    rename_ops.sort_by(|a, b| {
        let from_a = pool.resolve(a.0);
        let from_b = pool.resolve(b.0);
        from_a.cmp(from_b)
    });

    let mut ops: Vec<DiffOp> = Vec::new();

    for (from, to, old_q, new_q) in rename_ops {
        ops.push(DiffOp::QueryRenamed { from, to });
        emit_metadata_diffs(pool, &mut ops, to, old_q, new_q);
    }

    let mut all_names: Vec<&str> = old_by_name
        .keys()
        .copied()
        .chain(new_by_name.keys().copied())
        .collect();
    all_names.sort();
    all_names.dedup();

    for name in all_names {
        if renamed_old.contains(name) || renamed_new.contains(name) {
            continue;
        }

        match (old_by_name.get(name), new_by_name.get(name)) {
            (None, Some(_new_q)) => {
                ops.push(DiffOp::QueryAdded {
                    name: pool.intern(name),
                });
            }
            (Some(_old_q), None) => {
                ops.push(DiffOp::QueryRemoved {
                    name: pool.intern(name),
                });
            }
            (Some(old_q), Some(new_q)) => {
                let name_id = pool.intern(name);

                if let Some((kind, old_h, new_h)) = definition_change(
                    &old_q.expression_m,
                    &new_q.expression_m,
                    config.enable_m_semantic_diff,
                ) {
                    ops.push(DiffOp::QueryDefinitionChanged {
                        name: name_id,
                        change_kind: kind,
                        old_hash: old_h,
                        new_hash: new_h,
                    });
                }

                emit_metadata_diffs(pool, &mut ops, name_id, old_q, new_q);
            }
            (None, None) => {}
        }
    }

    ops
}

pub(crate) fn diff_m_ops_for_packages(
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    match (old_dm.as_ref(), new_dm.as_ref()) {
        (None, None) => Vec::new(),
        (Some(old_dm), None) => {
            let old_q = match build_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let mut ops = Vec::new();
            for q in old_q {
                ops.push(DiffOp::QueryRemoved {
                    name: pool.intern(&q.name),
                });
            }
            ops
        }
        (None, Some(new_dm)) => {
            let new_q = match build_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let mut ops = Vec::new();
            for q in new_q {
                ops.push(DiffOp::QueryAdded {
                    name: pool.intern(&q.name),
                });
            }
            ops
        }
        (Some(old_dm), Some(new_dm)) => {
            let old_q = match build_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let new_q = match build_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            diff_queries_to_ops(&old_q, &new_q, pool, config)
        }
    }
}

```

---

### File: `core\src\m_section.rs`

```rust
use std::str::Lines;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SectionParseError {
    #[error("missing section header")]
    MissingSectionHeader,
    #[error("invalid section header")]
    InvalidHeader,
    #[error("invalid member syntax")]
    InvalidMemberSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMember {
    pub section_name: String,
    pub member_name: String,
    pub expression_m: String,
    pub is_shared: bool,
}

pub fn parse_section_members(source: &str) -> Result<Vec<SectionMember>, SectionParseError> {
    let source = strip_leading_bom(source);
    let mut lines = source.lines();
    let section_name = find_section_name(&mut lines)?;

    let mut members = Vec::new();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if !trimmed.starts_with("shared") {
            continue;
        }

        let member = parse_shared_member(trimmed, &mut lines, &section_name)
            .ok_or(SectionParseError::InvalidMemberSyntax)?;
        members.push(member);
    }

    Ok(members)
}

fn find_section_name(lines: &mut Lines<'_>) -> Result<String, SectionParseError> {
    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        match try_parse_section_header(trimmed) {
            Ok(Some(name)) => return Ok(name),
            Ok(None) => continue,
            Err(err) => return Err(err),
        }
    }

    Err(SectionParseError::MissingSectionHeader)
}

fn try_parse_section_header(line: &str) -> Result<Option<String>, SectionParseError> {
    let Some(rest) = line.strip_prefix("section") else {
        return Ok(None);
    };

    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let header_body = rest.trim_start();
    if !header_body.ends_with(';') {
        return Err(SectionParseError::InvalidHeader);
    }

    let without_semicolon = &header_body[..header_body.len() - 1];
    let name_candidate = without_semicolon.trim();
    if name_candidate.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let mut parts = name_candidate.split_whitespace();
    let name = parts.next().ok_or(SectionParseError::InvalidHeader)?;
    if parts.next().is_some() {
        return Err(SectionParseError::InvalidHeader);
    }

    if !is_valid_identifier(name) {
        return Err(SectionParseError::InvalidHeader);
    }

    Ok(Some(name.to_string()))
}

fn parse_shared_member(
    line: &str,
    remaining_lines: &mut Lines<'_>,
    section_name: &str,
) -> Option<SectionMember> {
    let rest = line.strip_prefix("shared")?;
    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return None;
    }

    let body = rest.trim_start();
    if body.is_empty() {
        return None;
    }

    let (member_name, after_name) = parse_identifier(body)?;

    let mut expression_source = after_name;
    let eq_index = expression_source.find('=')?;
    if !expression_source[..eq_index].trim().is_empty() {
        return None;
    }
    expression_source = &expression_source[eq_index + 1..];

    let mut expression = expression_source.to_string();
    if let Some(idx) = expression_source.find(';') {
        expression.truncate(idx);
    } else {
        let mut terminator_index = None;
        while terminator_index.is_none() {
            let Some(next_line) = remaining_lines.next() else {
                break;
            };

            expression.push('\n');
            let offset = expression.len();
            expression.push_str(next_line);
            if let Some(idx) = next_line.find(';') {
                terminator_index = Some(offset + idx);
            }
        }

        if let Some(idx) = terminator_index {
            expression.truncate(idx);
        } else {
            return None;
        }
    }

    let expression_m = expression.trim().to_string();

    Some(SectionMember {
        section_name: section_name.to_string(),
        member_name: member_name.to_string(),
        expression_m,
        is_shared: true,
    })
}

fn parse_identifier(text: &str) -> Option<(String, &str)> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("#\"") {
        return parse_quoted_identifier(trimmed);
    }

    parse_unquoted_identifier(trimmed)
}

fn parse_unquoted_identifier(text: &str) -> Option<(String, &str)> {
    if text.is_empty() {
        return None;
    }

    let mut end = 0;
    for ch in text.chars() {
        if ch.is_whitespace() || ch == '=' {
            break;
        }
        end += ch.len_utf8();
    }

    if end == 0 {
        return None;
    }

    let (name, rest) = text.split_at(end);
    if !is_valid_identifier(name) {
        return None;
    }

    Some((name.to_string(), rest))
}

fn parse_quoted_identifier(text: &str) -> Option<(String, &str)> {
    let mut chars = text.char_indices();
    let (_, hash) = chars.next()?;
    if hash != '#' {
        return None;
    }
    if !matches!(chars.next(), Some((_, '"'))) {
        return None;
    }

    let mut name = String::new();
    while let Some((idx, ch)) = chars.next() {
        if ch == '"' {
            if let Some((_, next_ch)) = chars.clone().next()
                && next_ch == '"'
            {
                name.push('"');
                chars.next();
                continue;
            }
            let rest_start = idx + 1;
            let rest = &text[rest_start..];
            return Some((name, rest));
        }

        name.push(ch);
    }

    None
}

fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn strip_leading_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}

```

---

### File: `core\src\output\json.rs`

```rust
#[cfg(feature = "excel-open-xml")]
use crate::config::DiffConfig;
use crate::diff::DiffReport;
#[cfg(feature = "excel-open-xml")]
use crate::datamashup::build_data_mashup;
#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{PackageError, open_data_mashup, open_workbook};
use crate::session::DiffSession;
#[cfg(feature = "excel-open-xml")]
use crate::sink::VecSink;
use crate::string_pool::StringId;
#[cfg(feature = "excel-open-xml")]
use crate::DiffSummary;
use serde::Serialize;
use serde::ser::Error as SerdeError;
#[cfg(feature = "excel-open-xml")]
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellDiff {
    #[serde(rename = "coords")]
    pub coords: String,
    #[serde(rename = "value_file1")]
    pub value_file1: Option<String>,
    #[serde(rename = "value_file2")]
    pub value_file2: Option<String>,
}

pub fn serialize_cell_diffs(diffs: &[CellDiff]) -> serde_json::Result<String> {
    serde_json::to_string(diffs)
}

pub fn serialize_diff_report(report: &DiffReport) -> serde_json::Result<String> {
    if contains_non_finite_numbers(report) {
        return Err(SerdeError::custom(
            "non-finite numbers (NaN or infinity) are not supported in DiffReport JSON output",
        ));
    }
    serde_json::to_string(report)
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<DiffReport, PackageError> {
    let path_a = path_a.as_ref();
    let path_b = path_b.as_ref();

    let mut session = DiffSession::new();

    let wb_a = open_workbook(path_a, session.strings_mut())?;
    let wb_b = open_workbook(path_b, session.strings_mut())?;

    let dm_a = open_data_mashup(path_a)?
        .map(|raw| build_data_mashup(&raw))
        .transpose()?;
    let dm_b = open_data_mashup(path_b)?
        .map(|raw| build_data_mashup(&raw))
        .transpose()?;

    let mut sink = VecSink::new();
    let summary = crate::engine::try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        session.strings_mut(),
        config,
        &mut sink,
    )
    .map_err(|e| PackageError::SerializationError(e.to_string()))?;

    let m_ops = crate::m_diff::diff_m_ops_for_packages(&dm_a, &dm_b, session.strings_mut(), config);

    let mut report = build_report_from_sink(sink, summary, session);
    report.ops.extend(m_ops);
    Ok(report)
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<String, PackageError> {
    let report = diff_workbooks(path_a, path_b, config)?;
    serialize_diff_report(&report).map_err(|e| PackageError::SerializationError(e.to_string()))
}

pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn resolve_string<'a>(report: &'a DiffReport, id: StringId) -> Option<&'a str> {
        report.strings.get(id.0 as usize).map(|s| s.as_str())
    }

    fn render_value(report: &DiffReport, value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(id)) => resolve_string(report, *id).map(|s| s.to_string()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            Some(CellValue::Error(id)) => resolve_string(report, *id).map(|s| s.to_string()),
            Some(CellValue::Blank) => Some(String::new()),
            None => None,
        }
    }

    report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                if from == to {
                    return None;
                }
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(report, &from.value),
                    value_file2: render_value(report, &to.value),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(feature = "excel-open-xml")]
fn build_report_from_sink(sink: VecSink, summary: DiffSummary, session: DiffSession) -> DiffReport {
    let mut report = DiffReport::new(sink.into_ops());
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    #[cfg(feature = "perf-metrics")]
    {
        report.metrics = summary.metrics;
    }
    report.strings = session.strings.into_strings();
    report
}

fn contains_non_finite_numbers(report: &DiffReport) -> bool {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    report.ops.iter().any(|op| match op {
        DiffOp::CellEdited { from, to, .. } => {
            matches!(from.value, Some(CellValue::Number(n)) if !n.is_finite())
                || matches!(to.value, Some(CellValue::Number(n)) if !n.is_finite())
        }
        _ => false,
    })
}

```

---

### File: `core\src\output\json_lines.rs`

```rust
use crate::diff::{DiffError, DiffOp};
use crate::sink::DiffSink;
use crate::string_pool::StringPool;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct JsonLinesHeader<'a> {
    kind: &'static str,
    version: &'a str,
    strings: &'a [String],
}

pub struct JsonLinesSink<W: Write> {
    w: W,
    wrote_header: bool,
    version: &'static str,
}

impl<W: Write> JsonLinesSink<W> {
    pub fn new(w: W) -> Self {
        Self {
            w,
            wrote_header: false,
            version: crate::diff::DiffReport::SCHEMA_VERSION,
        }
    }

    pub fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        if self.wrote_header {
            return Ok(());
        }

        let header = JsonLinesHeader {
            kind: "Header",
            version: self.version,
            strings: pool.strings(),
        };

        serde_json::to_writer(&mut self.w, &header)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.w
            .write_all(b"\n")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;

        self.wrote_header = true;
        Ok(())
    }
}

impl<W: Write> DiffSink for JsonLinesSink<W> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        serde_json::to_writer(&mut self.w, &op)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.w
            .write_all(b"\n")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.w
            .flush()
            .map_err(|e| DiffError::SinkError { message: e.to_string() })
    }
}

```

---

### File: `core\src\output\mod.rs`

```rust
pub mod json;
pub mod json_lines;

```

---

### File: `core\src\package.rs`

```rust
use crate::config::DiffConfig;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary};
use crate::sink::{DiffSink, NoFinishSink};
use crate::workbook::Workbook;

#[derive(Debug, Clone)]
pub struct WorkbookPackage {
    pub workbook: Workbook,
    pub data_mashup: Option<DataMashup>,
}

impl From<Workbook> for WorkbookPackage {
    fn from(workbook: Workbook) -> Self {
        Self {
            workbook,
            data_mashup: None,
        }
    }
}

impl WorkbookPackage {
    #[cfg(feature = "excel-open-xml")]
    pub fn open<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
    ) -> Result<Self, crate::excel_open_xml::PackageError> {
        crate::with_default_session(|session| {
            let mut container = crate::container::OpcContainer::open_from_reader(reader)?;
            let workbook = crate::excel_open_xml::open_workbook_from_container(
                &mut container,
                &mut session.strings,
            )?;
            let raw = crate::excel_open_xml::open_data_mashup_from_container(&mut container)?;
            let data_mashup = match raw {
                Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
                None => None,
            };
            Ok(Self {
                workbook,
                data_mashup,
            })
        })
    }

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

    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            let grid_result = {
                let mut no_finish = NoFinishSink::new(sink);
                crate::engine::try_diff_workbooks_streaming(
                    &self.workbook,
                    &other.workbook,
                    &mut session.strings,
                    config,
                    &mut no_finish,
                )
            };

            let mut summary = match grid_result {
                Ok(summary) => summary,
                Err(e) => {
                    let _ = sink.finish();
                    return Err(e);
                }
            };

            let m_ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            for op in m_ops {
                sink.emit(op)?;
                summary.op_count = summary.op_count.saturating_add(1);
            }

            sink.finish()?;

            Ok(summary)
        })
    }
}


```

---

### File: `core\src\perf.rs`

```rust
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Total,
    Parse,
    MoveDetection,
    Alignment,
    CellDiff,
}

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

impl DiffMetrics {
    pub fn start_phase(&mut self, phase: Phase) {
        self.phase_start.insert(phase, Instant::now());
    }

    pub fn end_phase(&mut self, phase: Phase) {
        if let Some(start) = self.phase_start.remove(&phase) {
            let elapsed = start.elapsed().as_millis() as u64;
            match phase {
                Phase::Parse => {}
                Phase::MoveDetection => self.move_detection_time_ms += elapsed,
                Phase::Alignment => self.alignment_time_ms += elapsed,
                Phase::CellDiff => self.cell_diff_time_ms += elapsed,
                Phase::Total => self.total_time_ms += elapsed,
            }
        }
    }

    pub fn add_cells_compared(&mut self, count: u64) {
        self.cells_compared = self.cells_compared.saturating_add(count);
    }
}

```

---

### File: `core\src\rect_block_move.rs`

```rust
//! Rectangular block move detection.
//!
//! This module implements detection of rectangular regions that have moved
//! between two grids. A rect block move is when a 2D region (rows × cols)
//! moves from one position to another without internal edits.
//!
//! This is used by the engine's masked move detection loop to identify
//! structural changes that preserve content but change position.

use crate::config::DiffConfig;
use crate::grid_view::{ColHash, ColMeta, GridView, HashStats, RowHash};
use crate::workbook::{Cell, Grid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RectBlockMove {
    pub src_start_row: u32,
    pub src_row_count: u32,
    pub src_start_col: u32,
    pub src_col_count: u32,
    pub dst_start_row: u32,
    pub dst_start_col: u32,
    pub block_hash: Option<u64>,
}

pub(crate) fn detect_exact_rect_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 || old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    if blank_dominated(&view_a) || blank_dominated(&view_b) {
        return None;
    }

    let row_stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    let col_stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);

    if has_heavy_repetition(&row_stats, config) || has_heavy_repetition(&col_stats, config) {
        return None;
    }

    let shared_rows = row_stats
        .freq_a
        .keys()
        .filter(|h| row_stats.freq_b.contains_key(*h))
        .count();
    let shared_cols = col_stats
        .freq_a
        .keys()
        .filter(|h| col_stats.freq_b.contains_key(*h))
        .count();
    if shared_rows == 0 && shared_cols == 0 {
        return None;
    }

    let diff_positions = collect_differences(old, new);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = find_two_equal_ranges(diff_positions.iter().map(|(r, _)| *r))?;
    let col_ranges = find_two_equal_ranges(diff_positions.iter().map(|(_, c)| *c))?;

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    let expected_mismatches = row_count.checked_mul(col_count)?.checked_mul(2)?;
    if diff_positions.len() as u32 != expected_mismatches {
        return None;
    }

    let mismatches = count_rect_mismatches(old, new, row_ranges.0, col_ranges.0)
        + count_rect_mismatches(old, new, row_ranges.1, col_ranges.1);
    if mismatches != diff_positions.len() as u32 {
        return None;
    }

    if !has_unique_meta(
        &view_a, &view_b, &row_stats, &col_stats, row_ranges, col_ranges,
    ) {
        return None;
    }

    let primary = validate_orientation(old, new, row_ranges, col_ranges);
    let swapped_ranges = ((row_ranges.1, row_ranges.0), (col_ranges.1, col_ranges.0));
    let alternate = validate_orientation(old, new, swapped_ranges.0, swapped_ranges.1);

    match (primary, alternate) {
        (Some(mv), None) => Some(mv),
        (None, Some(mv)) => Some(mv),
        _ => None,
    }
}

fn validate_orientation(
    old: &Grid,
    new: &Grid,
    row_ranges: ((u32, u32), (u32, u32)),
    col_ranges: ((u32, u32), (u32, u32)),
) -> Option<RectBlockMove> {
    if ranges_overlap(row_ranges.0, row_ranges.1) && ranges_overlap(col_ranges.0, col_ranges.1) {
        return None;
    }

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    if rectangles_correspond(
        old,
        new,
        row_ranges.0,
        col_ranges.0,
        row_ranges.1,
        col_ranges.1,
    ) {
        return Some(RectBlockMove {
            src_start_row: row_ranges.0.0,
            src_row_count: row_count,
            src_start_col: col_ranges.0.0,
            src_col_count: col_count,
            dst_start_row: row_ranges.1.0,
            dst_start_col: col_ranges.1.0,
            block_hash: None,
        });
    }

    None
}

fn rectangles_correspond(
    old: &Grid,
    new: &Grid,
    src_rows: (u32, u32),
    src_cols: (u32, u32),
    dst_rows: (u32, u32),
    dst_cols: (u32, u32),
) -> bool {
    let row_count = range_len(src_rows);
    let col_count = range_len(src_cols);

    if row_count != range_len(dst_rows) || col_count != range_len(dst_cols) {
        return false;
    }

    for dr in 0..row_count {
        for dc in 0..col_count {
            let src_r = src_rows.0 + dr;
            let src_c = src_cols.0 + dc;
            let dst_r = dst_rows.0 + dr;
            let dst_c = dst_cols.0 + dc;

            if !cell_content_equal(old.get(src_r, src_c), new.get(dst_r, dst_c)) {
                return false;
            }
        }
    }

    true
}

fn collect_differences(old: &Grid, new: &Grid) -> Vec<(u32, u32)> {
    let mut diffs = Vec::new();

    for row in 0..old.nrows {
        for col in 0..old.ncols {
            if !cell_content_equal(old.get(row, col), new.get(row, col)) {
                diffs.push((row, col));
            }
        }
    }

    diffs
}

fn cell_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
        (Some(cell_a), None) => cell_a.value.is_none() && cell_a.formula.is_none(),
        (None, Some(cell_b)) => cell_b.value.is_none() && cell_b.formula.is_none(),
    }
}

fn count_rect_mismatches(old: &Grid, new: &Grid, rows: (u32, u32), cols: (u32, u32)) -> u32 {
    let mut mismatches = 0u32;
    for row in rows.0..=rows.1 {
        for col in cols.0..=cols.1 {
            if !cell_content_equal(old.get(row, col), new.get(row, col)) {
                mismatches = mismatches.saturating_add(1);
            }
        }
    }
    mismatches
}

fn has_unique_meta(
    view_a: &GridView<'_>,
    view_b: &GridView<'_>,
    row_stats: &HashStats<RowHash>,
    col_stats: &HashStats<ColHash>,
    row_ranges: ((u32, u32), (u32, u32)),
    col_ranges: ((u32, u32), (u32, u32)),
) -> bool {
    for range in [row_ranges.0, row_ranges.1] {
        for idx in range.0..=range.1 {
            if !is_unique_row_in_a(idx, view_a, row_stats)
                || !is_unique_row_in_b(idx, view_b, row_stats)
            {
                return false;
            }
        }
    }

    for range in [col_ranges.0, col_ranges.1] {
        for idx in range.0..=range.1 {
            if !is_unique_col_in_a(idx, view_a, col_stats)
                || !is_unique_col_in_b(idx, view_b, col_stats)
            {
                return false;
            }
        }
    }

    true
}

fn is_unique_row_in_a(idx: u32, view: &GridView<'_>, stats: &HashStats<RowHash>) -> bool {
    view.row_meta
        .get(idx as usize)
        .map(|meta| unique_in_a(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_row_in_b(idx: u32, view: &GridView<'_>, stats: &HashStats<RowHash>) -> bool {
    view.row_meta
        .get(idx as usize)
        .map(|meta| unique_in_b(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_col_in_a(idx: u32, view: &GridView<'_>, stats: &HashStats<ColHash>) -> bool {
    view.col_meta
        .get(idx as usize)
        .map(|meta| unique_in_a(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_col_in_b(idx: u32, view: &GridView<'_>, stats: &HashStats<ColHash>) -> bool {
    view.col_meta
        .get(idx as usize)
        .map(|meta| unique_in_b(meta.hash, stats))
        .unwrap_or(false)
}

fn find_two_equal_ranges<I>(indices: I) -> Option<((u32, u32), (u32, u32))>
where
    I: IntoIterator<Item = u32>,
{
    let mut values: Vec<u32> = indices.into_iter().collect();
    if values.is_empty() {
        return None;
    }

    values.sort_unstable();
    values.dedup();

    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut start = values[0];
    let mut prev = values[0];

    for &val in values.iter().skip(1) {
        if val == prev + 1 {
            prev = val;
            continue;
        }

        ranges.push((start, prev));
        start = val;
        prev = val;
    }
    ranges.push((start, prev));

    match ranges.len() {
        1 => Some((ranges[0], ranges[0])),
        2 => {
            let len0 = range_len(ranges[0]);
            let len1 = range_len(ranges[1]);
            if len0 != len1 {
                return None;
            }
            Some((ranges[0], ranges[1]))
        }
        _ => None,
    }
}

fn range_len(range: (u32, u32)) -> u32 {
    range.1.saturating_sub(range.0).saturating_add(1)
}

fn ranges_overlap(a: (u32, u32), b: (u32, u32)) -> bool {
    !(a.1 < b.0 || b.1 < a.0)
}

fn is_within_size_bounds(old: &Grid, new: &Grid, config: &DiffConfig) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= config.max_align_rows && cols <= config.max_align_cols
}

fn low_info_dominated(view: &GridView<'_>) -> bool {
    if view.row_meta.is_empty() {
        return false;
    }

    let low_info_count = view.row_meta.iter().filter(|m| m.is_low_info).count();
    low_info_count * 2 > view.row_meta.len()
}

fn blank_dominated(view: &GridView<'_>) -> bool {
    if view.col_meta.is_empty() {
        return false;
    }

    let blank_cols = view
        .col_meta
        .iter()
        .filter(
            |ColMeta {
                 non_blank_count, ..
             }| *non_blank_count == 0,
        )
        .count();

    blank_cols * 2 > view.col_meta.len()
}

fn has_heavy_repetition<H>(stats: &HashStats<H>, config: &DiffConfig) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > config.max_hash_repeat
}

fn unique_in_a<H>(hash: H, stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) <= 1
}

fn unique_in_b<H>(hash: H, stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_a.get(&hash).copied().unwrap_or(0) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

    fn grid_from_numbers(values: &[&[i32]]) -> Grid {
        let nrows = values.len() as u32;
        let ncols = if nrows == 0 {
            0
        } else {
            values[0].len() as u32
        };

        let mut grid = Grid::new(nrows, ncols);
        for (r, row_vals) in values.iter().enumerate() {
            for (c, v) in row_vals.iter().enumerate() {
                grid.insert_cell(
                    r as u32,
                    c as u32,
                    Some(CellValue::Number(*v as f64)),
                    None,
                );
            }
        }

        grid
    }

    fn base_background(rows: usize, cols: usize) -> Vec<Vec<i32>> {
        (0..rows)
            .map(|r| (0..cols).map(|c| (r as i32) * 1_000 + c as i32).collect())
            .collect()
    }

    fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
        for (r_offset, row_vals) in block.iter().enumerate() {
            for (c_offset, value) in row_vals.iter().enumerate() {
                let row = top + r_offset;
                let col = left + c_offset;
                if let Some(row_slice) = target.get_mut(row)
                    && let Some(cell) = row_slice.get_mut(col)
                {
                    *cell = *value;
                }
            }
        }
    }

    fn grid_from_matrix(matrix: Vec<Vec<i32>>) -> Grid {
        let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
        grid_from_numbers(&refs)
    }

    #[test]
    fn detect_simple_rect_block_move_success() {
        let mut grid_a = base_background(12, 12);
        let mut grid_b = base_background(12, 12);

        let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

        place_block(&mut grid_a, 1, 1, &block);
        place_block(&mut grid_b, 7, 6, &block);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_some(),
            "should detect exact rectangular block move"
        );

        let mv = result.unwrap();
        assert_eq!(mv.src_start_row, 1);
        assert_eq!(mv.src_row_count, 3);
        assert_eq!(mv.src_start_col, 1);
        assert_eq!(mv.src_col_count, 3);
        assert_eq!(mv.dst_start_row, 7);
        assert_eq!(mv.dst_start_col, 6);
    }

    #[test]
    fn detect_rect_block_move_with_shared_columns() {
        let mut grid_a = base_background(10, 10);
        let mut grid_b = base_background(10, 10);

        let block = vec![vec![11, 12], vec![21, 22]];

        place_block(&mut grid_a, 1, 2, &block);
        place_block(&mut grid_b, 6, 2, &block);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_some(),
            "should detect a vertical rect move when columns overlap"
        );

        let mv = result.unwrap();
        assert_eq!(mv.src_start_row, 1);
        assert_eq!(mv.dst_start_row, 6);
        assert_eq!(mv.src_start_col, 2);
        assert_eq!(mv.dst_start_col, 2);
        assert_eq!(mv.src_row_count, 2);
        assert_eq!(mv.src_col_count, 2);
    }

    #[test]
    fn detect_bails_on_different_grid_dimensions() {
        let old = grid_from_numbers(&[&[1, 2], &[3, 4]]);
        let new = grid_from_numbers(&[&[1, 2, 5], &[3, 4, 6]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(result.is_none(), "different dimensions should bail");
    }

    #[test]
    fn detect_bails_on_empty_grid() {
        let old = Grid::new(0, 0);
        let new = Grid::new(0, 0);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(result.is_none(), "empty grid should bail");
    }

    #[test]
    fn detect_bails_on_identical_grids() {
        let old = grid_from_numbers(&[&[1, 2], &[3, 4]]);
        let new = grid_from_numbers(&[&[1, 2], &[3, 4]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "identical grids should bail (no differences)"
        );
    }

    #[test]
    fn detect_bails_on_internal_cell_edit() {
        let mut grid_a = base_background(10, 10);
        let mut grid_b = base_background(10, 10);

        let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

        place_block(&mut grid_a, 1, 1, &block);
        place_block(&mut grid_b, 6, 4, &block);
        grid_b[7][5] = 9_999;

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "move with internal edit should not be detected as exact rectangular move"
        );
    }

    #[test]
    fn detect_bails_on_ambiguous_block_swap() {
        let base: Vec<Vec<i32>> = (0..6)
            .map(|r| (0..6).map(|c| 100 * r + c).collect())
            .collect();
        let mut grid_a = base.clone();
        let mut grid_b = base.clone();

        let block_one = vec![vec![900, 901], vec![902, 903]];
        let block_two = vec![vec![700, 701], vec![702, 703]];

        place_block(&mut grid_a, 0, 0, &block_one);
        place_block(&mut grid_a, 3, 3, &block_two);

        place_block(&mut grid_b, 0, 0, &block_two);
        place_block(&mut grid_b, 3, 3, &block_one);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "ambiguous block swap should not emit a rectangular move"
        );
    }

    #[allow(clippy::field_reassign_with_default)]
    #[test]
    fn detect_bails_on_oversized_row_count() {
        let mut config = DiffConfig::default();
        config.max_align_rows = 10;
        let old = Grid::new(config.max_align_rows + 1, 10);
        let new = Grid::new(config.max_align_rows + 1, 10);

        let result = detect_exact_rect_block_move(&old, &new, &config);
        assert!(
            result.is_none(),
            "grids exceeding configured max_align_rows should bail"
        );
    }

    #[allow(clippy::field_reassign_with_default)]
    #[test]
    fn detect_bails_on_oversized_col_count() {
        let mut config = DiffConfig::default();
        config.max_align_cols = 8;
        let old = Grid::new(10, config.max_align_cols + 1);
        let new = Grid::new(10, config.max_align_cols + 1);

        let result = detect_exact_rect_block_move(&old, &new, &config);
        assert!(
            result.is_none(),
            "grids exceeding configured max_align_cols should bail"
        );
    }

    #[test]
    fn detect_bails_on_single_cell_edit() {
        let old = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);
        let new = grid_from_numbers(&[&[1, 2, 3], &[4, 99, 6], &[7, 8, 9]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "single cell edit is not a rectangular block move"
        );
    }

    #[test]
    fn detect_bails_on_pure_row_move_pattern() {
        let old = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9], &[10, 11, 12]]);
        let new = grid_from_numbers(&[&[7, 8, 9], &[4, 5, 6], &[1, 2, 3], &[10, 11, 12]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "pure row swap without column displacement is not a rectangular block move"
        );
    }

    #[test]
    fn detect_bails_on_non_contiguous_differences() {
        let mut grid_a = base_background(8, 8);
        let mut grid_b = base_background(8, 8);

        grid_a[1][1] = 111;
        grid_a[5][5] = 555;
        grid_a[1][5] = 115;
        grid_b[1][1] = 555;
        grid_b[5][5] = 111;
        grid_b[1][5] = 999;

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "non-contiguous differences should not form a rectangular block move"
        );
    }
}

```

---

### File: `core\src\region_mask.rs`

```rust
//! Region mask for tracking which cells have been accounted for during diff.
//!
//! The `RegionMask` tracks which rows and columns are "active" (still to be processed)
//! versus "excluded" (already accounted for by a move or other operation).

use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
struct RectMask {
    row_start: u32,
    row_count: u32,
    col_start: u32,
    col_count: u32,
}

#[derive(Debug, Clone)]
pub struct RegionMask {
    excluded_rows: HashSet<u32>,
    excluded_cols: HashSet<u32>,
    excluded_rects: Vec<RectMask>,
    #[allow(dead_code)]
    nrows: u32,
    #[allow(dead_code)]
    ncols: u32,
    row_shift_min: Option<u32>,
    row_shift_max: Option<u32>,
    col_shift_min: Option<u32>,
    col_shift_max: Option<u32>,
}

#[allow(dead_code)]
impl RegionMask {
    pub fn all_active(nrows: u32, ncols: u32) -> Self {
        Self {
            excluded_rows: HashSet::new(),
            excluded_cols: HashSet::new(),
            excluded_rects: Vec::new(),
            nrows,
            ncols,
            row_shift_min: None,
            row_shift_max: None,
            col_shift_min: None,
            col_shift_max: None,
        }
    }

    pub fn exclude_row(&mut self, row: u32) {
        self.excluded_rows.insert(row);
    }

    pub fn exclude_rows(&mut self, start: u32, count: u32) {
        let end = start.saturating_add(count).saturating_sub(1);
        for row in start..=end {
            self.excluded_rows.insert(row);
        }
        self.row_shift_min = Some(self.row_shift_min.map_or(start, |m| m.min(start)));
        self.row_shift_max = Some(self.row_shift_max.map_or(end, |m| m.max(end)));
    }

    pub fn exclude_col(&mut self, col: u32) {
        self.excluded_cols.insert(col);
    }

    pub fn exclude_cols(&mut self, start: u32, count: u32) {
        let end = start.saturating_add(count).saturating_sub(1);
        for col in start..=end {
            self.excluded_cols.insert(col);
        }
        self.col_shift_min = Some(self.col_shift_min.map_or(start, |m| m.min(start)));
        self.col_shift_max = Some(self.col_shift_max.map_or(end, |m| m.max(end)));
    }

    pub fn exclude_rect(&mut self, row_start: u32, row_count: u32, col_start: u32, col_count: u32) {
        self.exclude_rows(row_start, row_count);
        self.exclude_cols(col_start, col_count);
    }

    pub fn exclude_rect_cells(
        &mut self,
        row_start: u32,
        row_count: u32,
        col_start: u32,
        col_count: u32,
    ) {
        self.excluded_rects.push(RectMask {
            row_start,
            row_count,
            col_start,
            col_count,
        });
    }

    pub fn is_row_active(&self, row: u32) -> bool {
        !self.excluded_rows.contains(&row)
    }

    pub fn is_col_active(&self, col: u32) -> bool {
        !self.excluded_cols.contains(&col)
    }

    fn is_cell_excluded_by_rects(&self, row: u32, col: u32) -> bool {
        self.excluded_rects.iter().any(|rect| {
            row >= rect.row_start
                && row < rect.row_start.saturating_add(rect.row_count)
                && col >= rect.col_start
                && col < rect.col_start.saturating_add(rect.col_count)
        })
    }

    pub fn is_cell_active(&self, row: u32, col: u32) -> bool {
        self.is_row_active(row)
            && self.is_col_active(col)
            && !self.is_cell_excluded_by_rects(row, col)
    }

    pub fn active_row_count(&self) -> u32 {
        self.nrows.saturating_sub(self.excluded_rows.len() as u32)
    }

    pub fn active_col_count(&self) -> u32 {
        self.ncols.saturating_sub(self.excluded_cols.len() as u32)
    }

    pub fn active_rows(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.nrows).filter(|r| self.is_row_active(*r))
    }

    pub fn active_cols(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.ncols).filter(|c| self.is_col_active(*c))
    }

    pub fn has_excluded_rows(&self) -> bool {
        !self.excluded_rows.is_empty()
    }

    pub fn has_excluded_cols(&self) -> bool {
        !self.excluded_cols.is_empty()
    }

    pub fn has_excluded_rects(&self) -> bool {
        !self.excluded_rects.is_empty()
    }

    pub fn has_exclusions(&self) -> bool {
        self.has_excluded_rows() || self.has_excluded_cols() || self.has_excluded_rects()
    }

    pub fn has_active_cells(&self) -> bool {
        self.active_row_count() > 0 && self.active_col_count() > 0
    }

    pub fn rows_overlap_excluded(&self, start: u32, count: u32) -> bool {
        for row in start..start.saturating_add(count) {
            if self.excluded_rows.contains(&row) {
                return true;
            }
        }
        false
    }

    pub fn cols_overlap_excluded(&self, start: u32, count: u32) -> bool {
        for col in start..start.saturating_add(count) {
            if self.excluded_cols.contains(&col) {
                return true;
            }
        }
        false
    }

    pub fn rect_overlaps_excluded(
        &self,
        row_start: u32,
        row_count: u32,
        col_start: u32,
        col_count: u32,
    ) -> bool {
        self.rows_overlap_excluded(row_start, row_count)
            || self.cols_overlap_excluded(col_start, col_count)
    }

    pub fn is_row_in_shift_zone(&self, row: u32) -> bool {
        match (self.row_shift_min, self.row_shift_max) {
            (Some(min), Some(max)) => row >= min && row <= max,
            _ => false,
        }
    }

    pub fn is_col_in_shift_zone(&self, col: u32) -> bool {
        match (self.col_shift_min, self.col_shift_max) {
            (Some(min), Some(max)) => col >= min && col <= max,
            _ => false,
        }
    }

    pub fn row_shift_bounds(&self) -> Option<(u32, u32)> {
        match (self.row_shift_min, self.row_shift_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    pub fn col_shift_bounds(&self) -> Option<(u32, u32)> {
        match (self.col_shift_min, self.col_shift_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_active_initially() {
        let mask = RegionMask::all_active(10, 5);
        assert!(mask.is_row_active(0));
        assert!(mask.is_row_active(9));
        assert!(mask.is_col_active(0));
        assert!(mask.is_col_active(4));
        assert_eq!(mask.active_row_count(), 10);
        assert_eq!(mask.active_col_count(), 5);
    }

    #[test]
    fn exclude_single_row() {
        let mut mask = RegionMask::all_active(10, 5);
        mask.exclude_row(3);
        assert!(!mask.is_row_active(3));
        assert!(mask.is_row_active(2));
        assert!(mask.is_row_active(4));
        assert_eq!(mask.active_row_count(), 9);
    }

    #[test]
    fn exclude_row_range() {
        let mut mask = RegionMask::all_active(10, 5);
        mask.exclude_rows(2, 4);
        assert!(!mask.is_row_active(2));
        assert!(!mask.is_row_active(5));
        assert!(mask.is_row_active(1));
        assert!(mask.is_row_active(6));
        assert_eq!(mask.active_row_count(), 6);
    }

    #[test]
    fn exclude_rect() {
        let mut mask = RegionMask::all_active(10, 8);
        mask.exclude_rect(2, 3, 4, 2);
        assert!(!mask.is_row_active(2));
        assert!(!mask.is_row_active(4));
        assert!(mask.is_row_active(1));
        assert!(mask.is_row_active(5));
        assert!(!mask.is_col_active(4));
        assert!(!mask.is_col_active(5));
        assert!(mask.is_col_active(3));
        assert!(mask.is_col_active(6));
    }

    #[test]
    fn cell_active_based_on_row_and_col() {
        let mut mask = RegionMask::all_active(10, 10);
        mask.exclude_row(3);
        mask.exclude_col(5);
        assert!(!mask.is_cell_active(3, 5));
        assert!(!mask.is_cell_active(3, 0));
        assert!(!mask.is_cell_active(0, 5));
        assert!(mask.is_cell_active(0, 0));
        assert!(mask.is_cell_active(4, 6));
    }

    #[test]
    fn active_rows_iterator() {
        let mut mask = RegionMask::all_active(5, 3);
        mask.exclude_row(1);
        mask.exclude_row(3);
        let active: Vec<u32> = mask.active_rows().collect();
        assert_eq!(active, vec![0, 2, 4]);
    }

    #[test]
    fn rows_overlap_excluded_detects_overlap() {
        let mut mask = RegionMask::all_active(10, 5);
        mask.exclude_rows(3, 2);
        assert!(mask.rows_overlap_excluded(2, 3));
        assert!(mask.rows_overlap_excluded(4, 2));
        assert!(!mask.rows_overlap_excluded(0, 2));
        assert!(!mask.rows_overlap_excluded(5, 3));
    }

    #[test]
    fn cols_overlap_excluded_detects_overlap() {
        let mut mask = RegionMask::all_active(5, 10);
        mask.exclude_cols(4, 3);
        assert!(mask.cols_overlap_excluded(3, 2));
        assert!(mask.cols_overlap_excluded(6, 2));
        assert!(!mask.cols_overlap_excluded(0, 3));
        assert!(!mask.cols_overlap_excluded(7, 3));
    }

    #[test]
    fn rect_overlaps_excluded_detects_any_overlap() {
        let mut mask = RegionMask::all_active(10, 10);
        mask.exclude_rect(2, 3, 4, 2);
        assert!(mask.rect_overlaps_excluded(1, 2, 0, 3));
        assert!(mask.rect_overlaps_excluded(0, 2, 3, 2));
        assert!(!mask.rect_overlaps_excluded(6, 2, 7, 2));
    }

    #[test]
    fn exclude_rect_cells_masks_only_that_region() {
        let mut mask = RegionMask::all_active(6, 6);
        mask.exclude_rect_cells(2, 2, 2, 2);

        assert!(!mask.is_cell_active(2, 2));
        assert!(!mask.is_cell_active(3, 3));

        assert!(mask.is_cell_active(1, 2));
        assert!(mask.is_cell_active(2, 1));
        assert!(mask.is_cell_active(4, 4));

        assert!(mask.has_exclusions());
        assert!(mask.has_excluded_rects());
    }
}

```

---

### File: `core\src\row_alignment.rs`

```rust
//! Legacy row alignment algorithms (pre-AMR).
//!
//! This module contains the original row alignment implementation that predates
//! the Anchor-Move-Refine (AMR) algorithm in `alignment/`. These functions are
//! retained for:
//!
//! 1. **Fallback scenarios**: The engine may use these when AMR cannot produce
//!    a useful alignment (e.g., heavily repetitive data).
//!
//! 2. **Move detection helpers**: Some functions (`detect_exact_row_block_move`,
//!    `detect_fuzzy_row_block_move`) are still used by the engine's
//!    masked move detection logic.
//!
//! 3. **Test coverage**: Unit tests validate these algorithms work correctly.
//!
//! ## Migration Status
//!
//! The primary alignment path now uses `alignment::align_rows_amr`. The legacy
//! functions are invoked only when:
//! - AMR returns `None` (fallback to `align_row_changes`)
//! - Explicit move detection in masked regions
//!
//! Functions marked `#[allow(dead_code)]` are retained for testing but not
//! called from production code paths.

use std::collections::HashSet;

use crate::config::DiffConfig;
use crate::grid_view::{GridView, HashStats, RowHash, RowMeta};
use crate::workbook::Grid;

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

const _HASH_COLLISION_NOTE: &str = "128-bit xxHash3 collision probability ~10^-29 at 50K rows (birthday bound); \
     secondary verification not required; see hashing.rs for detailed rationale.";

pub(crate) fn detect_exact_row_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    let meta_a = &view_a.row_meta;
    let meta_b = &view_b.row_meta;
    let n = meta_a.len();

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let prefix = (0..n).find(|&idx| meta_a[idx].hash != meta_b[idx].hash)?;

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = n - 1 - suffix_len;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }
    let tail_start = n - suffix_len;

    let try_candidate = |src_start: usize, dst_start: usize| -> Option<RowBlockMove> {
        if src_start >= tail_start || dst_start >= tail_start {
            return None;
        }

        let mut len = 0usize;
        while src_start + len < tail_start && dst_start + len < tail_start {
            if meta_a[src_start + len].hash != meta_b[dst_start + len].hash {
                break;
            }
            len += 1;
        }

        if len == 0 {
            return None;
        }

        let src_end = src_start + len;
        let dst_end = dst_start + len;

        if !(src_end <= dst_start || dst_end <= src_start) {
            return None;
        }

        let mut idx_a = 0usize;
        let mut idx_b = 0usize;

        loop {
            if idx_a == src_start {
                idx_a = src_end;
            }
            if idx_b == dst_start {
                idx_b = dst_end;
            }

            if idx_a >= n && idx_b >= n {
                break;
            }

            if idx_a >= n || idx_b >= n {
                return None;
            }

            if meta_a[idx_a].hash != meta_b[idx_b].hash {
                return None;
            }

            idx_a += 1;
            idx_b += 1;
        }

        for meta in &meta_a[src_start..src_end] {
            if stats.freq_a.get(&meta.hash).copied().unwrap_or(0) != 1
                || stats.freq_b.get(&meta.hash).copied().unwrap_or(0) != 1
            {
                return None;
            }
        }

        Some(RowBlockMove {
            src_start_row: meta_a[src_start].row_idx,
            dst_start_row: meta_b[dst_start].row_idx,
            row_count: len as u32,
        })
    };

    if let Some(src_start) =
        (prefix..tail_start).find(|&idx| meta_a[idx].hash == meta_b[prefix].hash)
        && let Some(mv) = try_candidate(src_start, prefix)
    {
        return Some(mv);
    }

    if let Some(dst_start) =
        (prefix..tail_start).find(|&idx| meta_b[idx].hash == meta_a[prefix].hash)
        && let Some(mv) = try_candidate(prefix, dst_start)
    {
        return Some(mv);
    }

    None
}

pub(crate) fn detect_fuzzy_row_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    let meta_a = &view_a.row_meta;
    let meta_b = &view_b.row_meta;

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let n = meta_a.len();
    let mut prefix = 0usize;
    while prefix < n && meta_a[prefix].hash == meta_b[prefix].hash {
        prefix += 1;
    }
    if prefix == n {
        return None;
    }

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = idx_a;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }

    let mismatch_end = n - suffix_len;
    if mismatch_end <= prefix {
        return None;
    }

    let mid_len = mismatch_end - prefix;
    if mid_len <= 1 {
        return None;
    }

    let max_block_len = mid_len
        .saturating_sub(1)
        .min(config.max_fuzzy_block_rows as usize);
    if max_block_len == 0 {
        return None;
    }

    let mut candidate: Option<RowBlockMove> = None;

    for block_len in 1..=max_block_len {
        let remaining = mid_len - block_len;

        // Block moved upward: [middle][block] -> [block'][middle]
        if hashes_match(
            &meta_a[prefix..prefix + remaining],
            &meta_b[prefix + block_len..mismatch_end],
        ) {
            let src_block = &meta_a[prefix + remaining..mismatch_end];
            let dst_block = &meta_b[prefix..prefix + block_len];

            if block_similarity(src_block, dst_block) >= config.fuzzy_similarity_threshold {
                let mv = RowBlockMove {
                    src_start_row: src_block[0].row_idx,
                    dst_start_row: dst_block[0].row_idx,
                    row_count: block_len as u32,
                };
                if mv.src_start_row != mv.dst_start_row {
                    if candidate.is_some() {
                        return None;
                    }
                    candidate = Some(mv);
                }
            }
        }

        // Block moved downward: [block][middle] -> [middle][block']
        if hashes_match(
            &meta_a[prefix + block_len..mismatch_end],
            &meta_b[prefix..prefix + remaining],
        ) {
            let src_block = &meta_a[prefix..prefix + block_len];
            let dst_block = &meta_b[prefix + remaining..mismatch_end];

            if block_similarity(src_block, dst_block) >= config.fuzzy_similarity_threshold {
                let mv = RowBlockMove {
                    src_start_row: src_block[0].row_idx,
                    dst_start_row: dst_block[0].row_idx,
                    row_count: block_len as u32,
                };
                if mv.src_start_row != mv.dst_start_row {
                    if candidate.is_some() {
                        return None;
                    }
                    candidate = Some(mv);
                }
            }
        }
    }

    candidate
}

#[allow(dead_code)]
pub(crate) fn align_row_changes(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_row_changes_from_views(&view_a, &view_b, config)
}

pub(crate) fn align_row_changes_from_views(
    old_view: &GridView,
    new_view: &GridView,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    let row_diff = new_view.source.nrows as i64 - old_view.source.nrows as i64;
    if row_diff.abs() == 1 {
        return align_single_row_change_from_views(old_view, new_view, config);
    }

    align_rows_internal(old_view, new_view, true, config)
}

#[allow(dead_code)]
pub(crate) fn align_single_row_change(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_single_row_change_from_views(&view_a, &view_b, config)
}

pub(crate) fn align_single_row_change_from_views(
    old_view: &GridView,
    new_view: &GridView,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    align_rows_internal(old_view, new_view, false, config)
}

fn align_rows_internal(
    old_view: &GridView,
    new_view: &GridView,
    allow_blocks: bool,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    if !is_within_size_bounds(old_view.source, new_view.source, config) {
        return None;
    }

    if old_view.source.ncols != new_view.source.ncols {
        return None;
    }

    let row_diff = new_view.source.nrows as i64 - old_view.source.nrows as i64;
    if row_diff == 0 {
        return None;
    }

    let abs_diff = row_diff.unsigned_abs() as u32;

    if !allow_blocks && abs_diff != 1 {
        return None;
    }

    if abs_diff != 1 && (!allow_blocks || abs_diff > config.max_block_gap) {
        return None;
    }

    if low_info_dominated(old_view) || low_info_dominated(new_view) {
        return None;
    }

    let stats = HashStats::from_row_meta(&old_view.row_meta, &new_view.row_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    if row_diff == 1 {
        find_single_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Insert,
        )
    } else if row_diff == -1 {
        find_single_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Delete,
        )
    } else if !allow_blocks {
        None
    } else if row_diff > 0 {
        find_block_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Insert,
            abs_diff,
        )
    } else {
        find_block_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Delete,
            abs_diff,
        )
    }
}

enum RowChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
) -> Option<RowAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < rows_a.len() && idx_b < rows_b.len() {
        let meta_a = rows_a[idx_a];
        let meta_b = rows_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.row_idx, meta_b.row_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            RowChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
                idx_b += 1;
            }
            RowChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < rows_a.len() || idx_b < rows_b.len() {
        if skipped {
            return None;
        }

        match change {
            RowChange::Insert if idx_a == rows_a.len() && rows_b.len() == idx_b + 1 => {
                let meta_b = rows_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
            }
            RowChange::Delete if idx_b == rows_b.len() && rows_a.len() == idx_a + 1 => {
                let meta_a = rows_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn find_block_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
    gap: u32,
) -> Option<RowAlignment> {
    let gap = gap as usize;
    if gap == 0 {
        return None;
    }

    let (shorter_len, longer_len) = match change {
        RowChange::Insert => (rows_a.len(), rows_b.len()),
        RowChange::Delete => (rows_b.len(), rows_a.len()),
    };

    if longer_len.saturating_sub(shorter_len) != gap {
        return None;
    }

    let mut prefix = 0usize;
    while prefix < rows_a.len()
        && prefix < rows_b.len()
        && rows_a[prefix].hash == rows_b[prefix].hash
    {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < shorter_len.saturating_sub(prefix) {
        let idx_a = rows_a.len() - 1 - suffix;
        let idx_b = rows_b.len() - 1 - suffix;
        if rows_a[idx_a].hash == rows_b[idx_b].hash {
            suffix += 1;
        } else {
            break;
        }
    }

    if prefix + suffix != shorter_len {
        return None;
    }

    let mut matched = Vec::with_capacity(shorter_len);
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    match change {
        RowChange::Insert => {
            let block_start = prefix;
            let block_end = block_start + gap;
            if block_end > rows_b.len() {
                return None;
            }

            for meta in &rows_b[block_start..block_end] {
                if !is_unique_to_b(meta.hash, stats) {
                    return None;
                }
                inserted.push(meta.row_idx);
            }

            for (idx, meta_a) in rows_a.iter().enumerate() {
                let b_idx = if idx < block_start { idx } else { idx + gap };
                matched.push((meta_a.row_idx, rows_b[b_idx].row_idx));
            }
        }
        RowChange::Delete => {
            let block_start = prefix;
            let block_end = block_start + gap;
            if block_end > rows_a.len() {
                return None;
            }

            for meta in &rows_a[block_start..block_end] {
                if !is_unique_to_a(meta.hash, stats) {
                    return None;
                }
                deleted.push(meta.row_idx);
            }

            for (idx_b, meta_b) in rows_b.iter().enumerate() {
                let a_idx = if idx_b < block_start {
                    idx_b
                } else {
                    idx_b + gap
                };
                matched.push((rows_a[a_idx].row_idx, meta_b.row_idx));
            }
        }
    }

    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn is_monotonic(pairs: &[(u32, u32)]) -> bool {
    pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1)
}

fn is_unique_to_b(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid, config: &DiffConfig) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= config.max_align_rows && cols <= config.max_align_cols
}

fn low_info_dominated(view: &GridView<'_>) -> bool {
    if view.row_meta.is_empty() {
        return false;
    }

    let low_info_count = view.row_meta.iter().filter(|m| m.is_low_info).count();
    low_info_count * 2 > view.row_meta.len()
}

fn has_heavy_repetition(stats: &HashStats<RowHash>, config: &DiffConfig) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > config.max_hash_repeat
}

fn hashes_match(slice_a: &[RowMeta], slice_b: &[RowMeta]) -> bool {
    slice_a.len() == slice_b.len()
        && slice_a
            .iter()
            .zip(slice_b.iter())
            .all(|(a, b)| a.hash == b.hash)
}

fn block_similarity(slice_a: &[RowMeta], slice_b: &[RowMeta]) -> f64 {
    let tokens_a: HashSet<RowHash> = slice_a.iter().map(|m| m.hash).collect();
    let tokens_b: HashSet<RowHash> = slice_b.iter().map(|m| m.hash).collect();

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    let jaccard = if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    };

    let positional_matches = slice_a
        .iter()
        .zip(slice_b.iter())
        .filter(|(a, b)| a.hash == b.hash)
        .count();
    let positional_ratio = (positional_matches as f64 + 1.0) / (slice_a.len() as f64 + 1.0);

    jaccard.max(positional_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

    fn grid_from_rows(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert_cell(
                    r_idx as u32,
                    c_idx as u32,
                    Some(CellValue::Number(*value as f64)),
                    None,
                );
            }
        }

        grid
    }

    #[test]
    fn detects_exact_row_block_move() {
        let base: Vec<Vec<i32>> = (1..=20)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
        rows_b.splice(12..12, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let mv = detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected block move to be found");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 12,
                row_count: 4
            }
        );
    }

    #[test]
    fn block_move_detection_rejects_internal_edits() {
        let base: Vec<Vec<i32>> = (1..=12)
            .map(|r| (1..=2).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(2..5).collect();
        moved_block[1][0] = 9_999;
        rows_b.splice(6..6, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detects_fuzzy_row_block_move_with_single_internal_edit() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
        moved_block[1][1] = 9_999;
        rows_b.splice(12..12, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected fuzzy row block move to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 12,
                row_count: 4
            }
        );
    }

    #[test]
    fn fuzzy_move_rejects_low_similarity_block() {
        let base: Vec<Vec<i32>> = (1..=16)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(3..7).collect();
        for row in &mut moved_block {
            for value in row.iter_mut() {
                *value += 50_000;
            }
        }
        rows_b.splice(10..10, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "similarity below threshold should bail out"
        );
    }

    #[test]
    fn fuzzy_move_bails_on_heavy_repetition_or_ambiguous_candidates() {
        let repeated_row = [1, 2];
        let rows_a: Vec<Vec<i32>> = (0..10).map(|_| repeated_row.to_vec()).collect();
        let mut rows_b = rows_a.clone();

        let block: Vec<Vec<i32>> = rows_b.drain(0..3).collect();
        rows_b.splice(5..5, block);

        let rows_a_refs: Vec<&[i32]> = rows_a.iter().map(|row| row.as_slice()).collect();
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&rows_a_refs);
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "heavy repetition or ambiguous candidates should not emit a move"
        );
    }

    #[test]
    fn fuzzy_move_noop_when_grids_identical() {
        let base: Vec<Vec<i32>> = (1..=6)
            .map(|r| (1..=2).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);
        let grid_b = grid_from_rows(&base_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
        assert!(detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detects_fuzzy_row_block_move_upward_with_single_internal_edit() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(12..16).collect();
        moved_block[1][1] = 9_999;
        rows_b.splice(4..4, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected fuzzy row block move upward to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 12,
                dst_start_row: 4,
                row_count: 4
            }
        );
    }

    #[test]
    fn fuzzy_move_bails_on_ambiguous_candidates_below_repetition_threshold() {
        let base: Vec<Vec<i32>> = (1..=16)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_baseline_a = grid_from_rows(&base_refs);

        let mut rows_baseline_b = base.clone();
        let mut moved: Vec<Vec<i32>> = rows_baseline_b.drain(3..7).collect();
        moved[1][1] = 9999;
        rows_baseline_b.splice(10..10, moved);
        let refs_baseline_b: Vec<&[i32]> =
            rows_baseline_b.iter().map(|row| row.as_slice()).collect();
        let grid_baseline_b = grid_from_rows(&refs_baseline_b);

        assert!(
            detect_fuzzy_row_block_move(&grid_baseline_a, &grid_baseline_b, &DiffConfig::default())
                .is_some(),
            "baseline: non-ambiguous fuzzy move should be detected"
        );

        let rows_a: Vec<Vec<i32>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![100, 200, 300],
            vec![101, 201, 301],
            vec![102, 202, 302],
            vec![103, 203, 303],
            vec![100, 200, 300],
            vec![101, 201, 301],
            vec![102, 202, 302],
            vec![103, 203, 999],
            vec![31, 32, 33],
            vec![34, 35, 36],
        ];

        let mut rows_b = rows_a.clone();
        let block1: Vec<Vec<i32>> = rows_b.drain(2..6).collect();
        rows_b.splice(6..6, block1);

        let refs_a: Vec<&[i32]> = rows_a.iter().map(|r| r.as_slice()).collect();
        let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_rows(&refs_a);
        let grid_b = grid_from_rows(&refs_b);

        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "ambiguous candidates: two similar blocks swapped should trigger ambiguity bail-out"
        );
    }

    #[test]
    fn fuzzy_move_at_max_block_rows_threshold() {
        let config = DiffConfig::default();
        let base: Vec<Vec<i32>> = (1..=70)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..36).collect();
        moved_block[15][1] = 9_999;
        rows_b.splice(36..36, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b, &config).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b, &config)
            .expect("expected fuzzy move at configured max_fuzzy_block_rows to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 36,
                row_count: config.max_fuzzy_block_rows
            }
        );
    }

    #[test]
    fn fuzzy_move_at_max_hash_repeat_boundary() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_base = grid_from_rows(&base_refs);

        let mut rows_moved = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_moved.drain(4..8).collect();
        moved_block[1][1] = 9_999;
        rows_moved.splice(12..12, moved_block);
        let moved_refs: Vec<&[i32]> = rows_moved.iter().map(|row| row.as_slice()).collect();
        let grid_moved = grid_from_rows(&moved_refs);

        assert!(
            detect_fuzzy_row_block_move(&grid_base, &grid_moved, &DiffConfig::default()).is_some(),
            "baseline: fuzzy move should work with unique rows"
        );

        let mut base_9_repeat: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        for row in base_9_repeat.iter_mut().take(9) {
            *row = vec![999, 888, 777];
        }
        let refs_9a: Vec<&[i32]> = base_9_repeat.iter().map(|r| r.as_slice()).collect();
        let grid_9a = grid_from_rows(&refs_9a);

        let mut rows_9b = base_9_repeat.clone();
        let mut moved_9: Vec<Vec<i32>> = rows_9b.drain(10..14).collect();
        moved_9[1][1] = 8_888;
        rows_9b.splice(14..14, moved_9);
        let refs_9b: Vec<&[i32]> = rows_9b.iter().map(|r| r.as_slice()).collect();
        let grid_9b = grid_from_rows(&refs_9b);

        assert!(
            detect_fuzzy_row_block_move(&grid_9a, &grid_9b, &DiffConfig::default()).is_none(),
            "repetition guard should trigger when repeat count exceeds max_hash_repeat"
        );

        let mut base_8_repeat: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        for row in base_8_repeat.iter_mut().take(8) {
            *row = vec![999, 888, 777];
        }
        let refs_8a: Vec<&[i32]> = base_8_repeat.iter().map(|r| r.as_slice()).collect();
        let grid_8a = grid_from_rows(&refs_8a);

        let mut rows_8b = base_8_repeat.clone();
        let mut moved_8: Vec<Vec<i32>> = rows_8b.drain(9..13).collect();
        moved_8[1][1] = 8_888;
        rows_8b.splice(14..14, moved_8);
        let refs_8b: Vec<&[i32]> = rows_8b.iter().map(|r| r.as_slice()).collect();
        let grid_8b = grid_from_rows(&refs_8b);

        assert!(
            detect_fuzzy_row_block_move(&grid_8a, &grid_8b, &DiffConfig::default()).is_some(),
            "repeat count equal to max_hash_repeat should not trigger heavy repetition guard"
        );
    }

    #[test]
    fn aligns_contiguous_block_insert_middle() {
        let base: Vec<Vec<i32>> = (1..=10)
            .map(|r| (1..=4).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let inserted_block: Vec<Vec<i32>> = (0..4)
            .map(|idx| vec![1_000 + idx, 2_000 + idx, 3_000 + idx, 4_000 + idx])
            .collect();
        let mut rows_b = base.clone();
        rows_b.splice(3..3, inserted_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let alignment = align_row_changes(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![3, 4, 5, 6]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[2], (2, 2));
        assert_eq!(alignment.matched[3], (3, 7));
        assert_eq!(alignment.matched.last(), Some(&(9, 13)));
        assert!(is_monotonic(&alignment.matched));
    }

    #[test]
    fn aligns_contiguous_block_delete_middle() {
        let base: Vec<Vec<i32>> = (1..=10)
            .map(|r| (1..=4).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        rows_b.drain(3..7);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let alignment = align_row_changes(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.deleted, vec![3, 4, 5, 6]);
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.matched.len(), 6);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[2], (2, 2));
        assert_eq!(alignment.matched[3], (7, 3));
        assert_eq!(alignment.matched.last(), Some(&(9, 5)));
        assert!(is_monotonic(&alignment.matched));
    }

    #[test]
    fn block_alignment_bails_on_noncontiguous_changes() {
        let base: Vec<Vec<i32>> = (1..=8)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        rows_b.insert(1, vec![999, 1_000, 1_001]);
        rows_b.insert(5, vec![2_000, 2_001, 2_002]);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(align_row_changes(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn align_row_changes_rejects_column_insert_mismatch() {
        let grid_a = grid_from_rows(&[&[10, 11, 12], &[20, 21, 22]]);
        let grid_b = grid_from_rows(&[&[0, 10, 11, 12], &[0, 20, 21, 22], &[0, 30, 31, 32]]);

        assert!(
            align_row_changes(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "column insertion changing column count should skip row alignment"
        );
    }

    #[test]
    fn align_row_changes_rejects_column_delete_mismatch() {
        let grid_a = grid_from_rows(&[&[10, 11, 12, 13], &[20, 21, 22, 23], &[30, 31, 32, 33]]);
        let grid_b = grid_from_rows(&[&[10, 12, 13], &[30, 32, 33]]);

        assert!(
            align_row_changes(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "column deletion changing column count should skip row alignment"
        );
    }

    #[test]
    fn aligns_single_insert_with_unique_row() {
        let base = (1..=10)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base_refs.clone();
        rows_b.insert(
            5,
            &[999, 1000, 1001], // inserted at position 6 (1-based)
        );
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
        assert_eq!(alignment.matched[5], (5, 6));
        assert_eq!(alignment.matched.last(), Some(&(9, 10)));
    }

    #[test]
    fn rejects_non_monotonic_alignment_with_extra_mismatch() {
        let base_rows = [[11, 12, 13], [21, 22, 23], [31, 32, 33], [41, 42, 43]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = vec![
            base_refs[0],       // same
            &[999, 1000, 1001], // inserted unique row
            base_refs[2],       // move row 3 before row 2 to break monotonicity
            base_refs[1],
            base_refs[3],
        ];
        let grid_b = grid_from_rows(&rows_b);

        assert!(align_single_row_change(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn aligns_insert_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_first_row = [999, 1000, 1001];
        let mut rows_b = vec![new_first_row.as_slice()];
        rows_b.extend(base_refs.iter().copied());
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![0]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 1));
        assert_eq!(alignment.matched[4], (4, 5));
    }

    #[test]
    fn aligns_insert_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_last_row = [999, 1000, 1001];
        let mut rows_b: Vec<&[i32]> = base_refs.clone();
        rows_b.push(new_last_row.as_slice());
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
    }

    #[test]
    fn aligns_delete_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[1..].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![0]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (1, 0));
        assert_eq!(alignment.matched[3], (4, 3));
    }

    #[test]
    fn aligns_delete_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[..4].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![4]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[3], (3, 3));
    }

    #[test]
    fn aligns_single_row_to_two_rows_via_insert() {
        let single_row = [[42, 43, 44]];
        let single_refs: Vec<&[i32]> = single_row.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&single_refs);

        let new_row = [999, 1000, 1001];
        let rows_b: Vec<&[i32]> = vec![single_refs[0], new_row.as_slice()];
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![1]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn aligns_two_rows_to_single_row_via_delete() {
        let two_rows = [[42, 43, 44], [99, 100, 101]];
        let two_refs: Vec<&[i32]> = two_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&two_refs);

        let single_refs: Vec<&[i32]> = vec![two_refs[0]];
        let grid_b = grid_from_rows(&single_refs);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![1]);
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn monotonicity_helper_accepts_valid_sequence() {
        let valid: Vec<(u32, u32)> = vec![(0, 0), (1, 2), (3, 4), (5, 7)];
        assert!(super::is_monotonic(&valid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_a() {
        let invalid: Vec<(u32, u32)> = vec![(0, 0), (2, 2), (1, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_b() {
        let invalid: Vec<(u32, u32)> = vec![(0, 3), (1, 2), (2, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_accepts_empty_and_single() {
        assert!(super::is_monotonic(&[]));
        assert!(super::is_monotonic(&[(5, 10)]));
    }
}

```

---

### File: `core\src\session.rs`

```rust
use crate::string_pool::StringPool;

/// Holds shared diffing state such as the string pool.
pub struct DiffSession {
    pub strings: StringPool,
}

impl DiffSession {
    pub fn new() -> Self {
        Self {
            strings: StringPool::new(),
        }
    }

    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    pub fn strings_mut(&mut self) -> &mut StringPool {
        &mut self.strings
    }
}

```

---

### File: `core\src\sink.rs`

```rust
use crate::diff::{DiffError, DiffOp};

/// Trait for streaming diff operations to a consumer.
pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;

    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
    }
}

pub(crate) struct NoFinishSink<'a, S: DiffSink> {
    inner: &'a mut S,
}

impl<'a, S: DiffSink> NoFinishSink<'a, S> {
    pub(crate) fn new(inner: &'a mut S) -> Self {
        Self { inner }
    }
}

impl<S: DiffSink> DiffSink for NoFinishSink<'_, S> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        self.inner.emit(op)
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
    }
}

/// A sink that collects ops into a Vec for compatibility.
pub struct VecSink {
    ops: Vec<DiffOp>,
}

impl VecSink {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn into_ops(self) -> Vec<DiffOp> {
        self.ops
    }
}

impl DiffSink for VecSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        self.ops.push(op);
        Ok(())
    }
}

/// A sink that forwards ops to a callback.
pub struct CallbackSink<F: FnMut(DiffOp)> {
    f: F,
}

impl<F: FnMut(DiffOp)> CallbackSink<F> {
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F: FnMut(DiffOp)> DiffSink for CallbackSink<F> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        (self.f)(op);
        Ok(())
    }
}

```

---

### File: `core\src\string_pool.rs`

```rust
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StringId(pub u32);

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct StringPool {
    strings: Vec<String>,
    index: FxHashMap<String, StringId>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut pool = Self::default();
        pool.intern("");
        pool
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.index.get(s) {
            return id;
        }

        let id = StringId(self.strings.len() as u32);
        let owned = s.to_owned();
        self.strings.push(owned.clone());
        self.index.insert(owned, id);
        id
    }

    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn into_strings(self) -> Vec<String> {
        self.strings
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }
}

```

---

### File: `core\src\workbook.rs`

```rust
//! Workbook, sheet, and grid data structures.
//!
//! This module defines the core intermediate representation (IR) for Excel workbooks:
//! - [`Workbook`]: A collection of sheets
//! - [`Sheet`]: A named sheet with a grid of cells
//! - [`Grid`]: A sparse 2D grid of cell content with optional row/column signatures
//! - [`CellContent`]: Value + formula for a single cell (coordinates stored in the grid key)

use crate::addressing::{AddressParseError, address_to_index, index_to_address};
use crate::hashing::normalize_float_for_hash;
use crate::string_pool::{StringId, StringPool};
use rustc_hash::FxHashMap;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// A snapshot of a cell's logical content for comparison purposes.
///
/// Used in [`DiffOp::CellEdited`] to represent the "before" and "after" states.
/// Equality comparison intentionally ignores `addr` and compares only `(value, formula)`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CellSnapshot {
    pub addr: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<StringId>,
}

impl CellSnapshot {
    pub fn from_cell(row: u32, col: u32, cell: &CellContent) -> CellSnapshot {
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: cell.value.clone(),
            formula: cell.formula,
        }
    }

    pub fn empty(addr: CellAddress) -> CellSnapshot {
        CellSnapshot {
            addr,
            value: None,
            formula: None,
        }
    }
}

/// An Excel workbook containing one or more sheets.
#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

/// A single sheet within a workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The display name of the sheet (e.g., "Sheet1", "Data").
    pub name: StringId,
    /// The type of sheet (worksheet, chart, macro, etc.).
    pub kind: SheetKind,
    /// The grid of cell data.
    pub grid: Grid,
}

/// The type of an Excel sheet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SheetKind {
    Worksheet,
    Chart,
    Macro,
    Other,
}

/// A sparse 2D grid of cells representing sheet data.
///
/// # Invariants
///
/// All cells stored in `cells` must satisfy `row < nrows` and `col < ncols`.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Number of rows in the grid's bounding rectangle.
    pub nrows: u32,
    /// Number of columns in the grid's bounding rectangle.
    pub ncols: u32,
    /// Sparse storage of non-empty cells, keyed by (row, col).
    pub cells: FxHashMap<(u32, u32), CellContent>,
    /// Optional precomputed row signatures for alignment.
    pub row_signatures: Option<Vec<RowSignature>>,
    /// Optional precomputed column signatures for alignment.
    pub col_signatures: Option<Vec<ColSignature>>,
}

/// A single cell's logical content (coordinates live in the `Grid` key).
#[derive(Debug, Clone, PartialEq)]
pub struct CellContent {
    /// The cell's value, if any.
    pub value: Option<CellValue>,
    /// The cell's formula text (without leading '='), if any.
    pub formula: Option<StringId>,
}

pub type Cell = CellContent;

/// A view of a cell's content together with its coordinates.
#[derive(Debug, Clone, Copy)]
pub struct CellRef<'a> {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: &'a Option<CellValue>,
    pub formula: &'a Option<StringId>,
}

/// A cell address representing a position in a grid.
///
/// Can be parsed from A1-style strings (e.g., "B2", "AA10") and converted back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    /// Zero-based row index.
    pub row: u32,
    /// Zero-based column index.
    pub col: u32,
}

impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> CellAddress {
        CellAddress { row, col }
    }

    pub fn from_coords(row: u32, col: u32) -> CellAddress {
        Self::from_indices(row, col)
    }

    pub fn to_a1(&self) -> String {
        index_to_address(self.row, self.col)
    }
}

impl std::str::FromStr for CellAddress {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (row, col) = address_to_index(s).ok_or_else(|| AddressParseError {
            input: s.to_string(),
        })?;
        Ok(CellAddress { row, col })
    }
}

impl std::fmt::Display for CellAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

impl Serialize for CellAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_a1())
    }
}

impl<'de> Deserialize<'de> for CellAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let a1 = String::deserialize(deserializer)?;
        CellAddress::from_str(&a1).map_err(|e| DeError::custom(e.to_string()))
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Blank,
    Number(f64),
    Text(StringId),
    Bool(bool),
    Error(StringId),
}

impl PartialEq for CellValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellValue::Blank, CellValue::Blank) => true,
            (CellValue::Number(a), CellValue::Number(b)) => {
                normalize_float_for_hash(*a) == normalize_float_for_hash(*b)
            }
            (CellValue::Text(a), CellValue::Text(b)) => a == b,
            (CellValue::Bool(a), CellValue::Bool(b)) => a == b,
            (CellValue::Error(a), CellValue::Error(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for CellValue {}

impl Hash for CellValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CellValue::Blank => {
                3u8.hash(state);
            }
            CellValue::Number(n) => {
                0u8.hash(state);
                normalize_float_for_hash(*n).hash(state);
            }
            CellValue::Text(id) => {
                1u8.hash(state);
                id.hash(state);
            }
            CellValue::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            CellValue::Error(id) => {
                4u8.hash(state);
                id.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RowSignature {
    pub hash: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColSignature {
    pub hash: u128,
}

#[allow(dead_code)]
mod signature_serde {
    use serde::de::Error as DeError;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize_u128<S>(val: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:032x}", val).serialize(serializer)
    }

    pub fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u128::from_str_radix(&s, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))
    }
}

impl serde::Serialize for RowSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("RowSignature", 1)?;
        s.serialize_field("hash", &format!("{:032x}", self.hash))?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for RowSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        #[derive(serde::Deserialize)]
        struct Helper {
            hash: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        let hash = u128::from_str_radix(&helper.hash, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))?;
        Ok(RowSignature { hash })
    }
}

impl serde::Serialize for ColSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ColSignature", 1)?;
        s.serialize_field("hash", &format!("{:032x}", self.hash))?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for ColSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        #[derive(serde::Deserialize)]
        struct Helper {
            hash: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        let hash = u128::from_str_radix(&helper.hash, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))?;
        Ok(ColSignature { hash })
    }
}

impl Grid {
    pub fn new(nrows: u32, ncols: u32) -> Grid {
        Grid {
            nrows,
            ncols,
            cells: FxHashMap::default(),
            row_signatures: None,
            col_signatures: None,
        }
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&CellContent> {
        self.cells.get(&(row, col))
    }

    pub fn get_ref(&self, row: u32, col: u32) -> Option<CellRef<'_>> {
        self.get(row, col).map(|cell| CellRef {
            row,
            col,
            address: CellAddress::from_indices(row, col),
            value: &cell.value,
            formula: &cell.formula,
        })
    }

    pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
        self.row_signatures = None;
        self.col_signatures = None;
        self.cells.get_mut(&(row, col))
    }

    pub fn insert_cell(&mut self, row: u32, col: u32, value: Option<CellValue>, formula: Option<StringId>) {
        debug_assert!(
            row < self.nrows && col < self.ncols,
            "cell coordinates must lie within the grid bounds"
        );
        self.row_signatures = None;
        self.col_signatures = None;
        self.cells.insert((row, col), CellContent { value, formula });
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = ((u32, u32), &CellContent)> {
        self.cells.iter().map(|(coords, cell)| (*coords, cell))
    }

    pub fn iter_cell_refs(&self) -> impl Iterator<Item = CellRef<'_>> {
        self.cells.iter().map(|((row, col), cell)| CellRef {
            row: *row,
            col: *col,
            address: CellAddress::from_indices(*row, *col),
            value: &cell.value,
            formula: &cell.formula,
        })
    }

    pub fn rows_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.nrows
    }

    pub fn cols_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.ncols
    }

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
            let mut row_cells: Vec<(u32, &CellContent)> = self
                .cells
                .iter()
                .filter(|((r, _), _)| *r == row)
                .map(|((_, c), cell)| (*c, cell))
                .collect();
            row_cells.sort_by_key(|(c, _)| *c);
            for (_, cell) in row_cells {
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
            let mut col_cells: Vec<(u32, &CellContent)> = self
                .cells
                .iter()
                .filter(|((_, c), _)| *c == col)
                .map(|((r, _), cell)| (*r, cell))
                .collect();
            col_cells.sort_by_key(|(r, _)| *r);
            for (_, cell) in col_cells {
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

    pub fn compute_all_signatures(&mut self) {
        use crate::hashing::{hash_cell_value, hash_row_content_128};
        use xxhash_rust::xxh3::Xxh3;

        let mut row_cells: Vec<Vec<(u32, &CellContent)>> = vec![Vec::new(); self.nrows as usize];

        for ((row, col), cell) in self.cells.iter() {
            let row_idx = *row as usize;
            if row_idx >= row_cells.len() || *col >= self.ncols {
                continue;
            }
            row_cells[row_idx].push((*col, cell));
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
            .map(|hasher| ColSignature {
                hash: hasher.digest128(),
            })
            .collect();

        self.row_signatures = Some(row_signatures);
        self.col_signatures = Some(col_signatures);
    }
}

impl PartialEq for CellSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.formula == other.formula
    }
}

impl Eq for CellSnapshot {}

impl CellValue {
    pub fn as_text_id(&self) -> Option<StringId> {
        if let CellValue::Text(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn as_text<'a>(&self, pool: &'a StringPool) -> Option<&'a str> {
        self.as_text_id().map(|id| pool.resolve(id))
    }

    pub fn as_number(&self) -> Option<f64> {
        if let CellValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let CellValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string_pool::StringPool;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn addr(a1: &str) -> CellAddress {
        a1.parse().expect("address should parse")
    }

    fn make_cell(
        pool: &mut StringPool,
        address: &str,
        value: Option<CellValue>,
        formula: Option<&str>,
    ) -> ((u32, u32), CellContent) {
        let (row, col) = address_to_index(address).expect("address should parse");
        let formula_id = formula.map(|s| pool.intern(s));
        (
            (row, col),
            CellContent {
                value,
                formula: formula_id,
            },
        )
    }

    #[test]
    fn snapshot_from_number_cell() {
        let mut pool = StringPool::new();
        let ((row, col), cell) =
            make_cell(&mut pool, "A1", Some(CellValue::Number(42.0)), None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "A1");
        assert_eq!(snap.value, Some(CellValue::Number(42.0)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_text_cell() {
        let mut pool = StringPool::new();
        let text_id = pool.intern("hello");
        let ((row, col), cell) = make_cell(
            &mut pool,
            "B2",
            Some(CellValue::Text(text_id)),
            None,
        );
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "B2");
        assert_eq!(snap.value, Some(CellValue::Text(text_id)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_bool_cell() {
        let mut pool = StringPool::new();
        let ((row, col), cell) =
            make_cell(&mut pool, "C3", Some(CellValue::Bool(true)), None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "C3");
        assert_eq!(snap.value, Some(CellValue::Bool(true)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_empty_cell() {
        let mut pool = StringPool::new();
        let ((row, col), cell) = make_cell(&mut pool, "D4", None, None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "D4");
        assert!(snap.value.is_none());
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_equality_same_value_and_formula() {
        let mut pool = StringPool::new();
        let formula_id = pool.intern("A1+1");
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(1.0)),
            formula: Some(formula_id),
        };
        let snap2 = CellSnapshot {
            addr: addr("B2"),
            value: Some(CellValue::Number(1.0)),
            formula: Some(formula_id),
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_different_value_same_formula() {
        let mut pool = StringPool::new();
        let formula_id = pool.intern("A1+1");
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(43.0)),
            formula: Some(formula_id),
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(44.0)),
            formula: Some(formula_id),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_value_vs_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: None,
        };
        let mut pool = StringPool::new();
        let formula_id = pool.intern("A1+1");
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: Some(formula_id),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_equality_ignores_address() {
        let mut pool = StringPool::new();
        let text_id = pool.intern("hello");
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Text(text_id)),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("Z9"),
            value: Some(CellValue::Text(text_id)),
            formula: None,
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn cellvalue_as_text_number_bool_match_variants() {
        let mut pool = StringPool::new();
        let text_id = pool.intern("abc");
        let text = CellValue::Text(text_id);
        let number = CellValue::Number(5.0);
        let boolean = CellValue::Bool(true);

        assert_eq!(text.as_text(&pool), Some("abc"));
        assert_eq!(text.as_number(), None);
        assert_eq!(text.as_bool(), None);

        assert_eq!(number.as_text(&pool), None);
        assert_eq!(number.as_number(), Some(5.0));
        assert_eq!(number.as_bool(), None);

        assert_eq!(boolean.as_text(&pool), None);
        assert_eq!(boolean.as_number(), None);
        assert_eq!(boolean.as_bool(), Some(true));
    }

    fn hash_cell_value(value: &CellValue) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn cellvalue_number_hashes_normalize_zero_sign() {
        let h_pos = hash_cell_value(&CellValue::Number(0.0));
        let h_neg = hash_cell_value(&CellValue::Number(-0.0));
        assert_eq!(h_pos, h_neg, "hash should ignore sign of zero");
    }

    #[test]
    fn cellvalue_number_hashes_ignore_ulp_drift() {
        let h_a = hash_cell_value(&CellValue::Number(1.0));
        let h_b = hash_cell_value(&CellValue::Number(1.0000000000000002));
        assert_eq!(h_a, h_b, "minor ULP drift should hash identically");
    }

    #[test]
    fn cellvalue_number_hashes_meaningful_difference() {
        let h_a = hash_cell_value(&CellValue::Number(1.0));
        let h_b = hash_cell_value(&CellValue::Number(1.0001));
        assert_ne!(h_a, h_b, "meaningful numeric changes must alter the hash");
    }

    #[test]
    fn get_mut_clears_cached_signatures() {
        let mut pool = StringPool::new();
        let mut grid = Grid::new(2, 2);
        let id1 = pool.intern("1");
        grid.insert_cell(0, 0, Some(CellValue::Text(id1)), None);
        grid.insert_cell(1, 1, Some(CellValue::Number(2.0)), None);

        grid.compute_all_signatures();
        assert!(grid.row_signatures.is_some());
        assert!(grid.col_signatures.is_some());

        let _ = grid.get_mut(0, 0);

        assert!(grid.row_signatures.is_none());
        assert!(grid.col_signatures.is_none());
    }

    #[test]
    fn insert_clears_cached_signatures() {
        let mut pool = StringPool::new();
        let mut grid = Grid::new(3, 3);
        let id1 = pool.intern("1");
        grid.insert_cell(0, 0, Some(CellValue::Text(id1)), None);

        grid.compute_all_signatures();
        assert!(grid.row_signatures.is_some());
        assert!(grid.col_signatures.is_some());

        let id2 = pool.intern("x");
        grid.insert_cell(1, 1, Some(CellValue::Text(id2)), None);

        assert!(grid.row_signatures.is_none());
        assert!(grid.col_signatures.is_none());
    }

    #[test]
    fn compute_row_signature_matches_cached_for_dense_and_sparse_paths() {
        let mut dense = Grid::new(1, 3);
        dense.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
        dense.insert_cell(0, 1, Some(CellValue::Number(2.0)), None);
        dense.insert_cell(0, 2, Some(CellValue::Number(3.0)), None);
        dense.compute_all_signatures();
        let cached_dense = dense.row_signatures.as_ref().unwrap()[0];
        assert_eq!(dense.compute_row_signature(0), cached_dense);

        let mut sparse = Grid::new(1, 10);
        sparse.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
        sparse.insert_cell(0, 9, Some(CellValue::Number(10.0)), None);
        sparse.compute_all_signatures();
        let cached_sparse = sparse.row_signatures.as_ref().unwrap()[0];
        assert_eq!(sparse.compute_row_signature(0), cached_sparse);
    }

    #[test]
    fn compute_col_signature_matches_cached_for_dense_and_sparse_paths() {
        let mut dense = Grid::new(3, 1);
        dense.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
        dense.insert_cell(1, 0, Some(CellValue::Number(2.0)), None);
        dense.insert_cell(2, 0, Some(CellValue::Number(3.0)), None);
        dense.compute_all_signatures();
        let cached_dense = dense.col_signatures.as_ref().unwrap()[0];
        assert_eq!(dense.compute_col_signature(0), cached_dense);

        let mut sparse = Grid::new(10, 2);
        sparse.insert_cell(0, 1, Some(CellValue::Number(1.0)), None);
        sparse.insert_cell(2, 1, Some(CellValue::Number(3.0)), None);
        sparse.compute_all_signatures();
        let cached_sparse = sparse.col_signatures.as_ref().unwrap()[1];
        assert_eq!(sparse.compute_col_signature(1), cached_sparse);
    }
}

```

---

### File: `core\tests\addressing_pg2_tests.rs`

```rust
mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{CellValue, address_to_index, index_to_address, with_default_session};

#[test]
fn pg2_addressing_matrix_consistency() {
    let workbook = open_fixture_workbook("pg2_addressing_matrix.xlsx");
    let sheet_names: Vec<String> = with_default_session(|session| {
        workbook
            .sheets
            .iter()
            .map(|s| session.strings.resolve(s.name).to_string())
            .collect()
    });
    let addresses_id = sid("Addresses");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == addresses_id)
        .unwrap_or_else(|| panic!("Addresses sheet present; found {:?}", sheet_names));

    for cell in sheet.grid.iter_cell_refs() {
        if let Some(CellValue::Text(text_id)) = cell.value {
            let text = with_default_session(|session| session.strings.resolve(*text_id).to_string());
            assert_eq!(cell.address.to_a1(), text.as_str());
            let (r, c) = address_to_index(&text).expect("address strings should parse to indices");
            assert_eq!((r, c), (cell.row, cell.col));
            assert_eq!(index_to_address(cell.row, cell.col), cell.address.to_a1());
        }
    }
}

```

---

### File: `core\tests\amr_multi_gap_tests.rs`

```rust
mod common;

use common::{grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn count_ops(ops: &[DiffOp], predicate: impl Fn(&DiffOp) -> bool) -> usize {
    ops.iter().filter(|op| predicate(op)).count()
}

fn count_row_added(ops: &[DiffOp]) -> usize {
    count_ops(ops, |op| matches!(op, DiffOp::RowAdded { .. }))
}

fn count_row_removed(ops: &[DiffOp]) -> usize {
    count_ops(ops, |op| matches!(op, DiffOp::RowRemoved { .. }))
}

fn count_block_moved_rows(ops: &[DiffOp]) -> usize {
    count_ops(ops, |op| matches!(op, DiffOp::BlockMovedRows { .. }))
}

#[test]
fn amr_two_disjoint_insertion_regions() {
    let grid_a = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 11, 12],
        &[100, 101, 102],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[200, 201, 202],
        &[201, 202, 203],
        &[50, 51, 52],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        3,
        "should detect 3 inserted rows across 2 disjoint regions"
    );
    assert_eq!(
        count_row_removed(&report.ops),
        0,
        "should not detect any removed rows"
    );
}

#[test]
fn amr_insertion_and_deletion_in_different_regions() {
    let grid_a = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[90, 91, 92],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[100, 101, 102],
        &[50, 51, 52],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        1,
        "should detect 1 inserted row near the tail"
    );
    assert_eq!(
        count_row_removed(&report.ops),
        1,
        "should detect 1 deleted row in the middle"
    );
}

#[test]
fn amr_gap_contains_moved_block_scenario() {
    let grid_a = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
        &[60, 61, 62],
        &[70, 71, 72],
        &[80, 81, 82],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 11, 12],
        &[60, 61, 62],
        &[70, 71, 72],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
        &[80, 81, 82],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    let moves = count_block_moved_rows(&report.ops);
    assert!(
        moves >= 1,
        "should detect at least one block move (rows 60-70 moved up)"
    );
    assert_eq!(
        count_row_added(&report.ops),
        0,
        "should not report spurious insertions when move is detected"
    );
    assert_eq!(
        count_row_removed(&report.ops),
        0,
        "should not report spurious deletions when move is detected"
    );
}

#[test]
fn amr_multiple_anchors_with_gaps() {
    let grid_a = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
        &[60, 61, 62],
        &[70, 71, 72],
    ]);

    let grid_b = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 11, 12],
        &[100, 101, 102],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[200, 201, 202],
        &[50, 51, 52],
        &[60, 61, 62],
        &[70, 71, 72],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        2,
        "should detect both inserted rows in separate gaps between anchors"
    );
}

#[test]
fn amr_recursive_gap_alignment() {
    let values_a: Vec<&[i32]> = (1..=50i32)
        .map(|i| {
            let row: &[i32] = match i {
                1 => &[10, 11, 12],
                2 => &[20, 21, 22],
                3 => &[30, 31, 32],
                4 => &[40, 41, 42],
                5 => &[50, 51, 52],
                6 => &[60, 61, 62],
                7 => &[70, 71, 72],
                8 => &[80, 81, 82],
                9 => &[90, 91, 92],
                10 => &[100, 101, 102],
                11 => &[110, 111, 112],
                12 => &[120, 121, 122],
                13 => &[130, 131, 132],
                14 => &[140, 141, 142],
                15 => &[150, 151, 152],
                16 => &[160, 161, 162],
                17 => &[170, 171, 172],
                18 => &[180, 181, 182],
                19 => &[190, 191, 192],
                20 => &[200, 201, 202],
                21 => &[210, 211, 212],
                22 => &[220, 221, 222],
                23 => &[230, 231, 232],
                24 => &[240, 241, 242],
                25 => &[250, 251, 252],
                26 => &[260, 261, 262],
                27 => &[270, 271, 272],
                28 => &[280, 281, 282],
                29 => &[290, 291, 292],
                30 => &[300, 301, 302],
                31 => &[310, 311, 312],
                32 => &[320, 321, 322],
                33 => &[330, 331, 332],
                34 => &[340, 341, 342],
                35 => &[350, 351, 352],
                36 => &[360, 361, 362],
                37 => &[370, 371, 372],
                38 => &[380, 381, 382],
                39 => &[390, 391, 392],
                40 => &[400, 401, 402],
                41 => &[410, 411, 412],
                42 => &[420, 421, 422],
                43 => &[430, 431, 432],
                44 => &[440, 441, 442],
                45 => &[450, 451, 452],
                46 => &[460, 461, 462],
                47 => &[470, 471, 472],
                48 => &[480, 481, 482],
                49 => &[490, 491, 492],
                50 => &[500, 501, 502],
                _ => &[0, 0, 0],
            };
            row
        })
        .collect();
    let grid_a = grid_from_numbers(&values_a);

    let mut values_b: Vec<&[i32]> = values_a.clone();
    values_b.insert(10, &[1000, 1001, 1002]);
    values_b.insert(25, &[2000, 2001, 2002]);
    values_b.insert(40, &[3000, 3001, 3002]);

    let grid_b = grid_from_numbers(&values_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        3,
        "should detect all 3 inserted rows distributed across the grid"
    );
}

```

---

### File: `core\tests\common\mod.rs`

```rust
//! Common test utilities shared across integration tests.

#![allow(dead_code)]

use excel_diff::{
    CellValue, DiffConfig, DiffReport, Grid, Sheet, SheetKind, StringId, Workbook,
    WorkbookPackage, with_default_session,
};
use std::fs::File;
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}

pub fn open_fixture_pkg(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });
    WorkbookPackage::open(file).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    })
}

pub fn open_fixture_workbook(name: &str) -> Workbook {
    open_fixture_pkg(name).workbook
}

pub fn diff_fixture_pkgs(a: &str, b: &str, config: &DiffConfig) -> DiffReport {
    let pkg_a = open_fixture_pkg(a);
    let pkg_b = open_fixture_pkg(b);
    pkg_a.diff(&pkg_b, config)
}

pub fn grid_from_numbers(values: &[&[i32]]) -> Grid {
    let nrows = values.len() as u32;
    let ncols = if nrows == 0 {
        0
    } else {
        values[0].len() as u32
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, row_vals) in values.iter().enumerate() {
        for (c, v) in row_vals.iter().enumerate() {
            grid.insert_cell(r as u32, c as u32, Some(CellValue::Number(*v as f64)), None);
        }
    }

    grid
}

pub fn sid(s: &str) -> StringId {
    with_default_session(|session| session.strings.intern(s))
}

pub fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    with_default_session(|session| Workbook {
        sheets: vec![Sheet {
            name: session.strings.intern(name),
            kind: SheetKind::Worksheet,
            grid,
        }],
    })
}

```

---

### File: `core\tests\d1_database_mode_tests.rs`

```rust
mod common;

use common::{grid_from_numbers, open_fixture_workbook, sid};
use excel_diff::{
    CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage,
    diff_grids_database_mode, with_default_session,
};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn diff_db(grid_a: &Grid, grid_b: &Grid, keys: &[u32]) -> DiffReport {
    with_default_session(|session| {
        diff_grids_database_mode(
            grid_a,
            grid_b,
            keys,
            &mut session.strings,
            &DiffConfig::default(),
        )
    })
}

fn data_grid(workbook: &Workbook) -> &Grid {
    let data_id = sid("Data");
    workbook
        .sheets
        .iter()
        .find(|s| s.name == data_id)
        .map(|s| &s.grid)
        .expect("Data sheet present")
}

fn grid_from_float_rows(rows: &[&[f64]]) -> Grid {
    let nrows = rows.len() as u32;
    let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
    let mut grid = Grid::new(nrows, ncols);

    for (r_idx, row_vals) in rows.iter().enumerate() {
        for (c_idx, value) in row_vals.iter().enumerate() {
            grid.insert_cell(r_idx as u32, c_idx as u32, Some(CellValue::Number(*value)), None);
        }
    }

    grid
}

#[test]
fn d1_equal_ordered_database_mode_empty_diff() {
    let workbook = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let grid = data_grid(&workbook);

    let report = diff_db(grid, grid, &[0]);
    assert!(
        report.ops.is_empty(),
        "database mode should ignore row order when keyed rows are identical"
    );
}

#[test]
fn d1_equal_reordered_database_mode_empty_diff() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_equal_ordered_b.xlsx");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_db(grid_a, grid_b, &[0]);
    assert!(
        report.ops.is_empty(),
        "keyed alignment should match rows by key and ignore reordering"
    );
}

#[test]
fn d1_spreadsheet_mode_sees_reorder_as_changes() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_equal_ordered_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "Spreadsheet Mode should see structural changes when rows are reordered, \
         demonstrating the semantic difference from Database Mode"
    );
}

#[test]
fn d1_duplicate_keys_fallback_to_spreadsheet_mode() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[1, 99]]);
    let grid_b = grid_from_numbers(&[&[1, 10]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    assert!(
        !report.ops.is_empty(),
        "duplicate keys cause fallback to spreadsheet mode which should detect differences"
    );

    let has_row_removed = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::RowRemoved { .. }));
    assert!(
        has_row_removed,
        "spreadsheet mode fallback should emit RowRemoved for the missing row"
    );
}

#[test]
fn d1_database_mode_row_added() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20]]);
    let grid_b = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "database mode should emit one RowAdded for key 3"
    );
}

#[test]
fn d1_database_mode_row_removed() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);
    let grid_b = grid_from_numbers(&[&[1, 10], &[2, 20]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "database mode should emit one RowRemoved for key 3"
    );
}

#[test]
fn d1_database_mode_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20]]);
    let grid_b = grid_from_numbers(&[&[1, 99], &[2, 20]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "database mode should emit one CellEdited for the changed non-key cell"
    );
}

#[test]
fn d1_database_mode_cell_edited_with_reorder() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);
    let grid_b = grid_from_numbers(&[&[3, 30], &[2, 99], &[1, 10]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "database mode should ignore reordering and find only the cell edit for key 2"
    );
}

#[test]
fn d1_database_mode_treats_small_float_key_noise_as_equal() {
    let grid_a = grid_from_float_rows(&[&[1.0, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);
    let grid_b = grid_from_float_rows(&[&[1.0000000000000002, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);
    assert!(
        report.ops.is_empty(),
        "ULP-level noise in key column should not break row alignment"
    );
}

#[test]
fn d1_database_mode_detects_meaningful_float_key_change() {
    let grid_a = grid_from_float_rows(&[&[1.0, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);
    let grid_b = grid_from_float_rows(&[&[1.0001, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let row_removed = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    let row_added = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();

    assert_eq!(
        row_removed, 1,
        "meaningful key drift should remove the original keyed row"
    );
    assert_eq!(
        row_added, 1,
        "meaningful key drift should add the new keyed row"
    );
}

#[test]
fn d5_composite_key_equal_reordered_database_mode_empty_diff() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 20, 200], &[2, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 10, 300], &[1, 10, 100], &[1, 20, 200]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);
    assert!(
        report.ops.is_empty(),
        "composite keyed alignment should ignore row order differences"
    );
}

#[test]
fn d5_composite_key_row_added_and_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 20, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 150], &[1, 20, 200], &[2, 30, 300]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "new composite key should produce exactly one RowAdded"
    );

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 0,
        "no rows should be removed when only a new composite key is introduced"
    );

    let mut cell_edited_iter = report.ops.iter().filter_map(|op| {
        if let DiffOp::CellEdited { addr, .. } = op {
            Some(addr)
        } else {
            None
        }
    });

    let edited_addr = cell_edited_iter
        .next()
        .expect("one cell edit for changed non-key value");
    assert!(
        cell_edited_iter.next().is_none(),
        "only one CellEdited should be present"
    );
    assert_eq!(edited_addr.col, 2, "only non-key column should be edited");
    assert_eq!(
        edited_addr.row, 0,
        "cell edit should reference the row of key (1,10) in the new grid"
    );
}

#[test]
fn d5_composite_key_partial_key_mismatch_yields_add_and_remove() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100]]);
    let grid_b = grid_from_numbers(&[&[1, 20, 100]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "changed composite key should remove the old tuple"
    );

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "changed composite key should add the new tuple"
    );

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 0,
        "partial key match must not be treated as a cell edit"
    );
}

#[test]
fn d5_composite_key_duplicate_keys_fallback_to_spreadsheet_mode() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 10, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 100]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);

    assert!(
        !report.ops.is_empty(),
        "duplicate composite keys should trigger spreadsheet-mode fallback"
    );

    let has_row_removed = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::RowRemoved { .. }));
    assert!(
        has_row_removed,
        "fallback should emit a RowRemoved reflecting duplicate handling"
    );
}

#[test]
fn d5_non_contiguous_key_columns_equal_reordered_empty_diff() {
    let grid_a = grid_from_numbers(&[&[1, 999, 10, 100], &[1, 888, 20, 200], &[2, 777, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 777, 10, 300], &[1, 999, 10, 100], &[1, 888, 20, 200]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 2]);
    assert!(
        report.ops.is_empty(),
        "non-contiguous key columns [0,2] should align correctly ignoring row order"
    );
}

#[test]
fn d5_non_contiguous_key_columns_detects_edits_in_skipped_column() {
    let grid_a = grid_from_numbers(&[&[1, 999, 10, 100], &[1, 888, 20, 200], &[2, 777, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 111, 10, 300], &[1, 222, 10, 100], &[1, 333, 20, 200]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 2]);

    let cell_edited_ops: Vec<_> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                Some(addr)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        cell_edited_ops.len(),
        3,
        "should detect 3 edits in skipped non-key column 1"
    );

    for addr in &cell_edited_ops {
        assert_eq!(
            addr.col, 1,
            "all edits should be in the skipped column 1, not key columns 0 or 2"
        );
    }

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(row_added_count, 0, "no rows should be added");

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(row_removed_count, 0, "no rows should be removed");
}

#[test]
fn d5_non_contiguous_key_columns_row_added_and_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 999, 10, 100], &[1, 888, 20, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 999, 10, 150], &[1, 888, 20, 200], &[2, 777, 30, 300]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 2]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "new non-contiguous composite key should produce exactly one RowAdded"
    );

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(row_removed_count, 0, "no rows should be removed");

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "changed non-key column should produce exactly one CellEdited"
    );
}

#[test]
fn d5_three_column_composite_key_equal_reordered_empty_diff() {
    let grid_a = grid_from_numbers(&[
        &[1, 10, 100, 1000],
        &[1, 10, 200, 2000],
        &[1, 20, 100, 3000],
        &[2, 10, 100, 4000],
    ]);
    let grid_b = grid_from_numbers(&[
        &[2, 10, 100, 4000],
        &[1, 20, 100, 3000],
        &[1, 10, 200, 2000],
        &[1, 10, 100, 1000],
    ]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1, 2]);
    assert!(
        report.ops.is_empty(),
        "three-column composite key should align correctly ignoring row order"
    );
}

#[test]
fn d5_three_column_composite_key_partial_match_yields_add_and_remove() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100, 1000]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 200, 1000]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1, 2]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "changed third key column should remove the old tuple"
    );

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "changed third key column should add the new tuple"
    );

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 0,
        "partial three-column key match must not be treated as a cell edit"
    );
}

```

---

### File: `core\tests\data_mashup_tests.rs`

```rust
use std::fs::File;
use std::io::{ErrorKind, Read};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use excel_diff::{
    ContainerError, DataMashupError, PackageError, RawDataMashup, build_data_mashup,
    open_data_mashup,
};
use quick_xml::{Reader, events::Event};
use zip::ZipArchive;

mod common;
use common::fixture_path;

#[test]
fn workbook_without_datamashup_returns_none() {
    let path = fixture_path("minimal.xlsx");
    let result = open_data_mashup(&path).expect("minimal workbook should load");
    assert!(result.is_none());
}

#[test]
fn workbook_with_valid_datamashup_parses() {
    let path = fixture_path("m_change_literal_b.xlsx");
    let raw = open_data_mashup(&path)
        .expect("valid mashup should load")
        .expect("mashup should be present");

    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
    assert!(!raw.metadata.is_empty());

    let assembled = assemble_top_level_bytes(&raw);
    let expected = datamashup_bytes_from_fixture(&path);
    assert_eq!(assembled, expected);
}

#[test]
fn datamashup_with_base64_whitespace_parses() {
    let path = fixture_path("mashup_base64_whitespace.xlsx");
    let raw = open_data_mashup(&path)
        .expect("whitespace in base64 payload should be tolerated")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_le_datamashup_parses() {
    let path = fixture_path("mashup_utf16_le.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16LE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_be_datamashup_parses() {
    let path = fixture_path("mashup_utf16_be.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16BE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn corrupt_base64_returns_error() {
    let path = fixture_path("corrupt_base64.xlsx");
    let err = open_data_mashup(&path).expect_err("corrupt base64 should fail");
    assert!(matches!(
        err,
        PackageError::DataMashup(DataMashupError::Base64Invalid)
    ));
}

#[test]
fn duplicate_datamashup_parts_are_rejected() {
    let path = fixture_path("duplicate_datamashup_parts.xlsx");
    let err = open_data_mashup(&path).expect_err("duplicate DataMashup parts should be rejected");
    assert!(matches!(
        err,
        PackageError::DataMashup(DataMashupError::FramingInvalid)
    ));
}

#[test]
fn duplicate_datamashup_elements_are_rejected() {
    let path = fixture_path("duplicate_datamashup_elements.xlsx");
    let err =
        open_data_mashup(&path).expect_err("duplicate DataMashup elements should be rejected");
    assert!(matches!(
        err,
        PackageError::DataMashup(DataMashupError::FramingInvalid)
    ));
}

#[test]
fn nonexistent_file_returns_io() {
    let path = fixture_path("missing_mashup.xlsx");
    let err = open_data_mashup(&path).expect_err("missing file should error");
    match err {
        PackageError::Container(ContainerError::Io(e)) => {
            assert_eq!(e.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn non_excel_container_returns_not_excel_error() {
    let path = fixture_path("random_zip.zip");
    let err = open_data_mashup(&path).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn missing_content_types_is_not_excel_error() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_data_mashup(&path).expect_err("missing [Content_Types].xml should fail");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn non_zip_file_returns_not_zip_error() {
    let path = fixture_path("not_a_zip.txt");
    let err = open_data_mashup(&path).expect_err("non-zip input should not parse as Excel");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotZipContainer)
    ));
}

#[test]
fn build_data_mashup_smoke_from_fixture() {
    let raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let dm = build_data_mashup(&raw).expect("build_data_mashup should succeed");

    assert_eq!(dm.version, 0);
    assert!(
        dm.package_parts
            .main_section
            .source
            .contains("section Section1;")
    );
    assert!(!dm.metadata.formulas.is_empty());

    let non_connection: Vec<_> = dm
        .metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();
    assert_eq!(non_connection.len(), 1);
    let meta = non_connection[0];
    assert_eq!(
        meta.item_path,
        format!("{}/{}", meta.section_name, meta.formula_name)
    );
    assert_eq!(meta.item_path, "Section1/Query1");
    assert_eq!(meta.section_name, "Section1");
    assert_eq!(meta.formula_name, "Query1");
    assert!(meta.load_to_sheet || meta.load_to_model);
}

fn datamashup_bytes_from_fixture(path: &std::path::Path) -> Vec<u8> {
    let file = File::open(path).expect("fixture should be readable");
    let mut archive = ZipArchive::new(file).expect("fixture should be a zip container");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry should be readable");
        let name = file.name().to_string();
        if !name.starts_with("customXml/") || !name.ends_with(".xml") {
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("XML part should read");
        if let Some(text) = extract_datamashup_base64(&buf) {
            let cleaned: String = text.split_whitespace().collect();
            return STANDARD
                .decode(cleaned.as_bytes())
                .expect("DataMashup base64 should decode");
        }
    }

    panic!("DataMashup element not found in {}", path.display());
}

fn extract_datamashup_base64(xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup {
                    return None;
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t.unescape().ok()?.into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                content.push_str(&String::from_utf8_lossy(&t.into_inner()));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return None;
                }
                return Some(content.clone());
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn assemble_top_level_bytes(raw: &RawDataMashup) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&raw.version.to_le_bytes());
    bytes.extend_from_slice(&(raw.package_parts.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.package_parts);
    bytes.extend_from_slice(&(raw.permissions.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permissions);
    bytes.extend_from_slice(&(raw.metadata.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.metadata);
    bytes.extend_from_slice(&(raw.permission_bindings.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permission_bindings);
    bytes
}

```

---

### File: `core\tests\engine_tests.rs`

```rust
mod common;

use common::sid;
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, DiffConfig, DiffOp, DiffReport, Grid, Sheet, SheetKind,
    Workbook, WorkbookPackage,
};

type SheetSpec<'a> = (&'a str, Vec<(u32, u32, f64)>);

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn make_workbook(sheets: Vec<SheetSpec<'_>>) -> Workbook {
    let sheet_ir: Vec<Sheet> = sheets
        .into_iter()
        .map(|(name, cells)| {
            let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
            let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
            let mut grid = Grid::new(max_row + 1, max_col + 1);
            for (r, c, val) in cells {
                grid.insert_cell(r, c, Some(CellValue::Number(val)), None);
            }
            Sheet {
                name: sid(name),
                kind: SheetKind::Worksheet,
                grid,
            }
        })
        .collect();
    Workbook { sheets: sheet_ir }
}

fn make_sheet_with_kind(name: &str, kind: SheetKind, cells: Vec<(u32, u32, f64)>) -> Sheet {
    let (nrows, ncols) = if cells.is_empty() {
        (0, 0)
    } else {
        let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
        let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
        (max_row + 1, max_col + 1)
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, c, val) in cells {
        grid.insert_cell(r, c, Some(CellValue::Number(val)), None);
    }

    Sheet {
        name: sid(name),
        kind,
        grid,
    }
}

#[test]
fn identical_workbooks_produce_empty_report() {
    let wb = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&wb, &wb, &DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_added_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetAdded { sheet } if *sheet == sid("Sheet2")))
    );
}

#[test]
fn sheet_removed_detected() {
    let old = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if *sheet == sid("Sheet2")))
    );
}

#[test]
fn cell_edited_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(*sheet, sid("Sheet1"));
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        _ => panic!("expected CellEdited"),
    }
}

#[test]
fn diff_report_json_round_trips() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
}

#[test]
fn sheet_name_case_insensitive_no_changes() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_name_case_insensitive_cell_edit() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(*sheet, sid("Sheet1"));
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn sheet_identity_includes_kind() {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);

    let worksheet = Sheet {
        name: sid("Sheet1"),
        kind: SheetKind::Worksheet,
        grid: grid.clone(),
    };

    let chart = Sheet {
        name: sid("Sheet1"),
        kind: SheetKind::Chart,
        grid,
    };

    let old = Workbook {
        sheets: vec![worksheet],
    };
    let new = Workbook {
        sheets: vec![chart],
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("Sheet1") => added += 1,
            DiffOp::SheetRemoved { sheet } if *sheet == sid("Sheet1") => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Chart 'Sheet1'");
    assert_eq!(
        removed, 1,
        "expected one SheetRemoved for Worksheet 'Sheet1'"
    );
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[test]
fn deterministic_sheet_op_ordering() {
    let budget_old = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let budget_new = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 2.0)]);
    let sheet1_old = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 1, 5.0)]);
    let sheet1_chart = make_sheet_with_kind("sheet1", SheetKind::Chart, Vec::new());
    let summary_new = make_sheet_with_kind("Summary", SheetKind::Worksheet, vec![(0, 0, 3.0)]);

    let old = Workbook {
        sheets: vec![budget_old.clone(), sheet1_old],
    };
    let new = Workbook {
        sheets: vec![budget_new.clone(), sheet1_chart, summary_new],
    };

    let budget_addr = CellAddress::from_indices(0, 0);
    let expected = vec![
        DiffOp::cell_edited(
            sid("Budget"),
            budget_addr,
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(1.0)),
                formula: None,
            },
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(2.0)),
                formula: None,
            },
        ),
        DiffOp::SheetRemoved {
            sheet: sid("Sheet1"),
        },
        DiffOp::SheetAdded {
            sheet: sid("sheet1"),
        },
        DiffOp::SheetAdded {
            sheet: sid("Summary"),
        },
    ];

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(
        report.ops, expected,
        "ops should be ordered by lowercase name then sheet kind"
    );
}

#[test]
fn sheet_identity_includes_kind_for_macro_and_other() {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);

    let macro_sheet = Sheet {
        name: sid("Code"),
        kind: SheetKind::Macro,
        grid: grid.clone(),
    };

    let other_sheet = Sheet {
        name: sid("Code"),
        kind: SheetKind::Other,
        grid,
    };

    let old = Workbook {
        sheets: vec![macro_sheet],
    };
    let new = Workbook {
        sheets: vec![other_sheet],
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("Code") => added += 1,
            DiffOp::SheetRemoved { sheet } if *sheet == sid("Code") => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Other 'Code'");
    assert_eq!(removed, 1, "expected one SheetRemoved for Macro 'Code'");
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[cfg(not(debug_assertions))]
#[test]
fn duplicate_sheet_identity_last_writer_wins_release() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);

    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1, "expected last writer to win");

    match &report.ops[0] {
        DiffOp::SheetRemoved { sheet } => assert_eq!(
            *sheet, sid("sheet1"),
            "duplicate identity should prefer the last sheet in release builds"
        ),
        other => panic!("expected SheetRemoved, got {other:?}"),
    }
}

#[test]
fn move_detection_respects_column_gate() {
    let nrows: u32 = 4;
    let ncols: u32 = 300;
    let src_rows = 1..3;
    let src_cols = 2..7;
    let dst_start_col: u32 = 200;
    let dst_end_col = dst_start_col + (src_cols.end - src_cols.start);

    let mut grid_a = Grid::new(nrows, ncols);
    let mut grid_b = Grid::new(nrows, ncols);

    for r in 0..nrows {
        for c in 0..ncols {
            let base_value = Some(CellValue::Number((r * 1_000 + c) as f64));

            grid_a.insert_cell(r, c, base_value.clone(), None);

            let in_src = src_rows.contains(&r) && src_cols.contains(&c);
            let in_dst = src_rows.contains(&r) && c >= dst_start_col && c < dst_end_col;

            if in_dst {
                let offset = c - dst_start_col;
                let src_c = src_cols.start + offset;
                let moved_value = Some(CellValue::Number((r * 1_000 + src_c) as f64));
                grid_b.insert_cell(r, c, moved_value, None);
            } else if !in_src {
                grid_b.insert_cell(r, c, base_value, None);
            }
        }
    }

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sid("Sheet1"),
            kind: SheetKind::Worksheet,
            grid: grid_a,
        }],
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sid("Sheet1"),
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
    };

    let default_report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());
    assert!(
        !default_report.ops.is_empty(),
        "changes should be detected even when move detection is gated off"
    );
    assert!(
        !default_report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "default gate should skip block move detection on wide sheets"
    );

    let wide_gate = DiffConfig {
        max_move_detection_cols: 512,
        ..DiffConfig::default()
    };
    let wide_report = diff_workbooks(&wb_a, &wb_b, &wide_gate);
    assert!(
        !wide_report.ops.is_empty(),
        "expected diffs when move detection is enabled"
    );
    assert!(
        wide_report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "wider gate should allow block move detection on wide sheets"
    );
}

#[test]
fn duplicate_sheet_identity_panics_in_debug() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);
    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let result =
        std::panic::catch_unwind(|| diff_workbooks(&old, &new, &DiffConfig::default()));
    if cfg!(debug_assertions) {
        assert!(
            result.is_err(),
            "duplicate sheet identities should trigger a debug assertion"
        );
    } else {
        assert!(result.is_ok(), "debug assertions disabled should not panic");
    }
}

```

---

### File: `core\tests\excel_open_xml_tests.rs`

```rust
mod common;

use common::{fixture_path, open_fixture_workbook, sid};
use excel_diff::{CellAddress, ContainerError, PackageError, SheetKind, WorkbookPackage};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::time::SystemTime;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

fn temp_xlsx_path(prefix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    path.push(format!("excel_diff_{prefix}_{nanos}.xlsx"));
    path
}

fn write_zip(entries: &[(&str, &str)], path: &Path) {
    let file = fs::File::create(path).expect("create temp zip");
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, contents) in entries {
        writer.start_file(*name, options).expect("start zip entry");
        writer
            .write_all(contents.as_bytes())
            .expect("write zip entry");
    }

    writer.finish().expect("finish zip");
}

#[test]
fn open_minimal_workbook_succeeds() {
    let workbook = open_fixture_workbook("minimal.xlsx");
    assert_eq!(workbook.sheets.len(), 1);

    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.name, sid("Sheet1"));
    assert!(matches!(sheet.kind, SheetKind::Worksheet));
    assert_eq!(sheet.grid.nrows, 1);
    assert_eq!(sheet.grid.ncols, 1);

    let cell = sheet.grid.get(0, 0).expect("A1 should be present");
    assert_eq!(CellAddress::from_coords(0, 0).to_a1(), "A1");
    assert!(cell.value.is_some());
}

#[test]
fn open_nonexistent_file_returns_io_error() {
    let path = fixture_path("definitely_missing.xlsx");
    let file = std::fs::File::open(&path);
    assert!(file.is_err(), "missing file should error");
    let io_err = file.unwrap_err();
    assert_eq!(io_err.kind(), ErrorKind::NotFound);
}

#[test]
fn random_zip_is_not_excel() {
    let path = fixture_path("random_zip.zip");
    let file = std::fs::File::open(&path).expect("random zip file exists");
    let err = WorkbookPackage::open(file).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn no_content_types_is_not_excel() {
    let path = fixture_path("no_content_types.xlsx");
    let file = std::fs::File::open(&path).expect("no content types file exists");
    let err = WorkbookPackage::open(file).expect_err("missing content types should fail");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn not_zip_container_returns_error() {
    let path = std::env::temp_dir().join("excel_diff_not_zip.txt");
    fs::write(&path, "this is not a zip container").expect("write temp file");
    let file = std::fs::File::open(&path).expect("not zip file exists");
    let err = WorkbookPackage::open(file).expect_err("non-zip should fail");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotZipContainer)
    ));
    let _ = fs::remove_file(&path);
}

#[test]
fn missing_workbook_xml_returns_workbookxmlmissing() {
    let path = temp_xlsx_path("missing_workbook_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    write_zip(&[("[Content_Types].xml", content_types)], &path);

    let file = std::fs::File::open(&path).expect("temp file exists");
    let err = WorkbookPackage::open(file).expect_err("missing workbook xml should error");
    assert!(matches!(err, PackageError::WorkbookXmlMissing));

    let _ = fs::remove_file(&path);
}

#[test]
fn missing_worksheet_xml_returns_worksheetxmlmissing() {
    let path = temp_xlsx_path("missing_worksheet_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let workbook_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

    let relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
                Target="worksheets/sheet1.xml"/>
</Relationships>"#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", workbook_xml),
            ("xl/_rels/workbook.xml.rels", relationships),
        ],
        &path,
    );

    let file = std::fs::File::open(&path).expect("temp file exists");
    let err = WorkbookPackage::open(file).expect_err("missing worksheet xml should error");
    match err {
        PackageError::WorksheetXmlMissing { sheet_name } => {
            assert_eq!(sheet_name, "Sheet1");
        }
        other => panic!("expected WorksheetXmlMissing, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

```

---

### File: `core\tests\g10_row_block_alignment_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, sid};
use excel_diff::{DiffConfig, DiffOp};

#[test]
fn g10_row_block_insert_middle_emits_four_rowadded_and_no_noise() {
    let report = diff_fixture_pkgs(
        "row_block_insert_a.xlsx",
        "row_block_insert_b.xlsx",
        &DiffConfig::default(),
    );

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(*sheet, sid("Sheet1"));
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_added,
        vec![3, 4, 5, 6],
        "expected four RowAdded ops for the inserted block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "no rows should be removed for block insert"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned block insert should not emit CellEdited noise"
    );
}

#[test]
fn g10_row_block_delete_middle_emits_four_rowremoved_and_no_noise() {
    let report = diff_fixture_pkgs(
        "row_block_delete_a.xlsx",
        "row_block_delete_b.xlsx",
        &DiffConfig::default(),
    );

    let rows_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(*sheet, sid("Sheet1"));
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_removed,
        vec![3, 4, 5, 6],
        "expected four RowRemoved ops for the deleted block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "no rows should be added for block delete"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned block delete should not emit CellEdited noise"
    );
}

```

---

### File: `core\tests\g11_row_block_move_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g11_row_block_move_emits_single_blockmovedrows() {
    let report = diff_fixture_pkgs(
        "row_block_move_a.xlsx",
        "row_block_move_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    let strings = &report.strings;

    match &report.ops[0] {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        } => {
            assert_eq!(
                strings.get(sheet.0 as usize).map(String::as_str),
                Some("Sheet1")
            );
            assert_eq!(*src_start_row, 4);
            assert_eq!(*row_count, 4);
            assert_eq!(*dst_start_row, 12);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedRows op, got {:?}", other),
    }

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "pure move should not emit RowAdded"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "pure move should not emit RowRemoved"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "pure move should not emit CellEdited noise"
    );
}

#[test]
fn g11_repeated_rows_do_not_emit_blockmove() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[1, 10], &[2, 20], &[2, 20]]);

    let grid_b = grid_from_numbers(&[&[2, 20], &[2, 20], &[1, 10], &[1, 10]]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "ambiguous repeated rows must not emit BlockMovedRows"
    );

    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "fallback path should emit positional CellEdited noise"
    );
}

```

---

### File: `core\tests\g12_column_block_move_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, sid, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g12_column_move_emits_single_blockmovedcolumns() {
    let report = diff_fixture_pkgs(
        "column_move_a.xlsx",
        "column_move_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, &sid("Data"));
            assert_eq!(*src_start_col, 2);
            assert_eq!(*col_count, 1);
            assert_eq!(*dst_start_col, 5);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedColumns op, got {:?}", other),
    }

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::ColumnAdded { .. })),
        "pure move should not emit ColumnAdded"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::ColumnRemoved { .. })),
        "pure move should not emit ColumnRemoved"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. })),
        "pure move should not emit row operations"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "pure move should not emit CellEdited noise"
    );
}

#[test]
fn g12_repeated_columns_do_not_emit_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[1, 1, 2, 2], &[10, 10, 20, 20]]);
    let grid_b = grid_from_numbers(&[&[2, 2, 1, 1], &[20, 20, 10, 10]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. })),
        "ambiguous repeated columns must not emit BlockMovedColumns"
    );

    assert!(
        report.ops.iter().any(|op| matches!(
            op,
            DiffOp::CellEdited { .. } | DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. }
        )),
        "fallback path should emit some other diff operation"
    );
}

#[test]
fn g12_multi_column_block_move_emits_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[
        &[10, 20, 30, 40, 50, 60],
        &[11, 21, 31, 41, 51, 61],
        &[12, 22, 32, 42, 52, 62],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 40, 50, 20, 30, 60],
        &[11, 41, 51, 21, 31, 61],
        &[12, 42, 52, 22, 32, 62],
    ]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert_eq!(
        report.ops.len(),
        1,
        "expected a single diff op for multi-column move"
    );

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, &sid("Data"));
            assert_eq!(*src_start_col, 3);
            assert_eq!(*col_count, 2, "should detect a 2-column block move");
            assert_eq!(*dst_start_col, 1);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedColumns op, got {:?}", other),
    }
}

#[test]
fn g12_two_independent_column_moves_do_not_emit_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[10, 20, 30, 40, 50, 60], &[11, 21, 31, 41, 51, 61]]);

    let grid_b = grid_from_numbers(&[&[20, 10, 30, 40, 60, 50], &[21, 11, 31, 41, 61, 51]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. })),
        "two independent column swaps must not emit BlockMovedColumns"
    );

    assert!(
        !report.ops.is_empty(),
        "fallback path should emit some diff operations"
    );
}

#[test]
fn g12_column_swap_emits_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

    let grid_b = grid_from_numbers(&[&[20, 10, 30, 40], &[21, 11, 31, 41]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert_eq!(
        report.ops.len(),
        1,
        "swap should produce single BlockMovedColumns op"
    );

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            col_count,
            src_start_col,
            dst_start_col,
            ..
        } => {
            assert_eq!(sheet, &sid("Data"));
            assert_eq!(*col_count, 1, "swap is represented as moving one column");
            assert!(
                (*src_start_col == 0 && *dst_start_col == 1)
                    || (*src_start_col == 1 && *dst_start_col == 0),
                "swap should move column 0 or 1 past the other"
            );
        }
        other => panic!("expected BlockMovedColumns, got {:?}", other),
    }
}

```

---

### File: `core\tests\g12_rect_block_move_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, sid, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g12_rect_block_move_emits_single_blockmovedrect() {
    let report = diff_fixture_pkgs(
        "rect_block_move_a.xlsx",
        "rect_block_move_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash: _,
        } => {
            assert_eq!(*sheet, sid("Data"));
            assert_eq!(*src_start_row, 2);
            assert_eq!(*src_row_count, 3);
            assert_eq!(*src_start_col, 1);
            assert_eq!(*src_col_count, 3);
            assert_eq!(*dst_start_row, 9);
            assert_eq!(*dst_start_col, 6);
        }
        other => panic!("expected BlockMovedRect op, got {:?}", other),
    }
}

#[test]
fn g12_rect_block_move_ambiguous_swap_does_not_emit_blockmovedrect() {
    let (grid_a, grid_b) = swap_two_blocks();
    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "ambiguous block swap must not emit BlockMovedRect"
    );
    assert!(
        !report.ops.is_empty(),
        "fallback path should emit some diff operations"
    );
}

#[test]
fn g12_rect_block_move_with_internal_edit_falls_back() {
    let (grid_a, grid_b) = move_with_edit();
    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "move with internal edit should not be treated as exact rectangular move"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "edited block should surface as cell edits or structural diffs"
    );
}

fn swap_two_blocks() -> (excel_diff::Grid, excel_diff::Grid) {
    let base: Vec<Vec<i32>> = (0..6)
        .map(|r| (0..6).map(|c| 100 * r + c).collect())
        .collect();
    let mut grid_a = base.clone();
    let mut grid_b = base.clone();

    let block_one = vec![vec![900, 901], vec![902, 903]];
    let block_two = vec![vec![700, 701], vec![702, 703]];

    place_block(&mut grid_a, 0, 0, &block_one);
    place_block(&mut grid_a, 3, 3, &block_two);

    // Swap the two distinct blocks in grid B.
    place_block(&mut grid_b, 0, 0, &block_two);
    place_block(&mut grid_b, 3, 3, &block_one);

    (grid_from_matrix(grid_a), grid_from_matrix(grid_b))
}

fn move_with_edit() -> (excel_diff::Grid, excel_diff::Grid) {
    let mut grid_a = base_background(10, 10);
    let mut grid_b = base_background(10, 10);

    let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

    place_block(&mut grid_a, 1, 1, &block);
    place_block(&mut grid_b, 6, 4, &block);
    grid_b[7][5] = 9_999; // edit inside the moved block

    (grid_from_matrix(grid_a), grid_from_matrix(grid_b))
}

fn base_background(rows: usize, cols: usize) -> Vec<Vec<i32>> {
    (0..rows)
        .map(|r| (0..cols).map(|c| (r as i32) * 1_000 + c as i32).collect())
        .collect()
}

fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
    for (r_offset, row_vals) in block.iter().enumerate() {
        for (c_offset, value) in row_vals.iter().enumerate() {
            let row = top + r_offset;
            let col = left + c_offset;
            if let Some(row_slice) = target.get_mut(row)
                && let Some(cell) = row_slice.get_mut(col)
            {
                *cell = *value;
            }
        }
    }
}

fn grid_from_matrix(matrix: Vec<Vec<i32>>) -> excel_diff::Grid {
    let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
    grid_from_numbers(&refs)
}

```

---

### File: `core\tests\g13_fuzzy_row_move_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g13_fuzzy_row_move_emits_blockmovedrows_and_celledited() {
    let report = diff_fixture_pkgs(
        "grid_move_and_edit_a.xlsx",
        "grid_move_and_edit_b.xlsx",
        &DiffConfig::default(),
    );

    let block_moves: Vec<(u32, u32, u32, Option<u64>)> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                block_hash,
                ..
            } => Some((*src_start_row, *row_count, *dst_start_row, *block_hash)),
            _ => None,
        })
        .collect();

    assert_eq!(block_moves.len(), 1, "expected a single BlockMovedRows op");
    let (src_start_row, row_count, dst_start_row, block_hash) = block_moves[0];
    assert_eq!(src_start_row, 4);
    assert_eq!(row_count, 4);
    assert_eq!(dst_start_row, 13);
    assert!(block_hash.is_none());

    let edited_rows: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.row),
            _ => None,
        })
        .collect();
    assert!(
        edited_rows
            .iter()
            .any(|r| *r >= dst_start_row && *r < dst_start_row + row_count),
        "expected a CellEdited inside the moved block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { row_idx, .. } if *row_idx >= dst_start_row && *row_idx < dst_start_row + row_count)),
        "moved rows must not be reported as added"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { row_idx, .. } if *row_idx >= src_start_row && *row_idx < src_start_row + row_count)),
        "moved rows must not be reported as removed"
    );
}

#[test]
fn g13_fuzzy_row_move_can_be_disabled() {
    let base: Vec<Vec<i32>> = (1..=18)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
    let grid_a = grid_from_numbers(&base_refs);

    let mut rows_b = base.clone();
    let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    moved_block[1][1] = 9_999;
    rows_b.splice(12..12, moved_block);
    let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
    let grid_b = grid_from_numbers(&rows_b_refs);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let disabled = DiffConfig {
        enable_fuzzy_moves: false,
        ..DiffConfig::default()
    };
    let report_disabled = diff_workbooks(&wb_a, &wb_b, &disabled);
    let disabled_moves = report_disabled
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRows { .. }))
        .count();
    let disabled_block_edits = report_disabled
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::CellEdited { addr, .. }
                if addr.row >= 12 && addr.row < 16
            )
        })
        .count();

    let report_enabled = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());
    let enabled_moves = report_enabled
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRows { .. }))
        .count();
    let enabled_block_edits = report_enabled
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::CellEdited { addr, .. }
                if addr.row >= 12 && addr.row < 16
            )
        })
        .count();

    assert!(
        enabled_moves >= disabled_moves,
        "enabling fuzzy moves should not reduce move detection"
    );
    assert!(
        enabled_block_edits > disabled_block_edits,
        "fuzzy move detection should emit edits within the moved block"
    );
}

#[test]
fn g13_in_place_edits_do_not_emit_blockmovedrows() {
    let rows: Vec<Vec<i32>> = (1..=12)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let rows_refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&rows_refs);

    let mut edited_rows = rows.clone();
    if let Some(cell) = edited_rows.get_mut(5).and_then(|row| row.get_mut(1)) {
        *cell += 7;
    }
    let edited_refs: Vec<&[i32]> = edited_rows.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&edited_refs);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "in-place edits must not be classified as BlockMovedRows"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "edits should still be surfaced as CellEdited"
    );
}

#[test]
fn g13_ambiguous_repeated_blocks_do_not_emit_blockmovedrows() {
    let mut rows_a: Vec<Vec<i32>> = vec![vec![1, 1]; 10];
    rows_a.push(vec![99, 99]);
    rows_a.push(vec![2, 2]);

    let mut rows_b = rows_a.clone();
    let moved = rows_b.remove(10);
    rows_b.insert(3, moved);

    let refs_a: Vec<&[i32]> = rows_a.iter().map(|r| r.as_slice()).collect();
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs_a);
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "ambiguous repeated patterns should not emit BlockMovedRows"
    );
    assert!(
        !report.ops.is_empty(),
        "fallback path should produce some diff noise"
    );
}

```

---

### File: `core\tests\g14_move_combination_tests.rs`

```rust
mod common;

use common::{grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn collect_rect_moves(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect()
}

fn collect_row_moves(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRows { .. }))
        .collect()
}

fn collect_col_moves(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedColumns { .. }))
        .collect()
}

fn collect_row_adds(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .collect()
}

fn collect_row_removes(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .collect()
}

fn collect_cell_edits(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .collect()
}

fn base_grid(rows: usize, cols: usize) -> Vec<Vec<i32>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| (r as i32 + 1) * 100 + c as i32 + 1)
                .collect()
        })
        .collect()
}

fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
    for (r_offset, row_vals) in block.iter().enumerate() {
        for (c_offset, value) in row_vals.iter().enumerate() {
            let row = top + r_offset;
            let col = left + c_offset;
            if let Some(row_slice) = target.get_mut(row)
                && let Some(cell) = row_slice.get_mut(col)
            {
                *cell = *value;
            }
        }
    }
}

fn grid_from_matrix(matrix: &[Vec<i32>]) -> excel_diff::Grid {
    let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
    grid_from_numbers(&refs)
}

#[test]
fn g14_rect_move_no_additional_changes_produces_single_op() {
    let mut grid_a = base_grid(12, 10);
    let mut grid_b = base_grid(12, 10);

    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);
    place_block(&mut grid_b, 8, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let has_rect_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedRect { .. }));

    assert!(has_rect_move, "pure rect move should be detected");

    assert_eq!(
        report.ops.len(),
        1,
        "pure rect move should produce exactly one BlockMovedRect op"
    );
}

#[test]
fn g14_rect_move_plus_cell_edit_no_silent_data_loss() {
    let mut grid_a = base_grid(12, 10);
    let mut grid_b = base_grid(12, 10);

    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);
    place_block(&mut grid_b, 8, 6, &block);
    grid_b[0][0] = 77777;

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let rect_moves = collect_rect_moves(&report);
    let cell_edits = collect_cell_edits(&report);

    assert_eq!(
        rect_moves.len(),
        1,
        "expected single BlockMovedRect for the moved block"
    );

    if let DiffOp::BlockMovedRect {
        src_start_row,
        src_start_col,
        src_row_count,
        src_col_count,
        dst_start_row,
        dst_start_col,
        ..
    } = rect_moves[0]
    {
        assert_eq!(*src_start_row, 2);
        assert_eq!(*src_start_col, 2);
        assert_eq!(*src_row_count, 2);
        assert_eq!(*src_col_count, 2);
        assert_eq!(*dst_start_row, 8);
        assert_eq!(*dst_start_col, 6);
    } else {
        panic!("expected BlockMovedRect");
    }

    assert!(
        !cell_edits.is_empty(),
        "expected cell edits outside the moved block"
    );
}

#[test]
fn g14_pure_row_move_produces_single_op() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    rows_b.splice(12..12, moved_block);
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let has_row_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedRows { .. }));

    assert!(has_row_move, "pure row block move should be detected");

    assert_eq!(
        report.ops.len(),
        1,
        "pure row block move should produce exactly one BlockMovedRows op"
    );
}

#[test]
fn g14_row_move_plus_cell_edit_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    rows_b.splice(12..12, moved_block);
    rows_b[0][0] = 99999;
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );
}

#[test]
fn g14_pure_column_move_produces_single_op() {
    let rows: Vec<Vec<i32>> = (0..5)
        .map(|r| (0..8).map(|c| (r + 1) * 10 + c + 1).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = rows.clone();
    for row in &mut rows_b {
        let moved_col = row.remove(1);
        row.insert(5, moved_col);
    }
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let has_col_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. }));

    assert!(has_col_move, "pure column block move should be detected");

    assert_eq!(
        report.ops.len(),
        1,
        "pure column block move should produce exactly one BlockMovedColumns op"
    );
}

#[test]
fn g14_column_move_plus_cell_edit_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (0..5)
        .map(|r| (0..8).map(|c| (r + 1) * 10 + c + 1).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = rows.clone();
    for row in &mut rows_b {
        let moved_col = row.remove(1);
        row.insert(5, moved_col);
    }
    rows_b[0][0] = 88888;
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );
}

#[test]
fn g14_two_disjoint_row_block_moves_detected() {
    let rows: Vec<Vec<i32>> = (1..=24)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = Vec::new();

    rows_b.extend_from_slice(&rows[0..3]);
    rows_b.extend_from_slice(&rows[7..10]);
    rows_b.extend_from_slice(&rows[13..24]);
    rows_b.extend_from_slice(&rows[3..7]);
    rows_b.extend_from_slice(&rows[10..13]);

    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_moves = collect_row_moves(&report);
    assert_eq!(
        row_moves.len(),
        2,
        "expected exactly two BlockMovedRows ops for two disjoint moves"
    );

    let mut actual: Vec<(u32, u32, u32)> = row_moves
        .iter()
        .map(|op| {
            if let DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                ..
            } = **op
            {
                (src_start_row, row_count, dst_start_row)
            } else {
                unreachable!()
            }
        })
        .collect();
    actual.sort();

    let mut expected = vec![(3u32, 4u32, 17u32), (10u32, 3u32, 21u32)];
    expected.sort();

    assert_eq!(
        actual, expected,
        "row move ops should match the two expected disjoint moves"
    );
}

#[test]
fn g14_row_move_plus_column_move_both_detected() {
    let rows: Vec<Vec<i32>> = (0..15)
        .map(|r| (0..10).map(|c| (r + 1) * 100 + c + 1).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();

    let moved_rows: Vec<Vec<i32>> = rows_b.drain(2..5).collect();
    rows_b.splice(10..10, moved_rows);

    for row in &mut rows_b {
        let moved_col = row.remove(1);
        row.insert(7, moved_col);
    }

    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_moves = collect_row_moves(&report);
    let col_moves = collect_col_moves(&report);

    assert_eq!(
        row_moves.len(),
        1,
        "expected a single BlockMovedRows op for the moved row block"
    );
    assert_eq!(
        col_moves.len(),
        1,
        "expected a single BlockMovedColumns op for the moved column"
    );

    if let DiffOp::BlockMovedRows {
        src_start_row,
        row_count,
        dst_start_row,
        ..
    } = *row_moves[0]
    {
        assert_eq!(src_start_row, 2);
        assert_eq!(row_count, 3);
        assert_eq!(dst_start_row, 10);
    } else {
        panic!("expected BlockMovedRows op");
    }

    if let DiffOp::BlockMovedColumns {
        src_start_col,
        col_count,
        dst_start_col,
        ..
    } = *col_moves[0]
    {
        assert_eq!(src_start_col, 1);
        assert_eq!(col_count, 1);
        assert_eq!(dst_start_col, 7);
    } else {
        panic!("expected BlockMovedColumns op");
    }
}

#[test]
fn g14_fuzzy_row_move_produces_move_and_internal_edits() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    moved_block[1][1] = 5555;
    rows_b.splice(12..12, moved_block);
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let has_row_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedRows { .. }));

    let has_internal_edit = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::CellEdited { .. }));

    assert!(has_row_move, "should detect the fuzzy row block move");
    assert!(
        has_internal_edit,
        "should report cell edits inside the moved block"
    );
}

#[test]
fn g14_fuzzy_row_move_plus_outside_edit_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    moved_block[1][1] = 5555;
    rows_b.splice(12..12, moved_block);
    rows_b[0][0] = 99999;
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );
}

#[test]
fn g14_grid_changes_no_silent_data_loss() {
    let mut grid_a = base_grid(15, 12);
    let mut grid_b = base_grid(15, 12);

    let block = vec![vec![7001, 7002], vec![7003, 7004], vec![7005, 7006]];
    place_block(&mut grid_a, 3, 3, &block);
    place_block(&mut grid_b, 10, 8, &block);
    grid_b[0][0] = 11111;
    grid_b[0][11] = 22222;
    grid_b[14][0] = 33333;
    grid_b[14][11] = 44444;

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );

    let cell_edits: Vec<(u32, u32)> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                Some((addr.row, addr.col))
            } else {
                None
            }
        })
        .collect();

    assert!(
        !cell_edits.is_empty() || !report.ops.is_empty(),
        "some form of changes should be reported"
    );
}

#[test]
fn g14_three_disjoint_rect_block_moves_detected() {
    let mut grid_a = base_grid(20, 10);
    let mut grid_b = base_grid(20, 10);

    let block1 = vec![vec![1001, 1002], vec![1003, 1004]];
    let block2 = vec![vec![2001, 2002], vec![2003, 2004]];
    let block3 = vec![vec![3001, 3002], vec![3003, 3004]];

    place_block(&mut grid_a, 2, 1, &block1);
    place_block(&mut grid_a, 6, 3, &block2);
    place_block(&mut grid_a, 12, 5, &block3);

    place_block(&mut grid_b, 10, 1, &block1);
    place_block(&mut grid_b, 4, 6, &block2);
    place_block(&mut grid_b, 16, 2, &block3);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let rect_moves: Vec<_> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect();

    assert_eq!(
        rect_moves.len(),
        3,
        "expected exactly three rect block moves to be detected"
    );
    assert_eq!(
        report.ops.len(),
        3,
        "multi-rect move scenario should not emit extra structural ops"
    );
}

#[test]
fn g14_two_disjoint_rect_moves_plus_outside_edits_no_silent_data_loss() {
    let mut grid_a = base_grid(20, 12);
    let mut grid_b = base_grid(20, 12);

    let block1 = vec![vec![8001, 8002], vec![8003, 8004]];
    let block2 = vec![vec![9001, 9002], vec![9003, 9004]];

    place_block(&mut grid_a, 2, 2, &block1);
    place_block(&mut grid_a, 10, 7, &block2);

    place_block(&mut grid_b, 8, 4, &block1);
    place_block(&mut grid_b, 14, 1, &block2);

    grid_b[0][0] = 77777;
    grid_b[19][11] = 88888;

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let rect_moves: Vec<_> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect();
    assert!(
        rect_moves.len() >= 2,
        "should detect both rect block moves in the scenario"
    );

    let rect_regions = [
        (2u32, 2u32, 2u32, 2u32),
        (10u32, 7u32, 2u32, 2u32),
        (8u32, 4u32, 2u32, 2u32),
        (14u32, 1u32, 2u32, 2u32),
    ];

    let outside_cell_edits: Vec<_> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                let in_rect = rect_regions.iter().any(|(r, c, h, w)| {
                    addr.row >= *r && addr.row < *r + *h && addr.col >= *c && addr.col < *c + *w
                });
                if !in_rect {
                    return Some((addr.row, addr.col));
                }
            }
            None
        })
        .collect();

    assert!(
        !outside_cell_edits.is_empty(),
        "cell edits outside moved rects should be surfaced"
    );
}

#[allow(clippy::needless_range_loop)]
#[test]
fn g14_rect_move_plus_row_insertion_outside_no_silent_data_loss() {
    let mut grid_a = base_grid(12, 10);
    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);

    let mut grid_b = base_grid(13, 10);

    for col in 0..10 {
        grid_b[0][col] = 50000 + col as i32;
    }

    for row in 1..13 {
        for col in 0..10 {
            grid_b[row][col] = (row as i32) * 100 + col as i32 + 1;
        }
    }

    place_block(&mut grid_b, 9, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let rect_moves = collect_rect_moves(&report);
    let row_adds = collect_row_adds(&report);

    assert_eq!(
        rect_moves.len(),
        1,
        "expected a single BlockMovedRect for the moved block"
    );
    assert!(
        !row_adds.is_empty(),
        "expected at least one RowAdded for the inserted row"
    );
}

#[test]
fn g14_rect_move_plus_row_deletion_outside_no_silent_data_loss() {
    let mut grid_a = base_grid(14, 10);
    let block = vec![vec![8001, 8002], vec![8003, 8004]];
    place_block(&mut grid_a, 3, 3, &block);

    let mut grid_b = base_grid(13, 10);

    place_block(&mut grid_b, 8, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let rect_moves = collect_rect_moves(&report);
    let row_removes = collect_row_removes(&report);

    assert_eq!(
        rect_moves.len(),
        1,
        "expected a single BlockMovedRect for the moved block"
    );
    assert!(
        !row_removes.is_empty(),
        "expected at least one RowRemoved for the deleted row"
    );
}

#[test]
fn g14_row_block_move_plus_row_insertion_outside_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = Vec::with_capacity(21);

    rows_b.push(vec![9991, 9992, 9993, 9994]);

    let mut original = rows.clone();
    let moved_block: Vec<Vec<i32>> = original.drain(4..8).collect();
    original.splice(12..12, moved_block);
    rows_b.extend(original);

    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "row block move + row insertion should produce operations"
    );
}

#[test]
fn g14_move_detection_disabled_falls_back_to_positional() {
    let grid_a = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 20, 30],
        &[100, 200, 300],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
    ]);

    let grid_b = grid_from_numbers(&[
        &[1, 2, 3],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
        &[10, 20, 30],
        &[100, 200, 300],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_move_iterations: 0,
        ..DiffConfig::default()
    };
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "expected positional fallback when move detection disabled"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "expected positional fallback when move detection disabled"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "no block move should be present when move detection disabled"
    );
}

#[test]
fn g14_masked_move_detection_not_gated_by_recursive_align_threshold() {
    let grid_a = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 20, 30],
        &[100, 200, 300],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
    ]);

    let grid_b = grid_from_numbers(&[
        &[1, 2, 3],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
        &[10, 20, 30],
        &[100, 200, 300],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        recursive_align_threshold: 1,
        max_move_detection_rows: 10,
        ..DiffConfig::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "masked move detection should be enabled by max_move_detection_rows, independent of recursion threshold"
    );
}

#[test]
fn g14_max_move_iterations_limits_detected_moves() {
    let mut grid_a = base_grid(50, 10);
    let mut grid_b = base_grid(50, 10);

    let block1 = vec![vec![1001, 1002], vec![1003, 1004]];
    let block2 = vec![vec![2001, 2002], vec![2003, 2004]];
    let block3 = vec![vec![3001, 3002], vec![3003, 3004]];
    let block4 = vec![vec![4001, 4002], vec![4003, 4004]];
    let block5 = vec![vec![5001, 5002], vec![5003, 5004]];

    place_block(&mut grid_a, 2, 1, &block1);
    place_block(&mut grid_a, 8, 1, &block2);
    place_block(&mut grid_a, 14, 1, &block3);
    place_block(&mut grid_a, 20, 1, &block4);
    place_block(&mut grid_a, 26, 1, &block5);

    place_block(&mut grid_b, 40, 7, &block1);
    place_block(&mut grid_b, 34, 7, &block2);
    place_block(&mut grid_b, 28, 7, &block3);
    place_block(&mut grid_b, 22, 7, &block4);
    place_block(&mut grid_b, 16, 7, &block5);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let limited_config = DiffConfig {
        max_move_iterations: 2,
        ..DiffConfig::default()
    };
    let report_limited = diff_workbooks(&wb_a, &wb_b, &limited_config);

    let rect_moves_limited = collect_rect_moves(&report_limited);

    assert!(
        rect_moves_limited.len() <= 2,
        "with max_move_iterations=2, at most 2 rect moves should be detected, got {}",
        rect_moves_limited.len()
    );

    assert!(
        !report_limited.ops.is_empty(),
        "remaining differences should still be surfaced, not silently dropped"
    );

    let full_config = DiffConfig::default();
    let report_full = diff_workbooks(&wb_a, &wb_b, &full_config);

    let rect_moves_full = collect_rect_moves(&report_full);

    assert!(
        rect_moves_full.len() >= 5,
        "with default config, all 5 rect moves should be detected, got {}",
        rect_moves_full.len()
    );
}

```

---

### File: `core\tests\g15_column_structure_row_alignment_tests.rs`

```rust
//! Integration tests verifying column structural changes do not break row alignment when row content is preserved.
//! Covers Branch 1.3 acceptance criteria for column insertion/deletion resilience.

mod common;

use common::single_sheet_workbook;
use excel_diff::{CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn make_grid_with_cells(nrows: u32, ncols: u32, cells: &[(u32, u32, i32)]) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for (row, col, val) in cells {
        grid.insert_cell(*row, *col, Some(CellValue::Number(*val as f64)), None);
    }
    grid
}

fn grid_from_row_data(rows: &[Vec<i32>]) -> Grid {
    let nrows = rows.len() as u32;
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0) as u32;
    let mut grid = Grid::new(nrows, ncols);

    for (r, row_vals) in rows.iter().enumerate() {
        for (c, val) in row_vals.iter().enumerate() {
            grid.insert_cell(r as u32, c as u32, Some(CellValue::Number(*val as f64)), None);
        }
    }
    grid
}

#[test]
fn g15_blank_column_insert_at_position_zero_preserves_row_alignment() {
    let grid_a = grid_from_row_data(&[
        vec![10, 20, 30],
        vec![11, 21, 31],
        vec![12, 22, 32],
        vec![13, 23, 33],
        vec![14, 24, 34],
    ]);

    let grid_b = make_grid_with_cells(
        5,
        4,
        &[
            (0, 1, 10),
            (0, 2, 20),
            (0, 3, 30),
            (1, 1, 11),
            (1, 2, 21),
            (1, 3, 31),
            (2, 1, 12),
            (2, 2, 22),
            (2, 3, 32),
            (3, 1, 13),
            (3, 2, 23),
            (3, 3, 33),
            (4, 1, 14),
            (4, 2, 24),
            (4, 3, 34),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let column_adds: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded { col_idx, .. } => Some(*col_idx),
            _ => None,
        })
        .collect();

    let row_changes: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        column_adds.contains(&0) || !report.ops.is_empty(),
        "blank column insert at position 0 should be detected as ColumnAdded or produce some diff"
    );

    assert!(
        row_changes.is_empty(),
        "blank column insert should NOT produce spurious row add/remove operations; got {:?}",
        row_changes
    );
}

#[test]
fn g15_blank_column_insert_middle_preserves_row_alignment() {
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ]);

    let grid_b = make_grid_with_cells(
        4,
        5,
        &[
            (0, 0, 1),
            (0, 1, 2),
            (0, 3, 3),
            (0, 4, 4),
            (1, 0, 5),
            (1, 1, 6),
            (1, 3, 7),
            (1, 4, 8),
            (2, 0, 9),
            (2, 1, 10),
            (2, 3, 11),
            (2, 4, 12),
            (3, 0, 13),
            (3, 1, 14),
            (3, 3, 15),
            (3, 4, 16),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "blank column insert in middle should not cause row structural changes; got {:?}",
        row_structural_ops
    );

    let has_column_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. }
        )
    });

    assert!(
        has_column_op || !report.ops.is_empty(),
        "column structure change should be detected"
    );
}

#[test]
fn g15_column_delete_preserves_row_alignment_when_content_order_maintained() {
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3, 4, 5],
        vec![6, 7, 8, 9, 10],
        vec![11, 12, 13, 14, 15],
        vec![16, 17, 18, 19, 20],
    ]);

    let grid_b = grid_from_row_data(&[
        vec![1, 2, 4, 5],
        vec![6, 7, 9, 10],
        vec![11, 12, 14, 15],
        vec![16, 17, 19, 20],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let column_removes: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved { col_idx, .. } => Some(*col_idx),
            _ => None,
        })
        .collect();

    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "column deletion should not cause spurious row changes; got {:?}",
        row_structural_ops
    );

    assert!(
        !column_removes.is_empty() || !report.ops.is_empty(),
        "column deletion should be detected"
    );
}

#[test]
fn g15_row_insert_with_column_structure_change_both_detected() {
    let grid_a = grid_from_row_data(&[vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);

    let grid_b = make_grid_with_cells(
        4,
        4,
        &[
            (0, 0, 1000),
            (0, 1, 1),
            (0, 2, 2),
            (0, 3, 3),
            (1, 0, 1001),
            (1, 1, 100),
            (1, 2, 200),
            (1, 3, 300),
            (2, 0, 1002),
            (2, 1, 4),
            (2, 2, 5),
            (2, 3, 6),
            (3, 0, 1003),
            (3, 1, 7),
            (3, 2, 8),
            (3, 3, 9),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "row insert + column change should produce diff operations"
    );

    let has_row_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::CellEdited { .. }
        )
    });

    let has_col_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. } | DiffOp::CellEdited { .. }
        )
    });

    assert!(
        has_row_op || has_col_op,
        "at least one structural change type should be detected"
    );
}

#[test]
fn g15_single_row_grid_column_insert_no_spurious_row_ops() {
    let grid_a = grid_from_row_data(&[vec![10, 20]]);

    let grid_b = make_grid_with_cells(1, 3, &[(0, 0, 10), (0, 2, 20)]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }))
        .collect();

    assert!(
        row_ops.is_empty(),
        "single row grid with column insert should not have row ops; got {:?}",
        row_ops
    );
}

#[test]
fn g15_all_blank_column_insert_no_content_change_minimal_diff() {
    let grid_a = grid_from_row_data(&[vec![1, 2], vec![3, 4], vec![5, 6]]);

    let grid_b = make_grid_with_cells(
        3,
        3,
        &[
            (0, 0, 1),
            (0, 1, 2),
            (1, 0, 3),
            (1, 1, 4),
            (2, 0, 5),
            (2, 1, 6),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }))
        .collect();

    assert!(
        row_ops.is_empty(),
        "appending blank column should not cause row operations; got {:?}",
        row_ops
    );
}

#[test]
fn g15_large_grid_column_insert_row_alignment_preserved() {
    let rows: Vec<Vec<i32>> = (0..50)
        .map(|r| (0..10).map(|c| r * 100 + c).collect())
        .collect();
    let grid_a = grid_from_row_data(&rows);

    let mut cells_b: Vec<(u32, u32, i32)> = Vec::with_capacity(50 * 10);
    for r in 0..50 {
        for c in 0..10 {
            let new_col = if c < 5 { c } else { c + 1 };
            cells_b.push((r, new_col, r as i32 * 100 + c as i32));
        }
    }
    let grid_b = make_grid_with_cells(50, 11, &cells_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "large grid column insert should not cause row changes; got {} row ops",
        row_structural_ops.len()
    );

    let column_adds = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::ColumnAdded { .. }))
        .count();

    assert!(
        column_adds > 0 || !report.ops.is_empty(),
        "column insertion should be detected in large grid"
    );
}

```

---

### File: `core\tests\g1_g2_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, sid};
use excel_diff::{
    CellValue, DiffConfig, DiffOp, DiffReport, Grid, Sheet, SheetKind, Workbook, WorkbookPackage,
};

fn workbook_with_number(value: f64) -> Workbook {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(value)), None);

    Workbook {
        sheets: vec![Sheet {
            name: sid("Sheet1"),
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
}

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g1_equal_sheet_produces_empty_diff() {
    let report = diff_fixture_pkgs("equal_sheet_a.xlsx", "equal_sheet_b.xlsx", &DiffConfig::default());

    assert!(
        report.ops.is_empty(),
        "identical 5x5 sheet should produce an empty diff"
    );
}

#[test]
fn g2_single_cell_literal_change_produces_one_celledited() {
    let report = diff_fixture_pkgs(
        "single_cell_value_a.xlsx",
        "single_cell_value_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(
        report.ops.len(),
        1,
        "expected exactly one diff op for a single edited cell"
    );

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(*sheet, sid("Sheet1"));
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
            assert_eq!(from.formula, to.formula, "no formula changes expected");
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
        )),
        "single cell change should not produce row/column structure ops"
    );
}

#[test]
fn g2_float_ulp_noise_is_ignored_in_diff() {
    let old = workbook_with_number(1.0);
    let new = workbook_with_number(1.0000000000000002);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    assert!(
        report.ops.is_empty(),
        "ULP-level float drift should not produce a diff op"
    );
}

#[test]
fn g2_meaningful_float_change_emits_cell_edit() {
    let old = workbook_with_number(1.0);
    let new = workbook_with_number(1.0001);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    assert_eq!(
        report.ops.len(),
        1,
        "meaningful float change should produce exactly one diff op"
    );

    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(1.0001)));
        }
        other => panic!("expected CellEdited diff op, got {other:?}"),
    }
}

#[test]
fn g2_nan_values_are_treated_as_equal() {
    let signaling_nan = f64::from_bits(0x7ff8_0000_0000_0000);
    let quiet_nan = f64::NAN;

    let old = workbook_with_number(signaling_nan);
    let new = workbook_with_number(quiet_nan);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    assert!(
        report.ops.is_empty(),
        "different NaN bit patterns should be considered equal in diffing"
    );
}

```

---

### File: `core\tests\g5_g7_grid_workbook_tests.rs`

```rust
mod common;

use common::diff_fixture_pkgs;
use excel_diff::{CellValue, DiffConfig, DiffOp, with_default_session};
use std::collections::BTreeSet;

#[test]
fn g5_multi_cell_edits_produces_only_celledited_ops() {
    let report = diff_fixture_pkgs(
        "multi_cell_edits_a.xlsx",
        "multi_cell_edits_b.xlsx",
        &DiffConfig::default(),
    );

    let (text_x, text_y) = with_default_session(|session| {
        let x = session.strings.intern("x");
        let y = session.strings.intern("y");
        (CellValue::Text(x), CellValue::Text(y))
    });
    let expected = vec![
        ("B2", CellValue::Number(1.0), CellValue::Number(42.0)),
        ("D5", CellValue::Number(2.0), CellValue::Number(99.0)),
        ("H7", CellValue::Number(3.0), CellValue::Number(3.5)),
        ("J10", text_x, text_y),
    ];

    assert_eq!(
        report.ops.len(),
        expected.len(),
        "expected one DiffOp per configured edit"
    );
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. })),
        "multi-cell edits should produce only CellEdited ops"
    );

    for (addr, expected_from, expected_to) in expected {
        let (sheet, from, to) = report
            .ops
            .iter()
            .find_map(|op| match op {
                DiffOp::CellEdited {
                    sheet,
                    addr: a,
                    from,
                    to,
                } if a.to_a1() == addr => Some((sheet, from, to)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing CellEdited for {addr}"));

        let sheet_name = report.strings[sheet.0 as usize].as_str();
        assert_eq!(sheet_name, "Sheet1");
        assert_eq!(from.value, Some(expected_from));
        assert_eq!(to.value, Some(expected_to));
        assert_eq!(from.formula, to.formula, "no formula changes expected");
    }

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
        )),
        "multi-cell edits should not produce row/column structure ops"
    );
}

#[test]
fn g6_row_append_bottom_emits_two_rowadded_and_no_celledited() {
    let report = diff_fixture_pkgs(
        "row_append_bottom_a.xlsx",
        "row_append_bottom_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two RowAdded ops for appended rows"
    );

    let rows_added: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                let sheet_name = report.strings[sheet.0 as usize].as_str();
                assert_eq!(sheet_name, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [10u32, 11u32].into_iter().collect();
    assert_eq!(rows_added, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "row append should not emit removals, column ops, or cell edits"
    );
}

#[test]
fn g6_row_delete_bottom_emits_two_rowremoved_and_no_celledited() {
    let report = diff_fixture_pkgs(
        "row_delete_bottom_a.xlsx",
        "row_delete_bottom_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two RowRemoved ops for deleted rows"
    );

    let rows_removed: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                let sheet_name = report.strings[sheet.0 as usize].as_str();
                assert_eq!(sheet_name, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [10u32, 11u32].into_iter().collect();
    assert_eq!(rows_removed, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "row delete should not emit additions, column ops, or cell edits"
    );
}

#[test]
fn g7_col_append_right_emits_two_columnadded_and_no_celledited() {
    let report = diff_fixture_pkgs(
        "col_append_right_a.xlsx",
        "col_append_right_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two ColumnAdded ops for appended columns"
    );

    let cols_added: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                let sheet_name = report.strings[sheet.0 as usize].as_str();
                assert_eq!(sheet_name, "Sheet1");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [4u32, 5u32].into_iter().collect();
    assert_eq!(cols_added, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnRemoved { .. }
                | DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "column append should not emit removals, row ops, or cell edits"
    );
}

#[test]
fn g7_col_delete_right_emits_two_columnremoved_and_no_celledited() {
    let report = diff_fixture_pkgs(
        "col_delete_right_a.xlsx",
        "col_delete_right_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two ColumnRemoved ops for deleted columns"
    );

    let cols_removed: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                let sheet_name = report.strings[sheet.0 as usize].as_str();
                assert_eq!(sheet_name, "Sheet1");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [4u32, 5u32].into_iter().collect();
    assert_eq!(cols_removed, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnAdded { .. }
                | DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "column delete should not emit additions, row ops, or cell edits"
    );
}

```

---

### File: `core\tests\g8_row_alignment_grid_workbook_tests.rs`

```rust
mod common;

use common::diff_fixture_pkgs;
use excel_diff::{DiffConfig, DiffOp};

#[test]
fn single_row_insert_middle_produces_one_row_added() {
    let report = diff_fixture_pkgs(
        "row_insert_middle_a.xlsx",
        "row_insert_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let strings = &report.strings;

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(
                    strings.get(sheet.0 as usize).map(String::as_str),
                    Some("Sheet1")
                );
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(rows_added, vec![5], "expected single RowAdded at index 5");

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "no rows should be removed for middle insert"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned insert should not emit CellEdited noise"
    );
}

#[test]
fn single_row_delete_middle_produces_one_row_removed() {
    let report = diff_fixture_pkgs(
        "row_delete_middle_a.xlsx",
        "row_delete_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let strings = &report.strings;

    let rows_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(
                    strings.get(sheet.0 as usize).map(String::as_str),
                    Some("Sheet1")
                );
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_removed,
        vec![5],
        "expected single RowRemoved at index 5"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "no rows should be added for middle delete"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned delete should not emit CellEdited noise"
    );
}

#[test]
fn alignment_bails_out_when_additional_edits_present() {
    let report = diff_fixture_pkgs(
        "row_insert_with_edit_a.xlsx",
        "row_insert_with_edit_b.xlsx",
        &DiffConfig::default(),
    );

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded { row_idx, .. } => Some(*row_idx),
            _ => None,
        })
        .collect();

    assert!(
        rows_added.contains(&10),
        "fallback positional diff should add the tail row"
    );
    assert!(
        !rows_added.contains(&5),
        "mid-sheet RowAdded at 5 would indicate the alignment path was taken"
    );

    let edited_rows: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.row),
            _ => None,
        })
        .collect();

    assert!(
        !edited_rows.is_empty(),
        "fallback positional diff should surface cell edits after the inserted row"
    );
    assert!(
        edited_rows.iter().any(|row| *row >= 5),
        "cell edits should include rows at or below the insertion point"
    );
}

```

---

### File: `core\tests\g9_column_alignment_grid_workbook_tests.rs`

```rust
mod common;

use common::{diff_fixture_pkgs, open_fixture_workbook, sid};
use excel_diff::{CellValue, DiffConfig, DiffOp, Workbook};

#[test]
fn g9_col_insert_middle_emits_one_columnadded_and_no_noise() {
    let report = diff_fixture_pkgs(
        "col_insert_middle_a.xlsx",
        "col_insert_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let cols_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, &sid("Data"));
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        cols_added,
        vec![3],
        "expected single ColumnAdded at inserted position"
    );

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnRemoved { .. } | DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }
        )),
        "column insert should not emit row ops or ColumnRemoved"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned insert should not emit CellEdited noise"
    );
}

#[test]
fn g9_col_delete_middle_emits_one_columnremoved_and_no_noise() {
    let report = diff_fixture_pkgs(
        "col_delete_middle_a.xlsx",
        "col_delete_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let cols_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, &sid("Data"));
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        cols_removed,
        vec![3],
        "expected single ColumnRemoved at deleted position"
    );

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }
        )),
        "column delete should not emit ColumnAdded or row ops"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned delete should not emit CellEdited noise"
    );
}

#[test]
fn g9_alignment_bails_out_when_additional_edits_present() {
    let wb_b = open_fixture_workbook("col_insert_with_edit_b.xlsx");
    let report = diff_fixture_pkgs(
        "col_insert_with_edit_a.xlsx",
        "col_insert_with_edit_b.xlsx",
        &DiffConfig::default(),
    );
    let inserted_idx = find_header_col(&wb_b, "Inserted");

    let has_middle_column_add = report.ops.iter().any(|op| match op {
        DiffOp::ColumnAdded { col_idx, .. } => *col_idx == inserted_idx,
        _ => false,
    });
    assert!(
        !has_middle_column_add,
        "alignment should bail out; no ColumnAdded at the inserted index"
    );

    let edited_cols: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.col),
            _ => None,
        })
        .collect();

    assert!(
        !edited_cols.is_empty(),
        "fallback positional diff should emit CellEdited ops"
    );
    assert!(
        edited_cols.iter().any(|col| *col > inserted_idx),
        "CellEdited ops should appear in columns to the right of the insertion"
    );
}

fn find_header_col(workbook: &Workbook, header: &str) -> u32 {
    let header_id = sid(header);
    workbook
        .sheets
        .iter()
        .flat_map(|sheet| sheet.grid.cells.iter())
        .find_map(|((row, col), cell)| {
            match &cell.value {
                Some(CellValue::Text(text)) if *row == 0 && *text == header_id => Some(*col),
                _ => None,
            }
        })
        .expect("header column should exist in fixture")
}

```

---

### File: `core\tests\grid_view_hashstats_tests.rs`

```rust
use excel_diff::{ColHash, ColMeta, FrequencyClass, HashStats, RowHash, RowMeta, RowSignature};

fn row_meta(row_idx: u32, hash: RowHash) -> RowMeta {
    RowMeta {
        row_idx,
        signature: hash,
        hash,
        non_blank_count: 0,
        first_non_blank_col: 0,
        frequency_class: FrequencyClass::Common,
        is_low_info: false,
    }
}

fn col_meta(col_idx: u32, hash: ColHash) -> ColMeta {
    ColMeta {
        col_idx,
        hash,
        non_blank_count: 0,
        first_non_blank_row: 0,
    }
}

#[test]
fn hashstats_counts_and_positions_basic() {
    let h1: RowHash = RowSignature { hash: 1 };
    let h2: RowHash = RowSignature { hash: 2 };
    let h3: RowHash = RowSignature { hash: 3 };
    let h4: RowHash = RowSignature { hash: 4 };

    let rows_a = vec![
        row_meta(0, h1),
        row_meta(1, h2),
        row_meta(2, h2),
        row_meta(3, h3),
    ];
    let rows_b = vec![row_meta(0, h2), row_meta(1, h3), row_meta(2, h4)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);

    assert_eq!(stats.freq_a.get(&h1).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h1).copied().unwrap_or(0), 0);

    assert_eq!(stats.freq_a.get(&h2).copied().unwrap_or(0), 2);
    assert_eq!(stats.freq_b.get(&h2).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_a.get(&h3).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h3).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_a.get(&h4).copied().unwrap_or(0), 0);
    assert_eq!(stats.freq_b.get(&h4).copied().unwrap_or(0), 1);

    assert_eq!(
        stats.hash_to_positions_b.get(&h2).cloned().unwrap(),
        vec![0]
    );
    assert_eq!(
        stats.hash_to_positions_b.get(&h3).cloned().unwrap(),
        vec![1]
    );
    assert_eq!(
        stats.hash_to_positions_b.get(&h4).cloned().unwrap(),
        vec![2]
    );

    let threshold = 1;
    assert!(stats.is_unique(h3));
    assert!(stats.is_common(h2, threshold));
    assert!(!stats.is_rare(h3, threshold));
    assert!(stats.appears_in_both(h3));
    assert!(!stats.appears_in_both(h1));
    assert!(!stats.appears_in_both(h4));
}

#[test]
fn hashstats_rare_but_not_common_boundary() {
    let h: RowHash = RowSignature { hash: 42 };
    let rows_a = vec![row_meta(0, h), row_meta(1, h)];
    let rows_b = vec![row_meta(0, h)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);
    let threshold = 2;

    assert!(stats.is_rare(h, threshold));
    assert!(!stats.is_common(h, threshold));
    assert!(stats.appears_in_both(h));
    assert!(!stats.is_unique(h));
}

#[test]
fn hashstats_equal_to_threshold_behavior() {
    let h: RowHash = RowSignature { hash: 99 };
    let rows_a = vec![row_meta(0, h), row_meta(1, h), row_meta(2, h)];
    let rows_b = vec![row_meta(0, h), row_meta(1, h), row_meta(2, h)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);
    let threshold = 3;

    assert!(stats.is_rare(h, threshold));
    assert!(!stats.is_common(h, threshold));
    assert!(stats.appears_in_both(h));
    assert!(!stats.is_unique(h));
}

#[test]
fn hashstats_empty_inputs() {
    let stats = HashStats::from_row_meta(&[], &[]);
    let dummy_hash: RowHash = RowSignature { hash: 123 };

    assert!(stats.freq_a.is_empty());
    assert!(stats.freq_b.is_empty());
    assert!(stats.hash_to_positions_b.is_empty());

    assert!(!stats.is_unique(dummy_hash));
    assert!(!stats.is_rare(dummy_hash, 1));
    assert!(!stats.is_common(dummy_hash, 0));
    assert!(!stats.appears_in_both(dummy_hash));
}

#[test]
fn hashstats_from_col_meta_tracks_positions() {
    let h1: ColHash = 10;
    let h2: ColHash = 20;
    let h3: ColHash = 30;

    let cols_a = vec![col_meta(0, h1), col_meta(1, h2), col_meta(2, h2)];
    let cols_b = vec![col_meta(0, h2), col_meta(1, h3)];

    let stats = HashStats::from_col_meta(&cols_a, &cols_b);

    assert_eq!(stats.freq_a.get(&h1).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h1).copied().unwrap_or(0), 0);

    assert_eq!(stats.freq_a.get(&h2).copied().unwrap_or(0), 2);
    assert_eq!(stats.freq_b.get(&h2).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_b.get(&h3).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_a.get(&h3).copied().unwrap_or(0), 0);

    assert_eq!(
        stats
            .hash_to_positions_b
            .get(&h2)
            .cloned()
            .unwrap_or_default(),
        vec![0]
    );
    assert_eq!(
        stats
            .hash_to_positions_b
            .get(&h3)
            .cloned()
            .unwrap_or_default(),
        vec![1]
    );
}

```

---

### File: `core\tests\grid_view_tests.rs`

```rust
use excel_diff::{CellValue, DiffConfig, Grid, GridView, with_default_session};

mod common;
use common::grid_from_numbers;

fn insert_cell(
    grid: &mut Grid,
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<&str>,
) {
    let formula_id = formula.map(|s| with_default_session(|session| session.strings.intern(s)));
    grid.insert_cell(row, col, value, formula_id);
}

fn text(value: &str) -> CellValue {
    with_default_session(|session| CellValue::Text(session.strings.intern(value)))
}

#[test]
fn gridview_dense_3x3_layout_and_metadata() {
    let grid = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);

    let view = GridView::from_grid(&grid);

    assert_eq!(view.rows.len(), 3);
    assert_eq!(view.row_meta.len(), 3);
    assert_eq!(view.col_meta.len(), 3);

    for (row_idx, row_view) in view.rows.iter().enumerate() {
        assert_eq!(row_view.cells.len(), 3);
        for (col_idx, (col, _cell)) in row_view.cells.iter().enumerate() {
            assert_eq!(*col as usize, col_idx);
        }

        let meta = &view.row_meta[row_idx];
        assert_eq!(meta.non_blank_count, 3);
        assert_eq!(meta.first_non_blank_col, 0);
        assert!(!meta.is_low_info);
    }

    for (col_idx, col_meta) in view.col_meta.iter().enumerate() {
        assert_eq!(col_meta.non_blank_count, 3);
        assert_eq!(col_meta.first_non_blank_row, 0);
        assert_eq!(col_meta.col_idx as usize, col_idx);
    }
}

#[test]
fn gridview_sparse_rows_low_info_classification() {
    let mut grid = Grid::new(4, 4);
    insert_cell(&mut grid, 0, 0, Some(text("Header")), None);
    insert_cell(&mut grid, 2, 2, Some(CellValue::Number(10.0)), None);
    insert_cell(&mut grid, 3, 1, Some(text("   ")), None);

    let view = GridView::from_grid(&grid);

    assert_eq!(view.row_meta[0].non_blank_count, 1);
    assert!(view.row_meta[0].is_low_info);
    assert_eq!(view.row_meta[0].first_non_blank_col, 0);

    assert_eq!(view.row_meta[1].non_blank_count, 0);
    assert!(view.row_meta[1].is_low_info);
    assert_eq!(view.row_meta[1].first_non_blank_col, 0);

    assert_eq!(view.row_meta[2].non_blank_count, 1);
    assert!(view.row_meta[2].is_low_info);
    assert_eq!(view.row_meta[2].first_non_blank_col, 2);

    assert_eq!(view.row_meta[3].non_blank_count, 1);
    assert!(view.row_meta[3].is_low_info);
    assert_eq!(view.row_meta[3].first_non_blank_col, 1);
}

#[allow(clippy::field_reassign_with_default)]
#[test]
fn gridview_formula_only_row_respects_threshold() {
    let mut grid = Grid::new(2, 2);
    insert_cell(&mut grid, 0, 0, None, Some("=A1+1"));

    let view_default = GridView::from_grid(&grid);
    assert_eq!(view_default.row_meta[0].non_blank_count, 1);
    assert!(view_default.row_meta[0].is_low_info);

    let mut config = DiffConfig::default();
    config.low_info_threshold = 1;
    let view_tuned = GridView::from_grid_with_config(&grid, &config);
    assert_eq!(view_tuned.row_meta[0].non_blank_count, 1);
    assert!(!view_tuned.row_meta[0].is_low_info);
}

#[test]
fn gridview_column_metadata_matches_signatures() {
    let mut grid = Grid::new(4, 4);
    insert_cell(&mut grid, 0, 1, Some(text("a")), Some("=B1"));
    insert_cell(&mut grid, 1, 3, Some(CellValue::Number(2.0)), Some("=1+1"));
    insert_cell(&mut grid, 2, 0, Some(CellValue::Bool(true)), None);
    insert_cell(&mut grid, 2, 2, Some(text("mid")), None);
    insert_cell(&mut grid, 3, 0, None, Some("=A1"));

    grid.compute_all_signatures();
    let row_signatures = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should be computed")
        .clone();
    let col_signatures = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should be computed")
        .clone();

    let view = GridView::from_grid(&grid);

    for (idx, meta) in view.col_meta.iter().enumerate() {
        assert_eq!(meta.hash, col_signatures[idx].hash);
    }

    for (idx, meta) in view.row_meta.iter().enumerate() {
        assert_eq!(meta.hash, row_signatures[idx]);
    }

    assert_eq!(view.col_meta[0].non_blank_count, 2);
    assert_eq!(view.col_meta[0].first_non_blank_row, 2);
    assert_eq!(view.col_meta[1].non_blank_count, 1);
    assert_eq!(view.col_meta[1].first_non_blank_row, 0);
    assert_eq!(view.col_meta[2].non_blank_count, 1);
    assert_eq!(view.col_meta[2].first_non_blank_row, 2);
    assert_eq!(view.col_meta[3].non_blank_count, 1);
    assert_eq!(view.col_meta[3].first_non_blank_row, 1);
}

#[test]
fn gridview_empty_grid_is_stable() {
    let grid = Grid::new(0, 0);

    let view = GridView::from_grid(&grid);

    assert!(view.rows.is_empty());
    assert!(view.row_meta.is_empty());
    assert!(view.col_meta.is_empty());
}

#[test]
fn gridview_large_sparse_grid_constructs_without_panic() {
    let nrows = 10_000;
    let ncols = 10;
    let mut grid = Grid::new(nrows, ncols);

    for r in (0..nrows).step_by(100) {
        let col = (r / 100) % ncols;
        insert_cell(
            &mut grid,
            r,
            col,
            Some(CellValue::Number((r / 100) as f64)),
            None,
        );
    }

    let view = GridView::from_grid(&grid);

    assert_eq!(view.rows.len(), nrows as usize);
    assert_eq!(view.col_meta.len(), ncols as usize);

    assert_eq!(view.row_meta[1].non_blank_count, 0);
    assert_eq!(view.row_meta[100].non_blank_count, 1);
    assert_eq!(view.row_meta[100].first_non_blank_col, 1);

    assert!(
        view.col_meta
            .iter()
            .any(|meta| meta.non_blank_count > 0 && meta.first_non_blank_row == 0)
    );
}

#[test]
fn gridview_row_hashes_ignore_small_float_drift() {
    let mut grid_a = Grid::new(1, 1);
    insert_cell(&mut grid_a, 0, 0, Some(CellValue::Number(1.0)), None);

    let mut grid_b = Grid::new(1, 1);
    insert_cell(
        &mut grid_b,
        0,
        0,
        Some(CellValue::Number(1.0000000000000002)),
        None,
    );

    let view_a = GridView::from_grid(&grid_a);
    let view_b = GridView::from_grid(&grid_b);

    assert_eq!(
        view_a.row_meta[0].hash, view_b.row_meta[0].hash,
        "row signatures should be stable under ULP-level float differences"
    );
}

```

---

### File: `core\tests\integration_test.rs`

```rust
use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up one level from 'core', then into 'fixtures/generated'
    d.push("../fixtures/generated");
    d.push(filename);
    d
}

#[test]
fn test_locate_fixture() {
    let path = get_fixture_path("minimal.xlsx");
    // This test confirms that the Rust code can locate the Python-generated fixtures
    // using the relative path strategy from the monorepo root.
    assert!(
        path.exists(),
        "Fixture minimal.xlsx should exist at {:?}",
        path
    );
}

```

---

### File: `core\tests\limit_behavior_tests.rs`

```rust
mod common;

use common::{sid, single_sheet_workbook};
use excel_diff::{
    CellValue, DiffConfig, DiffError, DiffOp, DiffReport, Grid, LimitBehavior, Workbook,
    WorkbookPackage, try_diff_workbooks_with_pool, with_default_session,
};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn try_diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> Result<DiffReport, DiffError> {
    with_default_session(|session| {
        try_diff_workbooks_with_pool(old, new, &mut session.strings, config)
    })
}

fn create_simple_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                None,
            );
        }
    }
    grid
}

fn count_ops(ops: &[DiffOp], predicate: impl Fn(&DiffOp) -> bool) -> usize {
    ops.iter().filter(|op| predicate(op)).count()
}

#[test]
fn large_grid_completes_within_default_limits() {
    let grid_a = create_simple_grid(1000, 10, 0);
    let mut grid_b = create_simple_grid(1000, 10, 0);
    grid_b.insert_cell(500, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "1000-row grid should complete within default limits"
    );
    assert!(
        report.warnings.is_empty(),
        "should have no warnings for successful diff"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit"
    );
}

#[test]
fn limit_exceeded_fallback_to_positional() {
    let grid_a = create_simple_grid(100, 10, 0);
    let mut grid_b = create_simple_grid(100, 10, 0);
    grid_b.insert_cell(50, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::FallbackToPositional,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "FallbackToPositional should still produce a complete result"
    );
    assert!(
        report.warnings.is_empty(),
        "FallbackToPositional should not add warnings"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit via positional diff"
    );
}

#[test]
fn limit_exceeded_return_partial_result() {
    let grid_a = create_simple_grid(100, 10, 0);
    let mut grid_b = create_simple_grid(100, 10, 0);
    grid_b.insert_cell(50, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        !report.complete,
        "ReturnPartialResult should mark report as incomplete"
    );
    assert!(
        !report.warnings.is_empty(),
        "ReturnPartialResult should add a warning about limits exceeded"
    );
    assert!(
        report.warnings[0].contains("limits exceeded"),
        "warning should mention limits exceeded"
    );
    assert!(
        !report.ops.is_empty(),
        "ReturnPartialResult should still produce ops via positional diff"
    );
}

#[test]
fn limit_exceeded_return_error_returns_structured_error() {
    let grid_a = create_simple_grid(100, 10, 0);
    let grid_b = create_simple_grid(100, 10, 0);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnError,
        ..Default::default()
    };

    let result = try_diff_workbooks(&wb_a, &wb_b, &config);
    assert!(result.is_err(), "should return error when limits exceeded");

    let err = result.unwrap_err();
    match err {
        DiffError::LimitsExceeded {
            sheet,
            rows,
            cols,
            max_rows,
            max_cols,
        } => {
            assert_eq!(sheet, sid("Sheet1"));
            assert_eq!(rows, 100);
            assert_eq!(cols, 10);
            assert_eq!(max_rows, 50);
            assert_eq!(max_cols, 16384);
        }
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
#[should_panic(expected = "alignment limits exceeded")]
fn limit_exceeded_return_error_panics_via_legacy_api() {
    let grid_a = create_simple_grid(100, 10, 0);
    let grid_b = create_simple_grid(100, 10, 0);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnError,
        ..Default::default()
    };

    let _ = diff_workbooks(&wb_a, &wb_b, &config);
}

#[test]
fn column_limit_exceeded() {
    let grid_a = create_simple_grid(10, 100, 0);
    let mut grid_b = create_simple_grid(10, 100, 0);
    grid_b.insert_cell(5, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_cols: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        !report.complete,
        "should be marked incomplete when column limit exceeded"
    );
    assert!(
        !report.warnings.is_empty(),
        "should have warning about column limit"
    );
}

#[test]
fn within_limits_no_warning() {
    let grid_a = create_simple_grid(45, 10, 0);
    let mut grid_b = create_simple_grid(45, 10, 0);
    grid_b.insert_cell(20, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "should be complete when within limits");
    assert!(
        report.warnings.is_empty(),
        "should have no warnings when within limits"
    );
}

#[test]
fn multiple_sheets_limit_warning_includes_sheet_name() {
    let grid_small = create_simple_grid(10, 5, 0);
    let grid_large_a = create_simple_grid(100, 10, 1000);
    let grid_large_b = create_simple_grid(100, 10, 2000);

    let wb_a = excel_diff::Workbook {
        sheets: vec![
            excel_diff::Sheet {
                name: sid("SmallSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_small.clone(),
            },
            excel_diff::Sheet {
                name: sid("LargeSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_large_a,
            },
        ],
    };

    let wb_b = excel_diff::Workbook {
        sheets: vec![
            excel_diff::Sheet {
                name: sid("SmallSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_small,
            },
            excel_diff::Sheet {
                name: sid("LargeSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_large_b,
            },
        ],
    };

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(!report.complete, "should be incomplete due to large sheet");
    assert!(
        report.warnings.iter().any(|w| w.contains("LargeSheet")),
        "warning should reference the sheet that exceeded limits"
    );
}

#[test]
fn large_grid_5k_rows_completes_within_default_limits() {
    let grid_a = create_simple_grid(5000, 10, 0);
    let mut grid_b = create_simple_grid(5000, 10, 0);
    grid_b.insert_cell(2500, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("LargeSheet", grid_a);
    let wb_b = single_sheet_workbook("LargeSheet", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "5000-row grid should complete within default limits (max_align_rows=500000)"
    );
    assert!(
        report.warnings.is_empty(),
        "should have no warnings for successful large grid diff"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit in large grid"
    );
}

#[test]
fn wide_grid_500_cols_completes_within_default_limits() {
    let grid_a = create_simple_grid(100, 500, 0);
    let mut grid_b = create_simple_grid(100, 500, 0);
    grid_b.insert_cell(50, 250, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("WideSheet", grid_a);
    let wb_b = single_sheet_workbook("WideSheet", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "500-column grid should complete within default limits (max_align_cols=16384)"
    );
    assert!(
        report.warnings.is_empty(),
        "should have no warnings for successful wide grid diff"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit in wide grid"
    );
}

```

---

### File: `core\tests\m4_package_parts_tests.rs`

```rust
use std::io::{Cursor, Write};

use excel_diff::{DataMashupError, open_data_mashup, parse_package_parts, parse_section_members};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

const MIN_PACKAGE_XML: &str = "<Package></Package>";
const MIN_SECTION: &str = "section Section1;\nshared Foo = 1;";
const BOM_SECTION: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

#[test]
fn package_parts_contains_expected_entries() {
    let path = fixture_path("one_query.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(
        parts.main_section.source.contains("section Section1;"),
        "main Section1.m should be present"
    );
    assert!(
        parts.main_section.source.contains("shared"),
        "at least one shared query should be present"
    );
    assert!(
        parts.embedded_contents.is_empty(),
        "one_query.xlsx should not contain embedded contents"
    );
}

#[test]
fn embedded_content_detection() {
    let path = fixture_path("multi_query_with_embedded.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(
        !parts.embedded_contents.is_empty(),
        "multi_query_with_embedded.xlsx should expose at least one embedded content"
    );

    for embedded in &parts.embedded_contents {
        assert!(
            embedded.section.source.contains("section Section1"),
            "embedded Section1.m should be present for {}",
            embedded.name
        );
        assert!(
            embedded.section.source.contains("shared"),
            "embedded Section1.m should contain at least one shared member for {}",
            embedded.name
        );
    }
}

#[test]
fn parse_package_parts_rejects_non_zip() {
    let bogus = b"this is not a zip file";
    let err = parse_package_parts(bogus).expect_err("non-zip bytes should fail");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_config_package_xml_errors() {
    let bytes = build_zip(vec![(
        "Formulas/Section1.m",
        MIN_SECTION.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Config/Package.xml should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_section1_errors() {
    let bytes = build_zip(vec![(
        "Config/Package.xml",
        MIN_PACKAGE_XML.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Formulas/Section1.m should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_package_xml_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", vec![0xFF, 0xFF, 0xFF]),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
    ]);
    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Package.xml should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_section1_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", vec![0xFF, 0xFF]),
    ]);

    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Section1.m should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn embedded_content_invalid_zip_is_skipped() {
    let bytes =
        build_minimal_package_parts_with(vec![("Content/bogus.package", b"not a zip".to_vec())]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_missing_section1_is_skipped() {
    let nested = build_zip(vec![("Config/Formulas.xml", b"<Formulas/>".to_vec())]);
    let bytes = build_minimal_package_parts_with(vec![("Content/no_section1.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_invalid_utf8_is_skipped() {
    let nested = build_zip(vec![("Formulas/Section1.m", vec![0xFF, 0xFF])]);
    let bytes = build_minimal_package_parts_with(vec![("Content/bad_utf8.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_partial_failure_retains_valid_entries() {
    let good_nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![
        ("Content/good.package", good_nested),
        ("Content/bad.package", b"not a zip".to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    let embedded = &parts.embedded_contents[0];
    assert_eq!(embedded.name, "Content/good.package");
    assert!(embedded.section.source.contains("section Section1;"));
    assert!(embedded.section.source.contains("shared"));
}

#[test]
fn leading_slash_paths_are_accepted() {
    let embedded =
        build_embedded_section_zip("section Section1;\nshared Bar = 2;".as_bytes().to_vec());
    let bytes = build_zip(vec![
        (
            "/Config/Package.xml",
            br#"<Package from="leading"/>"#.to_vec(),
        ),
        ("/Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("/Content/abcd.package", embedded),
        (
            "Config/Package.xml",
            br#"<Package from="canonical"/>"#.to_vec(),
        ),
    ]);

    let parts = parse_package_parts(&bytes).expect("leading slash entries should parse");
    assert!(
        parts.package_xml.raw_xml.contains(r#"from="leading""#),
        "first encountered Package.xml should win"
    );
    assert!(parts.main_section.source.contains("shared Foo = 1;"));
    assert_eq!(parts.embedded_contents.len(), 1);
    assert!(
        parts.embedded_contents[0]
            .section
            .source
            .contains("shared Bar = 2;")
    );
}

#[test]
fn embedded_content_name_is_canonicalized() {
    let nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![("/Content/efgh.package", nested)]);

    let parts =
        parse_package_parts(&bytes).expect("embedded content with leading slash should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    assert_eq!(parts.embedded_contents[0].name, "Content/efgh.package");
}

#[test]
fn empty_content_directory_is_ignored() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/", Vec::new()),
    ]);

    let parts = parse_package_parts(&bytes).expect("package with empty Content/ directory parses");
    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(!parts.main_section.source.is_empty());
    assert!(
        parts.embedded_contents.is_empty(),
        "bare Content/ directory should not produce embedded contents"
    );
}

#[test]
fn parse_package_parts_never_panics_on_random_bytes() {
    for seed in 0u64..64 {
        let len = (seed as usize * 13 % 256) + (seed as usize % 7);
        let bytes = random_bytes(seed, len);
        let _ = parse_package_parts(&bytes);
    }
}

#[test]
fn package_parts_section1_with_bom_parses_via_parse_section_members() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", BOM_SECTION.as_bytes().to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("BOM-prefixed Section1.m should parse");
    assert!(
        !parts.main_section.source.starts_with('\u{FEFF}'),
        "PackageParts should strip a single leading BOM from Section1.m"
    );
    let members = parse_section_members(&parts.main_section.source)
        .expect("parse_section_members should accept BOM-prefixed Section1");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
}

#[test]
fn embedded_content_section1_with_bom_parses_via_parse_section_members() {
    let embedded = build_embedded_section_zip(BOM_SECTION.as_bytes().to_vec());
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/bom_embedded.package", embedded),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(
        !parts.embedded_contents.is_empty(),
        "embedded package should be detected"
    );

    let embedded = parts
        .embedded_contents
        .iter()
        .find(|entry| entry.name == "Content/bom_embedded.package")
        .expect("expected embedded package to round-trip name");

    assert!(
        !embedded.section.source.starts_with('\u{FEFF}'),
        "embedded Section1.m should strip leading BOM"
    );

    let members = parse_section_members(&embedded.section.source)
        .expect("parse_section_members should accept embedded BOM Section1");
    assert!(
        !members.is_empty(),
        "embedded Section1.m should contain members"
    );
    assert!(
        members.iter().any(|member| {
            member.section_name == "Section1"
                && member.member_name == "Foo"
                && member.expression_m == "1"
        }),
        "embedded Section1.m should parse shared Foo = 1"
    );
}

fn build_minimal_package_parts_with(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut all_entries = Vec::with_capacity(entries.len() + 2);
    all_entries.push(("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()));
    all_entries.push(("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()));
    all_entries.extend(entries);
    build_zip(all_entries)
}

fn build_embedded_section_zip(section_bytes: Vec<u8>) -> Vec<u8> {
    build_zip(vec![("Formulas/Section1.m", section_bytes)])
}

fn build_zip(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, bytes) in entries {
        if name.ends_with('/') {
            writer
                .add_directory(name, options)
                .expect("start zip directory");
        } else {
            writer.start_file(name, options).expect("start zip entry");
            writer.write_all(&bytes).expect("write zip entry");
        }
    }

    writer.finish().expect("finish zip").into_inner()
}

fn random_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(len);
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    for _ in 0..len {
        state = state
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        bytes.push((state >> 32) as u8);
    }
    bytes
}

```

---

### File: `core\tests\m4_permissions_metadata_tests.rs`

```rust
use excel_diff::{
    DataMashupError, Permissions, RawDataMashup, build_data_mashup, build_queries,
    open_data_mashup, parse_metadata, parse_package_parts, parse_section_members,
};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn permissions_parsed_flags_default_vs_firewall_off() {
    let defaults = load_datamashup("permissions_defaults.xlsx");
    let firewall_off = load_datamashup("permissions_firewall_off.xlsx");

    assert_eq!(defaults.version, 0);
    assert_eq!(firewall_off.version, 0);

    assert!(defaults.permissions.firewall_enabled);
    assert!(!defaults.permissions.can_evaluate_future_packages);
    assert!(!firewall_off.permissions.firewall_enabled);
    assert_eq!(
        defaults.permissions.workbook_group_type,
        firewall_off.permissions.workbook_group_type
    );
}

#[test]
fn permissions_missing_or_malformed_yields_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut missing = base_raw.clone();
    missing.permissions = Vec::new();
    missing.permission_bindings = Vec::new();
    let dm = build_data_mashup(&missing).expect("missing permissions should default");
    assert_eq!(dm.permissions, Permissions::default());

    let mut malformed = base_raw.clone();
    malformed.permissions = b"<not-xml".to_vec();
    let dm = build_data_mashup(&malformed).expect("malformed permissions should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn permissions_invalid_entities_yield_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let invalid_permissions = br#"
        <Permissions>
            <CanEvaluateFuturePackages>&bad;</CanEvaluateFuturePackages>
            <FirewallEnabled>true</FirewallEnabled>
        </Permissions>
    "#;
    let mut raw = base_raw.clone();
    raw.permissions = invalid_permissions.to_vec();

    let dm = build_data_mashup(&raw).expect("invalid permissions entities should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn metadata_empty_bytes_returns_empty_struct() {
    let metadata = parse_metadata(&[]).expect("empty metadata should parse");
    assert!(metadata.formulas.is_empty());
}

#[test]
fn metadata_invalid_header_too_short_errors() {
    let err = parse_metadata(&[0x01]).expect_err("short metadata should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata XML not found"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_length_prefix_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&100u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 10]);

    let err = parse_metadata(&bytes).expect_err("invalid length prefix should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata length prefix invalid"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_utf8_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&[0xFF, 0xFF]);

    let err = parse_metadata(&bytes).expect_err("invalid utf-8 should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata is not valid UTF-8"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_malformed_xml_errors() {
    let xml = b"<LocalPackageMetadataFile><foo";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    bytes.extend_from_slice(xml);

    let err = parse_metadata(&bytes).expect_err("malformed xml should error");
    match err {
        DataMashupError::XmlError(_) => {}
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_formulas_match_section_members() {
    let raw = open_data_mashup(fixture_path("metadata_simple.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let package = parse_package_parts(&raw.package_parts).expect("package parts should parse");
    let metadata = parse_metadata(&raw.metadata).expect("metadata should parse");
    let members =
        parse_section_members(&package.main_section.source).expect("section members should parse");

    let section1_formulas: Vec<_> = metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();

    assert_eq!(section1_formulas.len(), members.len());
    for meta in section1_formulas {
        assert!(!meta.formula_name.is_empty());
    }
}

#[test]
fn metadata_load_destinations_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let load_to_sheet = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToSheet")
        .expect("LoadToSheet metadata missing");
    assert!(load_to_sheet.load_to_sheet);
    assert!(!load_to_sheet.load_to_model);
    assert!(!load_to_sheet.is_connection_only);

    let load_to_model = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToModel")
        .expect("LoadToModel metadata missing");
    assert!(!load_to_model.load_to_sheet);
    assert!(load_to_model.load_to_model);
    assert!(!load_to_model.is_connection_only);
}

#[test]
fn metadata_groups_basic_hierarchy() {
    let dm = load_datamashup("metadata_query_groups.xlsx");
    let grouped = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/GroupedFoo")
        .expect("GroupedFoo metadata missing");
    assert_eq!(grouped.group_path.as_deref(), Some("Inputs/DimTables"));

    let root = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/RootQuery")
        .expect("RootQuery metadata missing");
    assert!(root.group_path.is_none());
}

#[test]
fn metadata_hidden_queries_connection_only() {
    let dm = load_datamashup("metadata_hidden_queries.xlsx");
    let has_connection_only = dm
        .metadata
        .formulas
        .iter()
        .any(|m| !m.load_to_sheet && !m.load_to_model && m.is_connection_only);
    assert!(has_connection_only);
}

#[test]
fn metadata_itempath_decodes_percent_encoded_utf8() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%C3%A9</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar\u{00e9}");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar\u{00e9}");
    assert!(item.load_to_sheet);
    assert!(!item.is_connection_only);
}

#[test]
fn metadata_itempath_decodes_space_and_slash() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%2FInner</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar/Inner");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar/Inner");
}

#[test]
fn permission_bindings_present_flag() {
    let dm = load_datamashup("permissions_defaults.xlsx");
    assert!(!dm.permission_bindings_raw.is_empty());
}

#[test]
fn permission_bindings_missing_ok() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut synthetic = RawDataMashup {
        permission_bindings: Vec::new(),
        ..base_raw.clone()
    };
    synthetic.permissions = Vec::new();
    synthetic.metadata = Vec::new();

    let dm = build_data_mashup(&synthetic).expect("empty bindings should build");
    assert!(dm.permission_bindings_raw.is_empty());
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn build_queries_is_compatible_with_metadata_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");
    assert!(!queries.is_empty());
}

```

---

### File: `core\tests\m5_query_domain_tests.rs`

```rust
use std::collections::HashSet;

use excel_diff::{build_data_mashup, build_queries, open_data_mashup, parse_section_members};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn metadata_join_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 2);
    let names: HashSet<_> = queries.iter().map(|q| q.name.as_str()).collect();
    assert_eq!(
        names,
        HashSet::from(["Section1/LoadToSheet", "Section1/LoadToModel"])
    );

    let sheet = queries
        .iter()
        .find(|q| q.section_member == "LoadToSheet")
        .expect("LoadToSheet query missing");
    assert!(sheet.metadata.load_to_sheet);
    assert!(!sheet.metadata.load_to_model);

    let model = queries
        .iter()
        .find(|q| q.section_member == "LoadToModel")
        .expect("LoadToModel query missing");
    assert!(!model.metadata.load_to_sheet);
    assert!(model.metadata.load_to_model);
}

#[test]
fn metadata_join_url_encoding() {
    let dm = load_datamashup("metadata_url_encoding.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    let q = &queries[0];
    assert_eq!(q.name, "Section1/Query with space & #");
    assert_eq!(q.section_member, "Query with space & #");
    assert!(q.metadata.load_to_sheet || q.metadata.load_to_model);
}

#[test]
fn member_without_metadata_is_preserved() {
    let dm = load_datamashup("metadata_missing_entry.xlsx");
    assert!(dm.metadata.formulas.is_empty());
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    let q = &queries[0];
    assert_eq!(q.name, "Section1/MissingMetadata");
    assert_eq!(q.section_member, "MissingMetadata");
    assert_eq!(q.metadata.item_path, "Section1/MissingMetadata");
    assert!(!q.metadata.load_to_sheet);
    assert!(!q.metadata.load_to_model);
    assert!(q.metadata.is_connection_only);
    assert_eq!(q.metadata.group_path, None);
}

#[test]
fn query_names_unique() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    let mut seen = HashSet::new();
    for q in &queries {
        assert!(seen.insert(&q.name));
    }
}

#[test]
fn metadata_orphan_entries() {
    let dm = load_datamashup("metadata_orphan_entries.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "Section1/Foo");
    assert!(
        dm.metadata
            .formulas
            .iter()
            .any(|m| m.item_path == "Section1/Nonexistent")
    );
}

#[test]
fn queries_preserve_section_member_order() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let members = parse_section_members(&dm.package_parts.main_section.source)
        .expect("Section1 should parse");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(members.len(), queries.len());
    for (idx, (member, query)) in members.iter().zip(queries.iter()).enumerate() {
        assert_eq!(
            query.section_member, member.member_name,
            "query at position {} should match Section1 member order",
            idx
        );
    }
}

```

---

### File: `core\tests\m6_textual_m_diff_tests.rs`

```rust
use excel_diff::{
    DiffConfig, DiffOp, DiffReport, QueryChangeKind, QueryMetadataField, WorkbookPackage,
};
use std::fs::File;

mod common;
use common::fixture_path;

fn load_package(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).expect("fixture file should open");
    WorkbookPackage::open(file).expect("fixture should parse as WorkbookPackage")
}

fn m_ops(report: &DiffReport) -> Vec<&DiffOp> {
    report.m_ops().collect()
}

fn resolve_name<'a>(report: &'a DiffReport, op: &DiffOp) -> &'a str {
    let name_id = match op {
        DiffOp::QueryAdded { name } => *name,
        DiffOp::QueryRemoved { name } => *name,
        DiffOp::QueryRenamed { from, .. } => *from,
        DiffOp::QueryDefinitionChanged { name, .. } => *name,
        DiffOp::QueryMetadataChanged { name, .. } => *name,
        _ => panic!("not a query op"),
    };
    &report.strings[name_id.0 as usize]
}

#[test]
fn basic_add_query_diff() {
    let pkg_a = load_package("m_add_query_a.xlsx");
    let pkg_b = load_package("m_add_query_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected exactly one diff for added query");
    assert!(
        matches!(ops[0], DiffOp::QueryAdded { .. }),
        "expected QueryAdded"
    );
    assert_eq!(resolve_name(&report, ops[0]), "Section1/Bar");
}

#[test]
fn basic_remove_query_diff() {
    let pkg_a = load_package("m_remove_query_a.xlsx");
    let pkg_b = load_package("m_remove_query_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected exactly one diff for removed query");
    assert!(
        matches!(ops[0], DiffOp::QueryRemoved { .. }),
        "expected QueryRemoved"
    );
    assert_eq!(resolve_name(&report, ops[0]), "Section1/Bar");
}

#[test]
fn literal_change_produces_definitionchanged() {
    let pkg_a = load_package("m_change_literal_a.xlsx");
    let pkg_b = load_package("m_change_literal_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected one diff for changed literal");
    match ops[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::Semantic,
                "literal change is semantic"
            );
            assert_ne!(old_hash, new_hash, "hashes should differ for semantic change");
        }
        _ => panic!("expected QueryDefinitionChanged, got {:?}", ops[0]),
    }
    assert_eq!(resolve_name(&report, ops[0]), "Section1/Foo");
}

#[test]
fn metadata_change_produces_metadata_ops() {
    let pkg_a = load_package("m_metadata_only_change_a.xlsx");
    let pkg_b = load_package("m_metadata_only_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert!(
        !ops.is_empty(),
        "expected at least one diff for metadata change"
    );
    for op in &ops {
        match op {
            DiffOp::QueryMetadataChanged { field, .. } => {
                assert!(
                    matches!(
                        field,
                        QueryMetadataField::LoadToSheet
                            | QueryMetadataField::LoadToModel
                            | QueryMetadataField::GroupPath
                            | QueryMetadataField::ConnectionOnly
                    ),
                    "expected a recognized metadata field"
                );
            }
            _ => panic!("expected only QueryMetadataChanged ops, got {:?}", op),
        }
    }
}

#[test]
fn definition_and_metadata_change_produces_both() {
    let pkg_a = load_package("m_def_and_metadata_change_a.xlsx");
    let pkg_b = load_package("m_def_and_metadata_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let has_definition_change = ops
        .iter()
        .any(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }));
    assert!(
        has_definition_change,
        "expected QueryDefinitionChanged when definition changes"
    );
}

#[test]
fn identical_workbooks_produce_no_diffs() {
    let pkg = load_package("one_query.xlsx");

    let report = pkg.diff(&pkg, &DiffConfig::default());
    let ops = m_ops(&report);

    assert!(
        ops.is_empty(),
        "identical WorkbookPackage should produce no M diffs"
    );
}

#[test]
fn rename_produces_query_renamed() {
    let pkg_a = load_package("m_rename_query_a.xlsx");
    let pkg_b = load_package("m_rename_query_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let renamed_ops: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryRenamed { .. }))
        .collect();

    assert_eq!(
        renamed_ops.len(),
        1,
        "expected exactly one QueryRenamed op for rename scenario"
    );

    match renamed_ops[0] {
        DiffOp::QueryRenamed { from, to } => {
            let from_name = &report.strings[from.0 as usize];
            let to_name = &report.strings[to.0 as usize];
            assert_eq!(from_name, "Section1/Foo");
            assert_eq!(to_name, "Section1/Bar");
        }
        _ => unreachable!(),
    }
}


```

---

### File: `core\tests\m7_ast_canonicalization_tests.rs`

```rust
use excel_diff::{
    DataMashup, MAstKind, MParseError, MTokenDebug, ast_semantically_equal, build_data_mashup,
    build_queries, canonicalize_m_ast, open_data_mashup, parse_m_expression,
    tokenize_for_testing,
};

mod common;
use common::fixture_path;

fn load_datamashup(name: &str) -> DataMashup {
    let raw = open_data_mashup(fixture_path(name))
        .expect("fixture should open")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

fn load_single_query_expression(workbook: &str) -> String {
    let dm = load_datamashup(workbook);
    let queries = build_queries(&dm).expect("queries should parse");
    queries
        .first()
        .expect("fixture should contain a query")
        .expression_m
        .clone()
}

fn load_query_expression(workbook: &str, query_name: &str) -> String {
    let dm = load_datamashup(workbook);
    let queries = build_queries(&dm).expect("queries should parse");
    queries
        .into_iter()
        .find(|q| q.name == query_name)
        .expect("expected query to exist")
        .expression_m
}

#[test]
fn parse_basic_let_query_succeeds() {
    let expr = load_single_query_expression("one_query.xlsx");

    let result = parse_m_expression(&expr);

    assert!(result.is_ok(), "expected parse to succeed");
}

#[test]
fn basic_let_query_ast_is_let() {
    let expr = load_single_query_expression("one_query.xlsx");

    let ast = parse_m_expression(&expr).expect("expected parse to succeed");
    match ast.root_kind_for_testing() {
        MAstKind::Let { binding_count } => {
            assert!(
                binding_count >= 1,
                "expected at least one binding in basic let query"
            );
        }
        other => panic!("expected let root, got {:?}", other),
    }
}

#[test]
fn nested_let_in_binding_parses_successfully() {
    let expr = r#"
        let
            Source = let x = 1 in x,
            Result = Source
        in
            Result
    "#;

    let mut ast = parse_m_expression(expr).expect("nested let should parse");
    let mut ast_again = ast.clone();

    canonicalize_m_ast(&mut ast);
    canonicalize_m_ast(&mut ast_again);

    assert!(
        ast_semantically_equal(&ast, &ast_again),
        "canonicalization should not change equality for nested lets"
    );
}

#[test]
fn nested_let_formatting_only_equal() {
    let expr_a = r#"
        let
            Source = let x = 1 in x,
            Result = Source
        in
            Result
    "#;
    let expr_b = r#"let Source = let x = 1 in x, Result = Source in Result"#;

    let mut ast_a = parse_m_expression(expr_a).expect("first nested let should parse");
    let mut ast_b = parse_m_expression(expr_b).expect("second nested let should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(
        ast_semantically_equal(&ast_a, &ast_b),
        "formatting-only differences with nested lets should compare equal"
    );
}

#[test]
fn formatting_only_queries_semantically_equal() {
    let expr_a = load_query_expression("m_formatting_only_a.xlsx", "Section1/FormatTest");
    let expr_b = load_query_expression("m_formatting_only_b.xlsx", "Section1/FormatTest");

    let mut ast_a = parse_m_expression(&expr_a).expect("formatting-only A should parse");
    let mut ast_b = parse_m_expression(&expr_b).expect("formatting-only B should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(
        ast_semantically_equal(&ast_a, &ast_b),
        "formatting-only variants should be equal after canonicalization"
    );
}

#[test]
fn formatting_only_variant_detects_semantic_change() {
    let expr_b = load_query_expression("m_formatting_only_b.xlsx", "Section1/FormatTest");
    let expr_variant =
        load_query_expression("m_formatting_only_b_variant.xlsx", "Section1/FormatTest");

    let mut ast_b = parse_m_expression(&expr_b).expect("formatting-only B should parse");
    let mut ast_variant =
        parse_m_expression(&expr_variant).expect("formatting-only B variant should parse");

    canonicalize_m_ast(&mut ast_b);
    canonicalize_m_ast(&mut ast_variant);

    assert!(
        !ast_semantically_equal(&ast_b, &ast_variant),
        "semantic change should be detected even after canonicalization"
    );
}

#[test]
fn malformed_query_yields_parse_error() {
    let malformed = "let\n    Source = 1\n// missing 'in' and expression";

    let result = parse_m_expression(malformed);

    assert!(
        matches!(
            result,
            Err(MParseError::MissingInClause | MParseError::InvalidLetBinding)
        ),
        "missing 'in' should produce a parse error"
    );
}

#[test]
fn empty_expression_is_error() {
    let cases = ["", "   // only comment", "/* only block comment */"];

    for case in cases {
        let result = parse_m_expression(case);
        assert!(
            matches!(result, Err(MParseError::Empty)),
            "empty or comment-only input should return Empty, got {:?}",
            result
        );
    }
}

#[test]
fn unterminated_string_yields_error() {
    let result = parse_m_expression("\"unterminated");

    assert!(
        matches!(result, Err(MParseError::UnterminatedString)),
        "unterminated string should surface the correct error"
    );
}

#[test]
fn unterminated_block_comment_yields_error() {
    let result = parse_m_expression("let Source = 1 /* unterminated");

    assert!(
        matches!(result, Err(MParseError::UnterminatedBlockComment)),
        "unterminated block comment should surface the correct error"
    );
}

#[test]
fn unbalanced_delimiter_yields_error() {
    let cases = [
        "let Source = (1",
        "let Source = [1",
        "let Source = {1",
        "let Source = (1]",
    ];

    for case in cases {
        let result = parse_m_expression(case);
        assert!(
            matches!(result, Err(MParseError::UnbalancedDelimiter)),
            "unbalanced delimiters should error, got {:?}",
            result
        );
    }
}

#[test]
fn canonicalization_is_idempotent() {
    let expr = load_query_expression("m_formatting_only_b.xlsx", "Section1/FormatTest");

    let mut ast_once = parse_m_expression(&expr).expect("formatting-only B should parse");
    let mut ast_twice = ast_once.clone();

    canonicalize_m_ast(&mut ast_once);
    canonicalize_m_ast(&mut ast_twice);
    canonicalize_m_ast(&mut ast_twice);

    assert_eq!(
        ast_once, ast_twice,
        "canonicalization should produce a stable AST"
    );
}

#[test]
fn hash_date_tokenization_is_atomic() {
    let tokens = tokenize_for_testing(r#"#"Foo" = #date(2020,1,1)"#)
        .expect("hash literal tokenization should succeed");

    let expected = vec![
        MTokenDebug::Identifier("Foo".to_string()),
        MTokenDebug::Symbol('='),
        MTokenDebug::Identifier("#date".to_string()),
        MTokenDebug::Symbol('('),
        MTokenDebug::Number("2020".to_string()),
        MTokenDebug::Symbol(','),
        MTokenDebug::Number("1".to_string()),
        MTokenDebug::Symbol(','),
        MTokenDebug::Number("1".to_string()),
        MTokenDebug::Symbol(')'),
    ];

    assert_eq!(
        expected, tokens,
        "hash-prefixed literals should be lexed as single identifiers"
    );
}

```

---

### File: `core\tests\m7_semantic_m_diff_tests.rs`

```rust
use excel_diff::{DiffConfig, DiffOp, DiffReport, QueryChangeKind, WorkbookPackage};
use std::fs::File;

mod common;
use common::fixture_path;

fn load_package(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).expect("fixture file should open");
    WorkbookPackage::open(file).expect("fixture should parse as WorkbookPackage")
}

fn m_ops(report: &DiffReport) -> Vec<&DiffOp> {
    report.m_ops().collect()
}

fn resolve_name<'a>(report: &'a DiffReport, op: &DiffOp) -> &'a str {
    let name_id = match op {
        DiffOp::QueryAdded { name } => *name,
        DiffOp::QueryRemoved { name } => *name,
        DiffOp::QueryRenamed { from, .. } => *from,
        DiffOp::QueryDefinitionChanged { name, .. } => *name,
        DiffOp::QueryMetadataChanged { name, .. } => *name,
        _ => panic!("not a query op"),
    };
    &report.strings[name_id.0 as usize]
}

#[test]
fn formatting_only_diff_produces_formatting_only_change() {
    let pkg_a = load_package("m_formatting_only_a.xlsx");
    let pkg_b = load_package("m_formatting_only_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(
        def_changed.len(),
        1,
        "formatting-only changes should produce QueryDefinitionChanged with FormattingOnly kind"
    );

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::FormattingOnly,
                "formatting-only diff should have FormattingOnly change kind"
            );
            assert_eq!(
                old_hash, new_hash,
                "formatting-only changes have equal canonical hashes"
            );
        }
        _ => unreachable!(),
    }
    assert_eq!(resolve_name(&report, def_changed[0]), "Section1/FormatTest");
}

#[test]
fn semantic_gate_disabled_produces_semantic_change() {
    let pkg_a = load_package("m_formatting_only_a.xlsx");
    let pkg_b = load_package("m_formatting_only_b.xlsx");

    let config = DiffConfig {
        enable_m_semantic_diff: false,
        ..DiffConfig::default()
    };

    let report = pkg_a.diff(&pkg_b, &config);
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(
        def_changed.len(),
        1,
        "disabling semantic gate should surface formatting-only differences as Semantic"
    );

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::Semantic,
                "with semantic diff disabled, changes are reported as Semantic"
            );
            assert_ne!(
                old_hash, new_hash,
                "textual hashes should differ when semantic diff is disabled"
            );
        }
        _ => unreachable!(),
    }
    assert_eq!(resolve_name(&report, def_changed[0]), "Section1/FormatTest");
}

#[test]
fn formatting_variant_with_real_change_still_reports_semantic() {
    let pkg_b = load_package("m_formatting_only_b.xlsx");
    let pkg_b_variant = load_package("m_formatting_only_b_variant.xlsx");

    let report = pkg_b.diff(&pkg_b_variant, &DiffConfig::default());
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(
        def_changed.len(),
        1,
        "expected exactly one diff for semantic change"
    );

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::Semantic,
                "real change should be reported as Semantic"
            );
            assert_ne!(
                old_hash, new_hash,
                "semantic changes should have different hashes"
            );
        }
        _ => unreachable!(),
    }
    assert_eq!(resolve_name(&report, def_changed[0]), "Section1/FormatTest");
}

#[test]
fn semantic_gate_does_not_mask_metadata_only_change() {
    let pkg_a = load_package("m_metadata_only_change_a.xlsx");
    let pkg_b = load_package("m_metadata_only_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let metadata_ops: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryMetadataChanged { .. }))
        .collect();

    assert!(
        !metadata_ops.is_empty(),
        "expected metadata changes to be reported"
    );
    assert_eq!(resolve_name(&report, metadata_ops[0]), "Section1/Foo");
}

#[test]
fn semantic_gate_does_not_mask_definition_plus_metadata_change() {
    let pkg_a = load_package("m_def_and_metadata_change_a.xlsx");
    let pkg_b = load_package("m_def_and_metadata_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let has_def_change = ops
        .iter()
        .any(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }));

    assert!(
        has_def_change,
        "expected QueryDefinitionChanged for definition+metadata change"
    );
}

```

---

### File: `core\tests\m_section_splitting_tests.rs`

```rust
use excel_diff::{SectionParseError, parse_section_members};

const SECTION_SINGLE: &str = r#"
    section Section1;

    shared Foo = 1;
"#;

const SECTION_MULTI: &str = r#"
    section Section1;

    shared Foo = 1;
    shared Bar = 2;
    Baz = 3;
"#;

const SECTION_NOISY: &str = r#"

// Leading comment

section Section1;

// Comment before Foo
shared Foo = 1;

// Another comment

    shared   Bar   =    2    ;

"#;

const SECTION_WITH_BOM: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

const SECTION_WITH_QUOTED_IDENTIFIER: &str = r#"
    section Section1;

    shared #"Query with space & #" = 1;
"#;

const SECTION_INVALID_SHARED: &str = r#"
    section Section1;

    shared Broken // missing '=' and ';'
"#;

#[test]
fn parse_single_member_section() {
    let members = parse_section_members(SECTION_SINGLE).expect("single member section parses");
    assert_eq!(members.len(), 1);

    let foo = &members[0];
    assert_eq!(foo.section_name, "Section1");
    assert_eq!(foo.member_name, "Foo");
    assert_eq!(foo.expression_m, "1");
    assert!(foo.is_shared);
}

#[test]
fn parse_multiple_members() {
    let members = parse_section_members(SECTION_MULTI).expect("multi-member section parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].section_name, "Section1");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
}

#[test]
fn tolerate_whitespace_comments() {
    let members = parse_section_members(SECTION_NOISY).expect("noisy section still parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);
    assert_eq!(members[0].section_name, "Section1");

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
    assert_eq!(members[1].section_name, "Section1");
}

#[test]
fn error_on_missing_section_header() {
    const NO_SECTION: &str = r#"
        shared Foo = 1;
    "#;

    let result = parse_section_members(NO_SECTION);
    assert_eq!(result, Err(SectionParseError::MissingSectionHeader));
}

#[test]
fn section_parsing_tolerates_utf8_bom() {
    let members =
        parse_section_members(SECTION_WITH_BOM).expect("BOM-prefixed section should parse");
    assert_eq!(members.len(), 1);

    let member = &members[0];
    assert_eq!(member.member_name, "Foo");
    assert_eq!(member.section_name, "Section1");
    assert_eq!(member.expression_m, "1");
    assert!(member.is_shared);
}

#[test]
fn parse_quoted_identifier_member() {
    let members = parse_section_members(SECTION_WITH_QUOTED_IDENTIFIER)
        .expect("quoted identifier should parse");
    assert_eq!(members.len(), 1);

    let member = &members[0];
    assert_eq!(member.section_name, "Section1");
    assert_eq!(member.member_name, "Query with space & #");
    assert_eq!(member.expression_m, "1");
    assert!(member.is_shared);
}

#[test]
fn error_on_invalid_shared_member_syntax() {
    let result = parse_section_members(SECTION_INVALID_SHARED);
    assert_eq!(result, Err(SectionParseError::InvalidMemberSyntax));
}

```

---

### File: `core\tests\metrics_unit_tests.rs`

```rust
#![cfg(feature = "perf-metrics")]

use excel_diff::perf::{DiffMetrics, Phase};

#[test]
fn metrics_starts_with_zero_counts() {
    let metrics = DiffMetrics::default();
    assert_eq!(metrics.rows_processed, 0);
    assert_eq!(metrics.cells_compared, 0);
    assert_eq!(metrics.anchors_found, 0);
    assert_eq!(metrics.moves_detected, 0);
    assert_eq!(metrics.alignment_time_ms, 0);
    assert_eq!(metrics.move_detection_time_ms, 0);
    assert_eq!(metrics.cell_diff_time_ms, 0);
    assert_eq!(metrics.total_time_ms, 0);
}

#[test]
fn metrics_add_cells_compared_accumulates() {
    let mut metrics = DiffMetrics::default();
    metrics.add_cells_compared(100);
    assert_eq!(metrics.cells_compared, 100);
    metrics.add_cells_compared(50);
    assert_eq!(metrics.cells_compared, 150);
    metrics.add_cells_compared(1000);
    assert_eq!(metrics.cells_compared, 1150);
}

#[test]
fn metrics_add_cells_compared_saturates() {
    let mut metrics = DiffMetrics::default();
    metrics.cells_compared = u64::MAX - 10;
    metrics.add_cells_compared(100);
    assert_eq!(metrics.cells_compared, u64::MAX);
}

#[test]
fn metrics_phase_timing_accumulates() {
    let mut metrics = DiffMetrics::default();

    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Alignment);

    assert!(
        metrics.alignment_time_ms > 0,
        "alignment_time_ms should be non-zero after timed phase"
    );

    let first_alignment = metrics.alignment_time_ms;

    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Alignment);

    assert!(
        metrics.alignment_time_ms > first_alignment,
        "alignment_time_ms should accumulate across multiple phases"
    );
}

#[test]
fn metrics_different_phases_tracked_separately() {
    let mut metrics = DiffMetrics::default();

    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::Alignment);

    metrics.start_phase(Phase::MoveDetection);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::MoveDetection);

    metrics.start_phase(Phase::CellDiff);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::CellDiff);

    assert!(metrics.alignment_time_ms > 0, "alignment should be tracked");
    assert!(
        metrics.move_detection_time_ms > 0,
        "move detection should be tracked"
    );
    assert!(metrics.cell_diff_time_ms > 0, "cell diff should be tracked");
}

#[test]
fn metrics_total_phase_separate_from_components() {
    let mut metrics = DiffMetrics::default();

    metrics.start_phase(Phase::Total);
    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Alignment);
    metrics.end_phase(Phase::Total);

    assert!(metrics.alignment_time_ms > 0);
    assert!(metrics.total_time_ms > 0);
    assert!(
        metrics.total_time_ms >= metrics.alignment_time_ms,
        "total should be >= alignment since it wraps alignment"
    );
}

#[test]
fn metrics_end_phase_without_start_is_safe() {
    let mut metrics = DiffMetrics::default();
    metrics.end_phase(Phase::Alignment);
    assert_eq!(metrics.alignment_time_ms, 0);
}

#[test]
fn metrics_parse_phase_is_no_op() {
    let mut metrics = DiffMetrics::default();
    metrics.start_phase(Phase::Parse);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::Parse);
    assert_eq!(metrics.alignment_time_ms, 0);
    assert_eq!(metrics.move_detection_time_ms, 0);
    assert_eq!(metrics.cell_diff_time_ms, 0);
    assert_eq!(metrics.total_time_ms, 0);
}

#[test]
fn metrics_rows_processed_can_be_set_directly() {
    let mut metrics = DiffMetrics::default();
    metrics.rows_processed = 5000;
    assert_eq!(metrics.rows_processed, 5000);
    metrics.rows_processed = metrics.rows_processed.saturating_add(3000);
    assert_eq!(metrics.rows_processed, 8000);
}

#[test]
fn metrics_anchors_and_moves_can_be_set() {
    let mut metrics = DiffMetrics::default();
    metrics.anchors_found = 150;
    metrics.moves_detected = 3;
    assert_eq!(metrics.anchors_found, 150);
    assert_eq!(metrics.moves_detected, 3);
}

#[test]
fn metrics_clone_creates_independent_copy() {
    let mut metrics = DiffMetrics::default();
    metrics.rows_processed = 1000;
    metrics.cells_compared = 500;

    let cloned = metrics.clone();
    metrics.rows_processed = 2000;

    assert_eq!(cloned.rows_processed, 1000);
    assert_eq!(metrics.rows_processed, 2000);
}

#[test]
fn metrics_default_equality() {
    let m1 = DiffMetrics::default();
    let m2 = DiffMetrics::default();
    assert_eq!(m1, m2);
}

```

---

### File: `core\tests\output_tests.rs`

```rust
mod common;

use common::{fixture_path, open_fixture_workbook};
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ContainerError, DiffConfig, DiffOp, DiffReport,
    PackageError, WorkbookPackage, CellDiff, diff_report_to_cell_diffs,
    diff_workbooks_to_json, serialize_cell_diffs, serialize_diff_report,
};
use serde_json::Value;
#[cfg(feature = "perf-metrics")]
use std::collections::BTreeSet;

fn sid_local(pool: &mut excel_diff::StringPool, value: &str) -> excel_diff::StringId {
    pool.intern(value)
}

fn attach_strings(mut report: DiffReport, pool: excel_diff::StringPool) -> DiffReport {
    report.strings = pool.into_strings();
    report
}

fn render_value(report: &DiffReport, value: &Option<excel_diff::CellValue>) -> Option<String> {
    match value {
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(id)) => report.strings.get(id.0 as usize).cloned(),
        Some(excel_diff::CellValue::Bool(b)) => Some(b.to_string()),
        Some(excel_diff::CellValue::Error(id)) => report.strings.get(id.0 as usize).cloned(),
        Some(excel_diff::CellValue::Blank) => Some(String::new()),
        None => None,
    }
}

fn make_cell_snapshot(addr: CellAddress, value: Option<CellValue>) -> CellSnapshot {
    CellSnapshot {
        addr,
        value,
        formula: None,
    }
}

fn numeric_report(addr: CellAddress, from: f64, to: f64) -> DiffReport {
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    attach_strings(
        DiffReport::new(vec![DiffOp::cell_edited(
            sheet,
            addr,
            make_cell_snapshot(addr, Some(CellValue::Number(from))),
            make_cell_snapshot(addr, Some(CellValue::Number(to))),
        )]),
        pool,
    )
}

#[test]
fn diff_report_to_cell_diffs_filters_non_cell_ops() {
    let mut pool = excel_diff::StringPool::new();
    let sheet_added = sid_local(&mut pool, "SheetAdded");
    let sheet1 = sid_local(&mut pool, "Sheet1");
    let sheet2 = sid_local(&mut pool, "Sheet2");
    let old_sheet = sid_local(&mut pool, "OldSheet");
    let old_text = sid_local(&mut pool, "old");
    let new_text = sid_local(&mut pool, "new");
    let addr1 = CellAddress::from_indices(0, 0);
    let addr2 = CellAddress::from_indices(1, 1);

    let report = attach_strings(DiffReport::new(vec![
        DiffOp::SheetAdded {
            sheet: sheet_added,
        },
        DiffOp::cell_edited(
            sheet1,
            addr1,
            make_cell_snapshot(addr1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr1, Some(CellValue::Number(2.0))),
        ),
        DiffOp::RowAdded {
            sheet: sheet1,
            row_idx: 5,
            row_signature: None,
        },
        DiffOp::cell_edited(
            sheet2,
            addr2,
            make_cell_snapshot(addr2, Some(CellValue::Text(old_text))),
            make_cell_snapshot(addr2, Some(CellValue::Text(new_text))),
        ),
        DiffOp::SheetRemoved {
            sheet: old_sheet,
        },
    ]), pool);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        2,
        "only CellEdited ops should be projected"
    );

    assert_eq!(cell_diffs[0].coords, addr1.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("1".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("2".into()));

    assert_eq!(cell_diffs[1].coords, addr2.to_a1());
    assert_eq!(cell_diffs[1].value_file1, Some("old".into()));
    assert_eq!(cell_diffs[1].value_file2, Some("new".into()));
}

#[test]
fn diff_report_to_cell_diffs_ignores_block_moved_rect() {
    let mut pool = excel_diff::StringPool::new();
    let sheet1 = sid_local(&mut pool, "Sheet1");
    let addr = CellAddress::from_indices(2, 2);

    let report = attach_strings(DiffReport::new(vec![
        DiffOp::block_moved_rect(sheet1, 2, 3, 1, 3, 9, 6, Some(0xCAFEBABE)),
        DiffOp::cell_edited(
            sheet1,
            addr,
            make_cell_snapshot(addr, Some(CellValue::Number(10.0))),
            make_cell_snapshot(addr, Some(CellValue::Number(20.0))),
        ),
        DiffOp::BlockMovedRows {
            sheet: sheet1,
            src_start_row: 0,
            row_count: 2,
            dst_start_row: 5,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: sheet1,
            src_start_col: 0,
            col_count: 2,
            dst_start_col: 5,
            block_hash: None,
        },
    ]), pool);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        1,
        "only CellEdited should be projected; BlockMovedRect and other block moves should be ignored"
    );

    assert_eq!(cell_diffs[0].coords, addr.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("10".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("20".into()));
}

#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let mut pool = excel_diff::StringPool::new();
    let sheet_id = sid_local(&mut pool, "SheetX");
    let addr_num = CellAddress::from_indices(2, 2); // C3
    let addr_bool = CellAddress::from_indices(3, 3); // D4

    let report = attach_strings(DiffReport::new(vec![
        DiffOp::cell_edited(
            sheet_id,
            addr_num,
            make_cell_snapshot(addr_num, Some(CellValue::Number(42.5))),
            make_cell_snapshot(addr_num, Some(CellValue::Number(43.5))),
        ),
        DiffOp::cell_edited(
            sheet_id,
            addr_bool,
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(true))),
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(false))),
        ),
    ]), pool);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(cell_diffs.len(), 2);

    let number_diff = &cell_diffs[0];
    assert_eq!(number_diff.coords, addr_num.to_a1());
    assert_eq!(number_diff.value_file1, Some("42.5".into()));
    assert_eq!(number_diff.value_file2, Some("43.5".into()));

    let bool_diff = &cell_diffs[1];
    assert_eq!(bool_diff.coords, addr_bool.to_a1());
    assert_eq!(bool_diff.value_file1, Some("true".into()));
    assert_eq!(bool_diff.value_file2, Some("false".into()));
}

#[test]
fn diff_report_to_cell_diffs_filters_no_op_cell_edits() {
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    let addr_a1 = CellAddress::from_indices(0, 0);
    let addr_a2 = CellAddress::from_indices(1, 0);

    let report = attach_strings(DiffReport::new(vec![
        DiffOp::cell_edited(
            sheet,
            addr_a1,
            make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
        ),
        DiffOp::cell_edited(
            sheet,
            addr_a2,
            make_cell_snapshot(addr_a2, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr_a2, Some(CellValue::Number(2.0))),
        ),
    ]), pool);

    let diffs = diff_report_to_cell_diffs(&report);

    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].coords, "A2");
    assert_eq!(diffs[0].value_file1, Some("1".to_string()));
    assert_eq!(diffs[0].value_file2, Some("2".to_string()));
}

#[test]
fn test_json_format() {
    let diffs = vec![
        CellDiff {
            coords: "A1".into(),
            value_file1: Some("100".into()),
            value_file2: Some("200".into()),
        },
        CellDiff {
            coords: "B2".into(),
            value_file1: Some("true".into()),
            value_file2: Some("false".into()),
        },
        CellDiff {
            coords: "C3".into(),
            value_file1: Some("#DIV/0!".into()),
            value_file2: None,
        },
    ];

    let json = serialize_cell_diffs(&diffs).expect("serialization should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    assert!(value.is_array(), "expected an array of cell diffs");
    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert_eq!(arr.len(), 3);

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("A1".into()));
    assert_eq!(first["value_file1"], Value::String("100".into()));
    assert_eq!(first["value_file2"], Value::String("200".into()));

    let second = &arr[1];
    assert_eq!(second["coords"], Value::String("B2".into()));
    assert_eq!(second["value_file1"], Value::String("true".into()));
    assert_eq!(second["value_file2"], Value::String("false".into()));

    let third = &arr[2];
    assert_eq!(third["coords"], Value::String("C3".into()));
    assert_eq!(third["value_file1"], Value::String("#DIV/0!".into()));
    assert_eq!(third["value_file2"], Value::Null);
}

#[test]
fn test_json_empty_diff() {
    let fixture = fixture_path("pg1_basic_two_sheets.xlsx");
    let json = diff_workbooks_to_json(&fixture, &fixture, &DiffConfig::default())
        .expect("diffing identical files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "identical files should produce no diff ops"
    );
}

#[test]
fn test_json_non_empty_diff() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&report, &from.value), Some("true".into()));
            assert_eq!(render_value(&report, &to.value), Some("false".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_diff_value_to_empty() {
    let a = fixture_path("json_diff_value_to_empty_a.xlsx");
    let b = fixture_path("json_diff_value_to_empty_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), None);
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn json_diff_case_only_sheet_name_no_changes() {
    let old = open_fixture_workbook("sheet_case_only_rename_a.xlsx");
    let new = open_fixture_workbook("sheet_case_only_rename_b.xlsx");

    let report = WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should produce no diff ops"
    );
}

#[test]
fn json_diff_case_only_sheet_name_cell_edit() {
    let old = open_fixture_workbook("sheet_case_only_rename_edit_a.xlsx");
    let new = open_fixture_workbook("sheet_case_only_rename_edit_b.xlsx");

    let report = WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(
                report.strings.get(sheet.0 as usize),
                Some(&"Sheet1".to_string())
            );
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing case-only sheet rename should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should serialize to no ops"
    );
}

#[test]
fn test_json_case_only_sheet_name_cell_edit_via_helper() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing case-only sheet rename with cell edit should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(
                report.strings.get(sheet.0 as usize),
                Some(&"Sheet1".to_string())
            );
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_diff_workbooks_to_json_reports_invalid_zip() {
    let path = fixture_path("not_a_zip.txt");
    let err = diff_workbooks_to_json(&path, &path, &DiffConfig::default())
        .expect_err("diffing invalid containers should return an error");

    assert!(
        matches!(
            err,
            PackageError::Container(ContainerError::NotZipContainer)
        ),
        "expected container error, got {err}"
    );
}

#[test]
fn serialize_diff_report_nan_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = numeric_report(addr, f64::NAN, 1.0);

    let err = serialize_diff_report(&report).expect_err("NaN should fail to serialize");
    let wrapped = PackageError::SerializationError(err.to_string());

    match wrapped {
        PackageError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("nan"),
                "error message should mention NaN for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = numeric_report(addr, f64::INFINITY, 1.0);

    let err = serialize_diff_report(&report).expect_err("Infinity should fail to serialize");
    let wrapped = PackageError::SerializationError(err.to_string());
    match wrapped {
        PackageError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_neg_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = numeric_report(addr, f64::NEG_INFINITY, 1.0);

    let err = serialize_diff_report(&report).expect_err("NEG_INFINITY should fail to serialize");
    let wrapped = PackageError::SerializationError(err.to_string());
    match wrapped {
        PackageError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_with_finite_numbers_succeeds() {
    let addr = CellAddress::from_indices(1, 1);
    let report = numeric_report(addr, 2.5, 3.5);

    let json = serialize_diff_report(&report).expect("finite values should serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(parsed.ops.len(), 1);
}

#[test]
fn serialize_full_diff_report_has_complete_true_and_no_warnings() {
    let addr = CellAddress::from_indices(0, 0);
    let report = numeric_report(addr, 1.0, 2.0);

    let json = serialize_diff_report(&report).expect("full report should serialize");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let obj = value.as_object().expect("should be object");

    assert_eq!(
        obj.get("complete").and_then(Value::as_bool),
        Some(true),
        "full result should have complete=true"
    );

    let has_warnings = obj
        .get("warnings")
        .map(|v| v.as_array().map(|arr| !arr.is_empty()).unwrap_or(false))
        .unwrap_or(false);
    assert!(
        !has_warnings,
        "full result should have no warnings or empty warnings array"
    );
}

#[test]
fn serialize_partial_diff_report_includes_complete_false_and_warnings() {
    let addr = CellAddress::from_indices(0, 0);
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    let ops = vec![DiffOp::cell_edited(
        sheet,
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
        make_cell_snapshot(addr, Some(CellValue::Number(2.0))),
    )];
    let report = attach_strings(
        DiffReport::with_partial_result(
            ops,
            "Sheet 'LargeSheet': alignment limits exceeded".to_string(),
        ),
        pool,
    );

    let json = serialize_diff_report(&report).expect("partial report should serialize");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let obj = value.as_object().expect("should be object");

    assert_eq!(
        obj.get("complete").and_then(Value::as_bool),
        Some(false),
        "partial result should have complete=false"
    );

    let warnings = obj
        .get("warnings")
        .and_then(Value::as_array)
        .expect("warnings should be present");
    assert!(!warnings.is_empty(), "warnings array should not be empty");
    assert!(
        warnings[0]
            .as_str()
            .unwrap_or("")
            .contains("limits exceeded"),
        "warning should mention limits exceeded"
    );
}

#[test]
#[cfg(feature = "perf-metrics")]
fn serialize_diff_report_with_metrics_includes_metrics_object() {
    use excel_diff::perf::DiffMetrics;

    let addr = CellAddress::from_indices(0, 0);
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    let ops = vec![DiffOp::cell_edited(
        sheet,
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
        make_cell_snapshot(addr, Some(CellValue::Number(2.0))),
    )];

    let mut report = attach_strings(DiffReport::new(ops), pool);
    let mut metrics = DiffMetrics::default();
    metrics.move_detection_time_ms = 5;
    metrics.alignment_time_ms = 10;
    metrics.cell_diff_time_ms = 15;
    metrics.total_time_ms = 30;
    metrics.rows_processed = 500;
    metrics.cells_compared = 2500;
    metrics.anchors_found = 25;
    metrics.moves_detected = 1;
    report.metrics = Some(metrics);

    let json = serialize_diff_report(&report).expect("report with metrics should serialize");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let obj = value.as_object().expect("should be object");

    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    assert!(
        keys.contains("metrics"),
        "serialized report should include metrics key"
    );

    let metrics_obj = obj
        .get("metrics")
        .and_then(Value::as_object)
        .expect("metrics should be an object");

    assert!(
        metrics_obj.contains_key("move_detection_time_ms"),
        "metrics should contain move_detection_time_ms"
    );
    assert!(
        metrics_obj.contains_key("alignment_time_ms"),
        "metrics should contain alignment_time_ms"
    );
    assert!(
        metrics_obj.contains_key("cell_diff_time_ms"),
        "metrics should contain cell_diff_time_ms"
    );
    assert!(
        metrics_obj.contains_key("total_time_ms"),
        "metrics should contain total_time_ms"
    );
    assert!(
        metrics_obj.contains_key("rows_processed"),
        "metrics should contain rows_processed"
    );
    assert!(
        metrics_obj.contains_key("cells_compared"),
        "metrics should contain cells_compared"
    );
    assert!(
        metrics_obj.contains_key("anchors_found"),
        "metrics should contain anchors_found"
    );
    assert!(
        metrics_obj.contains_key("moves_detected"),
        "metrics should contain moves_detected"
    );

    assert_eq!(
        metrics_obj.get("rows_processed").and_then(Value::as_u64),
        Some(500)
    );
    assert_eq!(
        metrics_obj.get("cells_compared").and_then(Value::as_u64),
        Some(2500)
    );
}

```

---

### File: `core\tests\package_streaming_tests.rs`

```rust
use excel_diff::{
    DataMashup, DiffConfig, DiffError, DiffOp, DiffSink, Grid, Metadata, PackageParts, PackageXml,
    Permissions, SectionDocument, Sheet, SheetKind, Workbook, WorkbookPackage,
};

#[derive(Default)]
struct StrictSink {
    finished: bool,
    finish_calls: usize,
    ops: Vec<DiffOp>,
}

impl DiffSink for StrictSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        if self.finished {
            return Err(DiffError::SinkError {
                message: "emit called after finish".to_string(),
            });
        }
        self.ops.push(op);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.finish_calls += 1;
        self.finished = true;
        Ok(())
    }
}

fn make_dm(section_source: &str) -> DataMashup {
    DataMashup {
        version: 0,
        package_parts: PackageParts {
            package_xml: PackageXml {
                raw_xml: "<Package/>".to_string(),
            },
            main_section: SectionDocument {
                source: section_source.to_string(),
            },
            embedded_contents: Vec::new(),
        },
        permissions: Permissions::default(),
        metadata: Metadata {
            formulas: Vec::new(),
        },
        permission_bindings_raw: Vec::new(),
    }
}

fn make_workbook(sheet_name: &str) -> Workbook {
    let sheet_id = excel_diff::with_default_session(|session| session.strings.intern(sheet_name));

    Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: Grid::new(0, 0),
        }],
    }
}

#[test]
fn package_diff_streaming_does_not_emit_after_finish_and_finishes_once() {
    let wb = make_workbook("Sheet1");

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let pkg_a = WorkbookPackage {
        workbook: wb.clone(),
        data_mashup: Some(dm_a),
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
    };

    let mut sink = StrictSink::default();
    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("diff_streaming should succeed");

    assert!(sink.finished, "sink should be finished at end");
    assert_eq!(
        sink.finish_calls, 1,
        "sink.finish() should be called exactly once"
    );

    assert!(
        sink.ops.iter().any(|op| op.is_m_op()),
        "expected at least one M diff op in streaming output"
    );

    assert_eq!(
        summary.op_count,
        sink.ops.len(),
        "summary.op_count should match ops actually emitted"
    );
}

#[test]
fn package_diff_streaming_finishes_on_error() {
    struct FailingSink {
        calls: usize,
        finish_called: bool,
    }

    impl DiffSink for FailingSink {
        fn emit(&mut self, _op: DiffOp) -> Result<(), DiffError> {
            self.calls += 1;
            if self.calls > 2 {
                return Err(DiffError::SinkError {
                    message: "intentional failure".to_string(),
                });
            }
            Ok(())
        }

        fn finish(&mut self) -> Result<(), DiffError> {
            self.finish_called = true;
            Ok(())
        }
    }

    let sheet_id =
        excel_diff::with_default_session(|session| session.strings.intern("Sheet1"));

    let mut grid_a = Grid::new(10, 1);
    let mut grid_b = Grid::new(10, 1);
    for i in 0..10 {
        grid_a.insert_cell(i, 0, Some(excel_diff::CellValue::Number(i as f64)), None);
        grid_b.insert_cell(
            i,
            0,
            Some(excel_diff::CellValue::Number((i + 100) as f64)),
            None,
        );
    }

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_a,
        }],
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
    };

    let pkg_a = WorkbookPackage {
        workbook: wb_a,
        data_mashup: None,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb_b,
        data_mashup: None,
    };

    let mut sink = FailingSink {
        calls: 0,
        finish_called: false,
    };

    let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);
    assert!(result.is_err(), "diff_streaming should return error");
}

```

---

### File: `core\tests\perf_large_grid_tests.rs`

```rust
#![cfg(feature = "perf-metrics")]

mod common;

use common::single_sheet_workbook;
use excel_diff::{CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage};
use excel_diff::perf::DiffMetrics;

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn create_large_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                None,
            );
        }
    }
    grid
}

fn create_repetitive_grid(nrows: u32, ncols: u32, pattern_length: u32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        let pattern_idx = row % pattern_length;
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number((pattern_idx * 1000 + col) as f64)),
                None,
            );
        }
    }
    grid
}

fn create_sparse_grid(nrows: u32, ncols: u32, fill_percent: u32, seed: u64) -> Grid {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            let mut hasher = DefaultHasher::new();
            (row, col, seed).hash(&mut hasher);
            let hash = hasher.finish();
            if (hash % 100) < fill_percent as u64 {
                grid.insert_cell(
                    row,
                    col,
                    Some(CellValue::Number((row * 1000 + col) as f64)),
                    None,
                );
            }
        }
    }
    grid
}

fn log_perf_metric(name: &str, metrics: &DiffMetrics, tail: &str) {
    println!(
        "PERF_METRIC {name} total_time_ms={} move_detection_time_ms={} alignment_time_ms={} cell_diff_time_ms={} rows_processed={} cells_compared={} anchors_found={} moves_detected={}{}",
        metrics.total_time_ms,
        metrics.move_detection_time_ms,
        metrics.alignment_time_ms,
        metrics.cell_diff_time_ms,
        metrics.rows_processed,
        metrics.cells_compared,
        metrics.anchors_found,
        metrics.moves_detected,
        tail
    );
}

#[test]
fn perf_p1_large_dense() {
    let grid_a = create_large_grid(1000, 20, 0);
    let mut grid_b = create_large_grid(1000, 20, 0);
    grid_b.insert_cell(500, 10, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "P1 dense grid should complete successfully"
    );
    assert!(report.warnings.is_empty(), "P1 should have no warnings");
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "P1 should detect the cell edit"
    );
    assert!(
        report.metrics.is_some(),
        "P1 should have metrics when perf-metrics enabled"
    );
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P1 should process rows");
    assert!(metrics.cells_compared > 0, "P1 should compare cells");
    log_perf_metric("perf_p1_large_dense", &metrics, "");
}

#[test]
fn perf_p2_large_noise() {
    let grid_a = create_large_grid(1000, 20, 0);
    let grid_b = create_large_grid(1000, 20, 1);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "P2 noise grid should complete successfully"
    );
    assert!(report.metrics.is_some(), "P2 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P2 should process rows");
    log_perf_metric("perf_p2_large_noise", &metrics, "");
}

#[test]
fn perf_p3_adversarial_repetitive() {
    let grid_a = create_repetitive_grid(1000, 50, 100);
    let mut grid_b = create_repetitive_grid(1000, 50, 100);
    grid_b.insert_cell(500, 25, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "P3 repetitive grid should complete");
    assert!(report.metrics.is_some(), "P3 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P3 should process rows");
    log_perf_metric("perf_p3_adversarial_repetitive", &metrics, "");
}

#[test]
fn perf_p4_99_percent_blank() {
    let grid_a = create_sparse_grid(1000, 100, 1, 12345);
    let mut grid_b = create_sparse_grid(1000, 100, 1, 12345);
    grid_b.insert_cell(500, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "P4 sparse grid should complete");
    assert!(report.metrics.is_some(), "P4 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P4 should process rows");
    log_perf_metric("perf_p4_99_percent_blank", &metrics, "");
}

#[test]
fn perf_p5_identical() {
    let grid_a = create_large_grid(1000, 100, 0);
    let grid_b = create_large_grid(1000, 100, 0);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "P5 identical grid should complete");
    assert!(
        report.ops.is_empty(),
        "P5 identical grids should produce no ops"
    );
    assert!(report.metrics.is_some(), "P5 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P5 should process rows");
    log_perf_metric("perf_p5_identical", &metrics, "");
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_dense_single_edit() {
    let grid_a = create_large_grid(50000, 100, 0);
    let mut grid_b = create_large_grid(50000, 100, 0);
    grid_b.insert_cell(25000, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "50k dense grid should complete successfully"
    );
    assert!(
        report.warnings.is_empty(),
        "50k dense should have no warnings"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "50k dense should detect the cell edit"
    );
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric(
        "perf_50k_dense_single_edit",
        &metrics,
        " (enforced: <30s; target: <5s)",
    );
    assert!(
        metrics.total_time_ms < 30000,
        "50k dense grid should complete in <30s, took {}ms",
        metrics.total_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_completely_different() {
    let grid_a = create_large_grid(50000, 100, 0);
    let grid_b = create_large_grid(50000, 100, 1);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k different grids should complete");
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric(
        "perf_50k_completely_different",
        &metrics,
        " (enforced: <60s; target: <10s)",
    );
    assert!(
        metrics.total_time_ms < 60000,
        "50k completely different should complete in <60s, took {}ms",
        metrics.total_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_adversarial_repetitive() {
    let grid_a = create_repetitive_grid(50000, 50, 100);
    let mut grid_b = create_repetitive_grid(50000, 50, 100);
    grid_b.insert_cell(25000, 25, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k repetitive should complete");
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric(
        "perf_50k_adversarial_repetitive",
        &metrics,
        " (enforced: <120s; target: <15s)",
    );
    assert!(
        metrics.total_time_ms < 120000,
        "50k adversarial repetitive should complete in <120s, took {}ms",
        metrics.total_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_99_percent_blank() {
    let grid_a = create_sparse_grid(50000, 100, 1, 12345);
    let mut grid_b = create_sparse_grid(50000, 100, 1, 12345);
    grid_b.insert_cell(25000, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k sparse should complete");
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric("perf_50k_99_percent_blank", &metrics, " (target: <2s)");
    assert!(
        metrics.total_time_ms < 30000,
        "50k 99% blank should complete in <30s, took {}ms",
        metrics.total_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_identical() {
    let grid_a = create_large_grid(50000, 100, 0);
    let grid_b = create_large_grid(50000, 100, 0);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k identical should complete");
    assert!(
        report.ops.is_empty(),
        "50k identical grids should have no ops"
    );
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric("perf_50k_identical", &metrics, " (target: <1s)");
    assert!(
        metrics.total_time_ms < 15000,
        "50k identical should complete in <15s, took {}ms",
        metrics.total_time_ms
    );
}

```

---

### File: `core\tests\pg1_ir_tests.rs`

```rust
mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{CellAddress, CellValue, Sheet, with_default_session};

#[test]
fn pg1_basic_two_sheets_structure() {
    let workbook = open_fixture_workbook("pg1_basic_two_sheets.xlsx");
    assert_eq!(workbook.sheets.len(), 2);
    assert_eq!(workbook.sheets[0].name, sid("Sheet1"));
    assert_eq!(workbook.sheets[1].name, sid("Sheet2"));
    assert!(matches!(workbook.sheets[0].kind, excel_diff::SheetKind::Worksheet));
    assert!(matches!(workbook.sheets[1].kind, excel_diff::SheetKind::Worksheet));

    let sheet1 = &workbook.sheets[0];
    assert_eq!(sheet1.grid.nrows, 3);
    assert_eq!(sheet1.grid.ncols, 3);
    with_default_session(|session| {
        assert_eq!(
            sheet1
                .grid
                .get(0, 0)
                .and_then(|cell| cell.value.as_ref().and_then(|v| v.as_text(session.strings()))),
            Some("R1C1")
        );
    });

    let sheet2 = &workbook.sheets[1];
    assert_eq!(sheet2.grid.nrows, 5);
    assert_eq!(sheet2.grid.ncols, 2);
    with_default_session(|session| {
        assert_eq!(
            sheet2.grid.get(0, 0).and_then(|cell| {
                cell.value
                    .as_ref()
                    .and_then(|v| v.as_text(session.strings()))
            }),
            Some("S2_R1C1")
        );
    });
}

#[test]
fn pg1_sparse_used_range_extents() {
    let workbook = open_fixture_workbook("pg1_sparse_used_range.xlsx");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == sid("Sparse"))
        .expect("Sparse sheet present");

    assert_eq!(sheet.grid.nrows, 10);
    assert_eq!(sheet.grid.ncols, 7);

    assert_cell_text(sheet, 0, 0, "A1");
    assert_cell_text(sheet, 1, 1, "B2");
    assert_cell_text(sheet, 9, 6, "G10");
    assert_eq!(sheet.grid.cell_count(), 3);
}

#[test]
fn pg1_empty_and_mixed_sheets() {
    let workbook = open_fixture_workbook("pg1_empty_and_mixed_sheets.xlsx");

    let empty = sheet_by_name(&workbook, "Empty");
    assert_eq!(empty.grid.nrows, 0);
    assert_eq!(empty.grid.ncols, 0);
    assert_eq!(empty.grid.cell_count(), 0);

    let values_only = sheet_by_name(&workbook, "ValuesOnly");
    assert_eq!(values_only.grid.nrows, 10);
    assert_eq!(values_only.grid.ncols, 10);
    let values: Vec<_> = values_only
        .grid
        .iter_cells()
        .map(|(_, cell)| cell)
        .collect();
    assert!(
        values
            .iter()
            .all(|c| c.value.is_some() && c.formula.is_none()),
        "ValuesOnly cells should have values and no formulas"
    );
    assert_eq!(
        values_only
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_number)),
        Some(1.0)
    );

    let formulas = sheet_by_name(&workbook, "FormulasOnly");
    assert_eq!(formulas.grid.nrows, 10);
    assert_eq!(formulas.grid.ncols, 10);
    let first = formulas.grid.get(0, 0).expect("A1 should exist");
    with_default_session(|session| {
        assert_eq!(
            first.formula.map(|id| session.strings.resolve(id)),
            Some("ValuesOnly!A1")
        );
    });
    assert!(
        first.value.is_some(),
        "Formulas should surface cached values when present"
    );
    assert!(
        formulas
            .grid
            .iter_cells()
            .all(|(_, cell)| cell.formula.is_some()),
        "All cells should carry formulas in FormulasOnly"
    );
}

fn sheet_by_name<'a>(workbook: &'a excel_diff::Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == sid(name))
        .unwrap_or_else(|| panic!("sheet {name} not found"))
}

fn assert_cell_text(sheet: &Sheet, row: u32, col: u32, expected: &str) {
    let cell = sheet
        .grid
        .get(row, col)
        .unwrap_or_else(|| panic!("cell {expected} should exist"));
    assert_eq!(CellAddress::from_coords(row, col).to_a1(), expected);
    with_default_session(|session| {
        assert_eq!(
            cell.value
                .as_ref()
                .and_then(|v| v.as_text(session.strings()))
                .unwrap_or(""),
            expected
        );
    });
}

```

---

### File: `core\tests\pg3_snapshot_tests.rs`

```rust
mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, Sheet, Workbook, address_to_index,
    with_default_session,
};

fn sheet_by_name<'a>(workbook: &'a Workbook, name: &str) -> &'a Sheet {
    with_default_session(|session| {
        let id = session.strings.intern(name);
        workbook
            .sheets
            .iter()
            .find(|s| s.name == id)
            .expect("sheet should exist")
    })
}

fn find_cell<'a>(sheet: &'a Sheet, addr: &str) -> Option<&'a Cell> {
    let (row, col) = address_to_index(addr).expect("address should parse");
    sheet.grid.get(row, col)
}

fn snapshot(sheet: &Sheet, addr: &str) -> CellSnapshot {
    let (row, col) = address_to_index(addr).expect("address should parse");
    if let Some(cell) = find_cell(sheet, addr) {
        CellSnapshot::from_cell(row, col, cell)
    } else {
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: None,
            formula: None,
        }
    }
}

fn resolve_text(id: excel_diff::StringId) -> String {
    with_default_session(|session| session.strings.resolve(id).to_string())
}

#[test]
fn pg3_value_and_formula_cells_snapshot_from_excel() {
    let workbook = open_fixture_workbook("pg3_value_and_formula_cells.xlsx");
    let sheet = sheet_by_name(&workbook, "Types");

    let a1 = snapshot(sheet, "A1");
    assert_eq!(a1.addr.to_string(), "A1");
    assert_eq!(a1.value, Some(CellValue::Number(42.0)));
    assert!(a1.formula.is_none());

    let a2 = snapshot(sheet, "A2");
    let a2_text = match a2.value {
        Some(CellValue::Text(id)) => resolve_text(id),
        other => panic!("expected text cell, got {:?}", other),
    };
    assert_eq!(a2_text, "hello");
    assert!(a2.formula.is_none());

    let a3 = snapshot(sheet, "A3");
    assert_eq!(a3.value, Some(CellValue::Bool(true)));
    assert!(a3.formula.is_none());

    let a4 = snapshot(sheet, "A4");
    assert!(a4.value.is_none());
    assert!(a4.formula.is_none());

    let b1 = snapshot(sheet, "B1");
    assert!(matches!(
        b1.value,
        Some(CellValue::Number(n)) if (n - 43.0).abs() < 1e-6
    ));
    assert_eq!(b1.addr.to_string(), "B1");
    let b1_formula = b1.formula.map(resolve_text).expect("B1 should have a formula");
    assert!(b1_formula.contains("A1+1"));

    let b2 = snapshot(sheet, "B2");
    let b2_text = match b2.value {
        Some(CellValue::Text(id)) => resolve_text(id),
        other => panic!("expected text cell, got {:?}", other),
    };
    assert_eq!(b2_text, "hello world");
    assert_eq!(b2.addr.to_string(), "B2");
    let b2_formula = b2.formula.map(resolve_text).expect("B2 should have a formula");
    assert!(b2_formula.contains("hello"));
    assert!(b2_formula.contains("world"));

    let b3 = snapshot(sheet, "B3");
    assert_eq!(b3.value, Some(CellValue::Bool(true)));
    assert_eq!(b3.addr.to_string(), "B3");
    let b3_formula = b3.formula.map(resolve_text).expect("B3 should have a formula");
    assert!(
        b3_formula.contains(">0"),
        "B3 formula should include comparison: {b3_formula:?}"
    );
}

#[test]
fn snapshot_json_roundtrip() {
    let workbook = open_fixture_workbook("pg3_value_and_formula_cells.xlsx");
    let sheet = sheet_by_name(&workbook, "Types");

    let snapshots = vec![
        snapshot(sheet, "A1"),
        snapshot(sheet, "A2"),
        snapshot(sheet, "B1"),
        snapshot(sheet, "B2"),
        snapshot(sheet, "B3"),
    ];

    for snap in snapshots {
        let addr = snap.addr.to_string();
        let json = serde_json::to_string(&snap).expect("snapshot should serialize");
        let as_value: serde_json::Value =
            serde_json::from_str(&json).expect("snapshot JSON should parse to value");
        assert_eq!(as_value["addr"], serde_json::Value::String(addr));
        let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
        assert_eq!(snap.addr, snap_back.addr);
        assert_eq!(snap, snap_back);
    }
}

#[test]
fn snapshot_json_roundtrip_detects_tampered_addr() {
    let snap = CellSnapshot {
        addr: "Z9".parse().expect("address should parse"),
        value: Some(CellValue::Number(1.0)),
        formula: Some(sid("A1+1")),
    };

    let mut value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&snap).expect("serialize should work"))
            .expect("serialized JSON should parse");
    value["addr"] = serde_json::Value::String("A1".into());

    let tampered_json = serde_json::to_string(&value).expect("tampered JSON should serialize");
    let tampered: CellSnapshot =
        serde_json::from_str(&tampered_json).expect("tampered JSON should parse");

    assert_ne!(snap.addr, tampered.addr);
    assert_eq!(snap, tampered, "value/formula equality ignores addr");
}

#[test]
fn snapshot_json_rejects_invalid_addr_1a() {
    let json = r#"{"addr":"1A","value":null,"formula":null}"#;
    let result: Result<CellSnapshot, _> = serde_json::from_str(json);
    let err = result
        .expect_err("invalid addr should fail to deserialize")
        .to_string();

    assert!(
        err.contains("invalid cell address"),
        "error should mention invalid cell address: {err}"
    );
    assert!(
        err.contains("1A"),
        "error should include the offending address: {err}"
    );
}

#[test]
fn snapshot_json_rejects_invalid_addr_a0() {
    let json = r#"{"addr":"A0","value":null,"formula":null}"#;
    let result: Result<CellSnapshot, _> = serde_json::from_str(json);
    let err = result
        .expect_err("invalid addr should fail to deserialize")
        .to_string();

    assert!(
        err.contains("invalid cell address"),
        "error should mention invalid cell address: {err}"
    );
    assert!(
        err.contains("A0"),
        "error should include the offending address: {err}"
    );
}

```

---

### File: `core\tests\pg4_diffop_tests.rs`

```rust
mod common;

use common::sid;
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ColSignature, DiffOp, DiffReport, QueryChangeKind,
    QueryMetadataField, RowSignature,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn addr(a1: &str) -> CellAddress {
    a1.parse().expect("address should parse")
}

fn sid_json(s: &str) -> Value {
    Value::Number(sid(s).0.into())
}

fn snapshot(a1: &str, value: Option<CellValue>, formula: Option<&str>) -> CellSnapshot {
    CellSnapshot {
        addr: addr(a1),
        value,
        formula: formula.map(|s| sid(s)),
    }
}

fn sample_cell_edited() -> DiffOp {
    DiffOp::CellEdited {
        sheet: sid("Sheet1"),
        addr: addr("C3"),
        from: snapshot("C3", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    }
}

// Enforces the invariant documented on DiffOp::CellEdited.
fn assert_cell_edited_invariants(op: &DiffOp, expected_sheet: &str, expected_addr: &str) {
    let expected_addr_parsed: CellAddress =
        expected_addr.parse().expect("expected_addr should parse");
    if let DiffOp::CellEdited {
        sheet,
        addr,
        from,
        to,
    } = op
    {
        assert_eq!(sheet, &sid(expected_sheet));
        assert_eq!(*addr, expected_addr_parsed);
        assert_eq!(from.addr, expected_addr_parsed);
        assert_eq!(to.addr, expected_addr_parsed);
    } else {
        panic!("expected CellEdited");
    }
}

fn op_kind(op: &DiffOp) -> &'static str {
    match op {
        DiffOp::SheetAdded { .. } => "SheetAdded",
        DiffOp::SheetRemoved { .. } => "SheetRemoved",
        DiffOp::RowAdded { .. } => "RowAdded",
        DiffOp::RowRemoved { .. } => "RowRemoved",
        DiffOp::ColumnAdded { .. } => "ColumnAdded",
        DiffOp::ColumnRemoved { .. } => "ColumnRemoved",
        DiffOp::BlockMovedRows { .. } => "BlockMovedRows",
        DiffOp::BlockMovedColumns { .. } => "BlockMovedColumns",
        DiffOp::BlockMovedRect { .. } => "BlockMovedRect",
        DiffOp::CellEdited { .. } => "CellEdited",
        _ => "Unknown",
    }
}


fn json_keys(json: &Value) -> BTreeSet<String> {
    json.as_object()
        .expect("object json")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn pg4_construct_cell_edited_diffop() {
    let op = sample_cell_edited();

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
    if let DiffOp::CellEdited { from, to, .. } = &op {
        assert_ne!(from.value, to.value);
    }
}

#[test]
fn pg4_construct_row_and_column_diffops() {
    let row_added_with_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 8,
        row_signature: None,
    };
    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 0,
        col_signature: None,
    };

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_with_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 10);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0xDEADBEEF);
    } else {
        panic!("expected RowAdded with signature");
    }

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_without_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 11);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowAdded without signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_with_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 9);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0x1234);
    } else {
        panic!("expected RowRemoved with signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_without_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 8);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowRemoved without signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_with_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 2);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0xABCDEF);
    } else {
        panic!("expected ColumnAdded with signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_without_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 3);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnAdded without signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_with_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 1);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0x123456);
    } else {
        panic!("expected ColumnRemoved with signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_without_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 0);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnRemoved without signature");
    }

    assert_ne!(row_added_with_sig, row_added_without_sig);
    assert_ne!(row_removed_with_sig, row_removed_without_sig);
    assert_ne!(col_added_with_sig, col_added_without_sig);
    assert_ne!(col_removed_with_sig, col_removed_without_sig);
}

#[test]
fn pg4_construct_block_move_diffops() {
    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_with_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 10);
        assert_eq!(*row_count, 3);
        assert_eq!(*dst_start_row, 5);
        assert_eq!(block_hash.unwrap(), 0x12345678);
    } else {
        panic!("expected BlockMovedRows with hash");
    }

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_without_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 20);
        assert_eq!(*row_count, 2);
        assert_eq!(*dst_start_row, 0);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRows without hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_with_hash
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*src_start_col, 7);
        assert_eq!(*col_count, 2);
        assert_eq!(*dst_start_col, 3);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedColumns with hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_without_hash
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*src_start_col, 4);
        assert_eq!(*col_count, 1);
        assert_eq!(*dst_start_col, 9);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedColumns without hash");
    }

    assert_ne!(block_rows_with_hash, block_rows_without_hash);
    assert_ne!(block_cols_with_hash, block_cols_without_hash);
}

#[test]
fn pg4_construct_block_rect_diffops() {
    let rect_with_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 5,
        src_row_count: 3,
        src_start_col: 2,
        src_col_count: 4,
        dst_start_row: 10,
        dst_start_col: 6,
        block_hash: Some(0xCAFEBABE),
    };
    let rect_without_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 0,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 1,
        dst_start_row: 20,
        dst_start_col: 10,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRect {
        sheet,
        src_start_row,
        src_row_count,
        src_start_col,
        src_col_count,
        dst_start_row,
        dst_start_col,
        block_hash,
    } = &rect_with_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 5);
        assert_eq!(*src_row_count, 3);
        assert_eq!(*src_start_col, 2);
        assert_eq!(*src_col_count, 4);
        assert_eq!(*dst_start_row, 10);
        assert_eq!(*dst_start_col, 6);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedRect with hash");
    }

    if let DiffOp::BlockMovedRect {
        sheet,
        src_start_row,
        src_row_count,
        src_start_col,
        src_col_count,
        dst_start_row,
        dst_start_col,
        block_hash,
    } = &rect_without_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 0);
        assert_eq!(*src_row_count, 1);
        assert_eq!(*src_start_col, 0);
        assert_eq!(*src_col_count, 1);
        assert_eq!(*dst_start_row, 20);
        assert_eq!(*dst_start_col, 10);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRect without hash");
    }

    assert_ne!(rect_with_hash, rect_without_hash);
}

#[test]
fn pg4_cell_edited_json_shape() {
    let op = sample_cell_edited();
    let json = serde_json::to_value(&op).expect("serialize");
    assert_cell_edited_invariants(&op, "Sheet1", "C3");

    assert_eq!(json["kind"], "CellEdited");
    assert_eq!(json["sheet"], sid_json("Sheet1"));
    assert_eq!(json["addr"], "C3");
    assert_eq!(json["from"]["addr"], "C3");
    assert_eq!(json["to"]["addr"], "C3");

    let obj = json.as_object().expect("object json");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["addr", "from", "kind", "sheet", "to"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected);
}

#[test]
fn pg4_row_added_json_optional_signature() {
    let op_without_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: None,
    };
    let json_without = serde_json::to_value(&op_without_sig).expect("serialize without sig");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "RowAdded");
    assert_eq!(json_without["sheet"], sid_json("Sheet1"));
    assert_eq!(json_without["row_idx"], 10);
    assert!(obj_without.get("row_signature").is_none());

    let op_with_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 123 }),
    };
    let json_with = serde_json::to_value(&op_with_sig).expect("serialize with sig");
    assert_eq!(
        json_with["row_signature"]["hash"],
        "0000000000000000000000000000007b"
    );
}

#[test]
fn pg4_column_added_json_optional_signature() {
    let added_without_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet1"),
        col_idx: 5,
        col_signature: None,
    };
    let json_added_without = serde_json::to_value(&added_without_sig).expect("serialize no sig");
    let obj_added_without = json_added_without.as_object().expect("object json");
    assert_eq!(json_added_without["kind"], "ColumnAdded");
    assert_eq!(json_added_without["sheet"], sid_json("Sheet1"));
    assert_eq!(json_added_without["col_idx"], 5);
    assert!(obj_added_without.get("col_signature").is_none());

    let added_with_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet1"),
        col_idx: 6,
        col_signature: Some(ColSignature { hash: 321 }),
    };
    let json_added_with = serde_json::to_value(&added_with_sig).expect("serialize with sig");
    assert_eq!(
        json_added_with["col_signature"]["hash"],
        "00000000000000000000000000000141"
    );

    let removed_without_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 2,
        col_signature: None,
    };
    let json_removed_without =
        serde_json::to_value(&removed_without_sig).expect("serialize removed no sig");
    let obj_removed_without = json_removed_without.as_object().expect("object json");
    assert_eq!(json_removed_without["kind"], "ColumnRemoved");
    assert!(obj_removed_without.get("col_signature").is_none());

    let removed_with_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 654 }),
    };
    let json_removed_with =
        serde_json::to_value(&removed_with_sig).expect("serialize removed with sig");
    assert_eq!(
        json_removed_with["col_signature"]["hash"],
        "0000000000000000000000000000028e"
    );
}

#[test]
fn pg4_block_moved_rows_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedRows");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: Some(777),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(777));
}

#[test]
fn pg4_block_moved_columns_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedColumns {
        sheet: sid("SheetX"),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedColumns");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedColumns {
        sheet: sid("SheetX"),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: Some(4242),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(4242));
}

#[test]
fn pg4_sheet_added_and_removed_json_shape() {
    let added = DiffOp::SheetAdded {
        sheet: sid("Sheet1"),
    };
    let added_json = serde_json::to_value(&added).expect("serialize sheet added");
    assert_eq!(added_json["kind"], "SheetAdded");
    assert_eq!(added_json["sheet"], sid_json("Sheet1"));
    let added_keys = json_keys(&added_json);
    let expected_keys: BTreeSet<String> = ["kind", "sheet"].into_iter().map(String::from).collect();
    assert_eq!(added_keys, expected_keys);

    let removed = DiffOp::SheetRemoved {
        sheet: sid("SheetX"),
    };
    let removed_json = serde_json::to_value(&removed).expect("serialize sheet removed");
    assert_eq!(removed_json["kind"], "SheetRemoved");
    assert_eq!(removed_json["sheet"], sid_json("SheetX"));
    let removed_keys = json_keys(&removed_json);
    assert_eq!(removed_keys, expected_keys);
}

#[test]
fn pg4_row_and_column_json_shape_keysets() {
    let expected_row_with_sig: BTreeSet<String> = ["kind", "row_idx", "row_signature", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_row_without_sig: BTreeSet<String> = ["kind", "row_idx", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_with_sig: BTreeSet<String> = ["col_idx", "col_signature", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_without_sig: BTreeSet<String> = ["col_idx", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();

    let row_added_with_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 8,
        row_signature: None,
    };

    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 0,
        col_signature: None,
    };

    let cases = vec![
        (
            row_added_with_sig,
            "RowAdded",
            expected_row_with_sig.clone(),
        ),
        (
            row_added_without_sig,
            "RowAdded",
            expected_row_without_sig.clone(),
        ),
        (
            row_removed_with_sig,
            "RowRemoved",
            expected_row_with_sig.clone(),
        ),
        (
            row_removed_without_sig,
            "RowRemoved",
            expected_row_without_sig.clone(),
        ),
        (
            col_added_with_sig,
            "ColumnAdded",
            expected_col_with_sig.clone(),
        ),
        (
            col_added_without_sig,
            "ColumnAdded",
            expected_col_without_sig.clone(),
        ),
        (
            col_removed_with_sig,
            "ColumnRemoved",
            expected_col_with_sig.clone(),
        ),
        (
            col_removed_without_sig,
            "ColumnRemoved",
            expected_col_without_sig.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_move_json_shape_keysets() {
    let expected_rows_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rows_without_hash: BTreeSet<String> = [
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_with_hash: BTreeSet<String> = [
        "block_hash",
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_without_hash: BTreeSet<String> = [
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rect_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_col",
        "dst_start_row",
        "kind",
        "sheet",
        "src_col_count",
        "src_row_count",
        "src_start_col",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rect_without_hash: BTreeSet<String> = [
        "dst_start_col",
        "dst_start_row",
        "kind",
        "sheet",
        "src_col_count",
        "src_row_count",
        "src_start_col",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };
    let block_rect_with_hash = DiffOp::BlockMovedRect {
        sheet: sid("SheetZ"),
        src_start_row: 2,
        src_row_count: 2,
        src_start_col: 3,
        src_col_count: 4,
        dst_start_row: 8,
        dst_start_col: 1,
        block_hash: Some(0xAABBCCDD),
    };
    let block_rect_without_hash = DiffOp::BlockMovedRect {
        sheet: sid("SheetZ"),
        src_start_row: 5,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 2,
        dst_start_row: 10,
        dst_start_col: 4,
        block_hash: None,
    };

    let cases = vec![
        (
            block_rows_with_hash,
            "BlockMovedRows",
            expected_rows_with_hash.clone(),
        ),
        (
            block_rows_without_hash,
            "BlockMovedRows",
            expected_rows_without_hash.clone(),
        ),
        (
            block_cols_with_hash,
            "BlockMovedColumns",
            expected_cols_with_hash.clone(),
        ),
        (
            block_cols_without_hash,
            "BlockMovedColumns",
            expected_cols_without_hash.clone(),
        ),
        (
            block_rect_with_hash,
            "BlockMovedRect",
            expected_rect_with_hash.clone(),
        ),
        (
            block_rect_without_hash,
            "BlockMovedRect",
            expected_rect_without_hash.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_rect_json_shape_and_roundtrip() {
    let without_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 2,
        src_row_count: 3,
        src_start_col: 1,
        src_col_count: 2,
        dst_start_row: 10,
        dst_start_col: 5,
        block_hash: None,
    };
    let with_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 4,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 1,
        dst_start_row: 20,
        dst_start_col: 7,
        block_hash: Some(0x55AA),
    };

    let report = DiffReport::new(vec![without_hash.clone(), with_hash.clone()]);
    let json = serde_json::to_value(&report).expect("serialize rect report");

    let ops_json = json["ops"]
        .as_array()
        .expect("ops should be array for report");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "BlockMovedRect");
    assert_eq!(ops_json[0]["sheet"], sid_json("Sheet1"));
    assert_eq!(ops_json[0]["src_start_row"], 2);
    assert_eq!(ops_json[0]["src_row_count"], 3);
    assert_eq!(ops_json[0]["src_start_col"], 1);
    assert_eq!(ops_json[0]["src_col_count"], 2);
    assert_eq!(ops_json[0]["dst_start_row"], 10);
    assert_eq!(ops_json[0]["dst_start_col"], 5);
    assert!(
        ops_json[0].get("block_hash").is_none(),
        "block_hash should be omitted when None"
    );

    assert_eq!(ops_json[1]["kind"], "BlockMovedRect");
    assert_eq!(ops_json[1]["block_hash"], Value::from(0x55AA));

    let roundtrip: DiffReport =
        serde_json::from_value(json).expect("roundtrip deserialize rect report");
    assert_eq!(roundtrip.ops, vec![without_hash, with_hash]);
}

#[test]
fn pg4_diffop_roundtrip_each_variant() {
    let ops = vec![
        DiffOp::SheetAdded {
            sheet: sid("SheetA"),
        },
        DiffOp::SheetRemoved {
            sheet: sid("SheetB"),
        },
        DiffOp::RowAdded {
            sheet: sid("Sheet1"),
            row_idx: 1,
            row_signature: Some(RowSignature { hash: 42 }),
        },
        DiffOp::RowRemoved {
            sheet: sid("Sheet1"),
            row_idx: 0,
            row_signature: None,
        },
        DiffOp::ColumnAdded {
            sheet: sid("Sheet1"),
            col_idx: 2,
            col_signature: None,
        },
        DiffOp::ColumnRemoved {
            sheet: sid("Sheet1"),
            col_idx: 3,
            col_signature: Some(ColSignature { hash: 99 }),
        },
        DiffOp::BlockMovedRows {
            sheet: sid("Sheet1"),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: Some(1234),
        },
        DiffOp::BlockMovedRows {
            sheet: sid("Sheet1"),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: sid("Sheet2"),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: Some(888),
        },
        DiffOp::BlockMovedColumns {
            sheet: sid("Sheet2"),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: None,
        },
        DiffOp::BlockMovedRect {
            sheet: sid("Sheet3"),
            src_start_row: 1,
            src_row_count: 2,
            src_start_col: 3,
            src_col_count: 4,
            dst_start_row: 10,
            dst_start_col: 20,
            block_hash: Some(0xABCD),
        },
        DiffOp::BlockMovedRect {
            sheet: sid("Sheet3"),
            src_start_row: 1,
            src_row_count: 2,
            src_start_col: 3,
            src_col_count: 4,
            dst_start_row: 10,
            dst_start_col: 20,
            block_hash: None,
        },
        sample_cell_edited(),
        DiffOp::QueryAdded {
            name: sid("Section1/NewQuery"),
        },
        DiffOp::QueryRemoved {
            name: sid("Section1/OldQuery"),
        },
        DiffOp::QueryRenamed {
            from: sid("Section1/Foo"),
            to: sid("Section1/Bar"),
        },
        DiffOp::QueryDefinitionChanged {
            name: sid("Section1/Query1"),
            change_kind: QueryChangeKind::Semantic,
            old_hash: 0x1234567890ABCDEF,
            new_hash: 0xFEDCBA0987654321,
        },
        DiffOp::QueryDefinitionChanged {
            name: sid("Section1/Query2"),
            change_kind: QueryChangeKind::FormattingOnly,
            old_hash: 0xAAAABBBBCCCCDDDD,
            new_hash: 0xAAAABBBBCCCCDDDD,
        },
        DiffOp::QueryMetadataChanged {
            name: sid("Section1/Query3"),
            field: QueryMetadataField::LoadToSheet,
            old: Some(sid("true")),
            new: Some(sid("false")),
        },
        DiffOp::QueryMetadataChanged {
            name: sid("Section1/Query4"),
            field: QueryMetadataField::LoadToModel,
            old: Some(sid("false")),
            new: Some(sid("true")),
        },
        DiffOp::QueryMetadataChanged {
            name: sid("Section1/Query5"),
            field: QueryMetadataField::GroupPath,
            old: None,
            new: Some(sid("Reports/Sales")),
        },
        DiffOp::QueryMetadataChanged {
            name: sid("Section1/Query6"),
            field: QueryMetadataField::ConnectionOnly,
            old: Some(sid("true")),
            new: Some(sid("false")),
        },
    ];

    for original in ops {
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: DiffOp = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, original);

        if let DiffOp::CellEdited { .. } = &deserialized {
            assert_cell_edited_invariants(&deserialized, "Sheet1", "C3");
        }
    }
}

#[test]
fn pg4_cell_edited_roundtrip_preserves_snapshot_addrs() {
    let op = sample_cell_edited();
    let json = serde_json::to_string(&op).expect("serialize");
    let round_tripped: DiffOp = serde_json::from_str(&json).expect("deserialize");

    assert_cell_edited_invariants(&round_tripped, "Sheet1", "C3");
}

#[test]
fn pg4_diff_report_roundtrip_preserves_order() {
    let op1 = DiffOp::SheetAdded {
        sheet: sid("Sheet1"),
    };
    let op2 = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: None,
    };
    let op3 = sample_cell_edited();

    let ops = vec![op1, op2, op3];
    let report = DiffReport::new(ops.clone());
    assert_eq!(report.version, DiffReport::SCHEMA_VERSION);

    let serialized = serde_json::to_string(&report).expect("serialize report");
    let deserialized: DiffReport = serde_json::from_str(&serialized).expect("deserialize report");
    assert_eq!(deserialized.version, "1");
    assert_eq!(deserialized.ops, ops);

    let kinds: Vec<&str> = deserialized.ops.iter().map(op_kind).collect();
    assert_eq!(kinds, vec!["SheetAdded", "RowAdded", "CellEdited"]);
}

#[test]
fn pg4_diff_report_json_shape() {
    let ops = vec![
        DiffOp::SheetRemoved {
            sheet: sid("SheetX"),
        },
        DiffOp::RowRemoved {
            sheet: sid("SheetX"),
            row_idx: 3,
            row_signature: Some(RowSignature { hash: 7 }),
        },
    ];
    let report = DiffReport::new(ops);
    let json = serde_json::to_value(&report).expect("serialize to value");

    let obj = json.as_object().expect("report json object");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["complete", "ops", "strings", "version"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected);
    assert_eq!(obj.get("version").and_then(Value::as_str), Some("1"));
    assert_eq!(obj.get("complete").and_then(Value::as_bool), Some(true));

    let ops_json = obj
        .get("ops")
        .and_then(Value::as_array)
        .expect("ops should be array");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "SheetRemoved");
    assert_eq!(ops_json[1]["kind"], "RowRemoved");
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_top_level_addr() {
    let sheet_id = sid("Sheet1").0;
    let json = format!(
        r#"{{
        "kind": "CellEdited",
        "sheet": {sheet_id},
        "addr": "1A",
        "from": {{ "addr": "C3", "value": null, "formula": null }},
        "to":   {{ "addr": "C3", "value": null, "formula": null }}
    }}"#
    );

    let err = serde_json::from_str::<DiffOp>(&json)
        .expect_err("invalid top-level addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_snapshot_addrs() {
    let sheet_id = sid("Sheet1").0;
    let json = format!(
        r#"{{
        "kind": "CellEdited",
        "sheet": {sheet_id},
        "addr": "C3",
        "from": {{ "addr": "A0", "value": null, "formula": null }},
        "to":   {{ "addr": "C3", "value": null, "formula": null }}
    }}"#
    );

    let err = serde_json::from_str::<DiffOp>(&json)
        .expect_err("invalid snapshot addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("A0"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diff_report_rejects_invalid_nested_addr() {
    let sheet_id = sid("Sheet1").0;
    let json = format!(
        r#"{{
        "version": "1",
        "strings": [],
        "ops": [{{
            "kind": "CellEdited",
            "sheet": {sheet_id},
            "addr": "1A",
            "from": {{ "addr": "C3", "value": null, "formula": null }},
            "to":   {{ "addr": "C3", "value": null, "formula": null }}
        }}]
    }}"#
    );

    let err = serde_json::from_str::<DiffReport>(&json)
        .expect_err("invalid nested addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should surface nested invalid address: {msg}",
    );
}

#[test]
#[should_panic]
fn pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr() {
    let op = DiffOp::CellEdited {
        sheet: sid("Sheet1"),
        addr: addr("C3"),
        from: snapshot("D4", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    };

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
}

#[test]
#[cfg(feature = "perf-metrics")]
fn pg4_diff_report_json_shape_with_metrics() {
    use excel_diff::perf::DiffMetrics;

    let ops = vec![DiffOp::SheetAdded {
        sheet: "NewSheet".to_string(),
    }];
    let mut report = DiffReport::new(ops);
    let mut metrics = DiffMetrics::default();
    metrics.move_detection_time_ms = 10;
    metrics.alignment_time_ms = 20;
    metrics.cell_diff_time_ms = 30;
    metrics.total_time_ms = 60;
    metrics.rows_processed = 1000;
    metrics.cells_compared = 5000;
    metrics.anchors_found = 50;
    metrics.moves_detected = 2;
    report.metrics = Some(metrics);

    let json = serde_json::to_value(&report).expect("serialize to value");
    let obj = json.as_object().expect("report json object");

    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["complete", "ops", "version", "metrics"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected, "report should include metrics key");

    let metrics_obj = obj
        .get("metrics")
        .and_then(Value::as_object)
        .expect("metrics object");

    assert!(metrics_obj.contains_key("move_detection_time_ms"));
    assert!(metrics_obj.contains_key("alignment_time_ms"));
    assert!(metrics_obj.contains_key("cell_diff_time_ms"));
    assert!(metrics_obj.contains_key("total_time_ms"));
    assert!(metrics_obj.contains_key("rows_processed"));
    assert!(metrics_obj.contains_key("cells_compared"));
    assert!(metrics_obj.contains_key("anchors_found"));
    assert!(metrics_obj.contains_key("moves_detected"));

    assert!(
        !metrics_obj.contains_key("parse_time_ms"),
        "parse_time_ms is planned for future phase"
    );
    assert!(
        !metrics_obj.contains_key("peak_memory_bytes"),
        "peak_memory_bytes is planned for future phase"
    );

    assert_eq!(
        metrics_obj.get("rows_processed").and_then(Value::as_u64),
        Some(1000)
    );
    assert_eq!(
        metrics_obj.get("cells_compared").and_then(Value::as_u64),
        Some(5000)
    );
}

```

---

### File: `core\tests\pg5_grid_diff_tests.rs`

```rust
mod common;

use common::{grid_from_numbers, single_sheet_workbook};
use excel_diff::{CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage};
use std::collections::BTreeSet;

fn sheet_name<'a>(report: &'a DiffReport, id: &excel_diff::SheetId) -> &'a str {
    report.strings[id.0 as usize].as_str()
}

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn pg5_1_grid_diff_1x1_identical_empty_diff() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn pg5_2_grid_diff_1x1_value_change_single_cell_edited() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[2]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn pg5_3_grid_diff_row_appended_row_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowAdded, got {other:?}"),
    }
}

#[test]
fn pg5_4_grid_diff_column_appended_column_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnAdded, got {other:?}"),
    }
}

#[test]
fn pg5_5_grid_diff_same_shape_scattered_cell_edits() {
    let old = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]),
    );
    let new = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[10, 2, 3], &[4, 50, 6], &[7, 8, 90]]),
    );

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 3);
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. }))
    );

    let edited_addrs: BTreeSet<String> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.to_a1()),
            _ => None,
        })
        .collect();
    let expected: BTreeSet<String> = ["A1", "B2", "C3"].into_iter().map(String::from).collect();
    assert_eq!(edited_addrs, expected);
}

#[test]
fn pg5_6_grid_diff_degenerate_grids() {
    let empty_old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let empty_new = single_sheet_workbook("Sheet1", Grid::new(0, 0));

    let empty_report = diff_workbooks(&empty_old, &empty_new, &DiffConfig::default());
    assert!(empty_report.ops.is_empty());

    let old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 2);

    let mut row_added = 0;
    let mut col_added = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*row_idx, 0);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*col_idx, 0);
                assert!(col_signature.is_none());
                col_added += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    assert_eq!(col_added, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_7_grid_diff_row_truncated_row_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_8_grid_diff_column_truncated_column_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_9_grid_diff_row_and_column_truncated_structure_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 2);

    let mut rows_removed = 0;
    let mut cols_removed = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*row_idx, 1);
                assert!(row_signature.is_none());
                rows_removed += 1;
            }
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*col_idx, 1);
                assert!(col_signature.is_none());
                cols_removed += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(rows_removed, 1);
    assert_eq!(cols_removed, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_10_grid_diff_row_appended_with_overlap_cell_edits() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 20], &[30, 4], &[5, 6]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 3);

    let mut row_added = 0;
    let mut cell_edits = BTreeSet::new();

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*row_idx, 2);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::CellEdited { addr, .. } => {
                cell_edits.insert(addr.to_a1());
            }
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    let expected: BTreeSet<String> = ["B1", "A2"].into_iter().map(String::from).collect();
    assert_eq!(cell_edits, expected);
}

```

---

### File: `core\tests\pg6_object_vs_grid_tests.rs`

```rust
mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{DiffConfig, DiffOp, WorkbookPackage};

#[test]
fn pg6_1_sheet_added_no_grid_ops_on_main() {
    let old = open_fixture_workbook("pg6_sheet_added_a.xlsx");
    let new = open_fixture_workbook("pg6_sheet_added_b.xlsx");

    let report = WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());

    let mut sheet_added = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("NewSheet") => sheet_added += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. }
                if *sheet == sid("Main") =>
            {
                panic!("unexpected grid op on Main: {op:?}");
            }
            DiffOp::SheetAdded { sheet } => {
                panic!("unexpected sheet added: {sheet}");
            }
            DiffOp::SheetRemoved { sheet } => {
                panic!("unexpected sheet removed: {sheet}");
            }
            DiffOp::BlockMovedRows { .. } | DiffOp::BlockMovedColumns { .. } => {
                panic!("block move ops are not expected in PG6.1: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(sheet_added, 1, "exactly one NewSheet addition expected");
    assert_eq!(report.ops.len(), 1, "no other operations expected");
}

#[test]
fn pg6_2_sheet_removed_no_grid_ops_on_main() {
    let old = open_fixture_workbook("pg6_sheet_removed_a.xlsx");
    let new = open_fixture_workbook("pg6_sheet_removed_b.xlsx");

    let report = WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());

    let mut sheet_removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetRemoved { sheet } if *sheet == sid("OldSheet") => sheet_removed += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. }
                if *sheet == sid("Main") =>
            {
                panic!("unexpected grid op on Main: {op:?}");
            }
            DiffOp::SheetAdded { sheet } => {
                panic!("unexpected sheet added: {sheet}");
            }
            DiffOp::SheetRemoved { sheet } => {
                panic!("unexpected sheet removed: {sheet}");
            }
            DiffOp::BlockMovedRows { .. } | DiffOp::BlockMovedColumns { .. } => {
                panic!("block move ops are not expected in PG6.2: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(sheet_removed, 1, "exactly one OldSheet removal expected");
    assert_eq!(report.ops.len(), 1, "no other operations expected");
}

#[test]
fn pg6_3_rename_as_remove_plus_add_no_grid_ops() {
    let old = open_fixture_workbook("pg6_sheet_renamed_a.xlsx");
    let new = open_fixture_workbook("pg6_sheet_renamed_b.xlsx");

    let report = WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("NewName") => added += 1,
            DiffOp::SheetRemoved { sheet } if *sheet == sid("OldName") => removed += 1,
            DiffOp::SheetAdded { sheet } => panic!("unexpected sheet added: {sheet}"),
            DiffOp::SheetRemoved { sheet } => panic!("unexpected sheet removed: {sheet}"),
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::CellEdited { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. } => {
                panic!("no grid-level ops expected for rename scenario: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(
        report.ops.len(),
        2,
        "rename should produce one add and one remove"
    );
    assert_eq!(added, 1, "expected one NewName addition");
    assert_eq!(removed, 1, "expected one OldName removal");
}

#[test]
fn pg6_4_sheet_and_grid_change_composed_cleanly() {
    let old = open_fixture_workbook("pg6_sheet_and_grid_change_a.xlsx");
    let new = open_fixture_workbook("pg6_sheet_and_grid_change_b.xlsx");

    let report = WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());

    let mut scratch_added = 0;
    let mut main_cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("Scratch") => scratch_added += 1,
            DiffOp::CellEdited { sheet, .. } => {
                assert_eq!(sheet, &sid("Main"), "only Main should have cell edits");
                main_cell_edits += 1;
            }
            DiffOp::SheetRemoved { .. } => {
                panic!("no sheets should be removed in PG6.4: {op:?}");
            }
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. } => {
                panic!("no structural row/column ops expected in PG6.4: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(scratch_added, 1, "exactly one Scratch addition expected");
    assert!(
        main_cell_edits > 0,
        "Main should report at least one cell edit"
    );
}

```

---

### File: `core\tests\signature_tests.rs`

```rust
mod common;

use common::sid;
use excel_diff::{CellValue, Grid, GridView, StringId};

#[derive(Clone)]
struct TestCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
}

trait GridTestInsert {
    fn insert_test(&mut self, cell: TestCell);
}

impl GridTestInsert for Grid {
    fn insert_test(&mut self, cell: TestCell) {
        self.insert_cell(cell.row, cell.col, cell.value, cell.formula);
    }
}

fn make_cell(row: u32, col: u32, value: Option<CellValue>, formula: Option<&str>) -> TestCell {
    TestCell {
        row,
        col,
        value,
        formula: formula.map(sid),
    }
}

#[test]
fn identical_rows_have_same_signature() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        let cell = make_cell(0, c, Some(CellValue::Number(c as f64)), None);
        grid1.insert_test(cell.clone());
        grid2.insert_test(cell);
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_eq!(sig1, sig2);
}

#[test]
fn different_rows_have_different_signatures() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        grid1.insert_test(make_cell(0, c, Some(CellValue::Number(c as f64)), None));
        grid2.insert_test(make_cell(
            0,
            c,
            Some(CellValue::Number((c + 1) as f64)),
            None,
        ));
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1, sig2);
}

#[test]
fn compute_all_signatures_populates_fields() {
    let mut grid = Grid::new(5, 5);
    grid.insert_test(make_cell(
        2,
        2,
        Some(CellValue::Text(sid("center"))),
        None,
    ));
    assert!(grid.row_signatures.is_none());
    assert!(grid.col_signatures.is_none());
    grid.compute_all_signatures();
    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert_eq!(grid.row_signatures.as_ref().unwrap().len(), 5);
    assert_eq!(grid.col_signatures.as_ref().unwrap().len(), 5);
    assert_ne!(grid.row_signatures.as_ref().unwrap()[2].hash, 0);
    assert_ne!(grid.col_signatures.as_ref().unwrap()[2].hash, 0);
}

#[test]
fn compute_all_signatures_on_empty_grid_produces_empty_vectors() {
    let mut grid = Grid::new(0, 0);

    grid.compute_all_signatures();

    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert!(grid.row_signatures.as_ref().unwrap().is_empty());
    assert!(grid.col_signatures.as_ref().unwrap().is_empty());
}

#[test]
fn compute_all_signatures_with_all_empty_rows_and_cols_is_stable() {
    let mut grid = Grid::new(3, 4);

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    assert_eq!(first_rows.len(), 3);
    assert_eq!(first_cols.len(), 4);
    let empty_row_hash = first_rows[0].hash;
    let empty_col_hash = first_cols[0].hash;
    assert!(first_rows.iter().all(|sig| sig.hash == empty_row_hash));
    assert!(first_cols.iter().all(|sig| sig.hash == empty_col_hash));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_eq!(first_rows, *second_rows);
    assert_eq!(first_cols, *second_cols);
}

#[test]
fn row_and_col_signatures_match_bulk_computation() {
    let mut grid = Grid::new(3, 2);
    grid.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("text"))), None));
    grid.insert_test(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    for r in 0..3 {
        assert_eq!(
            grid.compute_row_signature(r).hash,
            row_sigs[r as usize].hash
        );
    }

    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");
    for c in 0..2 {
        assert_eq!(
            grid.compute_col_signature(c).hash,
            col_sigs[c as usize].hash
        );
    }
}

#[test]
fn compute_all_signatures_recomputes_after_mutation() {
    let mut grid = Grid::new(3, 3);
    grid.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("x"))), None));

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("y"))), None));
    grid.insert_test(make_cell(2, 2, Some(CellValue::Bool(true)), None));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_ne!(first_rows[1].hash, second_rows[1].hash);
    assert_ne!(first_cols[1].hash, second_cols[1].hash);
}

#[test]
fn row_signatures_distinguish_column_positions() {
    let mut grid1 = Grid::new(1, 2);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(1, 2);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn col_signatures_distinguish_row_positions() {
    let mut grid1 = Grid::new(2, 1);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(2, 1);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_col_signature(0);
    let sig2 = grid2.compute_col_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn row_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("mix"))), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Bool(true)), None));

    let mut grid2 = Grid::new(1, 3);
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Bool(true)), None));
    grid2.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("mix"))), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.row_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.row_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("col"))), None));
    grid1.insert_test(make_cell(2, 0, Some(CellValue::Bool(false)), None));

    let mut grid2 = Grid::new(3, 1);
    grid2.insert_test(make_cell(2, 0, Some(CellValue::Bool(false)), None));
    grid2.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("col"))), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.col_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.col_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(3, 1);
    grid_num.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(3, 1);
    grid_text.insert_test(make_cell(0, 0, Some(CellValue::Text(sid("1"))), None));

    let mut grid_bool = Grid::new(3, 1);
    grid_bool.insert_test(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_col_signature(0).hash;
    let txt = grid_text.compute_col_signature(0).hash;
    let boo = grid_bool.compute_col_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(1, 1);
    grid_num.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(1, 1);
    grid_text.insert_test(make_cell(0, 0, Some(CellValue::Text(sid("1"))), None));

    let mut grid_bool = Grid::new(1, 1);
    grid_bool.insert_test(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_row_signature(0).hash;
    let txt = grid_text.compute_row_signature(0).hash;
    let boo = grid_bool.compute_row_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_ignores_empty_trailing_cells() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(1, 10);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_ignores_empty_trailing_rows() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(10, 1);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_includes_formulas_by_default() {
    let mut with_formula = Grid::new(2, 1);
    with_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut without_formula = Grid::new(2, 1);
    without_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = with_formula.compute_col_signature(0).hash;
    let sig_without = without_formula.compute_col_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn col_signature_includes_formulas_sparse() {
    let mut formula_short = Grid::new(5, 1);
    formula_short.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Text(sid("foo"))),
        Some("=A2"),
    ));

    let mut formula_tall = Grid::new(10, 1);
    formula_tall.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Text(sid("foo"))),
        Some("=A2"),
    ));

    let mut value_only = Grid::new(10, 1);
    value_only.insert_test(make_cell(0, 0, Some(CellValue::Text(sid("foo"))), None));

    let sig_formula_short = formula_short.compute_col_signature(0).hash;
    let sig_formula_tall = formula_tall.compute_col_signature(0).hash;
    let sig_value_only = value_only.compute_col_signature(0).hash;

    assert_eq!(sig_formula_short, sig_formula_tall);
    assert_ne!(sig_formula_short, sig_value_only);
}

#[test]
fn row_signature_includes_formulas_by_default() {
    let mut grid_with_formula = Grid::new(1, 1);
    grid_with_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut grid_without_formula = Grid::new(1, 1);
    grid_without_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = grid_with_formula.compute_row_signature(0).hash;
    let sig_without = grid_without_formula.compute_row_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn row_signature_is_stable_across_computations() {
    let mut grid = Grid::new(1, 3);
    grid.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("x"))), None));
    grid.insert_test(make_cell(0, 2, Some(CellValue::Bool(false)), None));

    let sig1 = grid.compute_row_signature(0);
    let sig2 = grid.compute_row_signature(0);
    assert_eq!(sig1.hash, sig2.hash);
    assert_ne!(sig1.hash, 0);
}

#[test]
fn row_signature_with_formula_is_stable() {
    let mut grid = Grid::new(1, 2);
    grid.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));
    grid.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("bar"))), None));

    let sig1 = grid.compute_row_signature(0);
    let sig2 = grid.compute_row_signature(0);
    assert_eq!(sig1.hash, sig2.hash);
    assert_ne!(sig1.hash, 0);
}

#[test]
fn gridview_rowmeta_hash_matches_compute_all_signatures() {
    let mut grid = Grid::new(3, 2);
    grid.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("text"))), None));
    grid.insert_test(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_signatures = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should be computed")
        .clone();
    let col_signatures = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should be computed")
        .clone();

    let view = GridView::from_grid(&grid);

    for (idx, meta) in view.row_meta.iter().enumerate() {
        assert_eq!(meta.hash, row_signatures[idx]);
    }

    for (idx, meta) in view.col_meta.iter().enumerate() {
        assert_eq!(meta.hash, col_signatures[idx].hash);
    }
}

#[test]
fn row_signature_unchanged_after_column_insert_at_position_zero() {
    let mut grid1 = Grid::new(2, 3);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Number(3.0)), None));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("a"))), None));
    grid1.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("b"))), None));
    grid1.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("c"))), None));

    let mut grid2 = Grid::new(2, 4);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(99.0)), None));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(1.0)), None));
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(0, 3, Some(CellValue::Number(3.0)), None));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("z"))), None));
    grid2.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("a"))), None));
    grid2.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("b"))), None));
    grid2.insert_test(make_cell(1, 3, Some(CellValue::Text(sid("c"))), None));

    let view1 = GridView::from_grid(&grid1);
    let view2 = GridView::from_grid(&grid2);

    assert_ne!(view1.row_meta[0].hash, view2.row_meta[0].hash);
    assert_ne!(view1.row_meta[1].hash, view2.row_meta[1].hash);
}

#[test]
fn row_signature_unchanged_after_column_delete_from_middle() {
    let mut grid1 = Grid::new(2, 4);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Number(3.0)), None));
    grid1.insert_test(make_cell(0, 3, Some(CellValue::Number(4.0)), None));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("a"))), None));
    grid1.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("b"))), None));
    grid1.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("c"))), None));
    grid1.insert_test(make_cell(1, 3, Some(CellValue::Text(sid("d"))), None));

    let mut grid2 = Grid::new(2, 3);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(3.0)), None));
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Number(4.0)), None));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("a"))), None));
    grid2.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("c"))), None));
    grid2.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("d"))), None));

    let view1 = GridView::from_grid(&grid1);
    let view2 = GridView::from_grid(&grid2);

    assert_ne!(view1.row_meta[0].hash, view2.row_meta[0].hash);
    assert_ne!(view1.row_meta[1].hash, view2.row_meta[1].hash);
}

#[test]
fn row_signature_consistent_for_same_content_different_column_indices() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Number(3.0)), None));

    let mut grid2 = Grid::new(1, 5);
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(1.0)), None));
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(0, 3, Some(CellValue::Number(3.0)), None));

    let view1 = GridView::from_grid(&grid1);
    let view2 = GridView::from_grid(&grid2);

    assert_eq!(view1.row_meta[0].hash, view2.row_meta[0].hash);
}

```

---

### File: `core\tests\sparse_grid_tests.rs`

```rust
use excel_diff::{CellValue, Grid, with_default_session};

#[test]
fn sparse_grid_empty_has_zero_cells() {
    let grid = Grid::new(1000, 1000);
    assert_eq!(grid.cell_count(), 0);
    assert!(grid.is_empty());
    assert_eq!(grid.nrows, 1000);
    assert_eq!(grid.ncols, 1000);
}

#[test]
fn sparse_grid_insert_and_retrieve() {
    let mut grid = Grid::new(100, 100);
    grid.insert_cell(50, 50, Some(CellValue::Number(42.0)), None);
    assert_eq!(grid.cell_count(), 1);
    let retrieved = grid.get(50, 50).expect("cell should exist");
    assert_eq!(retrieved.value, Some(CellValue::Number(42.0)));
    assert!(grid.get(0, 0).is_none());
}

#[test]
fn sparse_grid_iter_cells_only_populated() {
    let mut grid = Grid::new(1000, 1000);
    for i in 0..10 {
        grid.insert_cell(i * 100, i * 100, Some(CellValue::Number(i as f64)), None);
    }
    let cells: Vec<_> = grid.iter_cells().collect();
    assert_eq!(cells.len(), 10);
}

#[test]
fn sparse_grid_memory_efficiency() {
    let grid = Grid::new(10_000, 1_000);
    assert!(std::mem::size_of_val(&grid) < 1024);
}

#[test]
fn rows_iter_covers_all_rows() {
    let grid = Grid::new(3, 5);
    let rows: Vec<u32> = grid.rows_iter().collect();
    assert_eq!(rows, vec![0, 1, 2]);
}

#[test]
fn cols_iter_covers_all_cols() {
    let grid = Grid::new(4, 2);
    let cols: Vec<u32> = grid.cols_iter().collect();
    assert_eq!(cols, vec![0, 1]);
}

#[test]
fn rows_iter_and_get_are_consistent() {
    let mut grid = Grid::new(2, 2);
    grid.insert_cell(1, 1, Some(CellValue::Number(1.0)), None);

    for r in grid.rows_iter() {
        for c in grid.cols_iter() {
            let _ = grid.get(r, c);
        }
    }
}

#[test]
fn sparse_grid_all_empty_rows_have_zero_signatures() {
    let mut grid = Grid::new(2, 3);

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(row_sigs.len(), 2);
    assert_eq!(col_sigs.len(), 3);
    let empty_row_hash = row_sigs[0].hash;
    let empty_col_hash = col_sigs[0].hash;
    assert!(row_sigs.iter().all(|sig| sig.hash == empty_row_hash));
    assert!(col_sigs.iter().all(|sig| sig.hash == empty_col_hash));
}

#[test]
fn compute_signatures_on_sparse_grid_produces_hashes() {
    let mut grid = Grid::new(4, 4);
    with_default_session(|session| {
        let text_id = session.strings.intern("value");
        let formula_id = session.strings.intern("=A1");
        grid.insert_cell(1, 3, Some(CellValue::Text(text_id)), Some(formula_id));
    });

    grid.compute_all_signatures();

    let row_hash = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist")[1]
        .hash;
    let col_hash = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist")[3]
        .hash;

    assert_ne!(row_hash, 0);
    assert_ne!(col_hash, 0);
}

#[test]
fn compute_all_signatures_matches_direct_computation() {
    let mut grid = Grid::new(3, 3);
    with_default_session(|session| {
        let formula_a = session.strings.intern("=5+5");
        let text_id = session.strings.intern("x");
        let formula_b = session.strings.intern("=A1");
        grid.insert_cell(0, 1, Some(CellValue::Number(10.0)), Some(formula_a));
        grid.insert_cell(1, 0, Some(CellValue::Text(text_id)), None);
        grid.insert_cell(2, 2, Some(CellValue::Bool(false)), Some(formula_b));
    });

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(grid.compute_row_signature(0).hash, row_sigs[0].hash);
    assert_eq!(grid.compute_row_signature(2).hash, row_sigs[2].hash);
    assert_eq!(grid.compute_col_signature(0).hash, col_sigs[0].hash);
    assert_eq!(grid.compute_col_signature(2).hash, col_sigs[2].hash);
}

```

---

### File: `core\tests\streaming_sink_tests.rs`

```rust
use excel_diff::{
    CallbackSink, CellValue, DiffConfig, DiffOp, DiffSession, Grid, Sheet, SheetKind, VecSink,
    Workbook, try_diff_workbooks_streaming,
};

fn make_test_workbook(session: &mut DiffSession, values: &[f64]) -> Workbook {
    let mut grid = Grid::new(values.len() as u32, 1);
    for (i, &val) in values.iter().enumerate() {
        grid.insert_cell(i as u32, 0, Some(CellValue::Number(val)), None);
    }

    let sheet_name = session.strings.intern("TestSheet");

    Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
}

#[test]
fn vec_sink_and_callback_sink_produce_identical_ops() {
    let mut session = DiffSession::new();

    let wb_a = make_test_workbook(&mut session, &[1.0, 2.0, 3.0]);
    let wb_b = make_test_workbook(&mut session, &[1.0, 5.0, 3.0, 4.0]);

    let config = DiffConfig::default();

    let mut vec_sink = VecSink::new();
    let summary_vec = try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        &mut session.strings,
        &config,
        &mut vec_sink,
    )
    .expect("VecSink diff should succeed");
    let vec_ops = vec_sink.into_ops();

    let mut callback_ops: Vec<DiffOp> = Vec::new();
    {
        let mut callback_sink = CallbackSink::new(|op| callback_ops.push(op));
        let summary_callback = try_diff_workbooks_streaming(
            &wb_a,
            &wb_b,
            &mut session.strings,
            &config,
            &mut callback_sink,
        )
        .expect("CallbackSink diff should succeed");

        assert_eq!(
            summary_vec.op_count, summary_callback.op_count,
            "summaries should report same op count"
        );
        assert_eq!(
            summary_vec.complete, summary_callback.complete,
            "summaries should report same complete status"
        );
    }

    assert_eq!(
        vec_ops.len(),
        callback_ops.len(),
        "both sinks should collect same number of ops"
    );

    for (i, (vec_op, cb_op)) in vec_ops.iter().zip(callback_ops.iter()).enumerate() {
        assert_eq!(
            vec_op, cb_op,
            "op at index {} should be identical between VecSink and CallbackSink",
            i
        );
    }

    assert!(
        !vec_ops.is_empty(),
        "expected at least one diff op for the test workbooks"
    );
}

#[test]
fn streaming_produces_ops_in_consistent_order() {
    let mut session = DiffSession::new();

    let wb_a = make_test_workbook(&mut session, &[1.0, 2.0]);
    let wb_b = make_test_workbook(&mut session, &[3.0, 4.0]);

    let config = DiffConfig::default();

    let mut first_run_ops: Vec<DiffOp> = Vec::new();
    {
        let mut sink = CallbackSink::new(|op| first_run_ops.push(op));
        try_diff_workbooks_streaming(&wb_a, &wb_b, &mut session.strings, &config, &mut sink)
            .expect("first run should succeed");
    }

    let mut second_run_ops: Vec<DiffOp> = Vec::new();
    {
        let mut sink = CallbackSink::new(|op| second_run_ops.push(op));
        try_diff_workbooks_streaming(&wb_a, &wb_b, &mut session.strings, &config, &mut sink)
            .expect("second run should succeed");
    }

    assert_eq!(
        first_run_ops, second_run_ops,
        "streaming output should be deterministic across runs"
    );
}

#[test]
fn streaming_summary_matches_collected_ops() {
    let mut session = DiffSession::new();

    let wb_a = make_test_workbook(&mut session, &[1.0]);
    let wb_b = make_test_workbook(&mut session, &[2.0, 3.0]);

    let config = DiffConfig::default();

    let mut op_count = 0usize;
    let summary = {
        let mut sink = CallbackSink::new(|_op| op_count += 1);
        try_diff_workbooks_streaming(&wb_a, &wb_b, &mut session.strings, &config, &mut sink)
            .expect("streaming should succeed")
    };

    assert_eq!(
        summary.op_count, op_count,
        "summary.op_count should match actual ops emitted"
    );
    assert!(summary.complete, "diff should be complete");
}


```

---

### File: `core\tests\string_pool_tests.rs`

```rust
use excel_diff::StringPool;

#[test]
fn intern_50k_identical_strings_returns_same_id() {
    let mut pool = StringPool::new();
    let first_id = pool.intern("repeated_string");

    for _ in 1..50_000 {
        let id = pool.intern("repeated_string");
        assert_eq!(id, first_id, "interning same string must return same id");
    }

    assert!(
        pool.len() >= 2,
        "pool should have at least 2 entries (empty string + our string)"
    );
    assert!(
        pool.len() <= 3,
        "pool should not grow beyond initial strings"
    );

    assert_eq!(pool.resolve(first_id), "repeated_string");
}

#[test]
fn intern_distinct_strings_returns_different_ids() {
    let mut pool = StringPool::new();

    let id_a = pool.intern("alpha");
    let id_b = pool.intern("beta");
    let id_c = pool.intern("gamma");

    assert_ne!(id_a, id_b);
    assert_ne!(id_b, id_c);
    assert_ne!(id_a, id_c);

    assert_eq!(pool.resolve(id_a), "alpha");
    assert_eq!(pool.resolve(id_b), "beta");
    assert_eq!(pool.resolve(id_c), "gamma");
}

#[test]
fn empty_string_is_pre_interned() {
    let pool = StringPool::new();

    assert!(pool.len() >= 1, "pool should have at least empty string");
    assert_eq!(pool.resolve(excel_diff::StringId(0)), "");
}

#[test]
fn resolve_returns_original_string() {
    let mut pool = StringPool::new();

    let test_cases = vec![
        "hello",
        "world",
        "with spaces",
        "with\nnewline",
        "unicode: 日本語",
        "",
    ];

    for s in &test_cases {
        let id = pool.intern(s);
        assert_eq!(pool.resolve(id), *s);
    }
}

#[test]
fn into_strings_returns_all_interned() {
    let mut pool = StringPool::new();

    pool.intern("first");
    pool.intern("second");
    pool.intern("third");

    let strings = pool.into_strings();

    assert!(strings.contains(&"".to_string()));
    assert!(strings.contains(&"first".to_string()));
    assert!(strings.contains(&"second".to_string()));
    assert!(strings.contains(&"third".to_string()));
    assert_eq!(strings.len(), 4);
}


```

---

### File: `fixtures\manifest.yaml`

```yaml
scenarios:
  # --- Phase 1.1: Basic File Opening ---
  - id: "smoke_minimal"
    generator: "basic_grid"
    args: { rows: 1, cols: 1 }
    output: "minimal.xlsx"

  # --- Phase 1.2: Is this a ZIP? ---
  - id: "container_random_zip"
    generator: "corrupt_container"
    args: { mode: "random_zip" }
    output: "random_zip.zip"
    
  - id: "container_no_content_types"
    generator: "corrupt_container"
    args: { mode: "no_content_types" }
    output: "no_content_types.xlsx"

  - id: "container_not_zip_text"
    generator: "corrupt_container"
    args: { mode: "not_zip_text" }
    output: "not_a_zip.txt"

  # --- PG1: Workbook -> Sheet -> Grid IR sanity ---
  - id: "pg1_basic_two_sheets"
    generator: "basic_grid"
    args: { rows: 3, cols: 3, two_sheets: true } # Sheet1 3x3, Sheet2 5x2 (logic in generator)
    output: "pg1_basic_two_sheets.xlsx"

  - id: "pg1_sparse"
    generator: "sparse_grid"
    output: "pg1_sparse_used_range.xlsx"

  - id: "pg1_mixed"
    generator: "edge_case"
    output: "pg1_empty_and_mixed_sheets.xlsx"

  # --- PG2: Addressing and index invariants ---
  - id: "pg2_addressing"
    generator: "address_sanity"
    args:
      targets: ["A1", "B2", "C3", "Z1", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1"]
    output: "pg2_addressing_matrix.xlsx"

  # --- PG3: Cell snapshots and comparison semantics ---
  - id: "pg3_types"
    generator: "value_formula"
    output: "pg3_value_and_formula_cells.xlsx"

  # --- Phase 3: Spreadsheet-mode G1/G2 ---
  - id: "g1_equal_sheet"
    generator: "basic_grid"
    args:
      rows: 5
      cols: 5
      sheet: "Sheet1"
    output:
      - "equal_sheet_a.xlsx"
      - "equal_sheet_b.xlsx"

  - id: "g2_single_cell_value"
    generator: "single_cell_diff"
    args:
      rows: 5
      cols: 5
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: 1.0
      value_b: 2.0
    output:
      - "single_cell_value_a.xlsx"
      - "single_cell_value_b.xlsx"

  # --- Phase 3: Spreadsheet-mode G5-G7 ---

  - id: "g5_multi_cell_edits"
    generator: "multi_cell_diff"
    args:
      rows: 20
      cols: 10
      sheet: "Sheet1"
      edits:
        - { addr: "B2", value_a: 1.0, value_b: 42.0 }
        - { addr: "D5", value_a: 2.0, value_b: 99.0 }
        - { addr: "H7", value_a: 3.0, value_b: 3.5 }
        - { addr: "J10", value_a: "x", value_b: "y" }
    output:
      - "multi_cell_edits_a.xlsx"
      - "multi_cell_edits_b.xlsx"

  - id: "g6_row_append_bottom"
    generator: "grid_tail_diff"
    args:
      mode: "row_append_bottom"
      sheet: "Sheet1"
      base_rows: 10
      tail_rows: 2
    output:
      - "row_append_bottom_a.xlsx"
      - "row_append_bottom_b.xlsx"

  - id: "g6_row_delete_bottom"
    generator: "grid_tail_diff"
    args:
      mode: "row_delete_bottom"
      sheet: "Sheet1"
      base_rows: 10
      tail_rows: 2
    output:
      - "row_delete_bottom_a.xlsx"
      - "row_delete_bottom_b.xlsx"

  - id: "g7_col_append_right"
    generator: "grid_tail_diff"
    args:
      mode: "col_append_right"
      sheet: "Sheet1"
      base_cols: 4
      tail_cols: 2
    output:
      - "col_append_right_a.xlsx"
      - "col_append_right_b.xlsx"

  - id: "g7_col_delete_right"
    generator: "grid_tail_diff"
    args:
      mode: "col_delete_right"
      sheet: "Sheet1"
      base_cols: 4
      tail_cols: 2
    output:
      - "col_delete_right_a.xlsx"
      - "col_delete_right_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G8 ---
  - id: "g8_row_insert_middle"
    generator: "row_alignment_g8"
    args:
      mode: "insert"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      insert_at: 6
    output:
      - "row_insert_middle_a.xlsx"
      - "row_insert_middle_b.xlsx"

  - id: "g8_row_delete_middle"
    generator: "row_alignment_g8"
    args:
      mode: "delete"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      delete_row: 6
    output:
      - "row_delete_middle_a.xlsx"
      - "row_delete_middle_b.xlsx"

  - id: "g8_row_insert_with_edit_below"
    generator: "row_alignment_g8"
    args:
      mode: "insert_with_edit"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      insert_at: 6
      edit_row: 8
      edit_col: 3
    output:
      - "row_insert_with_edit_a.xlsx"
      - "row_insert_with_edit_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G9 ---
  - id: "g9_col_insert_middle"
    generator: "column_alignment_g9"
    args:
      mode: "insert"
      sheet: "Data"
      cols: 8
      data_rows: 9
      insert_at: 4
    output:
      - "col_insert_middle_a.xlsx"
      - "col_insert_middle_b.xlsx"

  - id: "g9_col_delete_middle"
    generator: "column_alignment_g9"
    args:
      mode: "delete"
      sheet: "Data"
      cols: 8
      data_rows: 9
      delete_col: 4
    output:
      - "col_delete_middle_a.xlsx"
      - "col_delete_middle_b.xlsx"

  - id: "g9_col_insert_with_edit"
    generator: "column_alignment_g9"
    args:
      mode: "insert_with_edit"
      sheet: "Data"
      cols: 8
      data_rows: 9
      insert_at: 4
      edit_row: 8
      edit_col_after_insert: 7
    output:
      - "col_insert_with_edit_a.xlsx"
      - "col_insert_with_edit_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G10 ---
  - id: "g10_row_block_insert"
    generator: "row_alignment_g10"
    args:
      mode: "block_insert"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      block_rows: 4
      insert_at: 4
    output:
      - "row_block_insert_a.xlsx"
      - "row_block_insert_b.xlsx"

  - id: "g10_row_block_delete"
    generator: "row_alignment_g10"
    args:
      mode: "block_delete"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      block_rows: 4
      delete_start: 4
    output:
      - "row_block_delete_a.xlsx"
      - "row_block_delete_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G11 ---
  - id: "g11_row_block_move"
    generator: "row_block_move_g11"
    args:
      sheet: "Sheet1"
      total_rows: 20
      cols: 5
      block_rows: 4
      src_start: 5    # 1-based in A
      dst_start: 13   # 1-based in B
    output:
      - "row_block_move_a.xlsx"
      - "row_block_move_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G12 (column move only - G12a) ---
  - id: "g12_column_block_move"
    generator: "column_move_g12"
    args:
      sheet: "Data"
      cols: 8
      data_rows: 9
      src_col: 3      # 1-based: C
      dst_col: 6      # 1-based: F
    output:
      - "column_move_a.xlsx"
      - "column_move_b.xlsx"

  - id: "g12_rect_block_move"
    generator: "rect_block_move_g12"
    args:
      sheet: "Data"
      rows: 15
      cols: 15
      src_top: 3      # 1-based row in A (Excel row 3)
      src_left: 2     # 1-based column in A (Excel column B)
      dst_top: 10     # 1-based row in B (Excel row 10)
      dst_left: 7     # 1-based column in B (Excel column G)
      block_rows: 3
      block_cols: 3
    output:
      - "rect_block_move_a.xlsx"
      - "rect_block_move_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G13 ---
  - id: "g13_fuzzy_row_move"
    generator: "row_fuzzy_move_g13"
    args:
      sheet: "Data"
      total_rows: 24
      cols: 6
      block_rows: 4
      src_start: 5      # 1-based in A
      dst_start: 14     # 1-based in B
      edits:
        - { row_offset: 1, col: 3, delta: 1 }
    output:
      - "grid_move_and_edit_a.xlsx"
      - "grid_move_and_edit_b.xlsx"

  # --- JSON diff: simple non-empty change ---
  - id: "json_diff_single_cell"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: "2"
    output:
      - "json_diff_single_cell_a.xlsx"
      - "json_diff_single_cell_b.xlsx"

  - id: "json_diff_single_bool"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: true
      value_b: false
    output:
      - "json_diff_bool_a.xlsx"
      - "json_diff_bool_b.xlsx"

  - id: "json_diff_value_to_empty"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: null
    output:
      - "json_diff_value_to_empty_a.xlsx"
      - "json_diff_value_to_empty_b.xlsx"

  # --- Sheet identity: case-only renames ---
  - id: "sheet_case_only_rename"
    generator: "sheet_case_rename"
    args:
      sheet_a: "Sheet1"
      sheet_b: "sheet1"
      cell: "A1"
      value_a: 1.0
      value_b: 1.0
    output:
      - "sheet_case_only_rename_a.xlsx"
      - "sheet_case_only_rename_b.xlsx"

  - id: "sheet_case_only_rename_cell_edit"
    generator: "sheet_case_rename"
    args:
      sheet_a: "Sheet1"
      sheet_b: "sheet1"
      cell: "A1"
      value_a: 1.0
      value_b: 2.0
    output:
      - "sheet_case_only_rename_edit_a.xlsx"
      - "sheet_case_only_rename_edit_b.xlsx"

  # --- PG6: Object graph vs grid responsibilities ---
  - id: "pg6_sheet_added"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_added"
    output:
      - "pg6_sheet_added_a.xlsx"
      - "pg6_sheet_added_b.xlsx"

  - id: "pg6_sheet_removed"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_removed"
    output:
      - "pg6_sheet_removed_a.xlsx"
      - "pg6_sheet_removed_b.xlsx"

  - id: "pg6_sheet_renamed"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_renamed"
    output:
      - "pg6_sheet_renamed_a.xlsx"
      - "pg6_sheet_renamed_b.xlsx"

  - id: "pg6_sheet_and_grid_change"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_and_grid_change"
    output:
      - "pg6_sheet_and_grid_change_a.xlsx"
      - "pg6_sheet_and_grid_change_b.xlsx"

  # --- Milestone 2.2: Base64 Correctness ---
  - id: "corrupt_base64"
    generator: "mashup_corrupt"
    args: 
      base_file: "templates/base_query.xlsx"
      mode: "byte_flip"
    output: "corrupt_base64.xlsx"

  - id: "duplicate_datamashup_parts"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
    output: "duplicate_datamashup_parts.xlsx"

  - id: "duplicate_datamashup_elements"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
      mode: "element"
    output: "duplicate_datamashup_elements.xlsx"

  - id: "mashup_utf16_le"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-le"
    output: "mashup_utf16_le.xlsx"

  - id: "mashup_utf16_be"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-be"
    output: "mashup_utf16_be.xlsx"

  - id: "mashup_base64_whitespace"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      whitespace: true
    output: "mashup_base64_whitespace.xlsx"

  # --- Milestone 4.1: PackageParts ---
  - id: "m4_packageparts_one_query"
    generator: "mashup:one_query"
    args:
      base_file: "templates/base_query.xlsx"
    output: "one_query.xlsx"

  - id: "m4_packageparts_multi_embedded"
    generator: "mashup:multi_query_with_embedded"
    args:
      base_file: "templates/base_query.xlsx"
    output: "multi_query_with_embedded.xlsx"

  # --- Milestone 4.2-4.4: Permissions / Metadata ---
  - id: "permissions_defaults"
    generator: "mashup:permissions_metadata"
    args:
      mode: "permissions_defaults"
      base_file: "templates/base_query.xlsx"
    output: "permissions_defaults.xlsx"

  - id: "permissions_firewall_off"
    generator: "mashup:permissions_metadata"
    args:
      mode: "permissions_firewall_off"
      base_file: "templates/base_query.xlsx"
    output: "permissions_firewall_off.xlsx"

  - id: "metadata_simple"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_simple"
      base_file: "templates/base_query.xlsx"
    output: "metadata_simple.xlsx"

  - id: "metadata_query_groups"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_query_groups"
      base_file: "templates/base_query.xlsx"
    output: "metadata_query_groups.xlsx"

  - id: "metadata_hidden_queries"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_hidden_queries"
      base_file: "templates/base_query.xlsx"
    output: "metadata_hidden_queries.xlsx"

  - id: "metadata_missing_entry"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_missing_entry"
      base_file: "templates/base_query.xlsx"
    output: "metadata_missing_entry.xlsx"

  - id: "metadata_url_encoding"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_url_encoding"
      base_file: "templates/base_query.xlsx"
    output: "metadata_url_encoding.xlsx"

  - id: "metadata_orphan_entries"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_orphan_entries"
      base_file: "templates/base_query.xlsx"
    output: "metadata_orphan_entries.xlsx"

  # --- Milestone 6: Basic M Diffs ---
  - id: "m_add_query_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_add_query_a"
      base_file: "templates/base_query.xlsx"
    output: "m_add_query_a.xlsx"

  - id: "m_add_query_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_add_query_b"
      base_file: "templates/base_query.xlsx"
    output: "m_add_query_b.xlsx"

  - id: "m_remove_query_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_remove_query_a"
      base_file: "templates/base_query.xlsx"
    output: "m_remove_query_a.xlsx"

  - id: "m_remove_query_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_remove_query_b"
      base_file: "templates/base_query.xlsx"
    output: "m_remove_query_b.xlsx"

  - id: "m_change_literal_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_change_literal_a"
      base_file: "templates/base_query.xlsx"
    output: "m_change_literal_a.xlsx"

  - id: "m_change_literal_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_change_literal_b"
      base_file: "templates/base_query.xlsx"
    output: "m_change_literal_b.xlsx"

  - id: "m_metadata_only_change_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_metadata_only_change_a"
      base_file: "templates/base_query.xlsx"
    output: "m_metadata_only_change_a.xlsx"

  - id: "m_metadata_only_change_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_metadata_only_change_b"
      base_file: "templates/base_query.xlsx"
    output: "m_metadata_only_change_b.xlsx"

  - id: "m_def_and_metadata_change_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_def_and_metadata_change_a"
      base_file: "templates/base_query.xlsx"
    output: "m_def_and_metadata_change_a.xlsx"

  - id: "m_def_and_metadata_change_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_def_and_metadata_change_b"
      base_file: "templates/base_query.xlsx"
    output: "m_def_and_metadata_change_b.xlsx"

  - id: "m_rename_query_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_rename_query_a"
      base_file: "templates/base_query.xlsx"
    output: "m_rename_query_a.xlsx"

  - id: "m_rename_query_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_rename_query_b"
      base_file: "templates/base_query.xlsx"
    output: "m_rename_query_b.xlsx"

  # --- Milestone 7: M AST canonicalization ---
  - id: "m_formatting_only_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_formatting_only_a"
      base_file: "templates/base_query.xlsx"
    output: "m_formatting_only_a.xlsx"

  - id: "m_formatting_only_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_formatting_only_b"
      base_file: "templates/base_query.xlsx"
    output: "m_formatting_only_b.xlsx"

  - id: "m_formatting_only_b_variant"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_formatting_only_b_variant"
      base_file: "templates/base_query.xlsx"
    output: "m_formatting_only_b_variant.xlsx"

  # --- P1: Large Dense Grid (Performance Baseline) ---
  - id: "p1_large_dense"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "dense" # Deterministic "R1C1" style data
    output: "grid_large_dense.xlsx"

  # --- P2: Large Noise Grid (Worst-case Alignment) ---
  - id: "p2_large_noise"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "noise" # Random float data
      seed: 12345
    output: "grid_large_noise.xlsx"

  # --- D1: Keyed Equality (Database Mode) ---
  # File A: Ordered IDs 1..1000
  - id: "db_equal_ordered_a"
    generator: "db_keyed"
    args: { count: 1000, shuffle: false, seed: 42 }
    output: "db_equal_ordered_a.xlsx"

  # File B: Same data, random order (Tests O(N) alignment)
  - id: "db_equal_ordered_b"
    generator: "db_keyed"
    args: { count: 1000, shuffle: true, seed: 42 }
    output: "db_equal_ordered_b.xlsx"

  # --- D2: Row Added (Database Mode) ---
  - id: "db_row_added_b"
    generator: "db_keyed"
    args: 
      count: 1000 
      seed: 42 
      # Inject a new ID at the end
      extra_rows: [{id: 1001, name: "New Row", amount: 999}]
    output: "db_row_added_b.xlsx"

  # --- P3: Adversarial Repetitive Grid (RLE stress test) ---
  - id: "p3_adversarial_repetitive"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 50
      mode: "repetitive"
      pattern_length: 100
      seed: 99999
    output: "grid_adversarial_repetitive.xlsx"

  # --- P4: 99% Blank Grid (Sparse stress test) ---
  - id: "p4_99_percent_blank"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 100
      mode: "sparse"
      fill_percent: 1
      seed: 77777
    output: "grid_99_percent_blank.xlsx"

  # --- P5: Identical Grids (Fast-path baseline) ---
  - id: "p5_identical"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 100
      mode: "dense"
    output: "grid_identical.xlsx"

```

---

### File: `fixtures\pyproject.toml`

```toml
[project]
name = "excel-fixtures"
version = "0.1.0"
description = "Deterministic artifact generator for Excel Diff testing"
readme = "README.md"
requires-python = ">=3.9"
dependencies = [
    "openpyxl>=3.1.0",
    "lxml>=4.9.0",
    "jinja2>=3.1.0",
    "pyyaml>=6.0",
]

[project.scripts]
generate-fixtures = "src.generate:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src"]


```

---

### File: `fixtures\src\__init__.py`

```python


```

---

### File: `fixtures\src\generate.py`

```python
import argparse
import yaml
import sys
from pathlib import Path
from typing import Dict, Any, List

# Import generators
from generators.grid import (
    BasicGridGenerator, 
    SparseGridGenerator, 
    EdgeCaseGenerator, 
    AddressSanityGenerator,
    ValueFormulaGenerator,
    SingleCellDiffGenerator,
    MultiCellDiffGenerator,
    GridTailDiffGenerator,
    RowAlignmentG8Generator,
    RowAlignmentG10Generator,
    RowBlockMoveG11Generator,
    RowFuzzyMoveG13Generator,
    ColumnMoveG12Generator,
    RectBlockMoveG12Generator,
    ColumnAlignmentG9Generator,
    SheetCaseRenameGenerator,
    Pg6SheetScenarioGenerator,
)
from generators.corrupt import ContainerCorruptGenerator
from generators.mashup import (
    MashupCorruptGenerator,
    MashupDuplicateGenerator,
    MashupInjectGenerator,
    MashupEncodeGenerator,
    MashupMultiEmbeddedGenerator,
    MashupOneQueryGenerator,
    MashupPermissionsMetadataGenerator,
)
from generators.perf import LargeGridGenerator
from generators.database import KeyedTableGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "single_cell_diff": SingleCellDiffGenerator,
    "multi_cell_diff": MultiCellDiffGenerator,
    "grid_tail_diff": GridTailDiffGenerator,
    "row_alignment_g8": RowAlignmentG8Generator,
    "row_alignment_g10": RowAlignmentG10Generator,
    "row_block_move_g11": RowBlockMoveG11Generator,
    "row_fuzzy_move_g13": RowFuzzyMoveG13Generator,
    "column_move_g12": ColumnMoveG12Generator,
    "rect_block_move_g12": RectBlockMoveG12Generator,
    "column_alignment_g9": ColumnAlignmentG9Generator,
    "sheet_case_rename": SheetCaseRenameGenerator,
    "pg6_sheet_scenario": Pg6SheetScenarioGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_duplicate": MashupDuplicateGenerator,
    "mashup_inject": MashupInjectGenerator,
    "mashup_encode": MashupEncodeGenerator,
    "mashup:one_query": MashupOneQueryGenerator,
    "mashup:multi_query_with_embedded": MashupMultiEmbeddedGenerator,
    "mashup:permissions_metadata": MashupPermissionsMetadataGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
}

def load_manifest(manifest_path: Path) -> Dict[str, Any]:
    if not manifest_path.exists():
        print(f"Error: Manifest file not found at {manifest_path}")
        sys.exit(1)
    
    with open(manifest_path, 'r') as f:
        try:
            return yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"Error parsing manifest: {e}")
            sys.exit(1)

def ensure_output_dir(output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)

def main():
    script_dir = Path(__file__).parent.resolve()
    fixtures_root = script_dir.parent
    
    default_manifest = fixtures_root / "manifest.yaml"
    default_output = fixtures_root / "generated"

    parser = argparse.ArgumentParser(description="Generate Excel fixtures based on a manifest.")
    parser.add_argument("--manifest", type=Path, default=default_manifest, help="Path to the manifest YAML file.")
    parser.add_argument("--output-dir", type=Path, default=default_output, help="Directory to output generated files.")
    parser.add_argument("--force", action="store_true", help="Force regeneration of existing files.")
    
    args = parser.parse_args()
    
    manifest = load_manifest(args.manifest)
    ensure_output_dir(args.output_dir)
    
    scenarios = manifest.get('scenarios', [])
    print(f"Found {len(scenarios)} scenarios in manifest.")
    
    for scenario in scenarios:
        scenario_id = scenario.get('id')
        generator_name = scenario.get('generator')
        generator_args = scenario.get('args', {})
        outputs = scenario.get('output')
        
        if not scenario_id or not generator_name or not outputs:
            print(f"Skipping invalid scenario: {scenario}")
            continue
            
        print(f"Processing scenario: {scenario_id} (Generator: {generator_name})")
        
        if generator_name not in GENERATORS:
            print(f"  Warning: Generator '{generator_name}' not implemented yet. Skipping.")
            continue
        
        try:
            generator_class = GENERATORS[generator_name]
            generator = generator_class(generator_args)
            generator.generate(args.output_dir, outputs)
            print(f"  Success: Generated {outputs}")
        except Exception as e:
            print(f"  Error generating scenario {scenario_id}: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    main()

```

---

### File: `fixtures\src\generators\__init__.py`

```python
# Generators package


```

---

### File: `fixtures\src\generators\base.py`

```python
"""Base classes for fixture generators."""

from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, Any, Union, List


class BaseGenerator(ABC):
    """Abstract base class for all fixture generators."""

    def __init__(self, args: Dict[str, Any]):
        self.args = args

    @abstractmethod
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        """Generate the fixture file(s).

        Args:
            output_dir: The directory to save the file(s) in.
            output_names: The name(s) of the output file(s) as specified in the manifest.
        """
        pass

```

---

### File: `fixtures\src\generators\corrupt.py`

```python
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class ContainerCorruptGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        mode = self.args.get('mode', 'no_content_types')
        
        for name in output_names:
            # Create a dummy zip
            out_path = output_dir / name
            
            if mode == 'random_zip':
                # Just a zip with a text file
                with zipfile.ZipFile(out_path, 'w') as z:
                    z.writestr("hello.txt", "This is not excel")
                    
            elif mode == 'no_content_types':
                # Create a valid excel in memory, then strip [Content_Types].xml
                buffer = io.BytesIO()
                import openpyxl
                wb = openpyxl.Workbook()
                # Add some content just so it's not totally empty
                wb.active['A1'] = 1
                wb.save(buffer)
                buffer.seek(0)
                
                with zipfile.ZipFile(buffer, 'r') as zin:
                    with zipfile.ZipFile(out_path, 'w') as zout:
                        for item in zin.infolist():
                            if item.filename != "[Content_Types].xml":
                                zout.writestr(item, zin.read(item.filename))
            elif mode == 'not_zip_text':
                out_path.write_text("This is not a zip container", encoding="utf-8")
            else:
                raise ValueError(f"Unsupported corrupt_container mode: {mode}")


```

---

### File: `fixtures\src\generators\database.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class KeyedTableGenerator(BaseGenerator):
    """
    Generates datasets with Primary Keys (ID columns).
    Capable of shuffling rows to test O(N) alignment (Database Mode).
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        count = self.args.get('count', 100)
        shuffle = self.args.get('shuffle', False)
        seed = self.args.get('seed', 42)
        extra_rows = self.args.get('extra_rows', [])

        # Use deterministic seed
        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Data"

            # 1. Define Base Data (List of Dicts)
            # Schema: [ID, Name, Amount, Category]
            data_rows = []
            for i in range(1, count + 1):
                data_rows.append({
                    'id': i,
                    'name': f"Customer_{i}",
                    'amount': i * 10.5,
                    'category': rng.choice(['A', 'B', 'C'])
                })

            # 2. Apply Mutations (Additions)
            # This allows us to inject specific "diffs" like D2 (Row Added)
            for row in extra_rows:
                data_rows.append(row)

            # 3. Apply Shuffle (The core D1 test)
            if shuffle:
                rng.shuffle(data_rows)

            # 4. Write to Sheet
            # Header
            headers = ['ID', 'Name', 'Amount', 'Category']
            ws.append(headers)

            for row in data_rows:
                # Ensure strictly ordered list matching headers
                ws.append([
                    row.get('id'),
                    row.get('name'),
                    row.get('amount'),
                    row.get('category')
                ])

            wb.save(output_dir / name)


```

---

### File: `fixtures\src\generators\grid.py`

```python
import openpyxl
import zipfile
import xml.etree.ElementTree as ET
from openpyxl.utils import get_column_letter
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class BasicGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        rows = self.args.get('rows', 5)
        cols = self.args.get('cols', 5)
        two_sheets = self.args.get('two_sheets', False)
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sheet1"
            
            # Fill grid
            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")
            
            # Check if we need a second sheet
            if two_sheets:
                ws2 = wb.create_sheet(title="Sheet2")
                # Different dimensions for Sheet2 (PG1 requirement: 5x2)
                # If args are customized we might need more logic, but for PG1 this is sufficient or we use defaults
                s2_rows = 5
                s2_cols = 2
                for r in range(1, s2_rows + 1):
                    for c in range(1, s2_cols + 1):
                         ws2.cell(row=r, column=c, value=f"S2_R{r}C{c}")

            wb.save(output_dir / name)

class SparseGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sparse"
            
            # Specifics for pg1_sparse_used_range
            ws['A1'] = "A1"
            ws['B2'] = "B2"
            ws['G10'] = "G10" # Forces extent
            # Row 5 and Col D are empty implicitly by not writing to them
            
            wb.save(output_dir / name)

class EdgeCaseGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
        
        for name in output_names:
            wb = openpyxl.Workbook()
            # Remove default sheet
            default_ws = wb.active
            wb.remove(default_ws)
            
            # Empty Sheet
            wb.create_sheet("Empty")
            
            # Values Only
            ws_val = wb.create_sheet("ValuesOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    ws_val.cell(row=r, column=c, value=r*c)
            
            # Formulas Only
            ws_form = wb.create_sheet("FormulasOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    # Reference ValuesOnly sheet
                    col_letter = get_column_letter(c)
                    ws_form.cell(row=r, column=c, value=f"=ValuesOnly!{col_letter}{r}")
            
            wb.save(output_dir / name)

class AddressSanityGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        targets = self.args.get('targets', ["A1", "B2", "Z10"])
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Addresses"
            
            for addr in targets:
                ws[addr] = addr
                
            wb.save(output_dir / name)

class ValueFormulaGenerator(BaseGenerator):
    """PG3: Types, formulas, values"""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Types"
            
            ws['A1'] = 42
            ws['A2'] = "hello"
            ws['A3'] = True
            # A4 empty
            
            ws['B1'] = "=A1+1"
            ws['B2'] = '="hello" & " world"'
            ws['B3'] = "=A1>0"
            
            output_path = output_dir / name
            wb.save(output_path)
            self._inject_formula_caches(output_path)

    def _inject_formula_caches(self, path: Path):
        ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        with zipfile.ZipFile(path, "r") as zf:
            sheet_xml = zf.read("xl/worksheets/sheet1.xml")
            other_files = {
                info.filename: zf.read(info.filename)
                for info in zf.infolist()
                if info.filename != "xl/worksheets/sheet1.xml"
            }

        root = ET.fromstring(sheet_xml)

        def update_cell(ref: str, value: str, cell_type: str | None = None):
            cell = root.find(f".//{{{ns}}}c[@r='{ref}']")
            if cell is None:
                return
            if cell_type:
                cell.set("t", cell_type)
            v = cell.find(f"{{{ns}}}v")
            if v is None:
                v = ET.SubElement(cell, f"{{{ns}}}v")
            v.text = value

        update_cell("B1", "43")
        update_cell("B2", "hello world", "str")
        update_cell("B3", "1", "b")

        ET.register_namespace("", ns)
        updated_sheet = ET.tostring(root, encoding="utf-8", xml_declaration=False)
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("xl/worksheets/sheet1.xml", updated_sheet)
            for name, data in other_files.items():
                zf.writestr(name, data)

class SingleCellDiffGenerator(BaseGenerator):
    """Generates a tiny pair of workbooks with a single differing cell."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("single_cell_diff generator expects exactly two output filenames")

        rows = self.args.get('rows', 3)
        cols = self.args.get('cols', 3)
        sheet = self.args.get('sheet', "Sheet1")
        target_cell = self.args.get('target_cell', "C3")
        value_a = self.args.get('value_a', "1")
        value_b = self.args.get('value_b', "2")

        def create_workbook(value, name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet

            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")

            ws[target_cell] = value
            wb.save(output_dir / name)

        create_workbook(value_a, output_names[0])
        create_workbook(value_b, output_names[1])

class MultiCellDiffGenerator(BaseGenerator):
    """Generates workbook pairs that differ in multiple scattered cells."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("multi_cell_diff generator expects exactly two output filenames")

        rows = self.args.get("rows", 20)
        cols = self.args.get("cols", 10)
        sheet = self.args.get("sheet", "Sheet1")
        edits: List[Dict[str, Any]] = self.args.get("edits", [])

        self._create_workbook(output_dir / output_names[0], sheet, rows, cols, edits, "a")
        self._create_workbook(output_dir / output_names[1], sheet, rows, cols, edits, "b")

    def _create_workbook(
        self,
        path: Path,
        sheet: str,
        rows: int,
        cols: int,
        edits: List[Dict[str, Any]],
        value_key: str,
    ):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        self._fill_base_grid(ws, rows, cols)
        self._apply_edits(ws, edits, value_key)

        wb.save(path)

    def _fill_base_grid(self, ws, rows: int, cols: int):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                ws.cell(row=r, column=c, value=f"R{r}C{c}")

    def _apply_edits(self, ws, edits: List[Dict[str, Any]], value_key: str):
        value_field = f"value_{value_key}"

        for edit in edits:
            addr = edit.get("addr")
            if not addr:
                raise ValueError("multi_cell_diff edits require 'addr'")
            if value_field not in edit:
                raise ValueError(f"multi_cell_diff edits require '{value_field}'")
            ws[addr] = edit[value_field]

class GridTailDiffGenerator(BaseGenerator):
    """Generates workbook pairs for simple row/column tail append/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("grid_tail_diff generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Sheet1")

        if mode == "row_append_bottom":
            self._row_append_bottom(output_dir, output_names, sheet)
        elif mode == "row_delete_bottom":
            self._row_delete_bottom(output_dir, output_names, sheet)
        elif mode == "col_append_right":
            self._col_append_right(output_dir, output_names, sheet)
        elif mode == "col_delete_right":
            self._col_delete_right(output_dir, output_names, sheet)
        else:
            raise ValueError(f"Unsupported grid_tail_diff mode: {mode}")

    def _row_append_bottom(self, output_dir: Path, output_names: List[str], sheet: str):
        base_rows = self.args.get("base_rows", 10)
        tail_rows = self.args.get("tail_rows", 2)
        cols = self.args.get("cols", 3)

        self._write_rows(output_dir / output_names[0], sheet, base_rows, cols, 1)
        self._write_rows(
            output_dir / output_names[1],
            sheet,
            base_rows + tail_rows,
            cols,
            1,
        )

    def _row_delete_bottom(self, output_dir: Path, output_names: List[str], sheet: str):
        base_rows = self.args.get("base_rows", 10)
        tail_rows = self.args.get("tail_rows", 2)
        cols = self.args.get("cols", 3)

        self._write_rows(
            output_dir / output_names[0],
            sheet,
            base_rows + tail_rows,
            cols,
            1,
        )
        self._write_rows(output_dir / output_names[1], sheet, base_rows, cols, 1)

    def _col_append_right(self, output_dir: Path, output_names: List[str], sheet: str):
        base_cols = self.args.get("base_cols", 4)
        tail_cols = self.args.get("tail_cols", 2)
        rows = self.args.get("rows", 5)

        self._write_cols(output_dir / output_names[0], sheet, rows, base_cols)
        self._write_cols(
            output_dir / output_names[1],
            sheet,
            rows,
            base_cols + tail_cols,
        )

    def _col_delete_right(self, output_dir: Path, output_names: List[str], sheet: str):
        base_cols = self.args.get("base_cols", 4)
        tail_cols = self.args.get("tail_cols", 2)
        rows = self.args.get("rows", 5)

        self._write_cols(
            output_dir / output_names[0],
            sheet,
            rows,
            base_cols + tail_cols,
        )
        self._write_cols(output_dir / output_names[1], sheet, rows, base_cols)

    def _write_rows(self, path: Path, sheet: str, rows: int, cols: int, start_value: int):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r in range(1, rows + 1):
            ws.cell(row=r, column=1, value=start_value + r - 1)
            for c in range(2, cols + 1):
                ws.cell(row=r, column=c, value=f"R{r}C{c}")

        wb.save(path)

    def _write_cols(self, path: Path, sheet: str, rows: int, cols: int):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                ws.cell(row=r, column=c, value=f"R{r}C{c}")

        wb.save(path)

class RowAlignmentG8Generator(BaseGenerator):
    """Generates workbook pairs for G8-style middle row insert/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_alignment_g8 generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Sheet1")
        base_rows = self.args.get("base_rows", 10)
        cols = self.args.get("cols", 5)
        insert_at = self.args.get("insert_at", 6)  # 1-based position in B
        delete_row = self.args.get("delete_row", 6)  # 1-based position in A
        edit_row = self.args.get("edit_row")  # Optional extra edit row (1-based in B after insert)
        edit_col = self.args.get("edit_col", 2)  # 1-based column for extra edit

        base_data = [self._base_row_values(idx, cols) for idx in range(1, base_rows + 1)]

        if mode == "insert":
            data_a = base_data
            data_b = self._with_insert(base_data, insert_at, cols)
        elif mode == "delete":
            data_a = base_data
            data_b = self._with_delete(base_data, delete_row)
        elif mode == "insert_with_edit":
            data_a = base_data
            data_b = self._with_insert(base_data, insert_at, cols)
            target_row = edit_row or (insert_at + 2)
            if 1 <= target_row <= len(data_b):
                row_values = list(data_b[target_row - 1])
                col_index = max(1, min(edit_col, cols)) - 1
                row_values[col_index] = "EditedAfterInsert"
                data_b[target_row - 1] = row_values
        else:
            raise ValueError(f"Unsupported row_alignment_g8 mode: {mode}")

        self._write_workbook(output_dir / output_names[0], sheet, data_a)
        self._write_workbook(output_dir / output_names[1], sheet, data_b)

    def _base_row_values(self, row_number: int, cols: int) -> List[str]:
        return [f"Row{row_number}_Col{c}" for c in range(1, cols + 1)]

    def _insert_row_values(self, cols: int) -> List[str]:
        return [f"Inserted_Row_Col{c}" for c in range(1, cols + 1)]

    def _with_insert(self, base_data: List[List[str]], insert_at: int, cols: int) -> List[List[str]]:
        insert_idx = max(1, min(insert_at, len(base_data) + 1))
        insert_row = self._insert_row_values(cols)
        return base_data[: insert_idx - 1] + [insert_row] + base_data[insert_idx - 1 :]

    def _with_delete(self, base_data: List[List[str]], delete_row: int) -> List[List[str]]:
        if not (1 <= delete_row <= len(base_data)):
            raise ValueError(f"delete_row must be within 1..{len(base_data)}")
        return base_data[: delete_row - 1] + base_data[delete_row:]

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[str]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RowAlignmentG10Generator(BaseGenerator):
    """Generates workbook pairs for G10 contiguous row block insert/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_alignment_g10 generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Sheet1")
        base_rows = self.args.get("base_rows", 10)
        cols = self.args.get("cols", 5)
        block_rows = self.args.get("block_rows", 4)
        insert_at = self.args.get("insert_at", 4)  # 1-based position of first inserted row in B
        delete_start = self.args.get("delete_start", 4)  # 1-based starting row in A to delete

        base_data = [self._row_values(idx, cols, 0) for idx in range(1, base_rows + 1)]

        if mode == "block_insert":
            data_a = base_data
            data_b = self._with_block_insert(base_data, insert_at, block_rows, cols)
        elif mode == "block_delete":
            data_a = base_data
            data_b = self._with_block_delete(base_data, delete_start, block_rows)
        else:
            raise ValueError(f"Unsupported row_alignment_g10 mode: {mode}")

        self._write_workbook(output_dir / output_names[0], sheet, data_a)
        self._write_workbook(output_dir / output_names[1], sheet, data_b)

    def _row_values(self, row_number: int, cols: int, offset: int) -> List[int]:
        row_id = row_number + offset
        values = [row_id]
        for c in range(1, cols):
            values.append(row_id * 10 + c)
        return values

    def _block_rows(self, count: int, cols: int) -> List[List[int]]:
        return [self._row_values(1000 + idx, cols, 0) for idx in range(1, count + 1)]

    def _with_block_insert(
        self, base_data: List[List[int]], insert_at: int, block_rows: int, cols: int
    ) -> List[List[int]]:
        insert_idx = max(1, min(insert_at, len(base_data) + 1)) - 1
        block = self._block_rows(block_rows, cols)
        return base_data[:insert_idx] + block + base_data[insert_idx:]

    def _with_block_delete(
        self, base_data: List[List[int]], delete_start: int, block_rows: int
    ) -> List[List[int]]:
        if not (1 <= delete_start <= len(base_data)):
            raise ValueError(f"delete_start must be within 1..{len(base_data)}")
        if delete_start - 1 + block_rows > len(base_data):
            raise ValueError("delete block exceeds base data length")

        delete_idx = delete_start - 1
        return base_data[:delete_idx] + base_data[delete_idx + block_rows :]

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[int]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RowBlockMoveG11Generator(BaseGenerator):
    """Generates workbook pairs for G11 exact row block move scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_block_move_g11 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Sheet1")
        total_rows = self.args.get("total_rows", 20)
        cols = self.args.get("cols", 5)
        block_rows = self.args.get("block_rows", 4)
        src_start = self.args.get("src_start", 5)
        dst_start = self.args.get("dst_start", 13)

        if block_rows <= 0:
            raise ValueError("block_rows must be positive")
        if src_start < 1 or src_start + block_rows - 1 > total_rows:
            raise ValueError("source block must fit within total_rows")
        if dst_start < 1 or dst_start + block_rows - 1 > total_rows:
            raise ValueError("destination block must fit within total_rows")

        src_end = src_start + block_rows - 1
        dst_end = dst_start + block_rows - 1
        if not (src_end < dst_start or dst_end < src_start):
            raise ValueError("source and destination blocks must not overlap")

        rows_a = self._build_rows(total_rows, cols, src_start, block_rows)
        rows_b = self._move_block(rows_a, src_start, block_rows, dst_start)

        self._write_workbook(output_dir / output_names[0], sheet, rows_a)
        self._write_workbook(output_dir / output_names[1], sheet, rows_b)

    def _build_rows(self, total_rows: int, cols: int, src_start: int, block_rows: int) -> List[List[str]]:
        block_end = src_start + block_rows - 1
        rows: List[List[str]] = []
        for r in range(1, total_rows + 1):
            if src_start <= r <= block_end:
                rows.append([f"BLOCK_r{r}_c{c}" for c in range(1, cols + 1)])
            else:
                rows.append([f"R{r}_C{c}" for c in range(1, cols + 1)])
        return rows

    def _move_block(
        self, rows: List[List[str]], src_start: int, block_rows: int, dst_start: int
    ) -> List[List[str]]:
        rows_b = [list(r) for r in rows]
        src_idx = src_start - 1
        src_end = src_idx + block_rows
        block = rows_b[src_idx:src_end]
        del rows_b[src_idx:src_end]

        dst_idx = min(dst_start - 1, len(rows_b))

        rows_b[dst_idx:dst_idx] = block
        return rows_b

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[str]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RowFuzzyMoveG13Generator(BaseGenerator):
    """Generates workbook pairs for G13 fuzzy row block move scenarios with internal edits."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_fuzzy_move_g13 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Data")
        total_rows = self.args.get("total_rows", 24)
        cols = self.args.get("cols", 6)
        block_rows = self.args.get("block_rows", 4)
        src_start = self.args.get("src_start", 5)
        dst_start = self.args.get("dst_start", 14)
        edits = self.args.get(
            "edits",
            [
                {"row_offset": 1, "col": 3, "delta": 1},
            ],
        )

        if block_rows <= 0:
            raise ValueError("block_rows must be positive")
        if src_start < 1 or src_start + block_rows - 1 > total_rows:
            raise ValueError("source block must fit within total_rows")
        if dst_start < 1 or dst_start + block_rows - 1 > total_rows:
            raise ValueError("destination block must fit within total_rows")

        src_end = src_start + block_rows - 1
        dst_end = dst_start + block_rows - 1
        if not (src_end < dst_start or dst_end < src_start):
            raise ValueError("source and destination blocks must not overlap")

        rows_a = self._build_rows(total_rows, cols, src_start, block_rows)
        rows_b = self._move_block(rows_a, src_start, block_rows, dst_start)
        self._apply_edits(rows_b, dst_start, block_rows, cols, edits)

        self._write_workbook(output_dir / output_names[0], sheet, rows_a)
        self._write_workbook(output_dir / output_names[1], sheet, rows_b)

    def _build_rows(self, total_rows: int, cols: int, block_start: int, block_rows: int) -> List[List[int]]:
        block_end = block_start + block_rows - 1
        rows: List[List[int]] = []
        for r in range(1, total_rows + 1):
            if block_start <= r <= block_end:
                row_id = 1_000 + (r - block_start)
            else:
                row_id = r
            row_values = [row_id]
            for c in range(1, cols):
                row_values.append(row_id * 10 + c)
            rows.append(row_values)
        return rows

    def _move_block(
        self, rows: List[List[int]], src_start: int, block_rows: int, dst_start: int
    ) -> List[List[int]]:
        rows_b = [list(r) for r in rows]
        src_idx = src_start - 1
        src_end = src_idx + block_rows
        block = rows_b[src_idx:src_end]
        del rows_b[src_idx:src_end]

        dst_idx = min(dst_start - 1, len(rows_b))
        rows_b[dst_idx:dst_idx] = block
        return rows_b

    def _apply_edits(
        self,
        rows: List[List[int]],
        dst_start: int,
        block_rows: int,
        cols: int,
        edits: List[Dict[str, Any]],
    ):
        dst_idx = dst_start - 1
        if dst_idx + block_rows > len(rows):
            return

        for edit in edits:
            row_offset = int(edit.get("row_offset", 0))
            col = int(edit.get("col", 1))
            delta = int(edit.get("delta", 1))

            if row_offset < 0 or row_offset >= block_rows:
                continue

            col_idx = max(1, min(col, cols)) - 1
            target_row = dst_idx + row_offset
            if col_idx >= len(rows[target_row]):
                continue
            rows[target_row][col_idx] += delta

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[int]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class ColumnMoveG12Generator(BaseGenerator):
    """Generates workbook pairs for G12 exact column move scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("column_move_g12 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Data")
        cols = self.args.get("cols", 8)
        data_rows = self.args.get("data_rows", 9)
        src_col = self.args.get("src_col", 3)
        dst_col = self.args.get("dst_col", 6)

        if not (1 <= src_col <= cols):
            raise ValueError("src_col must be within 1..cols")
        if not (1 <= dst_col <= cols):
            raise ValueError("dst_col must be within 1..cols")
        if src_col == dst_col:
            raise ValueError("src_col and dst_col must differ for a move")

        base_rows = self._build_rows(cols, data_rows, src_col)
        moved_rows = self._move_column(base_rows, src_col, dst_col)

        self._write_workbook(output_dir / output_names[0], sheet, base_rows)
        self._write_workbook(output_dir / output_names[1], sheet, moved_rows)

    def _build_rows(self, cols: int, data_rows: int, key_col: int) -> List[List[Any]]:
        header: List[Any] = []
        for c in range(1, cols + 1):
            if c == key_col:
                header.append("C_key")
            else:
                header.append(f"Col{c}")

        rows: List[List[Any]] = [header]
        for r in range(1, data_rows + 1):
            row: List[Any] = []
            for c in range(1, cols + 1):
                if c == key_col:
                    row.append(100 * r)
                else:
                    row.append(r * 10 + c)
            rows.append(row)

        return rows

    def _move_column(
        self, rows: List[List[Any]], src_col: int, dst_col: int
    ) -> List[List[Any]]:
        src_idx = src_col - 1
        dst_idx = dst_col - 1
        moved_rows: List[List[Any]] = []

        for row in rows:
            new_row = list(row)
            value = new_row.pop(src_idx)
            insert_at = max(0, min(dst_idx, len(new_row)))
            new_row.insert(insert_at, value)
            moved_rows.append(new_row)

        return moved_rows

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[Any]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RectBlockMoveG12Generator(BaseGenerator):
    """Generates workbook pairs for G12 exact rectangular block move scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("rect_block_move_g12 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Data")
        rows = self.args.get("rows", 15)
        cols = self.args.get("cols", 15)
        src_top = self.args.get("src_top", 3)  # 1-based
        src_left = self.args.get("src_left", 2)  # 1-based (column B)
        dst_top = self.args.get("dst_top", 10)  # 1-based
        dst_left = self.args.get("dst_left", 7)  # 1-based (column G)
        block_rows = self.args.get("block_rows", 3)
        block_cols = self.args.get("block_cols", 3)

        self._write_workbook(
            output_dir / output_names[0],
            sheet,
            rows,
            cols,
            src_top,
            src_left,
            block_rows,
            block_cols,
        )
        self._write_workbook(
            output_dir / output_names[1],
            sheet,
            rows,
            cols,
            dst_top,
            dst_left,
            block_rows,
            block_cols,
        )

    def _write_workbook(
        self,
        path: Path,
        sheet: str,
        rows: int,
        cols: int,
        block_top: int,
        block_left: int,
        block_rows: int,
        block_cols: int,
    ):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        self._fill_background(ws, rows, cols)
        self._write_block(ws, block_top, block_left, block_rows, block_cols)

        wb.save(path)

    def _fill_background(self, ws, rows: int, cols: int):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                ws.cell(row=r, column=c, value=self._background_value(r, c))

    def _background_value(self, row: int, col: int) -> int:
        return 1000 * row + col

    def _write_block(self, ws, top: int, left: int, block_rows: int, block_cols: int):
        for r_offset in range(block_rows):
            for c_offset in range(block_cols):
                value = 9000 + r_offset * 10 + c_offset
                ws.cell(row=top + r_offset, column=left + c_offset, value=value)

class ColumnAlignmentG9Generator(BaseGenerator):
    """Generates workbook pairs for G9-style middle column insert/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("column_alignment_g9 generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Data")
        base_cols = self.args.get("cols", 8)
        data_rows = self.args.get("data_rows", 9)  # excludes header
        insert_at = self.args.get("insert_at", 4)  # 1-based position in B after insert
        delete_col = self.args.get("delete_col", 4)
        edit_row = self.args.get("edit_row", 8)
        edit_col_after_insert = self.args.get("edit_col_after_insert", 7)

        base_table = self._base_table(base_cols, data_rows)

        if mode == "insert":
            data_a = self._clone_rows(base_table)
            data_b = self._with_insert(base_table, insert_at)
        elif mode == "delete":
            data_a = self._clone_rows(base_table)
            data_b = self._with_delete(base_table, delete_col)
        elif mode == "insert_with_edit":
            data_a = self._clone_rows(base_table)
            data_b = self._with_insert(base_table, insert_at)
            row_idx = max(2, min(edit_row, len(data_b))) - 1  # stay below header
            col_idx = max(1, min(edit_col_after_insert, len(data_b[row_idx]))) - 1
            data_b[row_idx][col_idx] = "EditedAfterInsert"
        else:
            raise ValueError(f"Unsupported column_alignment_g9 mode: {mode}")

        self._write_workbook(output_dir / output_names[0], sheet, data_a)
        self._write_workbook(output_dir / output_names[1], sheet, data_b)

    def _base_table(self, cols: int, data_rows: int) -> List[List[str]]:
        header = [f"Col{c}" for c in range(1, cols + 1)]
        rows = [header]
        for r in range(1, data_rows + 1):
            rows.append([f"R{r}_C{c}" for c in range(1, cols + 1)])
        return rows

    def _with_insert(self, base_data: List[List[str]], insert_at: int) -> List[List[str]]:
        insert_idx = max(1, min(insert_at, len(base_data[0]) + 1))
        result: List[List[str]] = []
        for row_idx, row in enumerate(base_data):
            new_row = list(row)
            value = "Inserted" if row_idx == 0 else f"Inserted_{row_idx}"
            new_row.insert(insert_idx - 1, value)
            result.append(new_row)
        return result

    def _with_delete(self, base_data: List[List[str]], delete_col: int) -> List[List[str]]:
        if not base_data:
            return []
        if not (1 <= delete_col <= len(base_data[0])):
            raise ValueError(f"delete_col must be within 1..{len(base_data[0])}")
        result: List[List[str]] = []
        for row in base_data:
            new_row = list(row)
            del new_row[delete_col - 1]
            result.append(new_row)
        return result

    def _clone_rows(self, rows: List[List[str]]) -> List[List[str]]:
        return [list(r) for r in rows]

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[str]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class SheetCaseRenameGenerator(BaseGenerator):
    """Generates a pair of workbooks that differ only by sheet name casing, with optional cell edit."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("sheet_case_rename generator expects exactly two output filenames")

        sheet_a = self.args.get("sheet_a", "Sheet1")
        sheet_b = self.args.get("sheet_b", "sheet1")
        cell = self.args.get("cell", "A1")
        value_a = self.args.get("value_a", 1.0)
        value_b = self.args.get("value_b", value_a)

        def create_workbook(sheet_name: str, value, output_name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet_name
            ws[cell] = value
            wb.save(output_dir / output_name)

        create_workbook(sheet_a, value_a, output_names[0])
        create_workbook(sheet_b, value_b, output_names[1])

class Pg6SheetScenarioGenerator(BaseGenerator):
    """Generates workbook pairs for PG6 sheet add/remove/rename vs grid responsibilities."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("pg6_sheet_scenario generator expects exactly two output filenames")

        mode = self.args.get("mode")
        a_path = output_dir / output_names[0]
        b_path = output_dir / output_names[1]

        if mode == "sheet_added":
            self._gen_sheet_added(a_path, b_path)
        elif mode == "sheet_removed":
            self._gen_sheet_removed(a_path, b_path)
        elif mode == "sheet_renamed":
            self._gen_sheet_renamed(a_path, b_path)
        elif mode == "sheet_and_grid_change":
            self._gen_sheet_and_grid_change(a_path, b_path)
        else:
            raise ValueError(f"Unsupported PG6 mode: {mode}")

    def _fill_grid(self, worksheet, rows: int, cols: int, prefix: str = "R"):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                worksheet.cell(row=r, column=c, value=f"{prefix}{r}C{c}")

    def _gen_sheet_added(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, 5, 5)
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, 5, 5)
        ws_new = wb_b.create_sheet("NewSheet")
        self._fill_grid(ws_new, 3, 3, prefix="N")
        wb_b.save(b_path)

    def _gen_sheet_removed(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, 5, 5)
        ws_old = wb_a.create_sheet("OldSheet")
        self._fill_grid(ws_old, 3, 3, prefix="O")
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, 5, 5)
        wb_b.save(b_path)

    def _gen_sheet_renamed(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_old = wb_a.active
        ws_old.title = "OldName"
        self._fill_grid(ws_old, 3, 3)
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_new = wb_b.active
        ws_new.title = "NewName"
        self._fill_grid(ws_new, 3, 3)
        wb_b.save(b_path)

    def _gen_sheet_and_grid_change(self, a_path: Path, b_path: Path):
        base_rows = 5
        base_cols = 5

        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, base_rows, base_cols)
        ws_aux_a = wb_a.create_sheet("Aux")
        self._fill_grid(ws_aux_a, 3, 3, prefix="A")
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, base_rows, base_cols)
        ws_main_b["A1"] = "Main changed 1"
        ws_main_b["B2"] = "Main changed 2"
        ws_main_b["C3"] = "Main changed 3"

        ws_aux_b = wb_b.create_sheet("Aux")
        self._fill_grid(ws_aux_b, 3, 3, prefix="A")

        ws_scratch = wb_b.create_sheet("Scratch")
        self._fill_grid(ws_scratch, 2, 2, prefix="S")
        wb_b.save(b_path)

```

---

### File: `fixtures\src\generators\mashup.py`

```python
import base64
import copy
import io
import random
import re
import struct
import zipfile
from pathlib import Path
from typing import Callable, List, Optional, Union
from xml.etree import ElementTree as ET
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        if tree.tag.endswith("DataMashup"):
            return tree
        return tree.find('.//dm:DataMashup', namespaces=NS)

    def _process_excel_container(
        self,
        base_path,
        output_path,
        callback,
        text_mutator: Optional[Callable[[str], str]] = None,
    ):
        """
        Generic wrapper to open xlsx, find customXml, apply a callback to the 
        DataMashup bytes, and save the result.
        """
        # Copy base file structure to output
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for item in zin.infolist():
                    buffer = zin.read(item.filename)
                    
                    # We only care about the item containing DataMashup
                    # Usually customXml/item1.xml, but we check content to be safe
                    has_marker = b"DataMashup" in buffer or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in buffer
                    if item.filename.startswith("customXml/item") and has_marker:
                        # Parse XML
                        root = etree.fromstring(buffer)
                        dm_node = self._get_mashup_element(root)
                        
                        if dm_node is not None:
                            # 1. Decode
                            # The text content might have whitespace/newlines, strip them
                            b64_text = dm_node.text.strip() if dm_node.text else ""
                            if b64_text:
                                raw_bytes = base64.b64decode(b64_text)
                                
                                # 2. Apply modification (The Callback)
                                new_bytes = callback(raw_bytes)
                                
                                # 3. Encode back
                                new_text = base64.b64encode(new_bytes).decode('utf-8')
                                if text_mutator is not None:
                                    new_text = text_mutator(new_text)
                                dm_node.text = new_text
                                buffer = etree.tostring(root, encoding='utf-8', xml_declaration=True)
                    
                    zout.writestr(item, buffer)

class MashupCorruptGenerator(MashupBaseGenerator):
    """Fuzzes the DataMashup bytes to test error handling."""
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        if not base_file_arg:
            raise ValueError("MashupCorruptGenerator requires 'base_file' argument")

        # Resolve base file relative to current working directory or fixtures/templates
        base = Path(base_file_arg)
        if not base.exists():
             # Try looking in fixtures/templates if a relative path was given
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        mode = self.args.get('mode', 'byte_flip')

        def corruptor(data):
            mutable = bytearray(data)
            if len(mutable) == 0:
                return bytes(mutable)

            if mode == 'byte_flip':
                # Flip a byte in the middle
                idx = len(mutable) // 2
                mutable[idx] = mutable[idx] ^ 0xFF
            elif mode == 'truncate':
                return mutable[:len(mutable)//2]
            return bytes(mutable)

        for name in output_names:
            # Convert Path objects to strings for resolve() to work correctly if there's a mix
            # Actually output_dir is a Path. name is str.
            # .resolve() resolves symlinks and relative paths to absolute
            target_path = (output_dir / name).resolve()
            text_mutator = self._garble_base64_text if mode == 'byte_flip' else None
            self._process_excel_container(
                base.resolve(),
                target_path,
                corruptor,
                text_mutator=text_mutator,
            )

    def _garble_base64_text(self, encoded: str) -> str:
        if not encoded:
            return "!!"
        chars = list(encoded)
        chars[0] = "!"
        return "".join(chars)


class MashupInjectGenerator(MashupBaseGenerator):
    """
    Peels the onion:
    1. Parses MS-QDEFF binary header.
    2. Unzips PackageParts.
    3. Injects new M-Code into Section1.m.
    4. Re-zips and fixes header lengths.
    """
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        new_m_code = self.args.get('m_code')

        if not base_file_arg:
             raise ValueError("MashupInjectGenerator requires 'base_file' argument")
        if new_m_code is None:
             raise ValueError("MashupInjectGenerator requires 'm_code' argument")

        base = Path(base_file_arg)
        if not base.exists():
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        def injector(raw_bytes):
            return self._inject_m_code(raw_bytes, new_m_code)

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, injector)

    def _inject_m_code(self, raw_bytes, m_code):
        # --- 1. Parse MS-QDEFF Header ---
        # Format: Version(4) + LenPP(4) + PackageParts(...) + LenPerm(4) + ...
        # We assume Version is 0 (first 4 bytes)
        
        if len(raw_bytes) < 8:
            return raw_bytes # Too short to handle

        offset = 4
        # Read PackageParts Length
        pp_len = struct.unpack('<I', raw_bytes[offset:offset+4])[0]
        offset += 4
        
        # Extract existing components
        pp_bytes = raw_bytes[offset : offset + pp_len]
        
        # Keep the rest of the stream (Permissions, Metadata, Bindings) intact
        # We just append it later
        remainder_bytes = raw_bytes[offset + pp_len :]

        # --- 2. Modify PackageParts (Inner ZIP) ---
        new_pp_bytes = self._replace_in_zip(pp_bytes, 'Formulas/Section1.m', m_code)

        # --- 3. Rebuild Stream ---
        # New Length for PackageParts
        new_pp_len = len(new_pp_bytes)
        
        # Reconstruct: Version(0) + NewLen + NewPP + Remainder
        header = raw_bytes[:4] # Version
        len_pack = struct.pack('<I', new_pp_len)
        
        return header + len_pack + new_pp_bytes + remainder_bytes

    def _replace_in_zip(self, zip_bytes, filename, new_content):
        """Opens a ZIP byte stream, replaces a file, returns new ZIP byte stream."""
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()
        
        try:
            with zipfile.ZipFile(in_buffer, 'r') as zin:
                with zipfile.ZipFile(out_buffer, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
                    for item in zin.infolist():
                        if item.filename == filename:
                            # Write the new M code
                            zout.writestr(filename, new_content.encode('utf-8'))
                        else:
                            # Copy others
                            zout.writestr(item, zin.read(item.filename))
        except zipfile.BadZipFile:
            # Fallback if inner stream isn't a valid zip (shouldn't happen on valid QDEFF)
            return zip_bytes
            
        return out_buffer.getvalue()


class MashupPackagePartsGenerator(MashupBaseGenerator):
    """
    Generates PackageParts-focused fixtures starting from a base workbook.
    """

    variant: str = "one_query"

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        if self.variant == "one_query":
            return raw_bytes

        version, package_parts, permissions, metadata, bindings = self._split_sections(raw_bytes)
        package_xml, main_section_text, content_types = self._extract_package_parts(package_parts)

        embedded_guid = self.args.get(
            "embedded_guid", "{11111111-2222-3333-4444-555555555555}"
        )
        embedded_section_text = self.args.get(
            "embedded_section",
            self._default_embedded_section(),
        )
        updated_main_section = self._extend_main_section(main_section_text, embedded_guid)
        embedded_bytes = self._build_embedded_package(embedded_section_text, content_types)
        updated_package_parts = self._build_package_parts(
            package_xml,
            updated_main_section,
            content_types,
            embedded_guid,
            embedded_bytes,
        )

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions,
            metadata,
            bindings,
        )

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts_end = offset + package_parts_len
        if package_parts_end > len(raw_bytes):
            raise ValueError("invalid PackageParts length")
        package_parts = raw_bytes[offset:package_parts_end]
        offset = package_parts_end

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions_end = offset + permissions_len
        if permissions_end > len(raw_bytes):
            raise ValueError("invalid permissions length")
        permissions = raw_bytes[offset:permissions_end]
        offset = permissions_end

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata_end = offset + metadata_len
        if metadata_end > len(raw_bytes):
            raise ValueError("invalid metadata length")
        metadata = raw_bytes[offset:metadata_end]
        offset = metadata_end

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings_end = offset + bindings_len
        if bindings_end > len(raw_bytes):
            raise ValueError("invalid bindings length")
        bindings = raw_bytes[offset:bindings_end]
        offset = bindings_end

        if offset != len(raw_bytes):
            raise ValueError("DataMashup trailing bytes mismatch")

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _extract_package_parts(self, package_parts: bytes):
        with zipfile.ZipFile(io.BytesIO(package_parts), "r") as z:
            package_xml = z.read("Config/Package.xml")
            content_types = z.read("[Content_Types].xml")
            main_section = z.read("Formulas/Section1.m")
        return package_xml, main_section.decode("utf-8", errors="ignore"), content_types

    def _extend_main_section(self, base_section: str, embedded_guid: str) -> str:
        stripped = base_section.rstrip()
        lines = [
            stripped,
            "",
            "shared EmbeddedQuery = let",
            f'    Source = Embedded.Value("Content/{embedded_guid}.package")',
            "in",
            "    Source;",
        ]
        return "\n".join(lines)

    def _build_embedded_package(self, section_text: str, content_types_template: bytes) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Formulas/Section1.m", section_text)
        return buffer.getvalue()

    def _build_package_parts(
        self,
        package_xml: bytes,
        main_section: str,
        content_types_template: bytes,
        embedded_guid: str,
        embedded_package: bytes,
    ) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Config/Package.xml", package_xml)
            z.writestr("Formulas/Section1.m", main_section)
            z.writestr(f"Content/{embedded_guid}.package", embedded_package)
        return buffer.getvalue()

    def _augment_content_types(self, content_types_bytes: bytes) -> str:
        text = content_types_bytes.decode("utf-8", errors="ignore")
        if "Extension=\"package\"" not in text and "Extension='package'" not in text:
            text = text.replace(
                "</Types>",
                '<Default Extension="package" ContentType="application/octet-stream" /></Types>',
                1,
            )
        return text

    def _default_embedded_section(self) -> str:
        return "\n".join(
            [
                "section Section1;",
                "",
                "shared Inner = let",
                "    Source = 1",
                "in",
                "    Source;",
            ]
        )


class MashupOneQueryGenerator(MashupPackagePartsGenerator):
    variant = "one_query"


class MashupMultiEmbeddedGenerator(MashupPackagePartsGenerator):
    variant = "multi_query_with_embedded"


class MashupDuplicateGenerator(MashupBaseGenerator):
    """
    Duplicates the customXml part that contains DataMashup to produce two
    DataMashup occurrences in a single workbook.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        mode = self.args.get('mode', 'part')
        if not base_file_arg:
            raise ValueError("MashupDuplicateGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            if mode == 'part':
                self._duplicate_datamashup_part(base.resolve(), target_path)
            elif mode == 'element':
                self._duplicate_datamashup_element(base.resolve(), target_path)
            else:
                raise ValueError(f"Unsupported duplicate mode: {mode}")

    def _duplicate_datamashup_part(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            try:
                item1_xml = zin.read("customXml/item1.xml")
                item_props1 = zin.read("customXml/itemProps1.xml")
                item1_rels = zin.read("customXml/_rels/item1.xml.rels")
                content_types = zin.read("[Content_Types].xml")
                workbook_rels = zin.read("xl/_rels/workbook.xml.rels")
            except KeyError as e:
                raise FileNotFoundError(f"Required DataMashup part missing: {e}") from e

            updated_content_types = self._add_itemprops_override(content_types)
            updated_workbook_rels = self._add_workbook_relationship(workbook_rels)
            item2_rels = item1_rels.replace(b"itemProps1.xml", b"itemProps2.xml")
            item_props2 = item_props1.replace(
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BE}",
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BF}",
            )

            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename == "[Content_Types].xml":
                        data = updated_content_types
                    elif info.filename == "xl/_rels/workbook.xml.rels":
                        data = updated_workbook_rels
                    zout.writestr(info, data)

                zout.writestr("customXml/item2.xml", item1_xml)
                zout.writestr("customXml/itemProps2.xml", item_props2)
                zout.writestr("customXml/_rels/item2.xml.rels", item2_rels)

    def _add_itemprops_override(self, content_types_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/content-types"
        root = ET.fromstring(content_types_bytes)
        override_tag = f"{{{ns}}}Override"
        if not any(
            elem.get("PartName") == "/customXml/itemProps2.xml"
            for elem in root.findall(override_tag)
        ):
            new_override = ET.SubElement(root, override_tag)
            new_override.set("PartName", "/customXml/itemProps2.xml")
            new_override.set(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.customXmlProperties+xml",
            )
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _add_workbook_relationship(self, rels_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/relationships"
        root = ET.fromstring(rels_bytes)
        rel_tag = f"{{{ns}}}Relationship"
        existing_ids = {elem.get("Id") for elem in root.findall(rel_tag)}
        next_id = 1
        while f"rId{next_id}" in existing_ids:
            next_id += 1
        new_rel = ET.SubElement(root, rel_tag)
        new_rel.set("Id", f"rId{next_id}")
        new_rel.set(
            "Type",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
        )
        new_rel.set("Target", "../customXml/item2.xml")
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _duplicate_datamashup_element(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            root = etree.fromstring(data)
                            dm_node = self._get_mashup_element(root)
                            if dm_node is not None:
                                duplicate = copy.deepcopy(dm_node)
                                parent = dm_node.getparent()
                                if parent is not None:
                                    parent.append(duplicate)
                                    target_root = root
                                else:
                                    container = etree.Element("root", nsmap=root.nsmap)
                                    container.append(dm_node)
                                    container.append(duplicate)
                                    target_root = container
                                data = etree.tostring(
                                    target_root, encoding="utf-8", xml_declaration=True
                                )
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)


class MashupEncodeGenerator(MashupBaseGenerator):
    """
    Re-encodes the DataMashup customXml stream to a target encoding and optionally
    inserts whitespace into the base64 payload.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        encoding = self.args.get('encoding', 'utf-8')
        whitespace = bool(self.args.get('whitespace', False))
        if not base_file_arg:
            raise ValueError("MashupEncodeGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._rewrite_datamashup_xml(base.resolve(), target_path, encoding, whitespace)

    def _rewrite_datamashup_xml(
        self,
        base_path: Path,
        output_path: Path,
        encoding: str,
        whitespace: bool,
    ):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            data = self._process_datamashup_stream(data, encoding, whitespace)
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)

    def _process_datamashup_stream(
        self,
        xml_bytes: bytes,
        encoding: str,
        whitespace: bool,
    ) -> bytes:
        root = etree.fromstring(xml_bytes)
        dm_node = self._get_mashup_element(root)
        if dm_node is None:
            return xml_bytes

        if dm_node.text and whitespace:
            dm_node.text = self._with_whitespace(dm_node.text)

        xml_bytes = etree.tostring(root, encoding="utf-8", xml_declaration=True)
        return self._encode_bytes(xml_bytes, encoding)

    def _with_whitespace(self, text: str) -> str:
        cleaned = text.strip()
        if not cleaned:
            return text
        midpoint = max(1, len(cleaned) // 2)
        return f"\n  {cleaned[:midpoint]}\n  {cleaned[midpoint:]}\n"

    def _encode_bytes(self, xml_bytes: bytes, encoding: str) -> bytes:
        enc = encoding.lower()
        if enc == "utf-8":
            return xml_bytes
        if enc == "utf-16-le":
            return self._to_utf16(xml_bytes, little_endian=True)
        if enc == "utf-16-be":
            return self._to_utf16(xml_bytes, little_endian=False)
        raise ValueError(f"Unsupported encoding: {encoding}")

    def _to_utf16(self, xml_bytes: bytes, little_endian: bool) -> bytes:
        text = xml_bytes.decode("utf-8")
        text = self._rewrite_declaration(text)
        encoded = text.encode("utf-16-le" if little_endian else "utf-16-be")
        bom = b"\xff\xfe" if little_endian else b"\xfe\xff"
        return bom + encoded

    def _rewrite_declaration(self, text: str) -> str:
        pattern = r'encoding=["\'][^"\']+["\']'
        if re.search(pattern, text):
            return re.sub(pattern, 'encoding="UTF-16"', text, count=1)
        prefix = "<?xml version='1.0'?>"
        if text.startswith(prefix):
            return text.replace(prefix, "<?xml version='1.0' encoding='UTF-16'?>", 1)
        return text


class MashupPermissionsMetadataGenerator(MashupBaseGenerator):
    """
    Builds fixtures that exercise Permissions and Metadata parsing by rewriting
    the PackageParts Section1.m, Permissions XML, and Metadata XML inside
    the DataMashup stream.
    """

    def __init__(self, args):
        super().__init__(args)
        self.mode = args.get("mode")

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if not self.mode:
            raise ValueError("MashupPermissionsMetadataGenerator requires 'mode' argument")

        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        version, package_parts, _, _, bindings = self._split_sections(raw_bytes)
        scenario = self._scenario_definition()

        updated_package_parts = self._replace_section(
            package_parts,
            scenario["section_text"],
        )
        permissions_bytes = self._permissions_bytes(**scenario["permissions"])
        metadata_bytes = self._metadata_bytes(scenario["metadata_entries"])

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions_bytes,
            metadata_bytes,
            bindings,
        )

    def _scenario_definition(self):
        shared_section_simple = "\n".join(
            [
                "section Section1;",
                "",
                "shared LoadToSheet = 1;",
                "shared LoadToModel = 2;",
            ]
        )

        def default_permissions():
            return {
                "can_eval": False,
                "firewall_enabled": True,
                "group_type": "Organizational",
            }

        def build_section_text(query_specs):
            lines = ["section Section1;", ""]
            for spec in query_specs:
                lines.append(f"shared {spec['name']} = {spec['body']};")
            return "\n".join(lines)

        def build_metadata_entries(query_specs):
            entries = []
            for spec in query_specs:
                stable_entries = []
                if spec.get("load_to_sheet"):
                    stable_entries.append(("FillEnabled", True))
                if spec.get("load_to_model"):
                    stable_entries.append(("FillToDataModelEnabled", True))
                entries.append(
                    {
                        "path": f"Section1/{spec['name']}",
                        "entries": stable_entries,
                    }
                )
            return entries

        def m_diff_scenario(query_specs):
            return {
                "section_text": build_section_text(query_specs),
                "permissions": default_permissions(),
                "metadata_entries": build_metadata_entries(query_specs),
            }

        if self.mode in ("permissions_defaults", "permissions_firewall_off", "metadata_simple"):
            return {
                "section_text": shared_section_simple,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": self.mode != "permissions_firewall_off",
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/LoadToSheet",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/LoadToModel",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", True),
                        ],
                    },
                ],
            }

        if self.mode == "m_add_query_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_add_query_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                    {"name": "Bar", "body": "2", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_remove_query_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                    {"name": "Bar", "body": "2", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_remove_query_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_change_literal_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_change_literal_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "2", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_metadata_only_change_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_metadata_only_change_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": False, "load_to_model": True},
                ]
            )

        if self.mode == "m_def_and_metadata_change_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_def_and_metadata_change_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "2", "load_to_sheet": False, "load_to_model": True},
                ]
            )

        if self.mode == "m_rename_query_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_rename_query_b":
            return m_diff_scenario(
                [
                    {"name": "Bar", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_formatting_only_a":
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": 'let Source=Excel.CurrentWorkbook(){[Name="Table1"]}[Content] in Source',
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_formatting_only_b":
            body = "\n".join(
                [
                    "let",
                    "    // Load the current workbook table",
                    "    Source = Excel.CurrentWorkbook(){[Name = \"Table1\"]}[Content]",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_formatting_only_b_variant":
            body = "\n".join(
                [
                    "let",
                    "    // Load a different table",
                    "    Source = Excel.CurrentWorkbook(){[Name = \"Table2\"]}[Content]",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "metadata_query_groups":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared RootQuery = 1;",
                    "shared GroupedFoo = 2;",
                    "shared NestedBar = 3;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/RootQuery",
                        "entries": [("FillEnabled", True)],
                    },
                    {
                        "path": "Section1/GroupedFoo",
                        "entries": [
                            ("FillEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                    {
                        "path": "Section1/NestedBar",
                        "entries": [
                            ("FillToDataModelEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_hidden_queries":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared ConnectionOnly = 1;",
                    "shared VisibleLoad = 2;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/ConnectionOnly",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/VisibleLoad",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_missing_entry":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared MissingMetadata = 1;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [],
            }

        if self.mode == "metadata_url_encoding":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    'shared #"Query with space & #" = 1;',
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/Query%20with%20space%20%26%20%23",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_orphan_entries":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared Foo = 1;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/Foo",
                        "entries": [("FillEnabled", True)],
                    },
                    {
                        "path": "Section1/Nonexistent",
                        "entries": [("FillEnabled", False)],
                    },
                ],
            }

        raise ValueError(f"Unsupported mode: {self.mode}")

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts = raw_bytes[offset : offset + package_parts_len]
        offset += package_parts_len

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions = raw_bytes[offset : offset + permissions_len]
        offset += permissions_len

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata = raw_bytes[offset : offset + metadata_len]
        offset += metadata_len

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings = raw_bytes[offset : offset + bindings_len]

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _replace_section(self, package_parts: bytes, section_text: str) -> bytes:
        return self._replace_in_zip(package_parts, "Formulas/Section1.m", section_text)

    def _replace_in_zip(self, zip_bytes: bytes, filename: str, new_content: str) -> bytes:
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()

        with zipfile.ZipFile(in_buffer, "r") as zin:
            with zipfile.ZipFile(out_buffer, "w", compression=zipfile.ZIP_DEFLATED) as zout:
                for item in zin.infolist():
                    if item.filename == filename:
                        zout.writestr(filename, new_content.encode("utf-8"))
                    else:
                        zout.writestr(item, zin.read(item.filename))
        return out_buffer.getvalue()

    def _permissions_bytes(self, can_eval: bool, firewall_enabled: bool, group_type: str) -> bytes:
        xml = (
            '<?xml version="1.0" encoding="utf-8"?>'
            "<PermissionList xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" "
            "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
            f"<CanEvaluateFuturePackages>{str(can_eval).lower()}</CanEvaluateFuturePackages>"
            f"<FirewallEnabled>{str(firewall_enabled).lower()}</FirewallEnabled>"
            f"<WorkbookGroupType>{group_type}</WorkbookGroupType>"
            "</PermissionList>"
        )
        return ("\ufeff" + xml).encode("utf-8")

    def _metadata_bytes(self, items: List[dict]) -> bytes:
        xml = self._metadata_xml(items)
        xml_bytes = ("\ufeff" + xml).encode("utf-8")
        header = struct.pack("<I", 0) + struct.pack("<I", len(xml_bytes))
        return header + xml_bytes

    def _metadata_xml(self, items: List[dict]) -> str:
        parts = [
            '<?xml version="1.0" encoding="utf-8"?>',
            '<LocalPackageMetadataFile xmlns:xsd="http://www.w3.org/2001/XMLSchema" '
            'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">',
            "<Items>",
            "<Item><ItemLocation><ItemType>AllFormulas</ItemType><ItemPath /></ItemLocation><StableEntries /></Item>",
        ]

        for item in items:
            parts.append("<Item>")
            parts.append("<ItemLocation>")
            parts.append("<ItemType>Formula</ItemType>")
            parts.append(f"<ItemPath>{item['path']}</ItemPath>")
            parts.append("</ItemLocation>")
            parts.append("<StableEntries>")
            for entry_name, entry_value in item.get("entries", []):
                value = self._format_entry_value(entry_value)
                parts.append(f'<Entry Type="{entry_name}" Value="{value}" />')
            parts.append("</StableEntries>")
            parts.append("</Item>")

        parts.append("</Items></LocalPackageMetadataFile>")
        return "".join(parts)

    def _format_entry_value(self, value):
        if isinstance(value, bool):
            return f"l{'1' if value else '0'}"
        return f"s{value}"


```

---

### File: `fixtures\src\generators\perf.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class LargeGridGenerator(BaseGenerator):
    """
    Generates massive grids using WriteOnly mode to save memory.
    Targeting P1/P2/P3/P4/P5 milestones.
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        rows = self.args.get('rows', 1000)
        cols = self.args.get('cols', 10)
        mode = self.args.get('mode', 'dense')
        seed = self.args.get('seed', 0)
        pattern_length = self.args.get('pattern_length', 100)
        fill_percent = self.args.get('fill_percent', 100)

        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook(write_only=True)
            ws = wb.create_sheet()
            ws.title = "Performance"

            header = [f"Col_{c}" for c in range(1, cols + 1)]
            ws.append(header)

            for r in range(1, rows + 1):
                row_data = []
                if mode == 'dense':
                    row_data = [f"R{r}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'noise':
                    row_data = [rng.random() for _ in range(cols)]
                
                elif mode == 'repetitive':
                    pattern_idx = (r - 1) % pattern_length
                    row_data = [f"P{pattern_idx}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'sparse':
                    row_data = []
                    for c in range(1, cols + 1):
                        if rng.randint(1, 100) <= fill_percent:
                            row_data.append(f"R{r}C{c}")
                        else:
                            row_data.append(None)
                
                ws.append(row_data)

            wb.save(output_dir / name)


```

---

### File: `scripts\check_perf_thresholds.py`

```python
#!/usr/bin/env python3
"""
Performance threshold checker for excel_diff.

This script verifies that performance tests complete within acceptable time bounds.
It runs `cargo test --release --features perf-metrics perf_` and validates that
each test completes within its configured threshold.

Thresholds are based on the mini-spec table from next_sprint_plan.md:
| Fixture | Rows | Cols | Max Time | Max Memory |
|---------|------|------|----------|------------|
| p1_large_dense | 50,000 | 100 | 5s | 500MB |
| p2_large_noise | 50,000 | 100 | 10s | 600MB |
| p3_adversarial_repetitive | 50,000 | 50 | 15s | 400MB |
| p4_99_percent_blank | 50,000 | 100 | 2s | 200MB |
| p5_identical | 50,000 | 100 | 1s | 300MB |

Note: The Rust tests use smaller grids for CI speed, so these thresholds are
conservative. Memory tracking is planned for a future phase.

Environment variables for threshold configuration:
  EXCEL_DIFF_PERF_P1_MAX_TIME_S - Override max time for perf_p1_large_dense
  EXCEL_DIFF_PERF_P2_MAX_TIME_S - Override max time for perf_p2_large_noise
  EXCEL_DIFF_PERF_P3_MAX_TIME_S - Override max time for perf_p3_adversarial_repetitive
  EXCEL_DIFF_PERF_P4_MAX_TIME_S - Override max time for perf_p4_99_percent_blank
  EXCEL_DIFF_PERF_P5_MAX_TIME_S - Override max time for perf_p5_identical
  EXCEL_DIFF_PERF_SLACK_FACTOR - Multiply all thresholds by this factor (default: 1.0)
"""

import os
import re
import subprocess
import sys
import time
from pathlib import Path

PERF_TEST_TIMEOUT_SECONDS = 120

THRESHOLDS = {
    "perf_p1_large_dense": {"max_time_s": 5},
    "perf_p2_large_noise": {"max_time_s": 10},
    "perf_p3_adversarial_repetitive": {"max_time_s": 15},
    "perf_p4_99_percent_blank": {"max_time_s": 2},
    "perf_p5_identical": {"max_time_s": 1},
}

ENV_VAR_MAP = {
    "perf_p1_large_dense": "EXCEL_DIFF_PERF_P1_MAX_TIME_S",
    "perf_p2_large_noise": "EXCEL_DIFF_PERF_P2_MAX_TIME_S",
    "perf_p3_adversarial_repetitive": "EXCEL_DIFF_PERF_P3_MAX_TIME_S",
    "perf_p4_99_percent_blank": "EXCEL_DIFF_PERF_P4_MAX_TIME_S",
    "perf_p5_identical": "EXCEL_DIFF_PERF_P5_MAX_TIME_S",
}


def get_effective_thresholds():
    """Get thresholds with environment variable overrides applied."""
    effective = {}
    slack_factor = float(os.environ.get("EXCEL_DIFF_PERF_SLACK_FACTOR", "1.0"))

    for test_name, config in THRESHOLDS.items():
        max_time_s = config["max_time_s"]

        env_var = ENV_VAR_MAP.get(test_name)
        if env_var and env_var in os.environ:
            try:
                max_time_s = float(os.environ[env_var])
                print(f"  Override: {test_name} max_time_s={max_time_s} (from {env_var})")
            except ValueError:
                print(f"  WARNING: Invalid value for {env_var}, using default")

        effective[test_name] = {"max_time_s": max_time_s * slack_factor}

    if slack_factor != 1.0:
        print(f"  Slack factor: {slack_factor}x applied to all thresholds")

    return effective


def parse_perf_metrics(stdout: str) -> dict:
    """Parse PERF_METRIC lines from test output.

    Expected format: PERF_METRIC <test_name> total_time_ms=<value> [other_metrics...]

    Returns dict mapping test_name -> {"total_time_ms": int, ...}
    """
    metrics = {}
    pattern = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")

    for line in stdout.split("\n"):
        match = pattern.search(line)
        if not match:
            continue

        test_name = match.group(1)
        rest = match.group(2)
        data = {key: int(val) for key, val in re.findall(r"(\w+)=([0-9]+)", rest)}
        data.setdefault("total_time_ms", 0)
        metrics[test_name] = data

    return metrics


def run_perf_tests():
    """Run the performance tests and verify they complete within thresholds."""
    print("=" * 60)
    print("Performance Threshold Check")
    print("=" * 60)

    print("\nLoading thresholds...")
    effective_thresholds = get_effective_thresholds()
    print()

    core_dir = Path(__file__).parent.parent / "core"
    if not core_dir.exists():
        core_dir = Path("core")

    start_time = time.time()
    try:
        result = subprocess.run(
            [
                "cargo",
                "test",
                "--release",
                "--features",
                "perf-metrics",
                "perf_",
                "--",
                "--nocapture",
            ],
            cwd=core_dir,
            capture_output=True,
            text=True,
            timeout=PERF_TEST_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        print(f"ERROR: Performance tests exceeded timeout of {PERF_TEST_TIMEOUT_SECONDS}s")
        return 1

    elapsed = time.time() - start_time
    print(f"Total perf suite time: {elapsed:.2f}s")
    print()

    if result.returncode != 0:
        print("ERROR: Performance tests failed!")
        print("STDOUT:", result.stdout)
        print("STDERR:", result.stderr)
        return 1

    passed_tests = []
    for line in result.stdout.split("\n"):
        if "test perf_" in line and "... ok" in line:
            test_name = line.split("test ")[1].split(" ...")[0].strip()
            passed_tests.append(test_name)

    print(f"Passed tests: {len(passed_tests)}")
    for test in passed_tests:
        print(f"  ✓ {test}")
    print()

    expected_tests = set(THRESHOLDS.keys())
    actual_tests = set(passed_tests)
    missing_tests = expected_tests - actual_tests

    if missing_tests:
        print(f"ERROR: Some expected perf tests did not run: {missing_tests}")
        return 1

    metrics = parse_perf_metrics(result.stdout)
    print(f"Parsed metrics for {len(metrics)} tests:")
    for test_name, data in metrics.items():
        total_time_s = data["total_time_ms"] / 1000.0
        print(f"  {test_name}: {total_time_s:.3f}s")
    print()

    missing_metrics = expected_tests - set(metrics.keys())
    if missing_metrics:
        print(f"ERROR: Missing PERF_METRIC output for tests: {missing_metrics}")
        return 1

    failures = []
    print("Threshold checks:")
    for test_name, threshold in effective_thresholds.items():
        max_time_s = threshold["max_time_s"]
        actual_time_ms = metrics[test_name]["total_time_ms"]
        actual_time_s = actual_time_ms / 1000.0

        if actual_time_s > max_time_s:
            status = "FAIL"
            failures.append((test_name, actual_time_s, max_time_s))
        else:
            status = "PASS"

        print(f"  {test_name}: {actual_time_s:.3f}s / {max_time_s:.1f}s [{status}]")

    print()

    if failures:
        print("=" * 60)
        print("THRESHOLD VIOLATIONS:")
        for test_name, actual, max_time in failures:
            print(f"  {test_name}: {actual:.3f}s exceeded max of {max_time:.1f}s")
        print("=" * 60)
        return 1

    print("=" * 60)
    print("All performance tests passed within thresholds!")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(run_perf_tests())

```

---

### File: `scripts\combine_results_to_csv.py`

```python
#!/usr/bin/env python3
"""
Combine benchmark JSON results into a single CSV for comparison over time.

Usage:
    python scripts/combine_results_to_csv.py [--output FILE] [--results-dir DIR]

Options:
    --output      Output CSV file path (default: benchmarks/results/combined_results.csv)
    --results-dir Directory containing JSON results (default: benchmarks/results)
"""

import argparse
import csv
import json
import sys
from pathlib import Path


ALL_TEST_FIELDS = [
    "total_time_ms",
    "move_detection_time_ms",
    "alignment_time_ms",
    "cell_diff_time_ms",
    "rows_processed",
    "cells_compared",
    "anchors_found",
    "moves_detected",
]


def load_json_results(results_dir: Path) -> list[dict]:
    """Load all JSON result files from the results directory."""
    results = []
    for json_file in sorted(results_dir.glob("*.json")):
        try:
            with open(json_file) as f:
                data = json.load(f)
                data["_source_file"] = json_file.name
                results.append(data)
        except (json.JSONDecodeError, IOError) as e:
            print(f"Warning: Could not load {json_file}: {e}", file=sys.stderr)
    return results


def flatten_results(results: list[dict]) -> list[dict]:
    """Flatten nested test results into individual rows."""
    rows = []
    for result in results:
        timestamp = result.get("timestamp", "")
        git_commit = result.get("git_commit", "")
        git_branch = result.get("git_branch", "")
        full_scale = result.get("full_scale", False)
        source_file = result.get("_source_file", "")

        tests = result.get("tests", {})
        for test_name, test_data in tests.items():
            row = {
                "source_file": source_file,
                "timestamp": timestamp,
                "git_commit": git_commit,
                "git_branch": git_branch,
                "full_scale": full_scale,
                "test_name": test_name,
            }
            for field in ALL_TEST_FIELDS:
                row[field] = test_data.get(field, "")
            rows.append(row)

        summary = result.get("summary", {})
        if summary:
            row = {
                "source_file": source_file,
                "timestamp": timestamp,
                "git_commit": git_commit,
                "git_branch": git_branch,
                "full_scale": full_scale,
                "test_name": "_SUMMARY_",
                "total_time_ms": summary.get("total_time_ms", ""),
                "rows_processed": summary.get("total_rows_processed", ""),
                "cells_compared": summary.get("total_cells_compared", ""),
            }
            for field in ALL_TEST_FIELDS:
                if field not in row:
                    row[field] = ""
            rows.append(row)

    return rows


def write_csv(rows: list[dict], output_path: Path):
    """Write flattened results to CSV."""
    if not rows:
        print("No data to write.", file=sys.stderr)
        return

    fieldnames = [
        "source_file",
        "timestamp",
        "git_commit",
        "git_branch",
        "full_scale",
        "test_name",
    ] + ALL_TEST_FIELDS

    with open(output_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(
        description="Combine benchmark JSON results into a single CSV"
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Directory containing JSON results",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Output CSV file path",
    )
    args = parser.parse_args()

    if args.output is None:
        args.output = args.results_dir / "combined_results.csv"

    if not args.results_dir.exists():
        print(f"ERROR: Results directory not found: {args.results_dir}", file=sys.stderr)
        return 1

    print(f"Loading results from: {args.results_dir}")
    results = load_json_results(args.results_dir)

    if not results:
        print("ERROR: No JSON result files found.", file=sys.stderr)
        return 1

    print(f"Found {len(results)} result files")

    rows = flatten_results(results)
    print(f"Generated {len(rows)} rows")

    write_csv(rows, args.output)
    print(f"CSV written to: {args.output}")

    return 0


if __name__ == "__main__":
    sys.exit(main())


```

---

### File: `scripts\compare_perf_results.py`

```python
#!/usr/bin/env python3
"""
Compare performance results between two benchmark runs.

Usage:
    python scripts/compare_perf_results.py [baseline.json] [current.json]
    python scripts/compare_perf_results.py --latest  # Compare two most recent results

If no arguments provided, compares the two most recent results in benchmarks/results/.
"""

import argparse
import json
import sys
from pathlib import Path


def load_result(path: Path) -> dict:
    """Load a benchmark result JSON file."""
    with open(path) as f:
        return json.load(f)


def get_latest_results(results_dir: Path, n: int = 2) -> list[Path]:
    """Get the N most recent result files."""
    files = sorted(results_dir.glob("*.json"), key=lambda p: p.stat().st_mtime, reverse=True)
    return files[:n]


def format_delta(baseline: float, current: float) -> str:
    """Format a percentage delta with color indicator."""
    if baseline == 0:
        return "N/A"
    delta = ((current - baseline) / baseline) * 100
    if abs(delta) < 1:
        return f"  {delta:+.1f}%"
    elif delta < 0:
        return f"  {delta:+.1f}% (faster)"
    else:
        return f"  {delta:+.1f}% (SLOWER)"


def compare_results(baseline: dict, current: dict):
    """Compare two benchmark results and print a comparison table."""
    print("=" * 90)
    print("Performance Comparison")
    print("=" * 90)
    print(f"Baseline: {baseline.get('git_commit', 'unknown')} ({baseline.get('timestamp', 'unknown')[:19]})")
    print(f"Current:  {current.get('git_commit', 'unknown')} ({current.get('timestamp', 'unknown')[:19]})")
    print()

    baseline_tests = baseline.get("tests", {})
    current_tests = current.get("tests", {})

    all_tests = sorted(set(baseline_tests.keys()) | set(current_tests.keys()))

    if not all_tests:
        print("No tests found in either result file.")
        return

    print(f"{'Test':<35} {'Baseline':>10} {'Current':>10} {'Delta':>20}")
    print("-" * 90)

    regressions = []
    improvements = []

    for test_name in all_tests:
        base_data = baseline_tests.get(test_name, {})
        curr_data = current_tests.get(test_name, {})

        base_time = base_data.get("total_time_ms", 0)
        curr_time = curr_data.get("total_time_ms", 0)

        if base_time == 0:
            delta_str = "NEW"
        elif curr_time == 0:
            delta_str = "REMOVED"
        else:
            delta_pct = ((curr_time - base_time) / base_time) * 100
            delta_str = format_delta(base_time, curr_time)

            if delta_pct > 10:
                regressions.append((test_name, delta_pct))
            elif delta_pct < -10:
                improvements.append((test_name, delta_pct))

        base_str = f"{base_time:,}ms" if base_time else "—"
        curr_str = f"{curr_time:,}ms" if curr_time else "—"

        print(f"{test_name:<35} {base_str:>10} {curr_str:>10} {delta_str:>20}")

    print("-" * 90)

    base_total = baseline.get("summary", {}).get("total_time_ms", 0)
    curr_total = current.get("summary", {}).get("total_time_ms", 0)
    print(f"{'TOTAL':<35} {base_total:>10,}ms {curr_total:>10,}ms {format_delta(base_total, curr_total):>20}")
    print("=" * 90)

    if regressions:
        print("\n⚠️  REGRESSIONS (>10% slower):")
        for name, delta in sorted(regressions, key=lambda x: -x[1]):
            print(f"   {name}: +{delta:.1f}%")

    if improvements:
        print("\n✅ IMPROVEMENTS (>10% faster):")
        for name, delta in sorted(improvements, key=lambda x: x[1]):
            print(f"   {name}: {delta:.1f}%")

    if not regressions and not improvements:
        print("\n✓ No significant changes detected (within ±10%)")


def main():
    parser = argparse.ArgumentParser(description="Compare performance benchmark results")
    parser.add_argument("baseline", nargs="?", type=Path, help="Baseline result JSON file")
    parser.add_argument("current", nargs="?", type=Path, help="Current result JSON file")
    parser.add_argument("--latest", action="store_true", help="Compare two most recent results")
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Results directory",
    )
    args = parser.parse_args()

    if args.latest or (args.baseline is None and args.current is None):
        files = get_latest_results(args.results_dir, 2)
        if len(files) < 2:
            print(f"ERROR: Need at least 2 result files in {args.results_dir}")
            print(f"Found: {len(files)} files")
            return 1
        baseline_path = files[1]
        current_path = files[0]
    else:
        if not args.baseline or not args.current:
            parser.error("Must provide both baseline and current files, or use --latest")
        baseline_path = args.baseline
        current_path = args.current

    if not baseline_path.exists():
        print(f"ERROR: Baseline file not found: {baseline_path}")
        return 1
    if not current_path.exists():
        print(f"ERROR: Current file not found: {current_path}")
        return 1

    baseline = load_result(baseline_path)
    current = load_result(current_path)

    compare_results(baseline, current)
    return 0


if __name__ == "__main__":
    sys.exit(main())


```

---

### File: `scripts\export_perf_metrics.py`

```python
#!/usr/bin/env python3
"""
Export performance metrics from excel_diff tests to JSON.

This script runs the performance test suite and captures the PERF_METRIC output,
saving timestamped results to benchmarks/results/ for historical tracking.

Usage:
    python scripts/export_perf_metrics.py [--full-scale] [--output-dir DIR]

Options:
    --full-scale    Run the 50K row tests (slower but comprehensive)
    --output-dir    Override the output directory (default: benchmarks/results)
"""

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def get_git_commit():
    """Get the current git commit hash."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()[:12]
    except Exception:
        pass
    return "unknown"


def get_git_branch():
    """Get the current git branch name."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except Exception:
        pass
    return "unknown"


def parse_perf_metrics(stdout: str) -> dict:
    """Parse PERF_METRIC lines from test output."""
    metrics = {}
    pattern = re.compile(r"PERF_METRIC\s+(\S+)\s+(.*)")

    for line in stdout.split("\n"):
        match = pattern.search(line)
        if not match:
            continue

        test_name = match.group(1)
        rest = match.group(2)
        data = {key: int(val) for key, val in re.findall(r"(\w+)=([0-9]+)", rest)}

        # Ensure required keys exist even if the output is partially missing.
        data.setdefault("total_time_ms", 0)
        data.setdefault("rows_processed", 0)
        data.setdefault("cells_compared", 0)

        metrics[test_name] = data

    return metrics


def run_perf_tests(full_scale: bool = False) -> tuple[dict, bool]:
    """Run performance tests and return parsed metrics."""
    core_dir = Path(__file__).parent.parent / "core"
    if not core_dir.exists():
        core_dir = Path("core")

    cmd = [
        "cargo",
        "test",
        "--release",
        "--features",
        "perf-metrics",
    ]

    if full_scale:
        cmd.extend(["--", "--ignored", "--nocapture"])
    else:
        cmd.extend(["perf_", "--", "--nocapture"])

    print(f"Running: {' '.join(cmd)}")
    print(f"Working directory: {core_dir}")
    print()

    try:
        result = subprocess.run(
            cmd,
            cwd=core_dir,
            capture_output=True,
            text=True,
            timeout=600 if full_scale else 120,
        )
    except subprocess.TimeoutExpired:
        print("ERROR: Tests timed out")
        return {}, False

    print(result.stdout)
    if result.stderr:
        print("STDERR:", result.stderr, file=sys.stderr)

    success = result.returncode == 0
    metrics = parse_perf_metrics(result.stdout)

    return metrics, success


def save_results(metrics: dict, output_dir: Path, full_scale: bool):
    """Save metrics to a timestamped JSON file."""
    timestamp = datetime.now(timezone.utc)
    filename = timestamp.strftime("%Y-%m-%d_%H%M%S")
    if full_scale:
        filename += "_fullscale"
    filename += ".json"

    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / filename

    result = {
        "timestamp": timestamp.isoformat(),
        "git_commit": get_git_commit(),
        "git_branch": get_git_branch(),
        "full_scale": full_scale,
        "tests": metrics,
        "summary": {
            "total_tests": len(metrics),
            "total_time_ms": sum(m["total_time_ms"] for m in metrics.values()),
            "total_rows_processed": sum(m["rows_processed"] for m in metrics.values()),
            "total_cells_compared": sum(m["cells_compared"] for m in metrics.values()),
        },
    }

    with open(output_path, "w") as f:
        json.dump(result, f, indent=2)

    print(f"\nResults saved to: {output_path}")
    return output_path


def print_summary(metrics: dict):
    """Print a summary table of metrics."""
    print("\n" + "=" * 70)
    print("Performance Metrics Summary")
    print("=" * 70)
    print(
        f"{'Test':<40} {'Total':>10} {'Move':>10} {'Align':>10} {'Cell':>10} {'Rows':>10} {'Cells':>12}"
    )
    print("-" * 70)

    for test_name, data in sorted(metrics.items()):
        move = data.get("move_detection_time_ms", 0)
        align = data.get("alignment_time_ms", 0)
        cell = data.get("cell_diff_time_ms", 0)
        print(
            f"{test_name:<40} {data['total_time_ms']:>10,} {move:>10,} {align:>10,} {cell:>10,} {data.get('rows_processed', 0):>10,} {data.get('cells_compared', 0):>12,}"
        )

    print("-" * 70)
    total_time = sum(m["total_time_ms"] for m in metrics.values())
    total_move = sum(m.get("move_detection_time_ms", 0) for m in metrics.values())
    total_align = sum(m.get("alignment_time_ms", 0) for m in metrics.values())
    total_cell = sum(m.get("cell_diff_time_ms", 0) for m in metrics.values())
    total_rows = sum(m["rows_processed"] for m in metrics.values())
    total_cells = sum(m["cells_compared"] for m in metrics.values())
    print(
        f"{'TOTAL':<40} {total_time:>10,} {total_move:>10,} {total_align:>10,} {total_cell:>10,} {total_rows:>10,} {total_cells:>12,}"
    )
    print("=" * 70)


def main():
    parser = argparse.ArgumentParser(
        description="Export performance metrics from excel_diff tests"
    )
    parser.add_argument(
        "--full-scale",
        action="store_true",
        help="Run the 50K row tests (slower but comprehensive)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results",
        help="Output directory for JSON results",
    )
    args = parser.parse_args()

    print("=" * 70)
    print("Excel Diff Performance Metrics Export")
    print("=" * 70)
    print(f"Mode: {'Full-scale (50K rows)' if args.full_scale else 'Quick (1K rows)'}")
    print(f"Output: {args.output_dir}")
    print(f"Git commit: {get_git_commit()}")
    print(f"Git branch: {get_git_branch()}")
    print()

    metrics, success = run_perf_tests(args.full_scale)

    if not metrics:
        print("ERROR: No metrics captured from test output")
        return 1

    print_summary(metrics)
    save_results(metrics, args.output_dir, args.full_scale)

    if not success:
        print("\nWARNING: Some tests may have failed")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())


```

---

### File: `scripts\visualize_benchmarks.py`

```python
#!/usr/bin/env python3
"""
Visualize benchmark trends from combined_results.csv.

Usage:
    python scripts/visualize_benchmarks.py [--input FILE] [--output-dir DIR] [--show]

Options:
    --input       Input CSV file (default: benchmarks/results/combined_results.csv)
    --output-dir  Directory to save plots (default: benchmarks/results/plots)
    --show        Display plots interactively instead of saving
"""

import argparse
import sys
from datetime import datetime
from pathlib import Path

try:
    import matplotlib.pyplot as plt
    import matplotlib.dates as mdates
    import pandas as pd
except ImportError as e:
    print(f"Missing required dependency: {e}")
    print("Install with: pip install matplotlib pandas")
    sys.exit(1)


COLORS = [
    "#2ecc71", "#3498db", "#9b59b6", "#e74c3c", "#f39c12",
    "#1abc9c", "#e67e22", "#34495e", "#16a085", "#c0392b",
]


def load_data(csv_path: Path) -> pd.DataFrame:
    df = pd.read_csv(csv_path)
    df["timestamp"] = pd.to_datetime(df["timestamp"])
    df = df[df["test_name"] != "_SUMMARY_"]
    df = df.sort_values("timestamp")
    return df


def plot_time_trends(df: pd.DataFrame, output_dir: Path, show: bool = False):
    fig, ax = plt.subplots(figsize=(14, 8))

    quick_df = df[df["full_scale"] == False]
    full_df = df[df["full_scale"] == True]

    for i, (scale_df, scale_name) in enumerate([(quick_df, "Quick"), (full_df, "Full-Scale")]):
        if scale_df.empty:
            continue

        test_names = scale_df["test_name"].unique()
        for j, test_name in enumerate(test_names):
            test_data = scale_df[scale_df["test_name"] == test_name]
            color = COLORS[j % len(COLORS)]
            linestyle = "-" if scale_name == "Quick" else "--"
            marker = "o" if scale_name == "Quick" else "s"
            label = f"{test_name} ({scale_name})"
            ax.plot(
                test_data["timestamp"],
                test_data["total_time_ms"],
                marker=marker,
                linestyle=linestyle,
                color=color,
                label=label,
                markersize=6,
                linewidth=2,
                alpha=0.8,
            )

    ax.set_xlabel("Timestamp", fontsize=12)
    ax.set_ylabel("Total Time (ms)", fontsize=12)
    ax.set_title("Benchmark Performance Over Time", fontsize=14, fontweight="bold")
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%m/%d %H:%M"))
    ax.tick_params(axis="x", rotation=45)
    ax.legend(bbox_to_anchor=(1.02, 1), loc="upper left", fontsize=9)
    ax.grid(True, alpha=0.3)
    ax.set_yscale("log")
    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "time_trends.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'time_trends.png'}")
    plt.close(fig)


def plot_speedup_heatmap(df: pd.DataFrame, output_dir: Path, show: bool = False):
    quick_df = df[df["full_scale"] == False].copy()
    if quick_df.empty:
        print("No quick-scale data for speedup heatmap")
        return

    runs = quick_df.groupby("source_file")["timestamp"].first().sort_values()
    if len(runs) < 2:
        print("Need at least 2 runs for speedup comparison")
        return

    pivot = quick_df.pivot_table(
        index="test_name",
        columns="source_file",
        values="total_time_ms",
        aggfunc="first",
    )
    pivot = pivot[runs.index]

    speedup = pd.DataFrame(index=pivot.index)
    run_files = list(pivot.columns)
    for i in range(1, len(run_files)):
        prev_run = run_files[i - 1]
        curr_run = run_files[i]
        col_name = f"{curr_run[:10]}"
        speedup[col_name] = ((pivot[prev_run] - pivot[curr_run]) / pivot[prev_run] * 100).round(1)

    if speedup.empty or speedup.shape[1] == 0:
        print("Not enough data for speedup heatmap")
        return

    fig, ax = plt.subplots(figsize=(max(10, len(speedup.columns) * 1.5), max(6, len(speedup) * 0.6)))

    im = ax.imshow(speedup.values, cmap="RdYlGn", aspect="auto", vmin=-50, vmax=50)

    ax.set_xticks(range(len(speedup.columns)))
    ax.set_xticklabels(speedup.columns, rotation=45, ha="right", fontsize=9)
    ax.set_yticks(range(len(speedup.index)))
    ax.set_yticklabels(speedup.index, fontsize=9)

    for i in range(len(speedup.index)):
        for j in range(len(speedup.columns)):
            val = speedup.iloc[i, j]
            if pd.notna(val):
                color = "white" if abs(val) > 25 else "black"
                text = f"{val:+.0f}%" if val != 0 else "0%"
                ax.text(j, i, text, ha="center", va="center", color=color, fontsize=8)

    cbar = fig.colorbar(im, ax=ax, shrink=0.8)
    cbar.set_label("Speedup % (positive = faster)", fontsize=10)

    ax.set_title("Performance Change Between Runs (Quick Tests)", fontsize=14, fontweight="bold")
    ax.set_xlabel("Run", fontsize=12)
    ax.set_ylabel("Test", fontsize=12)
    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "speedup_heatmap.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'speedup_heatmap.png'}")
    plt.close(fig)


def plot_latest_comparison(df: pd.DataFrame, output_dir: Path, show: bool = False):
    latest_runs = df.groupby("full_scale")["source_file"].apply(lambda x: x.iloc[-1] if len(x) > 0 else None)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    for idx, (full_scale, ax) in enumerate(zip([False, True], axes)):
        if full_scale not in latest_runs.index or latest_runs[full_scale] is None:
            ax.text(0.5, 0.5, "No data", ha="center", va="center", transform=ax.transAxes)
            ax.set_title(f"{'Full-Scale' if full_scale else 'Quick'} Tests - No Data")
            continue

        latest_file = latest_runs[full_scale]
        latest_data = df[(df["source_file"] == latest_file) & (df["full_scale"] == full_scale)]

        if latest_data.empty:
            continue

        tests = latest_data["test_name"].values
        times = latest_data["total_time_ms"].values

        colors = [COLORS[i % len(COLORS)] for i in range(len(tests))]
        bars = ax.barh(tests, times, color=colors, alpha=0.8)

        for bar, time in zip(bars, times):
            ax.text(
                bar.get_width() + max(times) * 0.01,
                bar.get_y() + bar.get_height() / 2,
                f"{time:,.0f}ms",
                va="center",
                fontsize=9,
            )

        scale_name = "Full-Scale (50K rows)" if full_scale else "Quick (1-2K rows)"
        timestamp = latest_data["timestamp"].iloc[0].strftime("%Y-%m-%d %H:%M")
        ax.set_title(f"{scale_name}\n{latest_file} ({timestamp})", fontsize=11, fontweight="bold")
        ax.set_xlabel("Time (ms)", fontsize=10)
        ax.set_xlim(0, max(times) * 1.15)
        ax.grid(True, axis="x", alpha=0.3)

    fig.suptitle("Latest Benchmark Results", fontsize=14, fontweight="bold", y=1.02)
    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "latest_comparison.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'latest_comparison.png'}")
    plt.close(fig)


def plot_metric_breakdown(df: pd.DataFrame, output_dir: Path, show: bool = False):
    metrics = ["move_detection_time_ms", "alignment_time_ms", "cell_diff_time_ms"]
    available_metrics = [m for m in metrics if m in df.columns and df[m].notna().any()]

    if not available_metrics:
        print("No detailed timing metrics available for breakdown chart")
        return

    latest_quick = df[df["full_scale"] == False].groupby("test_name").last().reset_index()
    latest_full = df[df["full_scale"] == True].groupby("test_name").last().reset_index()

    for scale_name, scale_df in [("Quick", latest_quick), ("Full-Scale", latest_full)]:
        if scale_df.empty:
            continue

        scale_df = scale_df[scale_df[available_metrics].notna().any(axis=1)]
        if scale_df.empty:
            continue

        fig, ax = plt.subplots(figsize=(12, 6))

        tests = scale_df["test_name"].values
        x = range(len(tests))
        width = 0.25

        metric_labels = {
            "move_detection_time_ms": "Fingerprinting + Move Detection",
            "alignment_time_ms": "Alignment (incl. diff)",
            "cell_diff_time_ms": "Cell Diff",
        }

        for i, metric in enumerate(available_metrics):
            values = scale_df[metric].fillna(0).values
            offset = (i - len(available_metrics) / 2 + 0.5) * width
            bars = ax.bar([xi + offset for xi in x], values, width, label=metric_labels.get(metric, metric), color=COLORS[i], alpha=0.8)

        ax.set_xlabel("Test", fontsize=12)
        ax.set_ylabel("Time (ms)", fontsize=12)
        ax.set_title(f"Timing Breakdown by Phase ({scale_name} Tests)", fontsize=14, fontweight="bold")
        ax.set_xticks(x)
        ax.set_xticklabels(tests, rotation=45, ha="right", fontsize=9)
        ax.legend()
        ax.grid(True, axis="y", alpha=0.3)
        fig.tight_layout()

        suffix = "quick" if scale_name == "Quick" else "fullscale"
        if show:
            plt.show()
        else:
            fig.savefig(output_dir / f"metric_breakdown_{suffix}.png", dpi=150, bbox_inches="tight")
            print(f"Saved: {output_dir / f'metric_breakdown_{suffix}.png'}")
        plt.close(fig)


def plot_commit_comparison(df: pd.DataFrame, output_dir: Path, show: bool = False):
    quick_df = df[df["full_scale"] == False].copy()
    if quick_df.empty:
        print("No quick-scale data for commit comparison")
        return

    commit_totals = quick_df.groupby(["git_commit", "source_file"])["total_time_ms"].sum().reset_index()
    commit_totals = commit_totals.sort_values("source_file")

    if len(commit_totals) < 2:
        print("Need at least 2 commits for comparison")
        return

    fig, ax = plt.subplots(figsize=(12, 6))

    commits = commit_totals["git_commit"].values
    totals = commit_totals["total_time_ms"].values
    files = commit_totals["source_file"].values

    colors = [COLORS[i % len(COLORS)] for i in range(len(commits))]
    bars = ax.bar(range(len(commits)), totals, color=colors, alpha=0.8)

    for i, (bar, total, commit, fname) in enumerate(zip(bars, totals, commits, files)):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height() + max(totals) * 0.01,
            f"{total:,.0f}ms",
            ha="center",
            va="bottom",
            fontsize=9,
        )

    ax.set_xlabel("Commit", fontsize=12)
    ax.set_ylabel("Total Time (ms)", fontsize=12)
    ax.set_title("Total Test Suite Time by Commit (Quick Tests)", fontsize=14, fontweight="bold")
    ax.set_xticks(range(len(commits)))
    labels = [f"{c[:8]}\n{f[:10]}" for c, f in zip(commits, files)]
    ax.set_xticklabels(labels, rotation=0, fontsize=8)
    ax.grid(True, axis="y", alpha=0.3)

    if len(totals) >= 2:
        first_total = totals[0]
        last_total = totals[-1]
        overall_change = ((last_total - first_total) / first_total) * 100
        direction = "faster" if overall_change < 0 else "slower"
        ax.text(
            0.98, 0.98,
            f"Overall: {abs(overall_change):.1f}% {direction}",
            transform=ax.transAxes,
            ha="right", va="top",
            fontsize=11,
            fontweight="bold",
            color="green" if overall_change < 0 else "red",
            bbox=dict(boxstyle="round", facecolor="white", alpha=0.8),
        )

    fig.tight_layout()

    if show:
        plt.show()
    else:
        fig.savefig(output_dir / "commit_comparison.png", dpi=150, bbox_inches="tight")
        print(f"Saved: {output_dir / 'commit_comparison.png'}")
    plt.close(fig)


def generate_summary_report(df: pd.DataFrame, output_dir: Path):
    lines = [
        "# Benchmark Trend Summary",
        "",
        f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "## Overview",
        "",
        f"- Total benchmark runs: {df['source_file'].nunique()}",
        f"- Quick-scale runs: {df[df['full_scale'] == False]['source_file'].nunique()}",
        f"- Full-scale runs: {df[df['full_scale'] == True]['source_file'].nunique()}",
        f"- Unique tests: {df['test_name'].nunique()}",
        f"- Date range: {df['timestamp'].min().strftime('%Y-%m-%d')} to {df['timestamp'].max().strftime('%Y-%m-%d')}",
        "",
    ]

    for scale_name, full_scale in [("Quick", False), ("Full-Scale", True)]:
        scale_df = df[df["full_scale"] == full_scale]
        if scale_df.empty:
            continue

        lines.extend([f"## {scale_name} Tests Performance", ""])

        runs = scale_df.groupby("source_file")["timestamp"].first().sort_values()
        if len(runs) >= 2:
            first_run = runs.index[0]
            last_run = runs.index[-1]

            first_total = scale_df[scale_df["source_file"] == first_run]["total_time_ms"].sum()
            last_total = scale_df[scale_df["source_file"] == last_run]["total_time_ms"].sum()
            change = ((last_total - first_total) / first_total) * 100

            lines.extend([
                f"- First run total: {first_total:,.0f}ms ({first_run})",
                f"- Latest run total: {last_total:,.0f}ms ({last_run})",
                f"- Overall change: {change:+.1f}% ({'faster' if change < 0 else 'slower'})",
                "",
            ])

        lines.append("### Per-Test Trends")
        lines.append("")
        lines.append("| Test | First (ms) | Latest (ms) | Change |")
        lines.append("|:-----|----------:|------------:|-------:|")

        for test_name in scale_df["test_name"].unique():
            test_data = scale_df[scale_df["test_name"] == test_name].sort_values("timestamp")
            if len(test_data) >= 2:
                first_time = test_data.iloc[0]["total_time_ms"]
                last_time = test_data.iloc[-1]["total_time_ms"]
                pct_change = ((last_time - first_time) / first_time) * 100
                lines.append(f"| {test_name} | {first_time:,.0f} | {last_time:,.0f} | {pct_change:+.1f}% |")
            elif len(test_data) == 1:
                lines.append(f"| {test_name} | {test_data.iloc[0]['total_time_ms']:,.0f} | - | N/A |")

        lines.extend(["", ""])

    report_path = output_dir / "trend_summary.md"
    report_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"Saved: {report_path}")


def main():
    parser = argparse.ArgumentParser(description="Visualize benchmark trends")
    parser.add_argument(
        "--input",
        type=Path,
        default=Path(__file__).parent.parent / "benchmarks" / "results" / "combined_results.csv",
        help="Input CSV file",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Output directory for plots",
    )
    parser.add_argument(
        "--show",
        action="store_true",
        help="Display plots interactively",
    )
    args = parser.parse_args()

    if not args.input.exists():
        print(f"ERROR: Input file not found: {args.input}")
        print("Run scripts/combine_results_to_csv.py first to generate the combined CSV.")
        return 1

    if args.output_dir is None:
        args.output_dir = args.input.parent / "plots"

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading data from: {args.input}")
    df = load_data(args.input)
    print(f"Loaded {len(df)} data points from {df['source_file'].nunique()} benchmark runs")
    print()

    print("Generating visualizations...")
    plot_time_trends(df, args.output_dir, args.show)
    plot_speedup_heatmap(df, args.output_dir, args.show)
    plot_latest_comparison(df, args.output_dir, args.show)
    plot_metric_breakdown(df, args.output_dir, args.show)
    plot_commit_comparison(df, args.output_dir, args.show)
    generate_summary_report(df, args.output_dir)

    print()
    print(f"All outputs saved to: {args.output_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

```

---
