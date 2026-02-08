use std::collections::{HashMap, HashSet};

use excel_diff::{DiffOp, DiffReport, ExpressionChangeKind, FormulaDiffResult, QueryChangeKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NoiseFilters {
    #[serde(default)]
    pub hide_m_formatting_only: bool,
    #[serde(default)]
    pub hide_dax_formatting_only: bool,
    #[serde(default)]
    pub hide_formula_formatting_only: bool,
    #[serde(default)]
    pub collapse_moves: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpCategory {
    Grid,
    PowerQuery,
    Model,
    Objects,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpNoiseClass {
    Unknown,
    FormattingOnly,
    RenameOnly,
    Structural,
    Move,
    ValueChange,
    FormulaChange,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeverityCounts {
    pub high: u64,
    pub medium: u64,
    pub low: u64,
}

impl SeverityCounts {
    fn add(&mut self, severity: OpSeverity) {
        match severity {
            OpSeverity::High => self.high = self.high.saturating_add(1),
            OpSeverity::Medium => self.medium = self.medium.saturating_add(1),
            OpSeverity::Low => self.low = self.low.saturating_add(1),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCounts {
    pub grid: u64,
    pub power_query: u64,
    pub model: u64,
    pub objects: u64,
    pub other: u64,
}

impl CategoryCounts {
    fn add(&mut self, category: OpCategory) {
        match category {
            OpCategory::Grid => self.grid = self.grid.saturating_add(1),
            OpCategory::PowerQuery => self.power_query = self.power_query.saturating_add(1),
            OpCategory::Model => self.model = self.model.saturating_add(1),
            OpCategory::Objects => self.objects = self.objects.saturating_add(1),
            OpCategory::Other => self.other = self.other.saturating_add(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdownRow {
    pub category: OpCategory,
    pub total: u64,
    pub severity: SeverityCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetBreakdown {
    pub sheet_name: String,
    pub op_count: u64,
    pub counts: crate::outcome::ChangeCounts,
    pub severity: SeverityCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffAnalysis {
    pub op_count: u64,
    pub counts: crate::outcome::ChangeCounts,
    pub categories: CategoryCounts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub category_breakdown: Vec<CategoryBreakdownRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<SheetBreakdown>,
    pub severity: SeverityCounts,
}

#[derive(Debug, Clone, Copy)]
struct OpClass {
    category: OpCategory,
    severity: OpSeverity,
}

fn classify_op(op: &DiffOp) -> OpClass {
    // Category.
    let category = if op.is_m_op() {
        OpCategory::PowerQuery
    } else if op.is_model_op() {
        OpCategory::Model
    } else if matches!(
        op,
        DiffOp::NamedRangeAdded { .. }
            | DiffOp::NamedRangeRemoved { .. }
            | DiffOp::NamedRangeChanged { .. }
            | DiffOp::ChartAdded { .. }
            | DiffOp::ChartRemoved { .. }
            | DiffOp::ChartChanged { .. }
            | DiffOp::VbaModuleAdded { .. }
            | DiffOp::VbaModuleRemoved { .. }
            | DiffOp::VbaModuleChanged { .. }
    ) {
        OpCategory::Objects
    } else if matches!(
        op,
        DiffOp::SheetAdded { .. }
            | DiffOp::SheetRemoved { .. }
            | DiffOp::SheetRenamed { .. }
            | DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::RowReplaced { .. }
            | DiffOp::DuplicateKeyCluster { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. }
            | DiffOp::BlockMovedRect { .. }
            | DiffOp::RectReplaced { .. }
            | DiffOp::CellEdited { .. }
    ) {
        OpCategory::Grid
    } else {
        OpCategory::Other
    };

    // Severity (conservative and deterministic).
    let severity = match op {
        DiffOp::DuplicateKeyCluster { .. } => OpSeverity::High,
        DiffOp::QueryAdded { .. } | DiffOp::QueryRemoved { .. } => OpSeverity::High,
        DiffOp::QueryDefinitionChanged { change_kind, .. } => match change_kind {
            QueryChangeKind::Semantic => OpSeverity::High,
            QueryChangeKind::FormattingOnly => OpSeverity::Low,
            QueryChangeKind::Renamed => OpSeverity::Low,
        },
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { .. }
        | DiffOp::MeasureRemoved { .. }
        | DiffOp::TableAdded { .. }
        | DiffOp::TableRemoved { .. } => OpSeverity::High,
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureDefinitionChanged { change_kind, .. } => match change_kind {
            ExpressionChangeKind::Semantic => OpSeverity::High,
            ExpressionChangeKind::FormattingOnly => OpSeverity::Low,
            ExpressionChangeKind::Unknown => OpSeverity::Medium,
        },
        #[cfg(feature = "model-diff")]
        DiffOp::CalculatedColumnDefinitionChanged { change_kind, .. } => match change_kind {
            ExpressionChangeKind::Semantic => OpSeverity::High,
            ExpressionChangeKind::FormattingOnly => OpSeverity::Low,
            ExpressionChangeKind::Unknown => OpSeverity::Medium,
        },
        DiffOp::CellEdited { formula_diff, .. } => match formula_diff {
            FormulaDiffResult::SemanticChange => OpSeverity::High,
            FormulaDiffResult::FormattingOnly => OpSeverity::Low,
            // Filled is a real semantic impact in many sheets; keep medium.
            FormulaDiffResult::Filled => OpSeverity::Medium,
            // Unknown/TextChange may still be material; keep medium by default.
            _ => OpSeverity::Medium,
        },
        DiffOp::SheetRenamed { .. } | DiffOp::QueryRenamed { .. } => OpSeverity::Low,
        DiffOp::BlockMovedRows { .. }
        | DiffOp::BlockMovedColumns { .. }
        | DiffOp::BlockMovedRect { .. } => OpSeverity::Medium,
        DiffOp::SheetAdded { .. } | DiffOp::SheetRemoved { .. } => OpSeverity::High,
        DiffOp::RowAdded { .. }
        | DiffOp::RowRemoved { .. }
        | DiffOp::RowReplaced { .. }
        | DiffOp::ColumnAdded { .. }
        | DiffOp::ColumnRemoved { .. }
        | DiffOp::RectReplaced { .. } => OpSeverity::Medium,
        DiffOp::NamedRangeAdded { .. }
        | DiffOp::NamedRangeRemoved { .. }
        | DiffOp::NamedRangeChanged { .. }
        | DiffOp::ChartAdded { .. }
        | DiffOp::ChartRemoved { .. }
        | DiffOp::ChartChanged { .. }
        | DiffOp::VbaModuleAdded { .. }
        | DiffOp::VbaModuleRemoved { .. }
        | DiffOp::VbaModuleChanged { .. } => OpSeverity::Medium,
        _ => OpSeverity::Medium,
    };

    OpClass { category, severity }
}

fn should_skip_op(op: &DiffOp, filters: NoiseFilters) -> bool {
    if filters.hide_m_formatting_only {
        if matches!(
            op,
            DiffOp::QueryDefinitionChanged {
                change_kind: QueryChangeKind::FormattingOnly,
                ..
            }
        ) {
            return true;
        }
    }

    if filters.hide_dax_formatting_only {
        #[cfg(feature = "model-diff")]
        {
            if matches!(
                op,
                DiffOp::MeasureDefinitionChanged {
                    change_kind: ExpressionChangeKind::FormattingOnly,
                    ..
                } | DiffOp::CalculatedColumnDefinitionChanged {
                    change_kind: ExpressionChangeKind::FormattingOnly,
                    ..
                }
            ) {
                return true;
            }
        }
    }

    if filters.hide_formula_formatting_only {
        if matches!(
            op,
            DiffOp::CellEdited {
                formula_diff: FormulaDiffResult::FormattingOnly,
                ..
            }
        ) {
            return true;
        }
    }

    false
}

fn op_sheet_name(report: &DiffReport, op: &DiffOp) -> Option<String> {
    let id = match op {
        DiffOp::SheetAdded { sheet }
        | DiffOp::SheetRemoved { sheet }
        | DiffOp::SheetRenamed { sheet, .. }
        | DiffOp::RowAdded { sheet, .. }
        | DiffOp::RowRemoved { sheet, .. }
        | DiffOp::RowReplaced { sheet, .. }
        | DiffOp::DuplicateKeyCluster { sheet, .. }
        | DiffOp::ColumnAdded { sheet, .. }
        | DiffOp::ColumnRemoved { sheet, .. }
        | DiffOp::BlockMovedRows { sheet, .. }
        | DiffOp::BlockMovedColumns { sheet, .. }
        | DiffOp::BlockMovedRect { sheet, .. }
        | DiffOp::RectReplaced { sheet, .. }
        | DiffOp::CellEdited { sheet, .. } => Some(*sheet),
        _ => None,
    }?;
    Some(report.resolve(id).unwrap_or("<unknown>").to_string())
}

fn move_id_for_op(op: &DiffOp) -> Option<String> {
    match op {
        DiffOp::BlockMovedRows {
            src_start_row,
            row_count,
            dst_start_row,
            ..
        } => Some(format!(
            "r:{}+{}->{}",
            src_start_row, row_count, dst_start_row
        )),
        DiffOp::BlockMovedColumns {
            src_start_col,
            col_count,
            dst_start_col,
            ..
        } => Some(format!(
            "c:{}+{}->{}",
            src_start_col, col_count, dst_start_col
        )),
        DiffOp::BlockMovedRect {
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            ..
        } => Some(format!(
            "rect:{},{}+{}x{}->{},{}",
            src_start_row,
            src_start_col,
            src_row_count,
            src_col_count,
            dst_start_row,
            dst_start_col
        )),
        _ => None,
    }
}

pub fn analyze_report(report: &DiffReport, filters: NoiseFilters) -> DiffAnalysis {
    let mut analysis = DiffAnalysis::default();

    let mut by_category: HashMap<OpCategory, SeverityCounts> = HashMap::new();
    let mut sheet_map: HashMap<String, SheetBreakdown> = HashMap::new();
    let mut seen_moves: HashSet<String> = HashSet::new();

    for op in &report.ops {
        if should_skip_op(op, filters) {
            continue;
        }

        if filters.collapse_moves {
            if let Some(move_id) = move_id_for_op(op) {
                if !seen_moves.insert(move_id) {
                    continue;
                }
            }
        }

        analysis.op_count = analysis.op_count.saturating_add(1);
        analysis.counts.add_op(op);

        let class = classify_op(op);
        analysis.categories.add(class.category);
        analysis.severity.add(class.severity);
        by_category
            .entry(class.category)
            .or_default()
            .add(class.severity);

        if let Some(sheet_name) = op_sheet_name(report, op) {
            let entry = sheet_map
                .entry(sheet_name.clone())
                .or_insert_with(|| SheetBreakdown {
                    sheet_name,
                    ..SheetBreakdown::default()
                });
            entry.op_count = entry.op_count.saturating_add(1);
            entry.counts.add_op(op);
            entry.severity.add(class.severity);
        }
    }

    let mut breakdown = Vec::new();
    for (category, severity) in by_category {
        let total = severity.high + severity.medium + severity.low;
        breakdown.push(CategoryBreakdownRow {
            category,
            total,
            severity,
        });
    }
    breakdown.sort_by_key(|row| row.category as u8);
    analysis.category_breakdown = breakdown;

    let mut sheets = sheet_map.into_values().collect::<Vec<_>>();
    sheets.sort_by(|a, b| {
        b.severity
            .high
            .cmp(&a.severity.high)
            .then_with(|| b.op_count.cmp(&a.op_count))
            .then_with(|| {
                a.sheet_name
                    .to_lowercase()
                    .cmp(&b.sheet_name.to_lowercase())
            })
    });
    analysis.sheets = sheets;

    analysis
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_diff::{
        CellAddress, CellSnapshot, DiffOp, DiffReport, FormulaDiffResult, SheetId, StringId,
    };

    fn make_sheet_id() -> SheetId {
        StringId(1)
    }

    #[test]
    fn analyze_report_filters_formatting_only_formula_cell_edits() {
        let sheet = make_sheet_id();
        let addr = CellAddress::from_indices(0, 0);
        let from = CellSnapshot::empty(addr);
        let to = CellSnapshot::empty(addr);

        let ops = vec![
            DiffOp::CellEdited {
                sheet,
                addr,
                from: from.clone(),
                to: to.clone(),
                formula_diff: FormulaDiffResult::FormattingOnly,
            },
            DiffOp::CellEdited {
                sheet,
                addr,
                from,
                to,
                formula_diff: FormulaDiffResult::SemanticChange,
            },
        ];
        let report = DiffReport::new(ops);
        let analysis = analyze_report(
            &report,
            NoiseFilters {
                hide_formula_formatting_only: true,
                ..NoiseFilters::default()
            },
        );
        assert_eq!(analysis.op_count, 1);
        assert_eq!(analysis.severity.high, 1);
    }

    #[test]
    fn analyze_report_collapses_duplicate_moves_by_id() {
        let sheet = make_sheet_id();
        let ops = vec![
            DiffOp::BlockMovedRows {
                sheet,
                src_start_row: 10,
                row_count: 3,
                dst_start_row: 20,
                block_hash: None,
            },
            DiffOp::BlockMovedRows {
                sheet,
                src_start_row: 10,
                row_count: 3,
                dst_start_row: 20,
                block_hash: None,
            },
        ];
        let report = DiffReport::new(ops);
        let analysis = analyze_report(
            &report,
            NoiseFilters {
                collapse_moves: true,
                ..NoiseFilters::default()
            },
        );
        assert_eq!(analysis.op_count, 1);
        assert_eq!(analysis.counts.moved, 1);
    }
}
