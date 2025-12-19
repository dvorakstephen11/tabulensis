use std::process::Command;

fn excel_diff_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_excel-diff"))
}

fn fixture_path(name: &str) -> String {
    format!("../fixtures/generated/{}", name)
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
fn key_columns_not_implemented_error() {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--key-columns",
            "A,B",
            &fixture_path("equal_sheet_a.xlsx"),
            &fixture_path("equal_sheet_b.xlsx"),
        ])
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(2),
        "key-columns should exit 2 (not implemented)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not implemented"));
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

