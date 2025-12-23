#![cfg(feature = "parallel")]

use excel_diff::{CellValue, DiffConfig, DiffContext, Diffable, Grid, with_default_session};
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
