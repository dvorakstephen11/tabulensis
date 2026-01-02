use excel_diff::{
    CellValue, DiffConfig, Grid, LimitBehavior, diff_grids_database_mode, with_default_session,
};

#[test]
fn database_mode_wrapper_limits_exceeded_returns_incomplete_report() {
    let mut grid_a = Grid::new(2, 1);
    grid_a.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
    grid_a.insert_cell(1, 0, Some(CellValue::Number(1.0)), None);

    let mut grid_b = Grid::new(2, 1);
    grid_b.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
    grid_b.insert_cell(1, 0, Some(CellValue::Number(1.0)), None);

    let mut config = DiffConfig::default();
    config.alignment.max_align_rows = 1;
    config.hardening.on_limit_exceeded = LimitBehavior::ReturnError;

    let result = std::panic::catch_unwind(|| {
        with_default_session(|session| {
            diff_grids_database_mode(&grid_a, &grid_b, &[0], &mut session.strings, &config)
        })
    });
    assert!(result.is_ok(), "database mode wrapper should not panic");
    let report = result.unwrap();

    assert!(!report.complete, "report should be marked incomplete");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("alignment limits exceeded")),
        "expected limits exceeded warning; warnings: {:?}",
        report.warnings
    );
}

