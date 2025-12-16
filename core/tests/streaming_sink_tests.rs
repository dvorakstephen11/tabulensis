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

