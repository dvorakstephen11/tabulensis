use crate::output::{git_diff, json, text};
use crate::OutputFormat;
use anyhow::{Context, Result, bail};
use excel_diff::{DiffConfig, DiffReport, JsonLinesSink, WorkbookPackage};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

pub fn run(
    old_path: &str,
    new_path: &str,
    format: OutputFormat,
    git_diff_mode: bool,
    fast: bool,
    precise: bool,
    key_columns: Option<String>,
    quiet: bool,
    verbose: bool,
) -> Result<ExitCode> {
    if fast && precise {
        bail!("Cannot use both --fast and --precise flags together");
    }

    if key_columns.is_some() {
        bail!("Database mode is not implemented in CLI yet; use Branch 2 flags (--database --sheet --keys) once available");
    }

    if git_diff_mode && (format == OutputFormat::Json || format == OutputFormat::Jsonl) {
        bail!("Cannot use --git-diff with --format=json or --format=jsonl");
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

