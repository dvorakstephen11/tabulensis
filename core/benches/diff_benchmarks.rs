use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use excel_diff::{
    CellValue, DiffConfig, DiffSession, Grid, Sheet, SheetKind, Workbook,
    try_diff_workbooks_with_pool,
};
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

fn single_sheet_workbook(session: &mut DiffSession, name: &str, grid: Grid) -> Workbook {
    let sheet_name = session.strings.intern(name);
    Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    }
}

fn bench_identical_grids(c: &mut Criterion) {
    let mut group = c.benchmark_group("identical_grids");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let grid_b = create_large_grid(*size, 50, 0);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
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
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let mut grid_b = create_large_grid(*size, 50, 0);
        grid_b.insert_cell(size / 2, 25, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
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
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let grid_b = create_large_grid(*size, 50, 1);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
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
        let mut session = DiffSession::new();
        let grid_a = create_repetitive_grid(*size, 50, 100);
        let mut grid_b = create_repetitive_grid(*size, 50, 100);
        grid_b.insert_cell(size / 2, 25, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
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
        let mut session = DiffSession::new();
        let grid_a = create_sparse_grid(*size, 100, 1, 12345);
        let mut grid_b = create_sparse_grid(*size, 100, 1, 12345);
        grid_b.insert_cell(size / 2, 50, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 100));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
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
        let mut session = DiffSession::new();
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
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

#[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
fn bench_pbit_model_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("pbit_model_diff");
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/generated");
    let path_a = base.join("pbit_model_a.pbit");
    let path_b = base.join("pbit_model_b.pbit");

    let bytes_a = std::fs::read(&path_a).expect("read pbit_model_a.pbit");
    let bytes_b = std::fs::read(&path_b).expect("read pbit_model_b.pbit");
    let config = DiffConfig::default();

    group.bench_function("open_parse_diff", |b| {
        b.iter(|| {
            let cursor_a = std::io::Cursor::new(bytes_a.clone());
            let cursor_b = std::io::Cursor::new(bytes_b.clone());
            let pkg_a = excel_diff::PbixPackage::open(cursor_a).expect("open pbit a");
            let pkg_b = excel_diff::PbixPackage::open(cursor_b).expect("open pbit b");
            let report = pkg_a.diff(&pkg_b, &config);
            criterion::black_box(report);
        });
    });

    group.finish();
}

#[cfg(not(all(feature = "model-diff", feature = "excel-open-xml")))]
fn bench_pbit_model_diff(_c: &mut Criterion) {}

fn create_grid_with_block_move(nrows: u32, ncols: u32, move_start: u32, move_size: u32) -> (Grid, Grid) {
    let mut grid_a = Grid::new(nrows, ncols);
    let mut grid_b = Grid::new(nrows, ncols);

    for row in 0..nrows {
        for col in 0..ncols {
            let value = (row * 1000 + col) as f64;
            grid_a.insert_cell(row, col, Some(CellValue::Number(value)), None);
        }
    }

    let move_end = move_start + move_size;
    let dest_start = nrows - move_size - 100;

    for row in 0..move_start {
        for col in 0..ncols {
            let value = (row * 1000 + col) as f64;
            grid_b.insert_cell(row, col, Some(CellValue::Number(value)), None);
        }
    }

    for row in move_end..nrows {
        for col in 0..ncols {
            let value = (row * 1000 + col) as f64;
            let new_row = row - move_size + (dest_start - move_start + move_size);
            if new_row < nrows && new_row != dest_start && (new_row < dest_start || new_row >= dest_start + move_size) {
                grid_b.insert_cell(row - move_size, col, Some(CellValue::Number(value)), None);
            }
        }
    }

    for i in 0..move_size {
        for col in 0..ncols {
            let value = ((move_start + i) * 1000 + col) as f64;
            grid_b.insert_cell(dest_start + i, col, Some(CellValue::Number(value)), None);
        }
    }

    (grid_a, grid_b)
}

fn bench_block_move_alignment(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_move_alignment");
    group.measurement_time(Duration::from_secs(60));
    group.warm_up_time(Duration::from_secs(5));
    group.sample_size(10);

    for size in [5000u32, 10000].iter() {
        let mut session = DiffSession::new();
        let (grid_a, grid_b) = create_grid_with_block_move(*size, 20, 100, 50);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);

        let config = DiffConfig::builder()
            .preflight_min_rows(u32::MAX)
            .max_move_detection_rows(20000)
            .build()
            .expect("valid config");

        group.throughput(Throughput::Elements(*size as u64 * 20));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
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
    bench_pbit_model_diff,
);

criterion_group!(
    name = alignment_benches;
    config = Criterion::default().sample_size(10);
    targets = bench_block_move_alignment,
);

criterion_main!(benches, alignment_benches);
