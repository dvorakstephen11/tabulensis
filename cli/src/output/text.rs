use crate::commands::diff::Verbosity;
use anyhow::Result;
use excel_diff::{
    CellValue, DiffOp, DiffReport, QueryChangeKind, QueryMetadataField, StepChange, StepDiff,
    StepType, StringId, index_to_address,
};
use std::collections::BTreeMap;
use std::io::Write;

pub fn write_text_report<W: Write>(
    w: &mut W,
    report: &DiffReport,
    old_path: &str,
    new_path: &str,
    verbosity: Verbosity,
) -> Result<()> {
    if verbosity != Verbosity::Quiet {
        let old_name = std::path::Path::new(old_path)
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| old_path.into());
        let new_name = std::path::Path::new(new_path)
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| new_path.into());
        writeln!(w, "Comparing: {} -> {}", old_name, new_name)?;
        writeln!(w)?;
    }

    if verbosity == Verbosity::Quiet {
        write_summary(w, report)?;
        return Ok(());
    }

    if report.ops.is_empty() {
        writeln!(w, "No differences found.")?;
        write_summary(w, report)?;
        return Ok(());
    }

    let (workbook_ops, sheet_ops, m_ops) = partition_ops(report);

    if !workbook_ops.is_empty() {
        writeln!(w, "Workbook:")?;
        for op in &workbook_ops {
            let lines = render_op(report, op, verbosity);
            for line in lines {
                writeln!(w, "  {}", line)?;
            }
        }
        writeln!(w)?;
    }

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

    write_summary(w, report)?;
    Ok(())
}

fn partition_ops(
    report: &DiffReport,
) -> (Vec<&DiffOp>, BTreeMap<String, Vec<&DiffOp>>, Vec<&DiffOp>) {
    let mut workbook_ops: Vec<&DiffOp> = Vec::new();
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
        } else {
            workbook_ops.push(op);
        }
    }

    (workbook_ops, sheet_ops, m_ops)
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
        DiffOp::ChartAdded { sheet, .. } => Some(*sheet),
        DiffOp::ChartRemoved { sheet, .. } => Some(*sheet),
        DiffOp::ChartChanged { sheet, .. } => Some(*sheet),
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

            let mut lines = vec![format!(
                "Query \"{}\": definition changed ({})",
                report.resolve(*name).unwrap_or("<unknown>"),
                kind_str
            )];

            let Some(detail) = semantic_detail else {
                return lines;
            };

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

                lines.push(format!(
                    "  steps: +{} -{} ~{} r{}",
                    added, removed, modified, reordered
                ));

                let max_lines = if verbosity == Verbosity::Verbose { 50 } else { 5 };
                for d in detail.step_diffs.iter().take(max_lines) {
                    lines.push(format!("  {}", format_step_diff(report, d)));
                }
                if detail.step_diffs.len() > max_lines {
                    lines.push(format!(
                        "  ... ({} more)",
                        detail.step_diffs.len() - max_lines
                    ));
                }

                return lines;
            }

            if let Some(ast) = &detail.ast_summary {
                lines.push(format!(
                    "  ast: mode={:?} moved={} inserted={} deleted={} updated={}",
                    ast.mode, ast.moved, ast.inserted, ast.deleted, ast.updated
                ));
                if verbosity == Verbosity::Verbose && !ast.move_hints.is_empty() {
                    for mh in ast.move_hints.iter().take(8) {
                        lines.push(format!(
                            "  ast_move: hash={} size={} from={} to={}",
                            mh.subtree_hash, mh.subtree_size, mh.from_preorder, mh.to_preorder
                        ));
                    }
                }
            }

            lines
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
        DiffOp::VbaModuleAdded { name } => {
            vec![format!(
                "VBA module \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::VbaModuleRemoved { name } => {
            vec![format!(
                "VBA module \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::VbaModuleChanged { name } => {
            vec![format!(
                "VBA module \"{}\": CHANGED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::NamedRangeAdded { name } => {
            vec![format!(
                "Named range \"{}\": ADDED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::NamedRangeRemoved { name } => {
            vec![format!(
                "Named range \"{}\": REMOVED",
                report.resolve(*name).unwrap_or("<unknown>")
            )]
        }
        DiffOp::NamedRangeChanged {
            name,
            old_ref,
            new_ref,
        } => {
            let mut lines = vec![format!(
                "Named range \"{}\": CHANGED",
                report.resolve(*name).unwrap_or("<unknown>")
            )];
            if verbosity == Verbosity::Verbose {
                let old_str = report.resolve(*old_ref).unwrap_or("<unknown>");
                let new_str = report.resolve(*new_ref).unwrap_or("<unknown>");
                lines.push(format!("  refers_to: {} -> {}", old_str, new_str));
            }
            lines
        }
        DiffOp::ChartAdded { name, .. } => vec![format!(
            "Chart \"{}\": ADDED",
            report.resolve(*name).unwrap_or("<unknown>")
        )],
        DiffOp::ChartRemoved { name, .. } => vec![format!(
            "Chart \"{}\": REMOVED",
            report.resolve(*name).unwrap_or("<unknown>")
        )],
        DiffOp::ChartChanged { name, .. } => vec![format!(
            "Chart \"{}\": CHANGED",
            report.resolve(*name).unwrap_or("<unknown>")
        )],
        _ => vec![format!("{:?}", op)],
    }
}

fn col_letter(col: u32) -> String {
    index_to_address(0, col)
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect()
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

fn write_summary<W: Write>(w: &mut W, report: &DiffReport) -> Result<()> {
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

