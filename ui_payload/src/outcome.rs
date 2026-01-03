use std::collections::HashMap;

use excel_diff::{DiffOp, DiffReport, DiffSink, DiffSummary, QueryMetadataField, StringId};
use serde::{Deserialize, Serialize};

use crate::DiffWithSheets;
use crate::options::{DiffLimits, DiffPreset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffOutcomeMode {
    Payload,
    Large,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffOutcomeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<DiffPreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<DiffLimits>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_id: Option<String>,
    pub mode: DiffOutcomeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<DiffWithSheets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<DiffOutcomeSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<DiffOutcomeConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct SummaryMeta {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub old_name: Option<String>,
    pub new_name: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCounts {
    pub added: u64,
    pub removed: u64,
    pub modified: u64,
    pub moved: u64,
}

impl ChangeCounts {
    fn apply(&mut self, kind: ChangeKind) {
        match kind {
            ChangeKind::Added => self.added = self.added.saturating_add(1),
            ChangeKind::Removed => self.removed = self.removed.saturating_add(1),
            ChangeKind::Modified => self.modified = self.modified.saturating_add(1),
            ChangeKind::Moved => self.moved = self.moved.saturating_add(1),
        }
    }

    pub fn add_op(&mut self, op: &DiffOp) {
        if let Some(kind) = classify_op(op) {
            self.apply(kind);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSummary {
    pub sheet_name: String,
    pub op_count: u64,
    pub counts: ChangeCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffOutcomeSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
    pub complete: bool,
    pub op_count: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub counts: ChangeCounts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<SheetSummary>,
}

#[derive(Debug, Clone, Copy)]
enum ChangeKind {
    Added,
    Removed,
    Modified,
    Moved,
}

#[derive(Debug, Default, Clone)]
struct SheetStats {
    op_count: u64,
    counts: ChangeCounts,
}

pub struct SummarySink {
    counts: ChangeCounts,
    sheet_stats: HashMap<u32, SheetStats>,
    strings: Vec<String>,
}

impl SummarySink {
    pub fn new() -> Self {
        Self {
            counts: ChangeCounts::default(),
            sheet_stats: HashMap::new(),
            strings: Vec::new(),
        }
    }

    pub fn into_summary(self, summary: DiffSummary, meta: SummaryMeta) -> DiffOutcomeSummary {
        let mut sheets = Vec::with_capacity(self.sheet_stats.len());
        for (sheet_id, stats) in self.sheet_stats {
            let sheet_name = self
                .strings
                .get(sheet_id as usize)
                .cloned()
                .unwrap_or_else(|| "<unknown>".to_string());
            sheets.push(SheetSummary {
                sheet_name,
                op_count: stats.op_count,
                counts: stats.counts,
            });
        }
        sheets.sort_by(|a, b| a.sheet_name.cmp(&b.sheet_name));

        DiffOutcomeSummary {
            old_path: meta.old_path,
            new_path: meta.new_path,
            old_name: meta.old_name,
            new_name: meta.new_name,
            complete: summary.complete,
            op_count: summary.op_count as u64,
            warnings: summary.warnings,
            counts: self.counts,
            sheets,
        }
    }
}

impl DiffSink for SummarySink {
    fn begin(&mut self, pool: &excel_diff::StringPool) -> Result<(), excel_diff::DiffError> {
        self.strings = pool.strings().to_vec();
        Ok(())
    }

    fn emit(&mut self, op: DiffOp) -> Result<(), excel_diff::DiffError> {
        self.counts.add_op(&op);

        if let Some(sheet_id) = op_sheet_id(&op).map(|id| id.0) {
            let entry = self
                .sheet_stats
                .entry(sheet_id)
                .or_insert_with(SheetStats::default);
            entry.op_count = entry.op_count.saturating_add(1);
            entry.counts.add_op(&op);
        }

        Ok(())
    }
}

pub fn summarize_report(report: &DiffReport, meta: SummaryMeta) -> DiffOutcomeSummary {
    let mut counts = ChangeCounts::default();
    let mut stats: HashMap<u32, SheetStats> = HashMap::new();

    for op in &report.ops {
        counts.add_op(op);
        if let Some(sheet_id) = op_sheet_id(op).map(|id| id.0) {
            let entry = stats.entry(sheet_id).or_insert_with(SheetStats::default);
            entry.op_count = entry.op_count.saturating_add(1);
            entry.counts.add_op(op);
        }
    }

    let mut sheets = Vec::with_capacity(stats.len());
    for (sheet_id, sheet_stats) in stats {
        let sheet_name = report
            .strings
            .get(sheet_id as usize)
            .cloned()
            .unwrap_or_else(|| "<unknown>".to_string());
        sheets.push(SheetSummary {
            sheet_name,
            op_count: sheet_stats.op_count,
            counts: sheet_stats.counts,
        });
    }
    sheets.sort_by(|a, b| a.sheet_name.cmp(&b.sheet_name));

    DiffOutcomeSummary {
        old_path: meta.old_path,
        new_path: meta.new_path,
        old_name: meta.old_name,
        new_name: meta.new_name,
        complete: report.complete,
        op_count: report.ops.len() as u64,
        warnings: report.warnings.clone(),
        counts,
        sheets,
    }
}

fn op_sheet_id(op: &DiffOp) -> Option<StringId> {
    match op {
        DiffOp::SheetAdded { sheet }
        | DiffOp::SheetRemoved { sheet }
        | DiffOp::SheetRenamed { sheet, .. }
        | DiffOp::RowAdded { sheet, .. }
        | DiffOp::RowRemoved { sheet, .. }
        | DiffOp::RowReplaced { sheet, .. }
        | DiffOp::ColumnAdded { sheet, .. }
        | DiffOp::ColumnRemoved { sheet, .. }
        | DiffOp::BlockMovedRows { sheet, .. }
        | DiffOp::BlockMovedColumns { sheet, .. }
        | DiffOp::BlockMovedRect { sheet, .. }
        | DiffOp::RectReplaced { sheet, .. }
        | DiffOp::CellEdited { sheet, .. } => Some(*sheet),
        _ => None,
    }
}

fn classify_op(op: &DiffOp) -> Option<ChangeKind> {
    match op {
        DiffOp::SheetAdded { .. }
        | DiffOp::RowAdded { .. }
        | DiffOp::ColumnAdded { .. }
        | DiffOp::NamedRangeAdded { .. }
        | DiffOp::ChartAdded { .. }
        | DiffOp::VbaModuleAdded { .. }
        | DiffOp::QueryAdded { .. }
        | DiffOp::QueryMetadataChanged {
            field: QueryMetadataField::LoadToSheet,
            ..
        } => Some(ChangeKind::Added),
        DiffOp::SheetRemoved { .. }
        | DiffOp::RowRemoved { .. }
        | DiffOp::ColumnRemoved { .. }
        | DiffOp::NamedRangeRemoved { .. }
        | DiffOp::ChartRemoved { .. }
        | DiffOp::VbaModuleRemoved { .. }
        | DiffOp::QueryRemoved { .. } => Some(ChangeKind::Removed),
        DiffOp::BlockMovedRows { .. }
        | DiffOp::BlockMovedColumns { .. }
        | DiffOp::BlockMovedRect { .. } => Some(ChangeKind::Moved),
        DiffOp::RowReplaced { .. }
        | DiffOp::RectReplaced { .. }
        | DiffOp::CellEdited { .. }
        | DiffOp::SheetRenamed { .. }
        | DiffOp::NamedRangeChanged { .. }
        | DiffOp::ChartChanged { .. }
        | DiffOp::VbaModuleChanged { .. }
        | DiffOp::QueryRenamed { .. }
        | DiffOp::QueryDefinitionChanged { .. }
        | DiffOp::QueryMetadataChanged { .. } => Some(ChangeKind::Modified),
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { .. } => Some(ChangeKind::Added),
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureRemoved { .. } => Some(ChangeKind::Removed),
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureDefinitionChanged { .. } => Some(ChangeKind::Modified),
        _ => None,
    }
}
