use anyhow::{Context, Result};
use excel_diff::{build_embedded_queries, build_queries, DataMashup, SheetKind};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use crate::commands::host::{host_kind_from_path, open_host, Host};

pub fn run(path: &str, show_queries: bool) -> Result<ExitCode> {
    let path = Path::new(path);
    let kind = host_kind_from_path(path)
        .with_context(|| format!("Unsupported input extension: {}", path.display()))?;

    let host = open_host(path, kind, "input")?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    match host {
        Host::Workbook(pkg) => {
            writeln!(handle, "Workbook: {}", filename)?;
            writeln!(handle, "Sheets: {}", pkg.workbook.sheets.len())?;
            for sheet in &pkg.workbook.sheets {
                let sheet_name =
                    excel_diff::with_default_session(|session| session.strings.resolve(sheet.name).to_string());
                let kind_str = match sheet.kind {
                    SheetKind::Worksheet => "worksheet",
                    SheetKind::Chart => "chart",
                    SheetKind::Macro => "macro",
                    SheetKind::Other => "other",
                };
                writeln!(handle, "- {} ({})", sheet_name, kind_str)?;
            }

            if show_queries {
                write_power_query_section(&mut handle, pkg.data_mashup.as_ref())?;
            }
        }
        Host::Pbix(pkg) => {
            writeln!(handle, "PBIX/PBIT: {}", filename)?;
            if show_queries {
                write_power_query_section(&mut handle, pkg.data_mashup())?;
            }
        }
    }

    Ok(ExitCode::from(0))
}

fn write_power_query_section<W: Write>(w: &mut W, dm_opt: Option<&DataMashup>) -> Result<()> {
    writeln!(w)?;

    let Some(dm) = dm_opt else {
        writeln!(w, "Power Query: none")?;
        return Ok(());
    };

    writeln!(w, "Power Query:")?;

    match build_queries(dm) {
        Ok(mut top) => {
            top.sort_by(|a, b| a.name.cmp(&b.name));
            writeln!(w, "  Top-level: {}", top.len())?;
            for q in top {
                write_query_line(w, &q)?;
            }
        }
        Err(e) => {
            writeln!(w, "  Top-level: error parsing queries: {}", e)?;
        }
    }

    let mut embedded = build_embedded_queries(dm);
    embedded.sort_by(|a, b| a.name.cmp(&b.name));
    writeln!(w, "  Embedded: {}", embedded.len())?;
    for q in embedded {
        write_query_line(w, &q)?;
    }

    Ok(())
}

fn write_query_line<W: Write>(w: &mut W, q: &excel_diff::Query) -> Result<()> {
    let load_flags = format_load_flags(&q.metadata);
    let group_path = q
        .metadata
        .group_path
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "(none)".to_string());

    if load_flags.is_empty() {
        writeln!(w, "  - {}  [group={}]", q.name, group_path)?;
    } else {
        writeln!(w, "  - {}  [{}]  [group={}]", q.name, load_flags, group_path)?;
    }

    Ok(())
}

fn format_load_flags(meta: &excel_diff::QueryMetadata) -> String {
    let mut flags = Vec::new();
    if meta.load_to_sheet {
        flags.push("sheet");
    }
    if meta.load_to_model {
        flags.push("model");
    }
    if meta.is_connection_only {
        flags.push("connection-only");
    }
    flags.join(",")
}
