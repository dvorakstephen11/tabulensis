mod commands;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use excel_diff::{
    ContainerError, DataMashupError, DiffError, GridParseError, PackageError, SectionParseError,
};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "excel-diff")]
#[command(about = "Compare Excel workbooks and show differences")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Compare two Excel workbooks")]
    Diff {
        #[arg(help = "Path to the old/base workbook")]
        old: String,
        #[arg(help = "Path to the new/changed workbook")]
        new: String,
        #[arg(long, short, value_enum, default_value = "text", help = "Output format")]
        format: OutputFormat,
        #[arg(long, help = "Produce unified diff-style output for Git")]
        git_diff: bool,
        #[arg(long, help = "Use fastest diff preset (less precise move detection)")]
        fast: bool,
        #[arg(long, help = "Use most precise diff preset (slower, more accurate)")]
        precise: bool,
        #[arg(long, short, help = "Quiet mode: only show summary")]
        quiet: bool,
        #[arg(long, short, help = "Verbose mode: show additional details")]
        verbose: bool,
        #[arg(long, help = "Use database mode: align rows by key columns")]
        database: bool,
        #[arg(long, help = "Sheet name to diff in database mode")]
        sheet: Option<String>,
        #[arg(long, help = "Key columns for database mode (comma-separated column letters, e.g. A,B,C)")]
        keys: Option<String>,
        #[arg(long, help = "Auto-detect key columns for database mode")]
        auto_keys: bool,
        #[arg(long, help = "Show a progress bar on stderr")]
        progress: bool,
        #[arg(long, value_name = "MB", help = "Soft memory budget (MB) for advanced strategies")]
        max_memory: Option<u32>,
        #[arg(long, value_name = "SECONDS", help = "Abort diff after this many seconds")]
        timeout: Option<u32>,
        #[arg(long, value_name = "PATH", help = "Write perf metrics JSON to this path")]
        metrics_json: Option<String>,
    },
    #[command(about = "Show information about a workbook")]
    Info {
        #[arg(help = "Path to the workbook")]
        path: String,
        #[arg(long, help = "Include Power Query information")]
        queries: bool,
    },
}

#[derive(Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Jsonl,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Diff {
            old,
            new,
            format,
            git_diff,
            fast,
            precise,
            quiet,
            verbose,
            database,
            sheet,
            keys,
            auto_keys,
            progress,
            max_memory,
            timeout,
            metrics_json,
        } => commands::diff::run(
            &old,
            &new,
            format,
            git_diff,
            fast,
            precise,
            quiet,
            verbose,
            database,
            sheet,
            keys,
            auto_keys,
            progress,
            max_memory,
            timeout,
            metrics_json,
        ),
        Commands::Info { path, queries } => commands::info::run(&path, queries),
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            exit_code_for_error(&e)
        }
    }
}

fn exit_code_for_error(err: &anyhow::Error) -> ExitCode {
    if is_internal_error(err) {
        ExitCode::from(3)
    } else {
        ExitCode::from(2)
    }
}

fn is_internal_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        if let Some(diff_err) = cause.downcast_ref::<DiffError>() {
            return !matches!(diff_err, DiffError::SheetNotFound { .. });
        }
        cause.is::<PackageError>()
            || cause.is::<ContainerError>()
            || cause.is::<GridParseError>()
            || cause.is::<DataMashupError>()
            || cause.is::<SectionParseError>()
    })
}

