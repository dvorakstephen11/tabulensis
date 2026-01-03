use excel_diff::{
    CallbackSink, CellValue, DiffConfig, DiffSession, Grid, Sheet, SheetKind, Workbook,
    try_diff_workbooks_streaming,
};
use core::hint::black_box;

fn make_workbook(session: &mut DiffSession, value: f64) -> Workbook {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(value)), None);

    let sheet_name = session.strings.intern("WasmSmoke");

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

fn main() {
    let mut session = DiffSession::new();
    let wb_a = make_workbook(&mut session, 1.0);
    let wb_b = make_workbook(&mut session, 2.0);

    let mut op_count = 0usize;
    let mut sink = CallbackSink::new(|_op| op_count += 1);
    if let Ok(summary) = try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        &mut session.strings,
        &DiffConfig::default(),
        &mut sink,
    ) {
        black_box(summary.complete);
        black_box(summary.op_count);
        black_box(op_count);
    }
}
