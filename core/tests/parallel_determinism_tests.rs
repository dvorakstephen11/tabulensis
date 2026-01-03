#![cfg(feature = "parallel")]

use excel_diff::{
    CellValue, DiffConfig, DiffContext, DiffSession, Diffable, Grid, Sheet, SheetKind, StringPool,
    VecSink, Workbook, try_diff_grids_database_mode_streaming, try_diff_workbooks_streaming,
    with_default_session,
};
use rayon::ThreadPoolBuilder;

fn run_in_pool<T>(threads: usize, f: impl FnOnce() -> T + Send) -> T
where
    T: Send,
{
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("build pool");
    pool.install(f)
}

fn make_workbook(pool: &mut StringPool, value: f64) -> Workbook {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(value)), None);

    Workbook {
        sheets: vec![Sheet {
            name: pool.intern("Sheet1"),
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    }
}

fn make_keyed_grid(keys: &[i32], values: &[i32]) -> Grid {
    let rows = keys.len().max(values.len());
    let mut grid = Grid::new(rows as u32, 2);
    for row in 0..rows {
        let key = keys.get(row).copied().unwrap_or_default() as f64;
        let value = values.get(row).copied().unwrap_or_default() as f64;
        grid.insert_cell(row as u32, 0, Some(CellValue::Number(key)), None);
        grid.insert_cell(row as u32, 1, Some(CellValue::Number(value)), None);
    }
    grid
}

#[test]
fn ops_are_identical_across_thread_counts() {
    let rows = 10_000u32;
    let cols = 50u32;

    let mut a = Grid::new_dense(rows, cols);
    let mut b = Grid::new_dense(rows, cols);

    for r in 0..rows {
        for c in 0..cols {
            let value = Some(CellValue::Number((r * 100 + c) as f64));
            a.insert_cell(r, c, value.clone(), None);
            b.insert_cell(r, c, value, None);
        }
    }

    b.insert_cell(5000, 25, Some(CellValue::Number(123456.0)), None);

    let config = DiffConfig::default();

    let ops_1 = run_in_pool(1, || {
        with_default_session(|session| {
            let mut ctx = DiffContext::new(&mut session.strings, &config);
            let report = a.diff(&b, &mut ctx);
            report.ops
        })
    });

    let ops_4 = run_in_pool(4, || {
        with_default_session(|session| {
            let mut ctx = DiffContext::new(&mut session.strings, &config);
            let report = a.diff(&b, &mut ctx);
            report.ops
        })
    });

    assert_eq!(ops_1, ops_4);
}

#[test]
fn ops_are_identical_across_thread_counts_block_move() {
    let rows = 1000u32;
    let cols = 20u32;
    let block_size = 100u32;

    let mut a = Grid::new_dense(rows, cols);
    let mut b = Grid::new_dense(rows, cols);

    for r in 0..rows {
        for c in 0..cols {
            let value = Some(CellValue::Number((r * 100 + c) as f64));
            a.insert_cell(r, c, value.clone(), None);
            b.insert_cell(r, c, value, None);
        }
    }

    for r in 0..block_size {
        for c in 0..cols {
            let src_row = 200 + r;
            let dst_row = 600 + r;
            let value = a.get(src_row, c).and_then(|cell| cell.value.clone());
            b.insert_cell(dst_row, c, value, None);
        }
    }

    let config = DiffConfig::default();

    let ops_1 = run_in_pool(1, || {
        with_default_session(|session| {
            let mut ctx = DiffContext::new(&mut session.strings, &config);
            let report = a.diff(&b, &mut ctx);
            report.ops
        })
    });

    let ops_4 = run_in_pool(4, || {
        with_default_session(|session| {
            let mut ctx = DiffContext::new(&mut session.strings, &config);
            let report = a.diff(&b, &mut ctx);
            report.ops
        })
    });

    assert_eq!(ops_1, ops_4);
}

#[test]
fn streaming_workbook_ops_are_identical_across_thread_counts() {
    let config = DiffConfig::default();

    let output_1 = run_in_pool(1, || {
        let mut session = DiffSession::new();
        let wb_a = make_workbook(&mut session.strings, 1.0);
        let wb_b = make_workbook(&mut session.strings, 2.0);
        let mut sink = VecSink::new();
        let summary = try_diff_workbooks_streaming(
            &wb_a,
            &wb_b,
            &mut session.strings,
            &config,
            &mut sink,
        )
        .expect("streaming diff should succeed");
        (sink.into_ops(), summary)
    });

    let output_4 = run_in_pool(4, || {
        let mut session = DiffSession::new();
        let wb_a = make_workbook(&mut session.strings, 1.0);
        let wb_b = make_workbook(&mut session.strings, 2.0);
        let mut sink = VecSink::new();
        let summary = try_diff_workbooks_streaming(
            &wb_a,
            &wb_b,
            &mut session.strings,
            &config,
            &mut sink,
        )
        .expect("streaming diff should succeed");
        (sink.into_ops(), summary)
    });

    assert_eq!(output_1, output_4);
}

#[test]
fn streaming_database_mode_ops_are_identical_across_thread_counts() {
    let config = DiffConfig::default();

    let output_1 = run_in_pool(1, || {
        let mut session = DiffSession::new();
        let grid_a = make_keyed_grid(&[1, 2], &[10, 20]);
        let grid_b = make_keyed_grid(&[1, 2], &[10, 25]);
        let sheet_id = session.strings.intern("Data");
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let summary = try_diff_grids_database_mode_streaming(
            sheet_id,
            &grid_a,
            &grid_b,
            &[0],
            &mut session.strings,
            &config,
            &mut sink,
            &mut op_count,
        )
        .expect("database streaming diff should succeed");
        (sink.into_ops(), summary)
    });

    let output_4 = run_in_pool(4, || {
        let mut session = DiffSession::new();
        let grid_a = make_keyed_grid(&[1, 2], &[10, 20]);
        let grid_b = make_keyed_grid(&[1, 2], &[10, 25]);
        let sheet_id = session.strings.intern("Data");
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let summary = try_diff_grids_database_mode_streaming(
            sheet_id,
            &grid_a,
            &grid_b,
            &[0],
            &mut session.strings,
            &config,
            &mut sink,
            &mut op_count,
        )
        .expect("database streaming diff should succeed");
        (sink.into_ops(), summary)
    });

    assert_eq!(output_1, output_4);
}
