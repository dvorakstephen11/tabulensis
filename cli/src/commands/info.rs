use anyhow::{Context, Result};
use excel_diff::{SheetKind, WorkbookPackage, build_queries};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

pub fn run(path: &str, show_queries: bool) -> Result<ExitCode> {
    let file = File::open(path).with_context(|| format!("Failed to open workbook: {}", path))?;

    let pkg = WorkbookPackage::open(file)
        .with_context(|| format!("Failed to parse workbook: {}", path))?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let filename = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| path.into());

    writeln!(handle, "Workbook: {}", filename)?;
    writeln!(handle, "Sheets: {}", pkg.workbook.sheets.len())?;

    for sheet in &pkg.workbook.sheets {
        let name = excel_diff::with_default_session(|session| {
            session.strings.resolve(sheet.name).to_string()
        });
        let kind_str = match sheet.kind {
            SheetKind::Worksheet => "worksheet",
            SheetKind::Chart => "chart",
            SheetKind::Macro => "macro",
            SheetKind::Other => "other",
        };
        writeln!(
            handle,
            "  - \"{}\" ({}) {}x{}, {} cells",
            name,
            kind_str,
            sheet.grid.nrows,
            sheet.grid.ncols,
            sheet.grid.cell_count()
        )?;
    }

    if show_queries {
        writeln!(handle)?;
        match &pkg.data_mashup {
            None => {
                writeln!(handle, "Power Query: none")?;
            }
            Some(dm) => {
                match build_queries(dm) {
                    Ok(mut queries) => {
                        queries.sort_by(|a, b| a.name.cmp(&b.name));
                        writeln!(handle, "Power Query: {} queries", queries.len())?;
                        for query in &queries {
                            let load_flags = format_load_flags(&query.metadata);
                            let group = query
                                .metadata
                                .group_path
                                .as_deref()
                                .map(|g| format!(" [group: {}]", g))
                                .unwrap_or_default();
                            writeln!(handle, "  - \"{}\"{}{}", query.name, load_flags, group)?;
                        }
                    }
                    Err(e) => {
                        writeln!(handle, "Power Query: error parsing queries: {}", e)?;
                    }
                }
            }
        }
    }

    Ok(ExitCode::from(0))
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
    if flags.is_empty() {
        String::new()
    } else {
        format!(" ({})", flags.join(", "))
    }
}

