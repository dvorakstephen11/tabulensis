mod commands;
mod output;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use excel_diff::{
    ContainerError, DataMashupError, DiffError, GridParseError, PackageError, SectionParseError,
};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "excel-diff", disable_version_flag = true, arg_required_else_help = true)]
#[command(about = "Compare Excel workbooks and show differences")]
pub struct Cli {
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Show version and exit")]
    pub version: bool,
    #[arg(long, short, global = true, help = "Verbose output")]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Compare two Excel workbooks or PBIX/PBIT packages")]
    Diff {
        #[arg(help = "Path to the old/base file (.xlsx, .xlsm, .xltx, .xltm, .pbix, .pbit)")]
        old: String,
        #[arg(help = "Path to the new/changed file (.xlsx, .xlsm, .xltx, .xltm, .pbix, .pbit)")]
        new: String,
        #[arg(long, short, value_enum, default_value = "text", help = "Output format")]
        format: OutputFormat,
        #[arg(long, help = "Force JSON output even for large diffs (disable auto-switch to JSONL)")]
        force_json: bool,
        #[arg(long, help = "Produce unified diff-style output for Git")]
        git_diff: bool,
        #[arg(long, help = "Use fastest diff preset (less precise move detection)")]
        fast: bool,
        #[arg(long, help = "Use most precise diff preset (slower, more accurate)")]
        precise: bool,
        #[arg(long, value_enum, help = "Diff preset")]
        preset: Option<DiffPresetArg>,
        #[arg(long, short, help = "Quiet mode: only show summary")]
        quiet: bool,
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
        #[arg(long, value_name = "COUNT", help = "Maximum number of ops to emit before stopping")]
        max_ops: Option<usize>,
        #[arg(long, value_name = "PATH", help = "Write perf metrics JSON to this path")]
        metrics_json: Option<String>,
    },
    #[command(about = "Show information about a workbook or PBIX/PBIT package")]
    Info {
        #[arg(help = "Path to the file (.xlsx, .xlsm, .xltx, .xltm, .pbix, .pbit)")]
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
    Payload,
    Outcome,
}

#[derive(Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DiffPresetArg {
    Fastest,
    Balanced,
    MostPrecise,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.version {
        print_version(cli.verbose);
        return ExitCode::from(0);
    }

    let result = match cli.command {
        Some(Commands::Diff {
            old,
            new,
            format,
            force_json,
            git_diff,
            fast,
            precise,
            preset,
            quiet,
            database,
            sheet,
            keys,
            auto_keys,
            progress,
            max_memory,
            timeout,
            max_ops,
            metrics_json,
        }) => commands::diff::run(
            &old,
            &new,
            format,
            force_json,
            git_diff,
            fast,
            precise,
            preset,
            quiet,
            cli.verbose,
            database,
            sheet,
            keys,
            auto_keys,
            progress,
            max_memory,
            timeout,
            max_ops,
            metrics_json,
        ),
        Some(Commands::Info { path, queries }) => commands::info::run(&path, queries),
        None => {
            let mut cmd = Cli::command();
            let _ = cmd.print_help();
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            exit_code_for_error(&e)
        }
    }
}

fn print_version(verbose: bool) {
    println!("excel-diff {}", env!("CARGO_PKG_VERSION"));
    if !verbose {
        return;
    }

    let features = excel_diff::engine_features();
    println!(
        "features: vba={}, model-diff={}, parallel={}, std-fs={}",
        features.vba, features.model_diff, features.parallel, features.std_fs
    );
    println!("presets: fastest, balanced, most_precise");
    println!(
        "large_mode_threshold: {}",
        excel_diff::AUTO_STREAM_CELL_THRESHOLD
    );
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

