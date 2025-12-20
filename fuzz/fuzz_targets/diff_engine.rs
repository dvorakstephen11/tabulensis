#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let rows = data.first().copied().unwrap_or(0) % 16;
    let cols = data.get(1).copied().unwrap_or(0) % 16;
    let rows = rows as u32;
    let cols = cols as u32;

    let mut pool = excel_diff::StringPool::new();
    let sheet_id = pool.intern("Sheet1");

    let mut grid_a = excel_diff::Grid::new(rows, cols);
    let mut grid_b = excel_diff::Grid::new(rows, cols);

    if rows > 0 && cols > 0 {
        let mut i = 2usize;
        let mut inserted = 0usize;
        while i + 3 < data.len() && inserted < 64 {
            let which = data[i] & 1;
            let r = (data[i + 1] as u32) % rows;
            let c = (data[i + 2] as u32) % cols;
            let v = data[i + 3] as f64;
            let value = Some(excel_diff::CellValue::Number(v));

            if which == 0 {
                grid_a.insert_cell(r, c, value, None);
            } else {
                grid_b.insert_cell(r, c, value, None);
            }

            inserted += 1;
            i += 4;
        }
    }

    let wb_a = excel_diff::Workbook {
        sheets: vec![excel_diff::Sheet {
            name: sheet_id,
            kind: excel_diff::SheetKind::Worksheet,
            grid: grid_a,
        }],
    };
    let wb_b = excel_diff::Workbook {
        sheets: vec![excel_diff::Sheet {
            name: sheet_id,
            kind: excel_diff::SheetKind::Worksheet,
            grid: grid_b,
        }],
    };

    let config = excel_diff::DiffConfig::default();
    let mut op_count = 0usize;
    let mut sink = excel_diff::CallbackSink::new(|_op| op_count = op_count.saturating_add(1));
    let _ = excel_diff::advanced::try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        &mut pool,
        &config,
        &mut sink,
    );
});

