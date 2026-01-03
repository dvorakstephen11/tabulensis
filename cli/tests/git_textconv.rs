use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn run_git(repo: &PathBuf, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git failed: {:?}", out);
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn git_textconv_uses_excel_diff_info() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().to_path_buf();

    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "Test"]);

    fs::write(repo.join(".gitattributes"), "*.xlsx diff=xlsx\n").expect("write gitattributes");

    let exe = PathBuf::from(env!("CARGO_BIN_EXE_excel-diff"));
    let exe_str = exe.to_string_lossy().replace('\\', "/");
    let textconv = format!("\"{}\" info", exe_str);
    run_git(&repo, &["config", "diff.xlsx.binary", "true"]);
    run_git(&repo, &["config", "diff.xlsx.textconv", &textconv]);

    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures")
        .join("generated");

    let a = fixture_dir.join("pg1_basic_two_sheets.xlsx");
    let b = fixture_dir.join("one_query.xlsx");

    let target = repo.join("book.xlsx");
    fs::copy(&a, &target).expect("copy fixture a");
    run_git(&repo, &["add", "book.xlsx"]);
    run_git(&repo, &["commit", "-m", "add book"]);

    fs::copy(&b, &target).expect("copy fixture b");

    let diff = run_git(&repo, &["diff", "--textconv"]);

    assert!(diff.contains("Workbook:"), "expected textconv output");
    assert!(diff.contains("Sheets:"), "expected workbook structure");
}

