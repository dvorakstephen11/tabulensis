use std::collections::HashMap;

use excel_diff::{
    CellAddress, CellValue, DiffOp, DiffReport, ExpressionChangeKind, FormulaDiffResult,
    QueryChangeKind, QueryMetadataField, StringId,
};

fn format_cell_value_for_explain(report: &DiffReport, value: &Option<CellValue>) -> String {
    match value {
        None => "<empty>".to_string(),
        Some(CellValue::Blank) => "<blank>".to_string(),
        Some(CellValue::Number(n)) => format!("{n}"),
        Some(CellValue::Text(id)) => report.resolve(*id).unwrap_or("<unknown>").to_string(),
        Some(CellValue::Bool(b)) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Some(CellValue::Error(id)) => report.resolve(*id).unwrap_or("<error>").to_string(),
    }
}

fn format_formula_for_explain(report: &DiffReport, id: &Option<StringId>) -> String {
    match id {
        None => "<none>".to_string(),
        Some(id) => {
            let raw = report.resolve(*id).unwrap_or("<unknown>");
            if raw.trim().is_empty() {
                "<none>".to_string()
            } else if raw.starts_with('=') {
                raw.to_string()
            } else {
                format!("={raw}")
            }
        }
    }
}

pub(crate) fn build_cell_explain_text(
    report: &DiffReport,
    sheet_name: &str,
    addr: CellAddress,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let a1 = addr.to_a1();
    lines.push(format!("Explain: {sheet_name}!{a1}"));
    lines.push("".to_string());

    // Cell edit details.
    let mut cell_edit: Option<(
        excel_diff::CellSnapshot,
        excel_diff::CellSnapshot,
        FormulaDiffResult,
    )> = None;
    for op in &report.ops {
        if let DiffOp::CellEdited {
            sheet,
            addr: op_addr,
            from,
            to,
            formula_diff,
        } = op
        {
            let sheet_resolved = report.resolve(*sheet).unwrap_or("<unknown>");
            if !sheet_resolved.eq_ignore_ascii_case(sheet_name) {
                continue;
            }
            if op_addr.row == addr.row && op_addr.col == addr.col {
                cell_edit = Some((from.clone(), to.clone(), *formula_diff));
                break;
            }
        }
    }

    if let Some((from, to, formula_diff)) = cell_edit {
        let from_value = format_cell_value_for_explain(report, &from.value);
        let to_value = format_cell_value_for_explain(report, &to.value);
        let from_formula = format_formula_for_explain(report, &from.formula);
        let to_formula = format_formula_for_explain(report, &to.formula);
        lines.push("Cell change:".to_string());
        lines.push(format!("  Old value: {from_value}"));
        lines.push(format!("  New value: {to_value}"));
        lines.push(format!("  Old formula: {from_formula}"));
        lines.push(format!("  New formula: {to_formula}"));
        lines.push(format!("  formula_diff: {:?}", formula_diff));
    } else {
        lines.push("Cell change:".to_string());
        lines.push("  No CellEdited op found for this cell.".to_string());

        // Best-effort: mention structural ops that cover the cell.
        for op in &report.ops {
            match op {
                DiffOp::RectReplaced {
                    sheet,
                    start_row,
                    row_count,
                    start_col,
                    col_count,
                } => {
                    let sheet_resolved = report.resolve(*sheet).unwrap_or("<unknown>");
                    if !sheet_resolved.eq_ignore_ascii_case(sheet_name) {
                        continue;
                    }
                    let row_end = start_row.saturating_add(*row_count).saturating_sub(1);
                    let col_end = start_col.saturating_add(*col_count).saturating_sub(1);
                    if addr.row >= *start_row
                        && addr.row <= row_end
                        && addr.col >= *start_col
                        && addr.col <= col_end
                    {
                        lines.push(format!(
                            "  Covered by RectReplaced: rows {}-{}, cols {}-{}",
                            start_row + 1,
                            row_end + 1,
                            start_col,
                            col_end
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    // Power Query heuristics.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Strength {
        Weak,
        Strong,
    }

    impl Default for Strength {
        fn default() -> Self {
            Strength::Weak
        }
    }

    #[derive(Default)]
    struct QueryInfo {
        strength: Strength,
        reasons: Vec<String>,
        def_change: Option<QueryChangeKind>,
    }

    let mut query_map: HashMap<String, QueryInfo> = HashMap::new();
    let sheet_lower = sheet_name.to_lowercase();
    for op in &report.ops {
        match op {
            DiffOp::QueryMetadataChanged {
                name, field, new, ..
            } => {
                let query = report.resolve(*name).unwrap_or("<unknown>").to_string();
                match field {
                    QueryMetadataField::LoadToSheet => {
                        if let Some(new_id) = new {
                            let target = report.resolve(*new_id).unwrap_or("");
                            if target.eq_ignore_ascii_case(sheet_name) {
                                let entry = query_map.entry(query).or_default();
                                entry.strength = Strength::Strong;
                                entry
                                    .reasons
                                    .push("LoadToSheet matches this sheet".to_string());
                            }
                        }
                    }
                    QueryMetadataField::GroupPath => {
                        if let Some(new_id) = new {
                            let path = report.resolve(*new_id).unwrap_or("");
                            if path.to_lowercase().contains(&sheet_lower) {
                                let entry = query_map.entry(query).or_default();
                                entry.strength = entry.strength.max(Strength::Weak);
                                entry
                                    .reasons
                                    .push("GroupPath contains sheet name".to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            DiffOp::QueryDefinitionChanged {
                name, change_kind, ..
            } => {
                let query = report.resolve(*name).unwrap_or("<unknown>").to_string();
                if let Some(entry) = query_map.get_mut(&query) {
                    entry.def_change = Some(*change_kind);
                }
            }
            _ => {}
        }
    }

    let mut queries: Vec<(String, QueryInfo)> = query_map.into_iter().collect();
    queries.sort_by(|a, b| b.1.strength.cmp(&a.1.strength).then_with(|| a.0.cmp(&b.0)));
    lines.push("".to_string());
    lines.push("Power Query candidates (v1 heuristics):".to_string());
    if queries.is_empty() {
        lines.push("  (none)".to_string());
    } else {
        for (name, info) in queries.iter().take(8) {
            let strength = if info.strength == Strength::Strong {
                "strong"
            } else {
                "weak"
            };
            let mut detail = String::new();
            if let Some(kind) = info.def_change {
                detail.push_str(&format!("definition={kind:?}"));
            }
            let reasons = if info.reasons.is_empty() {
                "".to_string()
            } else {
                info.reasons.join("; ")
            };
            if !detail.is_empty() && !reasons.is_empty() {
                detail.push_str(" | ");
            }
            detail.push_str(&reasons);
            lines.push(format!(
                "  - {strength}: {name}{}",
                if detail.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({detail})")
                }
            ));
        }
    }

    // Model summary (best-effort).
    let mut dax_semantic = 0u64;
    let mut dax_formatting = 0u64;
    let mut dax_unknown = 0u64;
    for op in &report.ops {
        match op {
            DiffOp::MeasureDefinitionChanged { change_kind, .. }
            | DiffOp::CalculatedColumnDefinitionChanged { change_kind, .. } => match change_kind {
                ExpressionChangeKind::Semantic => dax_semantic += 1,
                ExpressionChangeKind::FormattingOnly => dax_formatting += 1,
                ExpressionChangeKind::Unknown => dax_unknown += 1,
            },
            _ => {}
        }
    }
    lines.push("".to_string());
    lines.push("Model (DAX) summary:".to_string());
    if dax_semantic == 0 && dax_formatting == 0 && dax_unknown == 0 {
        lines.push("  (none)".to_string());
    } else {
        lines.push(format!(
            "  measure/column definition changes: semantic={} formatting_only={} unknown={}",
            dax_semantic, dax_formatting, dax_unknown
        ));
    }

    lines.push("".to_string());
    lines.push("Notes: Explain is deterministic and best-effort (no external calls).".to_string());
    lines.join("\n")
}
