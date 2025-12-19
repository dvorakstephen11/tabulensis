use crate::output::{git_diff, json, text};
use crate::OutputFormat;
use anyhow::{Context, Result, bail};
use excel_diff::{
    DiffConfig, DiffReport, Grid, JsonLinesSink, WorkbookPackage,
    index_to_address, suggest_key_columns, with_default_session,
};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    old_path: &str,
    new_path: &str,
    format: OutputFormat,
    git_diff_mode: bool,
    fast: bool,
    precise: bool,
    quiet: bool,
    verbose: bool,
    database: bool,
    sheet: Option<String>,
    keys: Option<String>,
    auto_keys: bool,
) -> Result<ExitCode> {
    if fast && precise {
        bail!("Cannot use both --fast and --precise flags together");
    }

    if git_diff_mode && (format == OutputFormat::Json || format == OutputFormat::Jsonl) {
        bail!("Cannot use --git-diff with --format=json or --format=jsonl");
    }

    if !database && (sheet.is_some() || keys.is_some() || auto_keys) {
        bail!("--sheet, --keys, and --auto-keys require --database flag");
    }

    if database && keys.is_none() && !auto_keys {
        bail!("Database mode requires either --keys or --auto-keys");
    }

    if database && keys.is_some() && auto_keys {
        bail!("Cannot use both --keys and --auto-keys together");
    }

    let verbosity = if quiet {
        Verbosity::Quiet
    } else if verbose {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    let config = build_config(fast, precise);

    let old_file = File::open(old_path)
        .with_context(|| format!("Failed to open old workbook: {}", old_path))?;
    let new_file = File::open(new_path)
        .with_context(|| format!("Failed to open new workbook: {}", new_path))?;

    let old_pkg = WorkbookPackage::open(old_file)
        .with_context(|| format!("Failed to parse old workbook: {}", old_path))?;
    let new_pkg = WorkbookPackage::open(new_file)
        .with_context(|| format!("Failed to parse new workbook: {}", new_path))?;

    if database {
        return run_database_mode(
            &old_pkg,
            &new_pkg,
            old_path,
            new_path,
            format,
            git_diff_mode,
            &config,
            verbosity,
            sheet,
            keys,
            auto_keys,
        );
    }

    if format == OutputFormat::Jsonl && !git_diff_mode {
        return run_streaming(&old_pkg, &new_pkg, &config);
    }

    let report = old_pkg.diff(&new_pkg, &config);

    print_warnings_to_stderr(&report);

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if git_diff_mode {
        git_diff::write_git_diff(&mut handle, &report, old_path, new_path)?;
    } else {
        match format {
            OutputFormat::Text => {
                text::write_text_report(&mut handle, &report, old_path, new_path, verbosity)?;
            }
            OutputFormat::Json => {
                json::write_json_report(&mut handle, &report)?;
            }
            OutputFormat::Jsonl => {
                unreachable!("JSONL handled by streaming path");
            }
        }
    }

    Ok(exit_code_from_report(&report))
}

fn run_streaming(
    old_pkg: &WorkbookPackage,
    new_pkg: &WorkbookPackage,
    config: &DiffConfig,
) -> Result<ExitCode> {
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut writer = BufWriter::new(handle);
    let mut sink = JsonLinesSink::new(&mut writer);

    let summary = old_pkg
        .diff_streaming(new_pkg, config, &mut sink)
        .context("Streaming diff failed")?;

    writer.flush()?;

    for warning in &summary.warnings {
        eprintln!("Warning: {}", warning);
    }

    if summary.op_count == 0 && summary.complete {
        Ok(ExitCode::from(0))
    } else {
        Ok(ExitCode::from(1))
    }
}

#[allow(clippy::too_many_arguments)]
fn run_database_mode(
    old_pkg: &WorkbookPackage,
    new_pkg: &WorkbookPackage,
    old_path: &str,
    new_path: &str,
    format: OutputFormat,
    git_diff_mode: bool,
    config: &DiffConfig,
    verbosity: Verbosity,
    sheet: Option<String>,
    keys: Option<String>,
    auto_keys: bool,
) -> Result<ExitCode> {
    let sheet_name = determine_sheet_name(&old_pkg.workbook, &new_pkg.workbook, sheet)?;
    
    let key_columns = if let Some(keys_str) = keys {
        parse_key_columns(&keys_str)?
    } else if auto_keys {
        let grid = find_sheet_grid(&old_pkg.workbook, &sheet_name)?;
        let suggested = with_default_session(|session| {
            suggest_key_columns(grid, &session.strings)
        });
        if suggested.is_empty() {
            bail!("Could not auto-detect key columns for sheet '{}'. Please specify --keys manually.", sheet_name);
        }
        let col_letters: Vec<String> = suggested.iter().map(|&c| col_index_to_letters(c)).collect();
        eprintln!("Auto-detected key columns: {}", col_letters.join(","));
        suggested
    } else {
        bail!("Database mode requires either --keys or --auto-keys");
    };

    if format == OutputFormat::Jsonl && !git_diff_mode {
        return run_database_streaming(old_pkg, new_pkg, &sheet_name, &key_columns, config);
    }

    let report = old_pkg
        .diff_database_mode(new_pkg, &sheet_name, &key_columns, config)
        .context("Database mode diff failed")?;

    print_warnings_to_stderr(&report);
    print_fallback_suggestions(&report, auto_keys, &sheet_name, old_pkg);

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if git_diff_mode {
        git_diff::write_git_diff(&mut handle, &report, old_path, new_path)?;
    } else {
        match format {
            OutputFormat::Text => {
                text::write_text_report(&mut handle, &report, old_path, new_path, verbosity)?;
            }
            OutputFormat::Json => {
                json::write_json_report(&mut handle, &report)?;
            }
            OutputFormat::Jsonl => {
                unreachable!("JSONL handled by streaming path");
            }
        }
    }

    Ok(exit_code_from_report(&report))
}

fn run_database_streaming(
    old_pkg: &WorkbookPackage,
    new_pkg: &WorkbookPackage,
    sheet_name: &str,
    key_columns: &[u32],
    config: &DiffConfig,
) -> Result<ExitCode> {
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut writer = BufWriter::new(handle);
    let mut sink = JsonLinesSink::new(&mut writer);

    let summary = old_pkg
        .diff_database_mode_streaming(new_pkg, sheet_name, key_columns, config, &mut sink)
        .context("Database mode streaming diff failed")?;

    writer.flush()?;

    for warning in &summary.warnings {
        eprintln!("Warning: {}", warning);
    }

    if summary.op_count == 0 && summary.complete {
        Ok(ExitCode::from(0))
    } else {
        Ok(ExitCode::from(1))
    }
}

fn determine_sheet_name(
    old_wb: &excel_diff::Workbook,
    new_wb: &excel_diff::Workbook,
    sheet: Option<String>,
) -> Result<String> {
    if let Some(name) = sheet {
        return Ok(name);
    }

    let has_data_sheet = |wb: &excel_diff::Workbook| -> bool {
        with_default_session(|session| {
            wb.sheets.iter().any(|s| {
                session.strings.resolve(s.name).to_lowercase() == "data"
            })
        })
    };

    if has_data_sheet(old_wb) || has_data_sheet(new_wb) {
        return Ok("Data".to_string());
    }

    if old_wb.sheets.len() == 1 && new_wb.sheets.len() == 1 {
        let old_name = with_default_session(|session| {
            session.strings.resolve(old_wb.sheets[0].name).to_string()
        });
        return Ok(old_name);
    }

    bail!("Multiple sheets found; please specify --sheet")
}

fn find_sheet_grid<'a>(wb: &'a excel_diff::Workbook, sheet_name: &str) -> Result<&'a Grid> {
    let sheet_name_lower = sheet_name.to_lowercase();
    with_default_session(|session| {
        wb.sheets
            .iter()
            .find(|s| session.strings.resolve(s.name).to_lowercase() == sheet_name_lower)
            .map(|s| &s.grid)
            .ok_or_else(|| {
                let available: Vec<String> = wb
                    .sheets
                    .iter()
                    .map(|s| session.strings.resolve(s.name).to_string())
                    .collect();
                anyhow::anyhow!(
                    "Sheet '{}' not found. Available sheets: {}",
                    sheet_name,
                    available.join(", ")
                )
            })
    })
}

fn parse_key_columns(keys_str: &str) -> Result<Vec<u32>> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for token in keys_str.split(',') {
        let token = token.trim();
        if token.is_empty() {
            bail!("Invalid --keys: empty column token in '{}'", keys_str);
        }
        if !token.chars().all(|c| c.is_ascii_alphabetic()) {
            bail!(
                "Invalid --keys: '{}' is not a valid column letter (must be letters only, e.g. A, B, AA)",
                token
            );
        }
        let col_idx = col_letters_to_index(token)?;
        if !seen.insert(col_idx) {
            bail!("Invalid --keys: duplicate column '{}'", token);
        }
        result.push(col_idx);
    }

    if result.is_empty() {
        bail!("Invalid --keys: no columns specified");
    }

    Ok(result)
}

fn col_letters_to_index(letters: &str) -> Result<u32> {
    let mut col: u32 = 0;
    for ch in letters.chars() {
        let upper = ch.to_ascii_uppercase();
        if !upper.is_ascii_uppercase() {
            bail!("Invalid column letter: '{}'", ch);
        }
        col = col
            .checked_mul(26)
            .and_then(|c| c.checked_add((upper as u8 - b'A' + 1) as u32))
            .ok_or_else(|| anyhow::anyhow!("Column '{}' is out of range", letters))?;
    }
    Ok(col.saturating_sub(1))
}

fn col_index_to_letters(col: u32) -> String {
    let addr = index_to_address(0, col);
    addr.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

fn print_fallback_suggestions(
    report: &DiffReport,
    auto_keys: bool,
    sheet_name: &str,
    old_pkg: &WorkbookPackage,
) {
    let has_fallback_warning = report.warnings.iter().any(|w| {
        w.contains("duplicate keys") && w.contains("falling back")
    });

    if has_fallback_warning && !auto_keys {
        if let Ok(grid) = find_sheet_grid(&old_pkg.workbook, sheet_name) {
            let suggested = with_default_session(|session| {
                suggest_key_columns(grid, &session.strings)
            });
            if !suggested.is_empty() {
                let col_letters: Vec<String> = suggested.iter().map(|&c| col_index_to_letters(c)).collect();
                eprintln!("Hint: try --keys={} for unique key columns", col_letters.join(","));
            }
        }
    }
}

fn build_config(fast: bool, precise: bool) -> DiffConfig {
    if fast {
        DiffConfig::fastest()
    } else if precise {
        DiffConfig::most_precise()
    } else {
        DiffConfig::default()
    }
}

fn print_warnings_to_stderr(report: &DiffReport) {
    for warning in &report.warnings {
        eprintln!("Warning: {}", warning);
    }
}

fn exit_code_from_report(report: &DiffReport) -> ExitCode {
    if report.ops.is_empty() && report.complete {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

