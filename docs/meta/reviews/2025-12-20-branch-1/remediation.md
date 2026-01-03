You’re very close on Branch 1: the CLI crate exists, `diff`/`info` subcommands are wired up, JSON/JSONL output paths exist, and `--git-diff`, presets, and exit codes are implemented with tests.  

That said, there are a few concrete issues/gaps that keep it from being “fully and correctly implemented” per the sprint plan:

## What’s missing / incorrect

1. **`cli` likely won’t compile due to an incorrect core dependency path**
   `cli/Cargo.toml` points `excel_diff` at `./core` (relative to `cli/`), but your workspace layout has `core/` as a sibling of `cli/`, so it should be `../core`.  

2. **`--quiet` behavior is wrong vs its help text**
   The CLI says “Quiet mode: only show summary”  but `text::write_text_report` currently prints the full diff even when `Verbosity::Quiet`, and in the “no diffs” case it prints “No differences found.” and then **suppresses the summary** because `write_summary` returns early when quiet+empty.  

3. **Text output doesn’t include the required “Comparing: old -> new” header**
   The plan’s example output starts with a comparing header.  Your current formatter doesn’t print it. 

4. **There is a syntax error in `cli/src/output/text.rs`**
   `write_text_report` ends with `write_summary(...)?` followed by `Ok(())` without a semicolon, which won’t compile. 

5. **Docs + Git integration deliverables are not present in the codebase context**
   Branch 1 explicitly requires:

* document Git config in README,
* add `.gitattributes` example,
* test with real Git repos. 
  I don’t see any README/docs additions or a git-backed integration test in the updated context.

6. **`--key-columns` is present but hard-errors (“not implemented”)**
   The plan lists it as a Branch 1 deliverable.  Your CLI currently bails if it’s provided. 
   (If you intended it as a placeholder for Branch 2, that’s fine, but then Branch 1 isn’t “complete” per the checklist.)

---

## Fix plan (with targeted patches)

### A) Fix `cli/Cargo.toml` dependency path

Replace this in `cli/Cargo.toml`: 

```toml
excel_diff = { path = "./core" }
```

with:

```toml
excel_diff = { path = "../core" }
```

---

### B) Fix text output: compile error, quiet semantics, and “Comparing:” header

#### B1) Update `cli/src/commands/diff.rs` to pass filenames into the formatter

Replace this block in `cli/src/commands/diff.rs`: 

```rust
OutputFormat::Text => {
    text::write_text_report(&mut handle, &report, verbosity)?;
}
```

with:

```rust
OutputFormat::Text => {
    text::write_text_report(&mut handle, &report, old_path, new_path, verbosity)?;
}
```

#### B2) Replace `write_text_report` in `cli/src/output/text.rs`

Replace the existing function (shown here): 

```rust
pub fn write_text_report<W: Write>(
    w: &mut W,
    report: &DiffReport,
    verbosity: Verbosity,
) -> Result<()> {
    if report.ops.is_empty() {
        writeln!(w, "No differences found.")?;
        write_summary(w, report, verbosity)?;
        return Ok(());
    }

    let (sheet_ops, m_ops) = partition_ops(report);

    for (sheet_name, ops) in &sheet_ops {
        writeln!(w, "Sheet \"{}\":", sheet_name)?;
        for op in ops {
            let lines = render_op(report, op, verbosity);
            for line in lines {
                writeln!(w, "  {}", line)?;
            }
        }
        writeln!(w)?;
    }

    if !m_ops.is_empty() {
        writeln!(w, "Power Query:")?;
        for op in &m_ops {
            let lines = render_op(report, op, verbosity);
            for line in lines {
                writeln!(w, "  {}", line)?;
            }
        }
        writeln!(w)?;
    }

    write_summary(w, report, verbosity)?

    Ok(())
}
```

with:

```rust
pub fn write_text_report<W: Write>(
    w: &mut W,
    report: &DiffReport,
    old_path: &str,
    new_path: &str,
    verbosity: Verbosity,
) -> Result<()> {
    if verbosity != Verbosity::Quiet {
        let old_name = std::path::Path::new(old_path)
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| old_path.into());
        let new_name = std::path::Path::new(new_path)
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| new_path.into());
        writeln!(w, "Comparing: {} -> {}", old_name, new_name)?;
        writeln!(w)?;
    }

    if verbosity == Verbosity::Quiet {
        write_summary(w, report, verbosity)?;
        return Ok(());
    }

    if report.ops.is_empty() {
        writeln!(w, "No differences found.")?;
        write_summary(w, report, verbosity)?;
        return Ok(());
    }

    let (sheet_ops, m_ops) = partition_ops(report);

    for (sheet_name, ops) in &sheet_ops {
        writeln!(w, "Sheet \"{}\":", sheet_name)?;
        for op in ops {
            let lines = render_op(report, op, verbosity);
            for line in lines {
                writeln!(w, "  {}", line)?;
            }
        }
        writeln!(w)?;
    }

    if !m_ops.is_empty() {
        writeln!(w, "Power Query:")?;
        for op in &m_ops {
            let lines = render_op(report, op, verbosity);
            for line in lines {
                writeln!(w, "  {}", line)?;
            }
        }
        writeln!(w)?;
    }

    write_summary(w, report, verbosity)?;
    Ok(())
}
```

This fixes:

* the missing semicolon compile issue, 
* quiet mode now truly prints only summary, consistent with CLI help, 
* adds the required “Comparing:” header from the plan. 

#### B3) Replace `write_summary` so quiet+empty still prints a summary

Replace this function (current behavior returns early): 

```rust
fn write_summary<W: Write>(w: &mut W, report: &DiffReport, verbosity: Verbosity) -> Result<()> {
    if verbosity == Verbosity::Quiet && report.ops.is_empty() {
        return Ok(());
    }

    writeln!(w, "---")?;
    writeln!(w, "Summary:")?;
    writeln!(w, "  Total changes: {}", report.ops.len())?;
    // ...
    Ok(())
}
```

with:

```rust
fn write_summary<W: Write>(w: &mut W, report: &DiffReport, _verbosity: Verbosity) -> Result<()> {
    writeln!(w, "---")?;
    writeln!(w, "Summary:")?;
    writeln!(w, "  Total changes: {}", report.ops.len())?;

    let counts = count_ops(report);
    if counts.sheets > 0 {
        writeln!(w, "  Sheet changes: {}", counts.sheets)?;
    }
    if counts.rows > 0 {
        writeln!(w, "  Row changes: {}", counts.rows)?;
    }
    if counts.cols > 0 {
        writeln!(w, "  Column changes: {}", counts.cols)?;
    }
    if counts.blocks > 0 {
        writeln!(w, "  Block moves: {}", counts.blocks)?;
    }
    if counts.cells > 0 {
        writeln!(w, "  Cell edits: {}", counts.cells)?;
    }
    if counts.queries > 0 {
        writeln!(w, "  Query changes: {}", counts.queries)?;
    }

    if !report.complete {
        writeln!(w, "  Status: INCOMPLETE (some changes may be missing)")?;
    } else {
        writeln!(w, "  Status: complete")?;
    }

    Ok(())
}
```

---

### C) Add Branch 1 docs + Git examples (README/docs + `.gitattributes`)

Since the sprint plan calls these out explicitly, add **at least** one docs page and a copy-pastable `.gitattributes` example. 

Create a new file `docs/git.md`:

```md
# Git integration

## Configure difftool (cell-level diff output)

Add this to your ~/.gitconfig:

[difftool "excel-diff"]
    cmd = excel-diff diff --git-diff "$LOCAL" "$REMOTE"

Then run:

git difftool --tool=excel-diff

## Configure diff driver (structure-only textconv)

Add this to your ~/.gitconfig:

[diff "xlsx"]
    textconv = excel-diff info
    binary = true

Add this to your repo's .gitattributes:

*.xlsx diff=xlsx
*.xlsm diff=xlsx

Then run:

git diff --textconv
```

And add a repo-level example file `.gitattributes.example`:

```gitattributes
*.xlsx diff=xlsx
*.xlsm diff=xlsx
```

(Then link to `docs/git.md` from your README.)

---

### D) “Test with actual Git repositories” via an integration test

Add a test that shells out to `git` in a temp repo, sets the driver config, commits two different `.xlsx` fixtures, and asserts `git diff --textconv` contains `Workbook:`.

1. Add `tempfile` to `cli/Cargo.toml` dev-deps (if you don’t already have it):

```toml
[dev-dependencies]
tempfile = "3"
```

2. Add `cli/tests/git_textconv.rs`:

```rust
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
    let textconv = format!("{} info", exe.display());
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
```

This satisfies “tested with actual Git repositories” in CI without manual steps. 

---

### E) `--key-columns` (either implement or de-scope from Branch 1)

Right now the flag exists but hard-errors.  If you want Branch 1 to match its own checklist, you have two good options:

**Option 1 (fastest, keeps Branch 1 focused):** keep the flag, but change the Branch 1 checklist/docs to call it a placeholder for Branch 2.

**Option 2 (complete the deliverable):** implement “database mode” end-to-end per Branch 2 (add `WorkbookPackage::diff_database_mode` + CLI flags `--database --sheet --keys`). The plan already outlines that design. 
If you want, I can give you a concrete patch plan for the core + CLI wiring that reuses your existing `diff_grids_database_mode` engine path. 

---

If you apply A + B + C + D, you’ll meet all Branch 1 acceptance criteria except the `--key-columns` functional requirement (depending on whether you interpret that as placeholder or required behavior). 
