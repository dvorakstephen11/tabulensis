use std::process::Command;

fn excel_diff_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_excel-diff"))
}

fn fixture_path(name: &str) -> String {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures")
        .join("generated")
        .join(name);

    p.to_string_lossy().into_owned()
}

fn run_jsonl_diff(old_path: &str, new_path: &str) -> String {
    let output = excel_diff_cmd()
        .args(["diff", "--format", "jsonl", old_path, new_path])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "jsonl diff should detect changes: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn identical_files_exit_0() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            &fixture_path("equal_sheet_a.xlsx"),
            &fixture_path("equal_sheet_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert!(
        output.status.success(),
        "identical files should exit 0: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn different_files_exit_1() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "different files should exit 1: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn max_memory_zero_exits_1_and_warns() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--max-memory",
            "0",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "memory-capped diff should exit 1: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Warning:"), "should print a warning");
    assert!(
        stderr.to_lowercase().contains("memory"),
        "warning should mention memory: {}",
        stderr
    );
}

#[test]
fn timeout_zero_exits_1_and_warns() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--timeout",
            "0",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "timeout diff should exit 1: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Warning:"), "should print a warning");
    assert!(
        stderr.to_lowercase().contains("timeout"),
        "warning should mention timeout: {}",
        stderr
    );
}

#[test]
fn nonexistent_file_exit_2() {
    let output = excel_diff_cmd()
        .args(["diff", "nonexistent_a.xlsx", "nonexistent_b.xlsx"])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "nonexistent file should exit 2: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn json_output_is_valid_json() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "json",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    assert!(parsed.get("version").is_some(), "should have version field");
    assert!(parsed.get("ops").is_some(), "should have ops field");
    assert!(parsed.get("strings").is_some(), "should have strings field");
}

#[test]
fn jsonl_first_line_is_header() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "jsonl",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().expect("should have at least one line");
    let header: serde_json::Value =
        serde_json::from_str(first_line).expect("first line should be valid JSON");

    assert_eq!(header.get("kind").and_then(|v| v.as_str()), Some("Header"));
    assert!(header.get("version").is_some());
    assert!(header.get("strings").is_some());
}

#[test]
fn jsonl_progress_keeps_stdout_jsonl_and_writes_to_stderr() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "jsonl",
            "--progress",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "diff with progress should exit 1: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for (idx, line) in stdout.lines().enumerate() {
        serde_json::from_str::<serde_json::Value>(line).unwrap_or_else(|e| {
            panic!("stdout line {idx} should be valid JSON: {e}; line={line}");
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "progress should write to stderr (even in tests): stdout_len={}, stderr_len={}",
        stdout.len(),
        stderr.len()
    );
}

#[test]
fn info_shows_sheets() {
    let output = excel_diff_cmd()
        .args(["info", &fixture_path("pg1_basic_two_sheets.xlsx")])
        .output()
        .expect("failed to run excel-diff");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sheets:"));
}

#[test]
fn info_with_queries_shows_power_query() {
    let output = excel_diff_cmd()
        .args(["info", "--queries", &fixture_path("one_query.xlsx")])
        .output()
        .expect("failed to run excel-diff");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Power Query:"));
}

#[test]
fn fast_and_precise_are_mutually_exclusive() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--fast",
            "--precise",
            &fixture_path("equal_sheet_a.xlsx"),
            &fixture_path("equal_sheet_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "conflicting flags should exit 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot use both"));
}

#[test]
fn database_mode_requires_keys_or_auto_keys() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "database without keys should exit 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--keys") || stderr.contains("--auto-keys"));
}

#[test]
fn database_flags_require_database_mode() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--keys",
            "A",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "keys without database flag should exit 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--database"));
}

#[test]
fn git_diff_produces_unified_style() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--git-diff",
            &fixture_path("single_cell_value_a.xlsx"),
            &fixture_path("single_cell_value_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("diff --git"));
    assert!(stdout.contains("---"));
    assert!(stdout.contains("+++"));
    assert!(stdout.contains("@@"));
}

#[test]
fn git_diff_conflicts_with_json_format() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--git-diff",
            "--format",
            "json",
            &fixture_path("equal_sheet_a.xlsx"),
            &fixture_path("equal_sheet_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "git-diff with json format should exit 2"
    );
}

#[test]
fn row_changes_detected() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            &fixture_path("row_insert_middle_a.xlsx"),
            &fixture_path("row_insert_middle_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Row") && stdout.contains("ADDED"));
}

#[test]
fn column_changes_detected() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            &fixture_path("col_insert_middle_a.xlsx"),
            &fixture_path("col_insert_middle_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Column") && stdout.contains("ADDED"));
}

#[test]
fn power_query_changes_detected() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            &fixture_path("m_add_query_a.xlsx"),
            &fixture_path("m_add_query_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Power Query") || stdout.contains("Query"));
}

#[test]
fn diff_pbix_power_query_changes_detected() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "json",
            &fixture_path("pbix_legacy_multi_query_a.pbix"),
            &fixture_path("pbix_legacy_multi_query_b.pbix"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "pbix diff should detect changes: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let ops = parsed.get("ops").and_then(|v| v.as_array()).unwrap();
    let has_query_op = ops.iter().any(|op| {
        op.get("kind")
            .and_then(|k| k.as_str())
            .map(|k| k.starts_with("Query"))
            .unwrap_or(false)
    });
    assert!(has_query_op, "expected at least one Query op in pbix diff");
}

#[test]
fn diff_pbix_jsonl_writes_header_and_ops() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "jsonl",
            &fixture_path("pbix_legacy_multi_query_a.pbix"),
            &fixture_path("pbix_legacy_multi_query_b.pbix"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "pbix jsonl diff should detect changes: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let first_line = lines.next().expect("jsonl should have a header line");
    let header: serde_json::Value =
        serde_json::from_str(first_line).expect("header line should be valid JSON");
    assert_eq!(header.get("kind").and_then(|v| v.as_str()), Some("Header"));
    let strings = header
        .get("strings")
        .and_then(|v| v.as_array())
        .expect("header should include string table");
    assert!(!strings.is_empty(), "header string table should be non-empty");

    let mut has_query_op = false;
    for line in lines {
        let op: excel_diff::DiffOp =
            serde_json::from_str(line).expect("jsonl op line should be valid JSON");
        for id in collect_string_ids(&op) {
            assert!(
                (id.0 as usize) < strings.len(),
                "StringId {} out of range for header string table (len={})",
                id.0,
                strings.len()
            );
        }

        if op.is_m_op() {
            has_query_op = true;
        }
    }

    assert!(has_query_op, "expected at least one Query op in jsonl output");
}

#[test]
fn jsonl_output_is_deterministic_for_xlsx() {
    let first = run_jsonl_diff(
        &fixture_path("single_cell_value_a.xlsx"),
        &fixture_path("single_cell_value_b.xlsx"),
    );
    let second = run_jsonl_diff(
        &fixture_path("single_cell_value_a.xlsx"),
        &fixture_path("single_cell_value_b.xlsx"),
    );

    assert_eq!(first, second, "jsonl output should be deterministic");
}

#[test]
fn jsonl_output_is_deterministic_for_pbix() {
    let first = run_jsonl_diff(
        &fixture_path("pbix_legacy_multi_query_a.pbix"),
        &fixture_path("pbix_legacy_multi_query_b.pbix"),
    );
    let second = run_jsonl_diff(
        &fixture_path("pbix_legacy_multi_query_a.pbix"),
        &fixture_path("pbix_legacy_multi_query_b.pbix"),
    );

    assert_eq!(first, second, "jsonl output should be deterministic");
}

fn collect_string_ids(op: &excel_diff::DiffOp) -> Vec<excel_diff::StringId> {
    fn collect_cell_value(ids: &mut Vec<excel_diff::StringId>, value: &excel_diff::CellValue) {
        match value {
            excel_diff::CellValue::Text(id) | excel_diff::CellValue::Error(id) => ids.push(*id),
            excel_diff::CellValue::Number(_) | excel_diff::CellValue::Bool(_) | excel_diff::CellValue::Blank => {}
        }
    }

    fn collect_snapshot(ids: &mut Vec<excel_diff::StringId>, snap: &excel_diff::CellSnapshot) {
        if let Some(value) = &snap.value {
            collect_cell_value(ids, value);
        }
        if let Some(formula) = snap.formula {
            ids.push(formula);
        }
    }

    fn collect_extracted_string(ids: &mut Vec<excel_diff::StringId>, value: &excel_diff::ExtractedString) {
        if let excel_diff::ExtractedString::Known { value } = value {
            ids.push(*value);
        }
    }

    fn collect_extracted_string_list(
        ids: &mut Vec<excel_diff::StringId>,
        value: &excel_diff::ExtractedStringList,
    ) {
        if let excel_diff::ExtractedStringList::Known { values } = value {
            ids.extend(values.iter().copied());
        }
    }

    fn collect_rename_pairs(
        ids: &mut Vec<excel_diff::StringId>,
        value: &excel_diff::ExtractedRenamePairs,
    ) {
        if let excel_diff::ExtractedRenamePairs::Known { pairs } = value {
            for excel_diff::RenamePair { from, to } in pairs {
                ids.push(*from);
                ids.push(*to);
            }
        }
    }

    fn collect_column_type_changes(
        ids: &mut Vec<excel_diff::StringId>,
        value: &excel_diff::ExtractedColumnTypeChanges,
    ) {
        if let excel_diff::ExtractedColumnTypeChanges::Known { changes } = value {
            for change in changes {
                ids.push(change.column);
            }
        }
    }

    fn collect_step_params(ids: &mut Vec<excel_diff::StringId>, params: &excel_diff::StepParams) {
        match params {
            excel_diff::StepParams::TableSelectRows { .. } => {}
            excel_diff::StepParams::TableRemoveColumns { columns } => collect_extracted_string_list(ids, columns),
            excel_diff::StepParams::TableRenameColumns { renames } => collect_rename_pairs(ids, renames),
            excel_diff::StepParams::TableTransformColumnTypes { transforms } => {
                collect_column_type_changes(ids, transforms);
            }
            excel_diff::StepParams::TableNestedJoin {
                left_keys,
                right_keys,
                new_column,
                ..
            } => {
                collect_extracted_string_list(ids, left_keys);
                collect_extracted_string_list(ids, right_keys);
                collect_extracted_string(ids, new_column);
            }
            excel_diff::StepParams::TableJoin {
                left_keys,
                right_keys,
                ..
            } => {
                collect_extracted_string_list(ids, left_keys);
                collect_extracted_string_list(ids, right_keys);
            }
            excel_diff::StepParams::Other { .. } => {}
        }
    }

    fn collect_step_snapshot(ids: &mut Vec<excel_diff::StringId>, snapshot: &excel_diff::StepSnapshot) {
        ids.push(snapshot.name);
        ids.extend(snapshot.source_refs.iter().copied());
        if let Some(params) = &snapshot.params {
            collect_step_params(ids, params);
        }
    }

    fn collect_step_diff(ids: &mut Vec<excel_diff::StringId>, diff: &excel_diff::StepDiff) {
        match diff {
            excel_diff::StepDiff::StepAdded { step } | excel_diff::StepDiff::StepRemoved { step } => {
                collect_step_snapshot(ids, step);
            }
            excel_diff::StepDiff::StepReordered { name, .. } => ids.push(*name),
            excel_diff::StepDiff::StepModified { before, after, changes } => {
                collect_step_snapshot(ids, before);
                collect_step_snapshot(ids, after);
                for change in changes {
                    if let excel_diff::StepChange::Renamed { from, to } = change {
                        ids.push(*from);
                        ids.push(*to);
                    }
                    if let excel_diff::StepChange::SourceRefsChanged { removed, added } = change {
                        ids.extend(removed.iter().copied());
                        ids.extend(added.iter().copied());
                    }
                }
            }
        }
    }

    fn collect_semantic_detail(ids: &mut Vec<excel_diff::StringId>, detail: &excel_diff::QuerySemanticDetail) {
        for diff in &detail.step_diffs {
            collect_step_diff(ids, diff);
        }
    }

    let mut ids = Vec::new();
    match op {
        excel_diff::DiffOp::SheetAdded { sheet } | excel_diff::DiffOp::SheetRemoved { sheet } => ids.push(*sheet),
        excel_diff::DiffOp::RowAdded { sheet, .. }
        | excel_diff::DiffOp::RowRemoved { sheet, .. }
        | excel_diff::DiffOp::RowReplaced { sheet, .. } => ids.push(*sheet),
        excel_diff::DiffOp::ColumnAdded { sheet, .. } | excel_diff::DiffOp::ColumnRemoved { sheet, .. } => ids.push(*sheet),
        excel_diff::DiffOp::BlockMovedRows { sheet, .. }
        | excel_diff::DiffOp::BlockMovedColumns { sheet, .. }
        | excel_diff::DiffOp::BlockMovedRect { sheet, .. }
        | excel_diff::DiffOp::RectReplaced { sheet, .. } => ids.push(*sheet),
        excel_diff::DiffOp::CellEdited { sheet, from, to, .. } => {
            ids.push(*sheet);
            collect_snapshot(&mut ids, from);
            collect_snapshot(&mut ids, to);
        }
        excel_diff::DiffOp::VbaModuleAdded { name }
        | excel_diff::DiffOp::VbaModuleRemoved { name }
        | excel_diff::DiffOp::VbaModuleChanged { name } => ids.push(*name),
        excel_diff::DiffOp::NamedRangeAdded { name } | excel_diff::DiffOp::NamedRangeRemoved { name } => ids.push(*name),
        excel_diff::DiffOp::NamedRangeChanged { name, old_ref, new_ref } => {
            ids.push(*name);
            ids.push(*old_ref);
            ids.push(*new_ref);
        }
        excel_diff::DiffOp::ChartAdded { sheet, name }
        | excel_diff::DiffOp::ChartRemoved { sheet, name }
        | excel_diff::DiffOp::ChartChanged { sheet, name } => {
            ids.push(*sheet);
            ids.push(*name);
        }
        excel_diff::DiffOp::QueryAdded { name }
        | excel_diff::DiffOp::QueryRemoved { name }
        | excel_diff::DiffOp::QueryDefinitionChanged { name, .. } => ids.push(*name),
        excel_diff::DiffOp::QueryRenamed { from, to } => {
            ids.push(*from);
            ids.push(*to);
        }
        excel_diff::DiffOp::QueryMetadataChanged { name, old, new, .. } => {
            ids.push(*name);
            if let Some(old) = old {
                ids.push(*old);
            }
            if let Some(new) = new {
                ids.push(*new);
            }
        }
        excel_diff::DiffOp::MeasureAdded { name }
        | excel_diff::DiffOp::MeasureRemoved { name }
        | excel_diff::DiffOp::MeasureDefinitionChanged { name, .. } => ids.push(*name),
        _ => {}
    }

    if let excel_diff::DiffOp::QueryDefinitionChanged { semantic_detail, .. } = op {
        if let Some(detail) = semantic_detail {
            collect_semantic_detail(&mut ids, detail);
        }
    }

    ids
}

#[test]
fn diff_pbit_measure_changes_detected() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "json",
            &fixture_path("pbit_model_a.pbit"),
            &fixture_path("pbit_model_b.pbit"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "pbit diff should detect changes: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let ops = parsed.get("ops").and_then(|v| v.as_array()).unwrap();
    let has_measure_op = ops.iter().any(|op| {
        op.get("kind")
            .and_then(|k| k.as_str())
            .map(|k| k.starts_with("Measure"))
            .unwrap_or(false)
    });
    assert!(has_measure_op, "expected at least one Measure op in pbit diff");
}

#[test]
fn d1_database_reorder_no_diff() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "A",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert!(
        output.status.success(),
        "D1 reorder should exit 0 (no changes): stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn d1_database_reorder_json_empty_ops() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "A",
            "--format",
            "json",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let ops = parsed.get("ops").and_then(|v| v.as_array());
    assert!(
        ops.map(|o| o.is_empty()).unwrap_or(false),
        "D1 reorder should have empty ops array"
    );
}

#[test]
fn d2_database_row_added() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "A",
            "--format",
            "json",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_row_added_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "D2 row added should exit 1"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let ops = parsed.get("ops").and_then(|v| v.as_array()).unwrap();
    let has_row_added = ops.iter().any(|op| {
        op.get("kind").and_then(|k| k.as_str()) == Some("RowAdded")
    });
    assert!(has_row_added, "D2 should contain RowAdded op");
}

#[test]
fn d3_database_row_updated() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "A",
            "--format",
            "json",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_row_update_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "D3 row update should exit 1"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let ops = parsed.get("ops").and_then(|v| v.as_array()).unwrap();
    let has_cell_edited = ops.iter().any(|op| {
        op.get("kind").and_then(|k| k.as_str()) == Some("CellEdited")
    });
    assert!(has_cell_edited, "D3 should contain CellEdited op");
}

#[test]
fn d4_database_reorder_and_change() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "A",
            "--format",
            "json",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_reorder_and_change_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "D4 reorder+change should exit 1"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let ops = parsed.get("ops").and_then(|v| v.as_array()).unwrap();
    
    let has_cell_edited = ops.iter().any(|op| {
        op.get("kind").and_then(|k| k.as_str()) == Some("CellEdited")
    });
    assert!(has_cell_edited, "D4 should contain CellEdited op");
    
    assert!(
        ops.len() < 10,
        "D4 should have few ops (reorder ignored, only changes): got {} ops",
        ops.len()
    );
}

#[test]
fn database_multi_column_keys() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "A,C",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert!(
        output.status.success(),
        "Multi-column keys should work: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn database_invalid_column_error() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--keys",
            "1",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Invalid column should exit 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a valid column") || stderr.contains("Invalid"),
        "Should mention invalid column: {}",
        stderr
    );
}

#[test]
fn database_sheet_not_found_error() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "NoSuchSheet",
            "--keys",
            "A",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Sheet not found should exit 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Available"),
        "Should mention sheet not found: {}",
        stderr
    );
}

#[test]
fn database_auto_keys() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--database",
            "--sheet",
            "Data",
            "--auto-keys",
            &fixture_path("db_equal_ordered_a.xlsx"),
            &fixture_path("db_equal_ordered_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert!(
        output.status.success(),
        "Auto-keys should work: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Auto-detected") || stderr.is_empty(),
        "Should print auto-detected message or be silent"
    );
}

#[test]
fn info_pbix_includes_embedded_queries() {
    let output = excel_diff_cmd()
        .args([
            "info",
            "--queries",
            &fixture_path("pbix_embedded_queries.pbix"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert!(
        output.status.success(),
        "info should succeed for pbix: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PBIX/PBIT:"),
        "expected pbix header, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Embedded/"),
        "expected embedded queries to be listed, got: {}",
        stdout
    );
}

