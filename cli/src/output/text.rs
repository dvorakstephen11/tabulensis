use crate::commands::diff::Verbosity;
use anyhow::Result;
use excel_diff::{
    CellValue, DiffOp, DiffReport, QueryChangeKind, QueryMetadataField, StringId,
    index_to_address,
};
use std::collections::BTreeMap;
use std::io::Write;

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

    write_summary(w, report, verbosity)?;

    Ok(())
}

fn partition_ops(report: &DiffReport) -> (BTreeMap<String, Vec<&DiffOp>>, Vec<&DiffOp>) {
    let mut sheet_ops: BTreeMap<String, Vec<&DiffOp>> = BTreeMap::new();
    let mut m_ops: Vec<&DiffOp> = Vec::new();

    for op in &report.ops {
        if op.is_m_op() {
            m_ops.push(op);
        } else if let Some(sheet_id) = get_sheet_id(op) {
            let sheet_name = report
                .resolve(sheet_id)
                .unwrap_or("<unknown>")
                .to_string();
            sheet_ops.entry(sheet_name).or_default().push(op);
        }
    }

    (sheet_ops, m_ops)
}

fn get_sheet_id(op: &DiffOp) -> Option<StringId> {
    match op {
        DiffOp::SheetAdded { sheet } => Some(*sheet),
        DiffOp::SheetRemoved { sheet } => Some(*sheet),
        DiffOp::RowAdded { sheet, .. } => Some(*sheet),
        DiffOp::RowRemoved { sheet, .. } => Some(*sheet),
        DiffOp::ColumnAdded { sheet, .. } => Some(*sheet),
        DiffOp::ColumnRemoved { sheet, .. } => Some(*sheet),
        DiffOp::BlockMovedRows { sheet, .. } => Some(*sheet),
        DiffOp::BlockMovedColumns { sheet, .. } => Some(*sheet),
        DiffOp::BlockMovedRect { sheet, .. } => Some(*sheet),
        DiffOp::CellEdited { sheet, .. } => Some(*sheet),
        _ => None,
    }
}

fn render_op(report: &DiffReport, op: &DiffOp, verbosity: Verbosity) -> Vec<String> {
    match op {
        DiffOp::SheetAdded { sheet } => {
            vec![format!(
                "Sheet \"{}\": ADDED",
                report.resolve(*sheet).unwrap_or("<unknown>")
            )]
        }
        DiffOp::SheetRemoved { sheet } => {
            vec![format!(
                "Sheet \"{}\": REMOVED",
                report.resolve(*sheet).unwrap_or("<unknown>")
            )]
        }
        DiffOp::RowAdded { row_idx, .. } => {
            vec![format!("Row {}: ADDED", row_idx + 1)]
        }
        DiffOp::RowRemoved { row_idx, .. } => {
            vec![format!("Row {}: REMOVED", row_idx + 1)]
        }
        DiffOp::ColumnAdded { col_idx, .. } => {
            vec![format!("Column {}: ADDED", col_letter(*col_idx))]
        }
        DiffOp::ColumnRemoved { col_idx, .. } => {
            vec![format!("Column {}: REMOVED", col_letter(*col_idx))]
        }
        DiffOp::BlockMovedRows {
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
            ..
        } => {
            let src_end = src_start_row + row_count - 1;
            let dst_end = dst_start_row + row_count - 1;
            let mut result = vec![format!(
                "Block moved: rows {}-{} → rows {}-{}",
                src_start_row + 1,
                src_end + 1,
                dst_start_row + 1,
                dst_end + 1
            )];
            if verbosity == Verbosity::Verbose {
                if let Some(hash) = block_hash {
                    result.push(format!("  (hash: {:016x})", hash));
                }
            }
            result
        }
        DiffOp::BlockMovedColumns {
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
            ..
        } => {
            let src_end = src_start_col + col_count - 1;
            let dst_end = dst_start_col + col_count - 1;
            let mut result = vec![format!(
                "Block moved: columns {}-{} → columns {}-{}",
                col_letter(*src_start_col),
                col_letter(src_end),
                col_letter(*dst_start_col),
                col_letter(dst_end)
            )];
            if verbosity == Verbosity::Verbose {
                if let Some(hash) = block_hash {
                    result.push(format!("  (hash: {:016x})", hash));
                }
            }
            result
        }
        DiffOp::BlockMovedRect {
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash,
            ..
        } => {
            let src_range = format_range(
                *src_start_row,
                *src_start_col,
                *src_row_count,
                *src_col_count,
            );
            let dst_range = format_range(
                *dst_start_row,
                *dst_start_col,
                *src_row_count,
                *src_col_count,
            );
            let mut result = vec![format!("Block moved: {} → {}", src_range, dst_range)];
            if verbosity == Verbosity::Verbose {
                if let Some(hash) = block_hash {
                    result.push(format!("  (hash: {:016x})", hash));
                }
            }
            result
        }
        DiffOp::CellEdited {
            addr, from, to, ..
        } => {
            let old_str = format_cell_value(&from.value, report);
            let new_str = format_cell_value(&to.value, report);
            let mut result = vec![format!("Cell {}: {} → {}", addr, old_str, new_str)];
            if verbosity == Verbosity::Verbose {
                if let Some(formula_id) = from.formula {
                    if let Some(formula) = report.resolve(formula_id) {
                        result.push(format!("  old formula: ={}", formula));
                    }
                }
                if let Some(formula_id) = to.formula {
                    if let Some(formula) = report.resolve(formula_id) {
                        result.push(format!("  new formula: ={}", formula));
                    }
                }
            }
            result
        }
        DiffOp::QueryAdded { name } => {
            vec![format!(
                "Query \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::QueryRemoved { name } => {
            vec![format!(
                "Query \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::QueryRenamed { from, to } => {
            vec![format!(
                "Query renamed: \"{}\" → \"{}\"",
                report.resolve(*from).unwrap_or("<unknown>"),
                report.resolve(*to).unwrap_or("<unknown>")
            )]
        }
        DiffOp::QueryDefinitionChanged {
            name, change_kind, ..
        } => {
            let kind_str = match change_kind {
                QueryChangeKind::Semantic => "semantic change",
                QueryChangeKind::FormattingOnly => "formatting only",
                QueryChangeKind::Renamed => "renamed",
            };
            vec![format!(
                "Query \"{}\": definition changed ({})",
                report.resolve(*name).unwrap_or("<unknown>"),
                kind_str
            )]
        }
        DiffOp::QueryMetadataChanged {
            name,
            field,
            old,
            new,
        } => {
            let field_name = match field {
                QueryMetadataField::LoadToSheet => "load_to_sheet",
                QueryMetadataField::LoadToModel => "load_to_model",
                QueryMetadataField::GroupPath => "group_path",
                QueryMetadataField::ConnectionOnly => "connection_only",
            };
            let old_str = old
                .map(|id| report.resolve(id).unwrap_or("<unknown>").to_string())
                .unwrap_or_else(|| "<none>".to_string());
            let new_str = new
                .map(|id| report.resolve(id).unwrap_or("<unknown>").to_string())
                .unwrap_or_else(|| "<none>".to_string());
            vec![format!(
                "Query \"{}\": {} changed: {} → {}",
                report.resolve(*name).unwrap_or("<unknown>"),
                field_name,
                old_str,
                new_str
            )]
        }
        _ => vec![format!("{:?}", op)],
    }
}

fn col_letter(col: u32) -> String {
    index_to_address(0, col)
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect()
}

fn format_range(start_row: u32, start_col: u32, row_count: u32, col_count: u32) -> String {
    let tl = index_to_address(start_row, start_col);
    let br = index_to_address(start_row + row_count - 1, start_col + col_count - 1);
    format!("{}:{}", tl, br)
}

fn format_cell_value(value: &Option<CellValue>, report: &DiffReport) -> String {
    match value {
        None => "<empty>".to_string(),
        Some(CellValue::Blank) => "<blank>".to_string(),
        Some(CellValue::Number(n)) => format_number(*n),
        Some(CellValue::Text(id)) => {
            let text = report.resolve(*id).unwrap_or("<unknown>");
            format!("\"{}\"", escape_string(text))
        }
        Some(CellValue::Bool(b)) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Some(CellValue::Error(id)) => report.resolve(*id).unwrap_or("#ERROR").to_string(),
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{:.0}", n)
    } else {
        let s = format!("{:.10}", n);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('"', "\\\"")
}

fn write_summary<W: Write>(w: &mut W, report: &DiffReport, verbosity: Verbosity) -> Result<()> {
    if verbosity == Verbosity::Quiet && report.ops.is_empty() {
        return Ok(());
    }

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

struct OpCounts {
    sheets: usize,
    rows: usize,
    cols: usize,
    blocks: usize,
    cells: usize,
    queries: usize,
}

fn count_ops(report: &DiffReport) -> OpCounts {
    let mut counts = OpCounts {
        sheets: 0,
        rows: 0,
        cols: 0,
        blocks: 0,
        cells: 0,
        queries: 0,
    };

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { .. } | DiffOp::SheetRemoved { .. } => counts.sheets += 1,
            DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } => counts.rows += 1,
            DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. } => counts.cols += 1,
            DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. }
            | DiffOp::BlockMovedRect { .. } => counts.blocks += 1,
            DiffOp::CellEdited { .. } => counts.cells += 1,
            DiffOp::QueryAdded { .. }
            | DiffOp::QueryRemoved { .. }
            | DiffOp::QueryRenamed { .. }
            | DiffOp::QueryDefinitionChanged { .. }
            | DiffOp::QueryMetadataChanged { .. } => counts.queries += 1,
            _ => {}
        }
    }

    counts
}

