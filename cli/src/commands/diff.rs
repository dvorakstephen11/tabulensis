use crate::commands::host::{host_kind_from_path, open_host, Host, HostKind};
use crate::output::{git_diff, json, text};
use crate::{DiffPresetArg, OutputFormat};
use anyhow::{Context, Result, bail};
use excel_diff::{
    DiffConfig, DiffReport, DiffSummary, Grid, JsonLinesSink, ProgressCallback, SheetKind,
    Workbook, WorkbookPackage, index_to_address, suggest_key_columns, with_default_session,
};
use std::collections::HashMap;
#[cfg(feature = "perf-metrics")]
use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Mutex;
use ui_payload::{
    DiffOutcome, DiffOutcomeConfig, DiffOutcomeMode, DiffPreset, SummaryMeta, SummarySink,
    limits_from_config, summarize_report,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

 

#[allow(clippy::too_many_arguments)]
pub fn run(
    old_path: &str,
    new_path: &str,
    format: OutputFormat,
    force_json: bool,
    git_diff_mode: bool,
    fast: bool,
    precise: bool,
    preset: Option<DiffPresetArg>,
    quiet: bool,
    verbose: bool,
    database: bool,
    sheet: Option<String>,
    keys: Option<String>,
    auto_keys: bool,
    progress: bool,
    max_memory: Option<u32>,
    timeout: Option<u32>,
    max_ops: Option<usize>,
    metrics_json: Option<String>,
) -> Result<ExitCode> {
    if fast && precise {
        bail!("Cannot use both --fast and --precise flags together");
    }
    if preset.is_some() && (fast || precise) {
        bail!("Cannot combine --preset with --fast or --precise");
    }

    if git_diff_mode
        && matches!(
            format,
            OutputFormat::Json | OutputFormat::Jsonl | OutputFormat::Payload | OutputFormat::Outcome
        )
    {
        bail!("Cannot use --git-diff with --format json/jsonl/payload/outcome");
    }

    let mut format = format;

    let old_path_str = old_path;
    let new_path_str = new_path;
    let old_path = Path::new(old_path_str);
    let new_path = Path::new(new_path_str);

    let old_kind = host_kind_from_path(old_path)
        .ok_or_else(|| anyhow::anyhow!("unsupported input extension: {}", old_path.display()))?;
    let new_kind = host_kind_from_path(new_path)
        .ok_or_else(|| anyhow::anyhow!("unsupported input extension: {}", new_path.display()))?;

    if old_kind != new_kind {
        bail!("input host types must match");
    }

    if old_kind == HostKind::Pbix {
        if database || sheet.is_some() || keys.is_some() || auto_keys {
            bail!("database mode and sheet/key options are not supported for PBIX/PBIT");
        }
    } else {
        if !database && (sheet.is_some() || keys.is_some() || auto_keys) {
            bail!("--sheet, --keys, and --auto-keys require --database flag");
        }

        if database && keys.is_none() && !auto_keys {
            bail!("Database mode requires either --keys or --auto-keys");
        }

        if database && keys.is_some() && auto_keys {
            bail!("Cannot use both --keys and --auto-keys together");
        }
    }

    let verbosity = if quiet {
        Verbosity::Quiet
    } else if verbose {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    let preset = resolve_preset(preset, fast, precise)?;
    let mut config = build_config(preset);
    config.hardening.max_memory_mb = max_memory;
    config.hardening.timeout_seconds = timeout;
    config.hardening.max_ops = max_ops;

    let old_host = open_host(old_path, old_kind, "old")?;
    let new_host = open_host(new_path, new_kind, "new")?;

    let mut estimated_cells: Option<u64> = None;
    if !database {
        estimated_cells = match (&old_host, &new_host) {
            (Host::Workbook(old_pkg), Host::Workbook(new_pkg)) => {
                Some(estimate_diff_cell_volume(&old_pkg.workbook, &new_pkg.workbook))
            }
            _ => None,
        };
        let (new_format, switched_cells) =
            maybe_auto_switch_jsonl(format, force_json, git_diff_mode, estimated_cells, &config);
        if let Some(cells) = switched_cells {
            eprintln!(
                "Warning: estimated {} cells; switching to JSONL output. Use --force-json to keep JSON.",
                cells
            );
        }
        format = new_format;
    }

    if old_kind == HostKind::Workbook && database {
        let (Host::Workbook(old_pkg), Host::Workbook(new_pkg)) = (&old_host, &new_host) else {
            unreachable!();
        };
        return run_database_mode(
            old_pkg,
            new_pkg,
            old_path_str,
            new_path_str,
            format,
            git_diff_mode,
            force_json,
            &config,
            preset,
            verbosity,
            sheet,
            keys,
            auto_keys,
            metrics_json,
        );
    }

    let progress = progress.then(CliProgress::new);

    if format == OutputFormat::Payload {
        let payload = match (&old_host, &new_host) {
            (Host::Workbook(old_pkg), Host::Workbook(new_pkg)) => match progress.as_ref() {
                Some(p) => ui_payload::build_payload_from_workbooks_with_progress(
                    old_pkg, new_pkg, &config, p,
                ),
                None => ui_payload::build_payload_from_workbooks(old_pkg, new_pkg, &config),
            },
            (Host::Pbix(old_pkg), Host::Pbix(new_pkg)) => {
                ui_payload::build_payload_from_pbix(old_pkg, new_pkg, &config)
            }
            _ => unreachable!(),
        };

        if let Some(p) = progress.as_ref() {
            p.finish();
        }

        print_warnings_to_stderr(&payload.report);

        if let Some(path) = metrics_json.as_deref() {
            write_metrics_json_report(Path::new(path), &payload.report)?;
        }

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        json::write_json_value(&mut handle, &payload)?;
        return Ok(exit_code_from_report(&payload.report));
    }

    if format == OutputFormat::Outcome {
        let meta = summary_meta_from_paths(old_path_str, new_path_str);
        let outcome_config = DiffOutcomeConfig {
            preset: Some(preset),
            limits: Some(limits_from_config(&config)),
        };

        let outcome = match (&old_host, &new_host) {
            (Host::Workbook(old_pkg), Host::Workbook(new_pkg)) => {
                let use_large_mode = estimated_cells
                    .map(|cells| excel_diff::should_use_large_mode(cells, &config))
                    .unwrap_or(false);

                if use_large_mode {
                    let mut sink = SummarySink::new();
                    let summary = match progress.as_ref() {
                        Some(p) => old_pkg
                            .diff_streaming_with_progress(new_pkg, &config, &mut sink, p)
                            .context("Streaming diff failed")?,
                        None => old_pkg
                            .diff_streaming(new_pkg, &config, &mut sink)
                            .context("Streaming diff failed")?,
                    };

                    if let Some(p) = progress.as_ref() {
                        p.finish();
                    }

                    let summary = sink.into_summary(summary, meta.clone());
                    for warning in &summary.warnings {
                        eprintln!("Warning: {}", warning);
                    }

                    DiffOutcome {
                        diff_id: None,
                        mode: DiffOutcomeMode::Large,
                        payload: None,
                        summary: Some(summary),
                        config: Some(outcome_config),
                    }
                } else {
                    let payload = match progress.as_ref() {
                        Some(p) => ui_payload::build_payload_from_workbooks_with_progress(
                            old_pkg, new_pkg, &config, p,
                        ),
                        None => ui_payload::build_payload_from_workbooks(old_pkg, new_pkg, &config),
                    };

                    if let Some(p) = progress.as_ref() {
                        p.finish();
                    }

                    let summary = summarize_report(&payload.report, meta.clone());
                    print_warnings_to_stderr(&payload.report);

                    DiffOutcome {
                        diff_id: None,
                        mode: DiffOutcomeMode::Payload,
                        payload: Some(payload),
                        summary: Some(summary),
                        config: Some(outcome_config),
                    }
                }
            }
            (Host::Pbix(old_pkg), Host::Pbix(new_pkg)) => {
                let payload = ui_payload::build_payload_from_pbix(old_pkg, new_pkg, &config);
                let summary = summarize_report(&payload.report, meta);
                print_warnings_to_stderr(&payload.report);
                DiffOutcome {
                    diff_id: None,
                    mode: DiffOutcomeMode::Payload,
                    payload: Some(payload),
                    summary: Some(summary),
                    config: Some(outcome_config),
                }
            }
            _ => unreachable!(),
        };

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        json::write_json_value(&mut handle, &outcome)?;

        return Ok(match outcome.mode {
            DiffOutcomeMode::Payload => exit_code_from_report(
                outcome
                    .payload
                    .as_ref()
                    .map(|p| &p.report)
                    .expect("payload report exists"),
            ),
            DiffOutcomeMode::Large => {
                let summary = outcome.summary.as_ref().expect("summary exists");
                if summary.op_count == 0 && summary.complete {
                    ExitCode::from(0)
                } else {
                    ExitCode::from(1)
                }
            }
        });
    }

    if format == OutputFormat::Jsonl && !git_diff_mode {
        return run_streaming_host(&old_host, &new_host, &config, progress.as_ref(), metrics_json.as_deref());
    }

    let report = match (&old_host, &new_host) {
        (Host::Workbook(old_pkg), Host::Workbook(new_pkg)) => match progress.as_ref() {
            Some(p) => old_pkg.diff_with_progress(new_pkg, &config, p),
            None => old_pkg.diff(new_pkg, &config),
        },
        (Host::Pbix(old_pkg), Host::Pbix(new_pkg)) => old_pkg.diff(new_pkg, &config),
        _ => unreachable!(),
    };

    if let Some(p) = progress.as_ref() {
        p.finish();
    }

    print_warnings_to_stderr(&report);

    if let Some(path) = metrics_json.as_deref() {
        write_metrics_json_report(Path::new(path), &report)?;
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if git_diff_mode {
        git_diff::write_git_diff(&mut handle, &report, old_path_str, new_path_str)?;
    } else {
        match format {
            OutputFormat::Text => {
                text::write_text_report(
                    &mut handle,
                    &report,
                    old_path_str,
                    new_path_str,
                    verbosity,
                )?;
            }
            OutputFormat::Json => {
                json::write_json_report(&mut handle, &report)?;
            }
            OutputFormat::Jsonl => {
                bail!("Internal error: JSONL format should be handled by the streaming path");
            }
            OutputFormat::Payload | OutputFormat::Outcome => {
                bail!("Internal error: payload/outcome format should be handled earlier");
            }
        }
    }

    Ok(exit_code_from_report(&report))
}

fn run_streaming_host(
    old_host: &Host,
    new_host: &Host,
    config: &DiffConfig,
    progress: Option<&CliProgress>,
    metrics_json: Option<&str>,
) -> Result<ExitCode> {
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut writer = BufWriter::new(handle);
    let mut sink = JsonLinesSink::new(&mut writer);

    let summary = match (old_host, new_host, progress) {
        (Host::Workbook(old_pkg), Host::Workbook(new_pkg), Some(p)) => old_pkg
            .diff_streaming_with_progress(new_pkg, config, &mut sink, p)
            .context("Streaming diff failed")?,
        (Host::Workbook(old_pkg), Host::Workbook(new_pkg), None) => old_pkg
            .diff_streaming(new_pkg, config, &mut sink)
            .context("Streaming diff failed")?,
        (Host::Pbix(old_pkg), Host::Pbix(new_pkg), Some(p)) => old_pkg
            .diff_streaming_with_progress(new_pkg, config, &mut sink, p)
            .context("Streaming diff failed")?,
        (Host::Pbix(old_pkg), Host::Pbix(new_pkg), None) => old_pkg
            .diff_streaming(new_pkg, config, &mut sink)
            .context("Streaming diff failed")?,
        _ => unreachable!(),
    };

    writer.flush()?;

    if let Some(p) = progress {
        p.finish();
    }

    if let Some(path) = metrics_json {
        write_metrics_json_summary(Path::new(path), &summary)?;
    }

    for warning in &summary.warnings {
        eprintln!("Warning: {}", warning);
    }

    if summary.op_count == 0 && summary.complete {
        Ok(ExitCode::from(0))
    } else {
        Ok(ExitCode::from(1))
    }
}

struct CliProgress {
    state: Mutex<CliProgressState>,
}

struct CliProgressState {
    is_tty: bool,
    last_phase: String,
    last_percent: f32,
}

impl CliProgress {
    fn new() -> Self {
        Self {
            state: Mutex::new(CliProgressState {
                is_tty: io::stderr().is_terminal(),
                last_phase: String::new(),
                last_percent: -1.0,
            }),
        }
    }

    fn finish(&self) {
        let is_tty = self.lock_state().is_tty;
        if is_tty {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr);
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, CliProgressState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn render_bar(phase: &str, percent: f32) -> String {
        let pct = (percent.clamp(0.0, 1.0) * 100.0).clamp(0.0, 100.0);
        let width = 24usize;
        let filled = ((pct / 100.0) * width as f32).round() as usize;
        let filled = filled.min(width);
        let empty = width - filled;
        format!(
            "{:>14} [{}{}] {:>6.1}%",
            phase,
            "#".repeat(filled),
            "-".repeat(empty),
            pct
        )
    }
}

impl ProgressCallback for CliProgress {
    fn on_progress(&self, phase: &str, percent: f32) {
        let mut state = self.lock_state();

        if state.is_tty {
            let line = Self::render_bar(phase, percent);
            let mut stderr = io::stderr().lock();
            let _ = write!(stderr, "\r{}", line);
            let _ = stderr.flush();
        } else {
            let phase_changed = state.last_phase != phase;
            let pct = percent.clamp(0.0, 1.0);
            let emit = phase_changed || pct == 0.0 || pct == 1.0 || (pct - state.last_percent) >= 0.25;
            if emit {
                eprintln!("Progress: {} {:.0}%", phase, pct * 100.0);
                state.last_phase = phase.to_string();
                state.last_percent = pct;
            }
        }
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
    force_json: bool,
    config: &DiffConfig,
    preset: DiffPreset,
    verbosity: Verbosity,
    sheet: Option<String>,
    keys: Option<String>,
    auto_keys: bool,
    metrics_json: Option<String>,
) -> Result<ExitCode> {
    let sheet_name = determine_sheet_name(&old_pkg.workbook, &new_pkg.workbook, sheet)?;

    let mut format = format;
    let estimated_cells = estimate_sheet_cell_volume(old_pkg, new_pkg, &sheet_name)?;
    let (new_format, switched_cells) =
        maybe_auto_switch_jsonl(format, force_json, git_diff_mode, Some(estimated_cells), config);
    if let Some(cells) = switched_cells {
        eprintln!(
            "Warning: estimated {} cells in sheet '{}'; switching to JSONL output. Use --force-json to keep JSON.",
            cells,
            sheet_name
        );
    }
    format = new_format;
    
    let key_columns = if let Some(keys_str) = keys {
        parse_key_columns(&keys_str)?
    } else if auto_keys {
        let grid = find_sheet_grid(&old_pkg.workbook, &sheet_name)?;
        let suggested = with_default_session(|session| {
            suggest_key_columns(grid, &session.strings)
        });
        if suggested.is_empty() {
            eprintln!(
                "Warning: Could not auto-detect key columns for sheet '{}'; falling back to spreadsheet mode.",
                sheet_name
            );
            Vec::new()
        }
        else {
            let col_letters: Vec<String> = suggested.iter().map(|&c| col_index_to_letters(c)).collect();
            eprintln!("Auto-detected key columns: {}", col_letters.join(","));
            suggested
        }
    } else {
        bail!("Database mode requires either --keys or --auto-keys");
    };

    if format == OutputFormat::Outcome {
        let meta = summary_meta_from_paths(old_path, new_path);
        let outcome_config = DiffOutcomeConfig {
            preset: Some(preset),
            limits: Some(limits_from_config(config)),
        };
        let use_large_mode = excel_diff::should_use_large_mode(estimated_cells, config);

        let outcome = if use_large_mode {
            let mut sink = SummarySink::new();
            let summary = old_pkg
                .diff_database_mode_streaming(new_pkg, &sheet_name, &key_columns, config, &mut sink)
                .context("Database mode streaming diff failed")?;

            if let Some(path) = metrics_json.as_deref() {
                write_metrics_json_summary(Path::new(path), &summary)?;
            }

            let summary = sink.into_summary(summary, meta);
            for warning in &summary.warnings {
                eprintln!("Warning: {}", warning);
            }

            DiffOutcome {
                diff_id: None,
                mode: DiffOutcomeMode::Large,
                payload: None,
                summary: Some(summary),
                config: Some(outcome_config),
            }
        } else {
            let report = old_pkg
                .diff_database_mode(new_pkg, &sheet_name, &key_columns, config)
                .context("Database mode diff failed")?;

            print_warnings_to_stderr(&report);
            print_fallback_suggestions(&report, auto_keys, &sheet_name, old_pkg);

            if let Some(path) = metrics_json.as_deref() {
                write_metrics_json_report(Path::new(path), &report)?;
            }

            let payload = ui_payload::build_payload_from_workbook_report(report, old_pkg, new_pkg);
            let summary = summarize_report(&payload.report, meta);
            DiffOutcome {
                diff_id: None,
                mode: DiffOutcomeMode::Payload,
                payload: Some(payload),
                summary: Some(summary),
                config: Some(outcome_config),
            }
        };

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        json::write_json_value(&mut handle, &outcome)?;

        return Ok(match outcome.mode {
            DiffOutcomeMode::Payload => exit_code_from_report(
                outcome
                    .payload
                    .as_ref()
                    .map(|p| &p.report)
                    .expect("payload report exists"),
            ),
            DiffOutcomeMode::Large => {
                let summary = outcome.summary.as_ref().expect("summary exists");
                if summary.op_count == 0 && summary.complete {
                    ExitCode::from(0)
                } else {
                    ExitCode::from(1)
                }
            }
        });
    }

    if format == OutputFormat::Payload {
        let report = old_pkg
            .diff_database_mode(new_pkg, &sheet_name, &key_columns, config)
            .context("Database mode diff failed")?;

        print_warnings_to_stderr(&report);
        print_fallback_suggestions(&report, auto_keys, &sheet_name, old_pkg);

        if let Some(path) = metrics_json.as_deref() {
            write_metrics_json_report(Path::new(path), &report)?;
        }

        let payload = ui_payload::build_payload_from_workbook_report(report, old_pkg, new_pkg);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        json::write_json_value(&mut handle, &payload)?;
        return Ok(exit_code_from_report(&payload.report));
    }

    if format == OutputFormat::Jsonl && !git_diff_mode {
        return run_database_streaming(
            old_pkg,
            new_pkg,
            &sheet_name,
            &key_columns,
            config,
            metrics_json.as_deref(),
        );
    }

    let report = old_pkg
        .diff_database_mode(new_pkg, &sheet_name, &key_columns, config)
        .context("Database mode diff failed")?;

    print_warnings_to_stderr(&report);
    print_fallback_suggestions(&report, auto_keys, &sheet_name, old_pkg);

    if let Some(path) = metrics_json.as_deref() {
        write_metrics_json_report(Path::new(path), &report)?;
    }

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
                bail!("Internal error: JSONL format should be handled by the streaming path");
            }
            OutputFormat::Payload | OutputFormat::Outcome => {
                bail!("Internal error: payload/outcome format should be handled earlier");
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
    metrics_json: Option<&str>,
) -> Result<ExitCode> {
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut writer = BufWriter::new(handle);
    let mut sink = JsonLinesSink::new(&mut writer);

    let summary = old_pkg
        .diff_database_mode_streaming(new_pkg, sheet_name, key_columns, config, &mut sink)
        .context("Database mode streaming diff failed")?;

    writer.flush()?;

    if let Some(path) = metrics_json {
        write_metrics_json_summary(Path::new(path), &summary)?;
    }

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
    let has_fallback_warning = report
        .warnings
        .iter()
        .any(|w| w.contains("database-mode:") && w.contains("falling back"));

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
    if has_fallback_warning && auto_keys {
        eprintln!("Hint: specify --keys to force database mode when auto-detection is ambiguous.");
    }
}

fn resolve_preset(
    preset: Option<DiffPresetArg>,
    fast: bool,
    precise: bool,
) -> Result<DiffPreset> {
    if fast {
        return Ok(DiffPreset::Fastest);
    }
    if precise {
        return Ok(DiffPreset::MostPrecise);
    }
    if let Some(preset) = preset {
        return Ok(match preset {
            DiffPresetArg::Fastest => DiffPreset::Fastest,
            DiffPresetArg::Balanced => DiffPreset::Balanced,
            DiffPresetArg::MostPrecise => DiffPreset::MostPrecise,
        });
    }
    Ok(DiffPreset::Balanced)
}

fn build_config(preset: DiffPreset) -> DiffConfig {
    preset.to_config()
}

fn estimate_diff_cell_volume(old: &Workbook, new: &Workbook) -> u64 {
    with_default_session(|session| {
        let mut max_counts: HashMap<SheetKey, u64> = HashMap::new();
        for sheet in old.sheets.iter().chain(new.sheets.iter()) {
            let name_lower = session.strings.resolve(sheet.name).to_lowercase();
            let key = SheetKey {
                name_lower,
                kind: sheet.kind.clone(),
            };
            let cell_count = sheet.grid.cell_count() as u64;
            max_counts
                .entry(key)
                .and_modify(|v| {
                    if cell_count > *v {
                        *v = cell_count;
                    }
                })
                .or_insert(cell_count);
        }

        max_counts.values().copied().sum()
    })
}

fn estimate_sheet_cell_volume(
    old_pkg: &WorkbookPackage,
    new_pkg: &WorkbookPackage,
    sheet_name: &str,
) -> Result<u64> {
    let old_cells = find_sheet_grid(&old_pkg.workbook, sheet_name)?.cell_count() as u64;
    let new_cells = find_sheet_grid(&new_pkg.workbook, sheet_name)?.cell_count() as u64;
    Ok(old_cells.max(new_cells))
}

fn maybe_auto_switch_jsonl(
    format: OutputFormat,
    force_json: bool,
    git_diff_mode: bool,
    estimated_cells: Option<u64>,
    config: &DiffConfig,
) -> (OutputFormat, Option<u64>) {
    if format == OutputFormat::Json && !force_json && !git_diff_mode {
        if let Some(cells) = estimated_cells {
            if excel_diff::should_use_large_mode(cells, config) {
                return (OutputFormat::Jsonl, Some(cells));
            }
        }
    }
    (format, None)
}

fn summary_meta_from_paths(old_path: &str, new_path: &str) -> SummaryMeta {
    let old_name = Path::new(old_path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());
    let new_name = Path::new(new_path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    SummaryMeta {
        old_path: Some(old_path.to_string()),
        new_path: Some(new_path.to_string()),
        old_name,
        new_name,
    }
}

fn print_warnings_to_stderr(report: &DiffReport) {
    for warning in &report.warnings {
        eprintln!("Warning: {}", warning);
    }
}

#[cfg(feature = "perf-metrics")]
fn write_metrics_json_report(path: &Path, report: &DiffReport) -> Result<()> {
    let metrics = report
        .metrics
        .as_ref()
        .context("Perf metrics not available; build with --features perf-metrics")?;
    write_metrics_json(path, metrics)
}

#[cfg(feature = "perf-metrics")]
fn write_metrics_json_summary(path: &Path, summary: &DiffSummary) -> Result<()> {
    let metrics = summary
        .metrics
        .as_ref()
        .context("Perf metrics not available; build with --features perf-metrics")?;
    write_metrics_json(path, metrics)
}

#[cfg(feature = "perf-metrics")]
fn write_metrics_json(
    path: &Path,
    metrics: &excel_diff::perf::DiffMetrics,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create metrics directory: {}", parent.display()))?;
    }
    let mut file = File::create(path)
        .with_context(|| format!("Failed to create metrics file: {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, metrics)?;
    writeln!(file)?;
    Ok(())
}

#[cfg(not(feature = "perf-metrics"))]
fn write_metrics_json_report(_path: &Path, _report: &DiffReport) -> Result<()> {
    bail!("--metrics-json requires tabulensis to be built with --features perf-metrics")
}

#[cfg(not(feature = "perf-metrics"))]
fn write_metrics_json_summary(_path: &Path, _summary: &DiffSummary) -> Result<()> {
    bail!("--metrics-json requires tabulensis to be built with --features perf-metrics")
}

fn exit_code_from_report(report: &DiffReport) -> ExitCode {
    if report.ops.is_empty() && report.complete {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_switches_to_jsonl_for_large_estimate() {
        let config = DiffConfig::balanced();
        let cells = excel_diff::AUTO_STREAM_CELL_THRESHOLD + 1;
        let (format, switched) =
            maybe_auto_switch_jsonl(OutputFormat::Json, false, false, Some(cells), &config);
        assert_eq!(format, OutputFormat::Jsonl);
        assert_eq!(switched, Some(cells));
    }
}

