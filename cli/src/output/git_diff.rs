use anyhow::Result;
use excel_diff::{
    CellValue, DiffOp, DiffReport, QueryChangeKind, QueryMetadataField, StepChange, StepDiff,
    StepType, index_to_address,
};
use std::collections::BTreeMap;
use std::io::Write;

pub fn write_git_diff<W: Write>(
    w: &mut W,
    report: &DiffReport,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    writeln!(w, "diff --git a/{} b/{}", old_path, new_path)?;
    writeln!(w, "--- a/{}", old_path)?;
    writeln!(w, "+++ b/{}", new_path)?;

    if report.ops.is_empty() {
        writeln!(w, "@@ No differences @@")?;
        return Ok(());
    }

    let (workbook_ops, sheet_ops, query_ops, measure_ops) = partition_ops(report);

    if !workbook_ops.is_empty() {
        writeln!(w, "@@ Workbook @@")?;
        for op in &workbook_ops {
            write_op_diff_lines(w, report, op)?;
        }
    }

    for (sheet_name, ops) in &sheet_ops {
        writeln!(w, "@@ Sheet \"{}\" @@", sheet_name)?;
        for op in ops {
            write_op_diff_lines(w, report, op)?;
        }
    }

    if !query_ops.is_empty() {
        writeln!(w, "@@ Power Query @@")?;
        for op in &query_ops {
            write_op_diff_lines(w, report, op)?;
        }
    }

    if !measure_ops.is_empty() {
        writeln!(w, "@@ Measures @@")?;
        for op in &measure_ops {
            write_op_diff_lines(w, report, op)?;
        }
    }

    Ok(())
}

fn partition_ops(
    report: &DiffReport,
) -> (
    Vec<&DiffOp>,
    BTreeMap<String, Vec<&DiffOp>>,
    Vec<&DiffOp>,
    Vec<&DiffOp>,
) {
    let mut workbook_ops: Vec<&DiffOp> = Vec::new();
    let mut sheet_ops: BTreeMap<String, Vec<&DiffOp>> = BTreeMap::new();
    let mut query_ops: Vec<&DiffOp> = Vec::new();
    let mut measure_ops: Vec<&DiffOp> = Vec::new();

    for op in &report.ops {
        if op.is_m_op() {
            query_ops.push(op);
        } else if is_measure_op(op) {
            measure_ops.push(op);
        } else if let Some(sheet_id) = get_sheet_id(op) {
            let sheet_name = report
                .resolve(sheet_id)
                .unwrap_or("<unknown>")
                .to_string();
            sheet_ops.entry(sheet_name).or_default().push(op);
        } else {
            workbook_ops.push(op);
        }
    }

    (workbook_ops, sheet_ops, query_ops, measure_ops)
}

fn is_measure_op(op: &DiffOp) -> bool {
    matches!(
        op,
        DiffOp::MeasureAdded { .. }
            | DiffOp::MeasureRemoved { .. }
            | DiffOp::MeasureDefinitionChanged { .. }
    )
}

fn get_sheet_id(op: &DiffOp) -> Option<excel_diff::StringId> {
    match op {
        DiffOp::SheetAdded { sheet } => Some(*sheet),
        DiffOp::SheetRemoved { sheet } => Some(*sheet),
        DiffOp::SheetRenamed { sheet, .. } => Some(*sheet),
        DiffOp::RowAdded { sheet, .. } => Some(*sheet),
        DiffOp::RowRemoved { sheet, .. } => Some(*sheet),
        DiffOp::RowReplaced { sheet, .. } => Some(*sheet),
        DiffOp::ColumnAdded { sheet, .. } => Some(*sheet),
        DiffOp::ColumnRemoved { sheet, .. } => Some(*sheet),
        DiffOp::BlockMovedRows { sheet, .. } => Some(*sheet),
        DiffOp::BlockMovedColumns { sheet, .. } => Some(*sheet),
        DiffOp::BlockMovedRect { sheet, .. } => Some(*sheet),
        DiffOp::RectReplaced { sheet, .. } => Some(*sheet),
        DiffOp::CellEdited { sheet, .. } => Some(*sheet),
        DiffOp::ChartAdded { sheet, .. } => Some(*sheet),
        DiffOp::ChartRemoved { sheet, .. } => Some(*sheet),
        DiffOp::ChartChanged { sheet, .. } => Some(*sheet),
        _ => None,
    }
}

fn write_op_diff_lines<W: Write>(w: &mut W, report: &DiffReport, op: &DiffOp) -> Result<()> {
    match op {
        DiffOp::SheetAdded { sheet } => {
            writeln!(
                w,
                "+ Sheet \"{}\": ADDED",
                report.resolve(*sheet).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::SheetRemoved { sheet } => {
            writeln!(
                w,
                "- Sheet \"{}\": REMOVED",
                report.resolve(*sheet).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::SheetRenamed { from, to, .. } => {
            writeln!(
                w,
                "~ Sheet renamed: \"{}\" -> \"{}\"",
                report.resolve(*from).unwrap_or("<unknown>"),
                report.resolve(*to).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::RowAdded { row_idx, .. } => {
            writeln!(w, "+ Row {}: ADDED", row_idx + 1)?;
        }
        DiffOp::RowRemoved { row_idx, .. } => {
            writeln!(w, "- Row {}: REMOVED", row_idx + 1)?;
        }
        DiffOp::RowReplaced { row_idx, .. } => {
            writeln!(w, "~ Row {}: REPLACED", row_idx + 1)?;
        }
        DiffOp::ColumnAdded { col_idx, .. } => {
            writeln!(w, "+ Column {}: ADDED", col_letter(*col_idx))?;
        }
        DiffOp::ColumnRemoved { col_idx, .. } => {
            writeln!(w, "- Column {}: REMOVED", col_letter(*col_idx))?;
        }
        DiffOp::BlockMovedRows {
            src_start_row,
            row_count,
            dst_start_row,
            ..
        } => {
            let src_end = src_start_row + row_count - 1;
            let dst_end = dst_start_row + row_count - 1;
            writeln!(
                w,
                "- Block: rows {}-{} (moved)",
                src_start_row + 1,
                src_end + 1
            )?;
            writeln!(
                w,
                "+ Block: rows {}-{} (moved)",
                dst_start_row + 1,
                dst_end + 1
            )?;
        }
        DiffOp::BlockMovedColumns {
            src_start_col,
            col_count,
            dst_start_col,
            ..
        } => {
            let src_end = src_start_col + col_count - 1;
            let dst_end = dst_start_col + col_count - 1;
            writeln!(
                w,
                "- Block: columns {}-{} (moved)",
                col_letter(*src_start_col),
                col_letter(src_end)
            )?;
            writeln!(
                w,
                "+ Block: columns {}-{} (moved)",
                col_letter(*dst_start_col),
                col_letter(dst_end)
            )?;
        }
        DiffOp::BlockMovedRect {
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
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
            writeln!(w, "- Block: {} (moved)", src_range)?;
            writeln!(w, "+ Block: {} (moved)", dst_range)?;
        }
        DiffOp::RectReplaced {
            start_row,
            row_count,
            start_col,
            col_count,
            ..
        } => {
            let range = format_range(*start_row, *start_col, *row_count, *col_count);
            writeln!(w, "~ Rect replaced: {}", range)?;
        }
        DiffOp::CellEdited {
            addr, from, to, ..
        } => {
            let old_str = format_cell_value(&from.value, report);
            let new_str = format_cell_value(&to.value, report);
            writeln!(w, "- Cell {}: {}", addr, old_str)?;
            writeln!(w, "+ Cell {}: {}", addr, new_str)?;
        }
        DiffOp::QueryAdded { name } => {
            writeln!(
                w,
                "+ Query \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::QueryRemoved { name } => {
            writeln!(
                w,
                "- Query \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::QueryRenamed { from, to } => {
            writeln!(
                w,
                "- Query \"{}\"",
                report.resolve(*from).unwrap_or("<unknown>")
            )?;
            writeln!(
                w,
                "+ Query \"{}\" (renamed)",
                report.resolve(*to).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::QueryDefinitionChanged {
            name,
            change_kind,
            semantic_detail,
            ..
        } => {
            let kind_str = match change_kind {
                QueryChangeKind::Semantic => "semantic change",
                QueryChangeKind::FormattingOnly => "formatting only",
                QueryChangeKind::Renamed => "renamed",
            };
            writeln!(
                w,
                "~ Query \"{}\": definition changed ({})",
                report.resolve(*name).unwrap_or("<unknown>"),
                kind_str
            )?;

            if let Some(detail) = semantic_detail {
                if !detail.step_diffs.is_empty() {
                    let mut added = 0usize;
                    let mut removed = 0usize;
                    let mut modified = 0usize;
                    let mut reordered = 0usize;

                    for d in &detail.step_diffs {
                        match d {
                            StepDiff::StepAdded { .. } => added += 1,
                            StepDiff::StepRemoved { .. } => removed += 1,
                            StepDiff::StepModified { .. } => modified += 1,
                            StepDiff::StepReordered { .. } => reordered += 1,
                        }
                    }

                    writeln!(w, "~   steps: +{} -{} ~{} r{}", added, removed, modified, reordered)?;

                    let max_lines = 3;
                    for d in detail.step_diffs.iter().take(max_lines) {
                        writeln!(w, "~   {}", format_step_diff(report, d))?;
                    }
                    if detail.step_diffs.len() > max_lines {
                        writeln!(
                            w,
                            "~   ... ({} more)",
                            detail.step_diffs.len() - max_lines
                        )?;
                    }
                } else if let Some(ast) = &detail.ast_summary {
                    writeln!(
                        w,
                        "~   ast: moved={} inserted={} deleted={} updated={}",
                        ast.moved, ast.inserted, ast.deleted, ast.updated
                    )?;
                }
            }
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
            writeln!(
                w,
                "- Query \"{}\".{}: {}",
                report.resolve(*name).unwrap_or("<unknown>"),
                field_name,
                old_str
            )?;
            writeln!(
                w,
                "+ Query \"{}\".{}: {}",
                report.resolve(*name).unwrap_or("<unknown>"),
                field_name,
                new_str
            )?;
        }
        DiffOp::VbaModuleAdded { name } => {
            writeln!(
                w,
                "+ VBA module \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::VbaModuleRemoved { name } => {
            writeln!(
                w,
                "- VBA module \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::VbaModuleChanged { name } => {
            writeln!(
                w,
                "~ VBA module \"{}\": CHANGED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::NamedRangeAdded { name } => {
            writeln!(
                w,
                "+ Named range \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::NamedRangeRemoved { name } => {
            writeln!(
                w,
                "- Named range \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::NamedRangeChanged {
            name,
            old_ref,
            new_ref,
        } => {
            writeln!(
                w,
                "- Named range \"{}\": {}",
                report.resolve(*name).unwrap_or("<unknown>"),
                report.resolve(*old_ref).unwrap_or("<unknown>")
            )?;
            writeln!(
                w,
                "+ Named range \"{}\": {}",
                report.resolve(*name).unwrap_or("<unknown>"),
                report.resolve(*new_ref).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::ChartAdded { name, .. } => {
            writeln!(
                w,
                "+ Chart \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::ChartRemoved { name, .. } => {
            writeln!(
                w,
                "- Chart \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::ChartChanged { name, .. } => {
            writeln!(
                w,
                "~ Chart \"{}\": CHANGED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::MeasureAdded { name } => {
            writeln!(
                w,
                "+ Measure \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::MeasureRemoved { name } => {
            writeln!(
                w,
                "- Measure \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        DiffOp::MeasureDefinitionChanged { name, .. } => {
            writeln!(
                w,
                "~ Measure \"{}\": definition changed",
                report.resolve(*name).unwrap_or("<unknown>")
            )?;
        }
        _ => {
            writeln!(w, "~ {:?}", op)?;
        }
    }
    Ok(())
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

fn format_step_diff(report: &DiffReport, d: &StepDiff) -> String {
    match d {
        StepDiff::StepAdded { step } => format!(
            "+ {} ({})",
            report.resolve(step.name).unwrap_or("<unknown>"),
            format_step_type(step.step_type)
        ),
        StepDiff::StepRemoved { step } => format!(
            "- {} ({})",
            report.resolve(step.name).unwrap_or("<unknown>"),
            format_step_type(step.step_type)
        ),
        StepDiff::StepReordered {
            name,
            from_index,
            to_index,
        } => format!(
            "r {} {} -> {}",
            report.resolve(*name).unwrap_or("<unknown>"),
            from_index,
            to_index
        ),
        StepDiff::StepModified {
            before: _,
            after,
            changes,
        } => {
            let mut line = format!(
                "~ {} ({})",
                report.resolve(after.name).unwrap_or("<unknown>"),
                format_step_type(after.step_type)
            );
            if !changes.is_empty() {
                let mut parts = Vec::new();
                for change in changes {
                    match change {
                        StepChange::Renamed { from, to } => parts.push(format!(
                            "renamed {} -> {}",
                            report.resolve(*from).unwrap_or("<unknown>"),
                            report.resolve(*to).unwrap_or("<unknown>")
                        )),
                        StepChange::SourceRefsChanged { removed, added } => parts.push(format!(
                            "refs -{} +{}",
                            removed.len(),
                            added.len()
                        )),
                        StepChange::ParamsChanged => parts.push("params".to_string()),
                    }
                }
                line.push_str(&format!(" [{}]", parts.join(", ")));
            }
            line
        }
    }
}

fn format_step_type(t: StepType) -> &'static str {
    match t {
        StepType::TableSelectRows => "Table.SelectRows",
        StepType::TableRemoveColumns => "Table.RemoveColumns",
        StepType::TableRenameColumns => "Table.RenameColumns",
        StepType::TableTransformColumnTypes => "Table.TransformColumnTypes",
        StepType::TableNestedJoin => "Table.NestedJoin",
        StepType::TableJoin => "Table.Join",
        StepType::Other => "Other",
    }
}

