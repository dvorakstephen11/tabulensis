mod commands;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
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
        #[arg(long, help = "Key columns for database mode (not yet implemented)")]
        key_columns: Option<String>,
        #[arg(long, short, help = "Quiet mode: only show summary")]
        quiet: bool,
        #[arg(long, short, help = "Verbose mode: show additional details")]
        verbose: bool,
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
            key_columns,
            quiet,
            verbose,
        } => commands::diff::run(
            &old,
            &new,
            format,
            git_diff,
            fast,
            precise,
            key_columns,
            quiet,
            verbose,
        ),
        Commands::Info { path, queries } => commands::info::run(&path, queries),
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            ExitCode::from(2)
        }
    }
}

