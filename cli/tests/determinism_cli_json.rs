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

fn run_json_diff_with_threads(threads: &str) -> serde_json::Value {
    let output = excel_diff_cmd()
        .args([
            "diff",
            "--format",
            "json",
            &fixture_path("composed_grid_mashup_a.xlsx"),
            &fixture_path("composed_grid_mashup_b.xlsx"),
        ])
        .env("RAYON_NUM_THREADS", threads)
        .output()
        .expect("failed to run excel-diff");

    assert_eq!(
        output.status.code(),
        Some(1),
        "diff should detect changes: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("output should be valid JSON")
}

#[test]
fn json_output_is_deterministic_across_thread_counts() {
    let one = run_json_diff_with_threads("1");
    let two = run_json_diff_with_threads("2");
    let eight = run_json_diff_with_threads("8");

    assert_eq!(one, two, "json output should be stable across threads");
    assert_eq!(one, eight, "json output should be stable across threads");
}
