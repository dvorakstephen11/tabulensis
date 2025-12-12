use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use excel_diff::config::DiffConfig;
use excel_diff::engine::diff_workbooks_with_config;
use excel_diff::{Cell, CellAddress, CellValue, Grid, Sheet, SheetKind, Workbook};
use std::time::Duration;

const MAX_BENCH_TIME_SECS: u64 = 30;
const WARMUP_SECS: u64 = 3;
const SAMPLE_SIZE: usize = 10;

fn create_large_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert(Cell {
                row,
                col,
                address: CellAddress::from_indices(row, col),
                value: Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                formula: None,
            });
        }
    }
    grid
}

fn create_repetitive_grid(nrows: u32, ncols: u32, pattern_length: u32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        let pattern_idx = row % pattern_length;
        for col in 0..ncols {
            grid.insert(Cell {
                row,
                col,
                address: CellAddress::from_indices(row, col),
                value: Some(CellValue::Number((pattern_idx * 1000 + col) as f64)),
                formula: None,
            });
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
                grid.insert(Cell {
                    row,
                    col,
                    address: CellAddress::from_indices(row, col),
                    value: Some(CellValue::Number((row * 1000 + col) as f64)),
                    formula: None,
                });
            }
        }
    }
    grid
}

fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    Workbook {
        sheets: vec![Sheet {
            name: name.to_string(),
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
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
            b.iter(|| diff_workbooks_with_config(&wb_a, &wb_b, &config));
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
        grid_b.insert(Cell {
            row: size / 2,
            col: 25,
            address: CellAddress::from_indices(size / 2, 25),
            value: Some(CellValue::Number(999999.0)),
            formula: None,
        });
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks_with_config(&wb_a, &wb_b, &config));
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
            b.iter(|| diff_workbooks_with_config(&wb_a, &wb_b, &config));
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
        grid_b.insert(Cell {
            row: size / 2,
            col: 25,
            address: CellAddress::from_indices(size / 2, 25),
            value: Some(CellValue::Number(999999.0)),
            formula: None,
        });
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks_with_config(&wb_a, &wb_b, &config));
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
        grid_b.insert(Cell {
            row: size / 2,
            col: 50,
            address: CellAddress::from_indices(size / 2, 50),
            value: Some(CellValue::Number(999999.0)),
            formula: None,
        });
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 100));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks_with_config(&wb_a, &wb_b, &config));
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
                grid_b.insert(Cell {
                    row,
                    col,
                    address: CellAddress::from_indices(row, col),
                    value: Some(CellValue::Number((row as i64 * 1000 + col as i64) as f64)),
                    formula: None,
                });
            }
        }
        for col in 0..50 {
            for i in 0..100 {
                let row = size / 2 + i;
                grid_b.insert(Cell {
                    row,
                    col,
                    address: CellAddress::from_indices(row, col),
                    value: Some(CellValue::Text(format!("NEW_ROW_{}", i))),
                    formula: None,
                });
            }
        }
        for row in (size / 2)..*size {
            for col in 0..50 {
                let new_row = row + 100;
                grid_b.insert(Cell {
                    row: new_row,
                    col,
                    address: CellAddress::from_indices(new_row, col),
                    value: Some(CellValue::Number((row as i64 * 1000 + col as i64) as f64)),
                    formula: None,
                });
            }
        }
        let wb_a = single_sheet_workbook("Bench", grid_a);
        let wb_b = single_sheet_workbook("Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, |b, _| {
            b.iter(|| diff_workbooks_with_config(&wb_a, &wb_b, &config));
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
