#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;

use excel_diff::{
    Cell, CellValue, DiffConfig, Grid, Sheet, SheetKind, StringPool, Workbook,
    advanced::try_diff_workbooks_with_pool,
};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    old_rows: u8,
    old_cols: u8,
    new_rows: u8,
    new_cols: u8,
    old_cells: Vec<FuzzCell>,
    new_cells: Vec<FuzzCell>,
}

#[derive(Arbitrary, Debug)]
struct FuzzCell {
    row: u8,
    col: u8,
    value_type: u8,
    number_value: f64,
    text_idx: u8,
}

fn build_grid(rows: u8, cols: u8, cells: &[FuzzCell], pool: &mut StringPool) -> Grid {
    let nrows = (rows as u32).min(100).max(1);
    let ncols = (cols as u32).min(100).max(1);
    let mut grid = Grid::new(nrows, ncols);

    for cell in cells.iter().take(200) {
        let row = (cell.row as u32) % nrows;
        let col = (cell.col as u32) % ncols;

        let value = match cell.value_type % 4 {
            0 => None,
            1 => Some(CellValue::Number(if cell.number_value.is_finite() {
                cell.number_value
            } else {
                0.0
            })),
            2 => Some(CellValue::Bool(cell.number_value > 0.5)),
            _ => {
                let texts = ["A", "B", "C", "test", "value", ""];
                let idx = (cell.text_idx as usize) % texts.len();
                Some(CellValue::Text(pool.intern(texts[idx])))
            }
        };

        grid.insert_cell(row, col, value, None);
    }

    grid
}

fuzz_target!(|input: FuzzInput| {
    let mut pool = StringPool::new();

    let old_grid = build_grid(input.old_rows, input.old_cols, &input.old_cells, &mut pool);
    let new_grid = build_grid(input.new_rows, input.new_cols, &input.new_cells, &mut pool);

    let sheet_name = pool.intern("Sheet1");
    let old_wb = Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            kind: SheetKind::Worksheet,
            grid: old_grid,
        }],
    };
    let new_wb = Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            kind: SheetKind::Worksheet,
            grid: new_grid,
        }],
    };

    let config = DiffConfig {
        max_align_rows: 50,
        max_align_cols: 50,
        ..Default::default()
    };

    let _ = try_diff_workbooks_with_pool(&old_wb, &new_wb, &mut pool, &config);
});

