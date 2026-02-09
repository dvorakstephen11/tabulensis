use std::path::Path;

use excel_diff::{
    DiffOp, DiffReport, DiffSummary, PbipDocDiff as CorePbipDocDiff,
    PbipEntityDiff as CorePbipEntityDiff, PbipNormalizationProfile as CorePbipProfile,
};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ui_payload::{
    CategoryBreakdownRow, CategoryCounts, DiffAnalysis, NoiseFilters, OpCategory, OpSeverity,
    DetailsPayload, DiffDomain, NavigatorModel, NavigatorRow, SelectionKind, SelectionTarget,
    SeverityCounts, SheetBreakdown,
};
use uuid::Uuid;

use super::types::{
    accumulate_sheet_stats, op_index_fields, ChangeCounts, OpIndexFields, SheetStats,
};

const SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Missing diff run: {0}")]
    MissingRun(String),
    #[error("Missing sheet: {0}")]
    MissingSheet(String),
    #[error("Invalid store data: {0}")]
    InvalidData(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Complete,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffMode {
    Payload,
    Large,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSummary {
    pub sheet_id: u32,
    pub sheet_name: String,
    pub op_count: u64,
    pub counts: ChangeCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffRunSummary {
    pub diff_id: String,
    #[serde(default)]
    pub domain: DiffDomain,
    pub old_path: String,
    pub new_path: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub engine_version: String,
    pub app_version: String,
    pub mode: DiffMode,
    pub status: RunStatus,
    pub trusted: bool,
    pub complete: bool,
    pub op_count: u64,
    pub warnings: Vec<String>,
    pub counts: ChangeCounts,
    pub sheets: Vec<SheetSummary>,
}

pub struct OpStore {
    conn: Connection,
}

impl OpStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Self::apply_schema(&conn)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Self::apply_schema(&conn)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)]
    pub fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn into_connection(self) -> Connection {
        self.conn
    }

    pub fn start_run(
        &self,
        old_path: &str,
        new_path: &str,
        config_json: &str,
        engine_version: &str,
        app_version: &str,
        mode: DiffMode,
        trusted: bool,
    ) -> Result<String, StoreError> {
        let diff_id = Uuid::new_v4().to_string();
        let started_at = now_iso();
        let status = RunStatus::Running;

        self.conn.execute(
            "INSERT INTO diff_runs (diff_id, old_path, new_path, started_at, config_json, engine_version, app_version, schema_version, mode, status, trusted)\
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                diff_id,
                old_path,
                new_path,
                started_at,
                config_json,
                engine_version,
                app_version,
                SCHEMA_VERSION,
                mode.as_str(),
                status.as_str(),
                if trusted { 1 } else { 0 },
            ],
        )?;

        Ok(diff_id)
    }

    pub fn set_mode(&self, diff_id: &str, mode: DiffMode) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE diff_runs SET mode = ?1 WHERE diff_id = ?2",
            params![mode.as_str(), diff_id],
        )?;
        Ok(())
    }

    pub fn set_domain(&self, diff_id: &str, domain: DiffDomain) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO diff_domains (diff_id, domain) VALUES (?1, ?2)",
            params![diff_id, domain.as_str()],
        )?;
        Ok(())
    }

    pub fn load_domain(&self, diff_id: &str) -> Result<DiffDomain, StoreError> {
        let row: Option<String> = self
            .conn
            .query_row(
                "SELECT domain FROM diff_domains WHERE diff_id = ?1",
                params![diff_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(match row.as_deref().map(str::trim).unwrap_or("") {
            "pbip_project" => DiffDomain::PbipProject,
            _ => DiffDomain::ExcelWorkbook,
        })
    }

    pub fn finish_run(
        &self,
        diff_id: &str,
        summary: &DiffSummary,
        strings: &[String],
        counts: &ChangeCounts,
        sheet_stats: &[SheetStatsResolved],
        status: RunStatus,
    ) -> Result<(), StoreError> {
        let finished_at = now_iso();
        let strings_json = serde_json::to_string(strings)?;

        self.conn.execute(
            "UPDATE diff_runs SET finished_at = ?1, status = ?2, complete = ?3, op_count = ?4, warnings_count = ?5,\
             added_count = ?6, removed_count = ?7, modified_count = ?8, moved_count = ?9, strings_json = ?10 WHERE diff_id = ?11",
            params![
                finished_at,
                status.as_str(),
                if summary.complete { 1 } else { 0 },
                summary.op_count as i64,
                summary.warnings.len() as i64,
                counts.added as i64,
                counts.removed as i64,
                counts.modified as i64,
                counts.moved as i64,
                strings_json,
                diff_id,
            ],
        )?;

        self.conn.execute(
            "DELETE FROM diff_warnings WHERE diff_id = ?1",
            params![diff_id],
        )?;
        for (idx, warning) in summary.warnings.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO diff_warnings (diff_id, idx, text) VALUES (?1, ?2, ?3)",
                params![diff_id, idx as i64, warning],
            )?;
        }

        self.conn.execute(
            "DELETE FROM diff_sheets WHERE diff_id = ?1",
            params![diff_id],
        )?;
        for sheet in sheet_stats {
            self.conn.execute(
                "INSERT INTO diff_sheets (diff_id, sheet_id, sheet_name, op_count, added_count, removed_count, modified_count, moved_count)\
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    diff_id,
                    sheet.sheet_id as i64,
                    sheet.sheet_name,
                    sheet.op_count as i64,
                    sheet.counts.added as i64,
                    sheet.counts.removed as i64,
                    sheet.counts.modified as i64,
                    sheet.counts.moved as i64,
                ],
            )?;
        }

        Ok(())
    }

    pub fn fail_run(
        &self,
        diff_id: &str,
        status: RunStatus,
        message: &str,
    ) -> Result<(), StoreError> {
        let finished_at = now_iso();
        self.conn.execute(
            "UPDATE diff_runs SET finished_at = ?1, status = ?2, complete = 0, warnings_count = warnings_count + 1 WHERE diff_id = ?3",
            params![finished_at, status.as_str(), diff_id],
        )?;
        let idx: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(idx) + 1, 0) FROM diff_warnings WHERE diff_id = ?1",
                params![diff_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO diff_warnings (diff_id, idx, text) VALUES (?1, ?2, ?3)",
            params![diff_id, idx, message],
        )?;
        Ok(())
    }

    pub fn insert_ops_from_report(
        &self,
        diff_id: &str,
        report: &DiffReport,
    ) -> Result<(ChangeCounts, Vec<SheetStats>), StoreError> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;

        let mut counts = ChangeCounts::default();
        let mut stats_map: std::collections::HashMap<u32, SheetStats> =
            std::collections::HashMap::new();

        for (idx, op) in report.ops.iter().enumerate() {
            counts.add_op(op);
            accumulate_sheet_stats(&mut stats_map, op);

            let fields: OpIndexFields = op_index_fields(op);
            let payload_json = serde_json::to_string(op)?;

            self.conn.execute(
                "INSERT INTO diff_ops (diff_id, op_idx, kind, sheet_id, row, col, row_end, col_end, move_id, payload_json)\
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    diff_id,
                    idx as i64,
                    fields.kind,
                    fields.sheet_id.map(|v| v as i64),
                    fields.row.map(|v| v as i64),
                    fields.col.map(|v| v as i64),
                    fields.row_end.map(|v| v as i64),
                    fields.col_end.map(|v| v as i64),
                    fields.move_id,
                    payload_json,
                ],
            )?;
        }

        self.conn.execute_batch("COMMIT")?;

        let mut stats: Vec<SheetStats> = stats_map.into_values().collect();
        stats.sort_by_key(|s| s.sheet_id);
        Ok((counts, stats))
    }

    pub fn load_summary(&self, diff_id: &str) -> Result<DiffRunSummary, StoreError> {
        let row = self.conn.query_row(
            "SELECT old_path, new_path, started_at, finished_at, engine_version, app_version, mode, status, trusted, complete, op_count,\
                    added_count, removed_count, modified_count, moved_count \
             FROM diff_runs WHERE diff_id = ?1",
            params![diff_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                ))
            },
        ).optional()?;

        let Some((
            old_path,
            new_path,
            started_at,
            finished_at,
            engine_version,
            app_version,
            mode,
            status,
            trusted,
            complete,
            op_count,
            added,
            removed,
            modified,
            moved,
        )) = row
        else {
            return Err(StoreError::MissingRun(diff_id.to_string()));
        };

        let warnings = self.load_warnings(diff_id)?;
        let sheets = self.load_sheet_summaries(diff_id)?;
        let domain = self.load_domain(diff_id).unwrap_or_default();

        Ok(DiffRunSummary {
            diff_id: diff_id.to_string(),
            domain,
            old_path,
            new_path,
            started_at,
            finished_at,
            engine_version,
            app_version,
            mode: DiffMode::from_str(&mode),
            status: RunStatus::from_str(&status),
            trusted: trusted != 0,
            complete: complete != 0,
            op_count: op_count as u64,
            warnings,
            counts: ChangeCounts {
                added: added as u64,
                removed: removed as u64,
                modified: modified as u64,
                moved: moved as u64,
            },
            sheets,
        })
    }

    pub fn load_report(&self, diff_id: &str) -> Result<DiffReport, StoreError> {
        let summary = self.load_summary(diff_id)?;
        let strings = self.load_strings(diff_id)?;
        let ops = self.load_ops(diff_id)?;
        let mut report = DiffReport::new(ops);
        report.strings = strings;
        report.complete = summary.complete;
        report.warnings = summary.warnings;
        Ok(report)
    }

    pub fn load_sheet_ops(
        &self,
        diff_id: &str,
        sheet_name: &str,
    ) -> Result<Vec<DiffOp>, StoreError> {
        let sheet_id = self.sheet_id_for_name(diff_id, sheet_name)?;
        let mut stmt = self.conn.prepare(
            "SELECT payload_json FROM diff_ops WHERE diff_id = ?1 AND sheet_id = ?2 ORDER BY op_idx",
        )?;
        let rows = stmt.query_map(params![diff_id, sheet_id as i64], |row| {
            row.get::<_, String>(0)
        })?;
        let mut ops = Vec::new();
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            ops.push(op);
        }
        Ok(ops)
    }

    pub fn load_ops_in_range(
        &self,
        diff_id: &str,
        sheet_name: &str,
        row_start: Option<u32>,
        row_end: Option<u32>,
        col_start: Option<u32>,
        col_end: Option<u32>,
    ) -> Result<Vec<DiffOp>, StoreError> {
        let sheet_id = self.sheet_id_for_name(diff_id, sheet_name)?;
        let mut sql =
            String::from("SELECT payload_json FROM diff_ops WHERE diff_id = ?1 AND sheet_id = ?2");
        let mut params_list: Vec<Value> =
            vec![diff_id.to_string().into(), (sheet_id as i64).into()];
        let mut idx = 3;

        let row_start = row_start.or(row_end);
        let row_end = row_end.or(row_start);
        if let (Some(start), Some(end)) = (row_start, row_end) {
            sql.push_str(&format!(
                " AND (row IS NULL OR (row_end IS NULL AND row BETWEEN ?{idx} AND ?{idx_plus}) OR (row_end IS NOT NULL AND row_end >= ?{idx} AND row <= ?{idx_plus}))",
                idx = idx,
                idx_plus = idx + 1
            ));
            params_list.push((start as i64).into());
            params_list.push((end as i64).into());
            idx += 2;
        }

        let col_start = col_start.or(col_end);
        let col_end = col_end.or(col_start);
        if let (Some(start), Some(end)) = (col_start, col_end) {
            sql.push_str(&format!(
                " AND (col IS NULL OR (col_end IS NULL AND col BETWEEN ?{idx} AND ?{idx_plus}) OR (col_end IS NOT NULL AND col_end >= ?{idx} AND col <= ?{idx_plus}))",
                idx = idx,
                idx_plus = idx + 1
            ));
            params_list.push((start as i64).into());
            params_list.push((end as i64).into());
        }

        sql.push_str(" ORDER BY op_idx");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_list), |row| row.get::<_, String>(0))?;
        let mut ops = Vec::new();
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            ops.push(op);
        }
        Ok(ops)
    }

    pub fn load_ops(&self, diff_id: &str) -> Result<Vec<DiffOp>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM diff_ops WHERE diff_id = ?1 ORDER BY op_idx")?;
        let rows = stmt.query_map(params![diff_id], |row| row.get::<_, String>(0))?;
        let mut ops = Vec::new();
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            ops.push(op);
        }
        Ok(ops)
    }

    pub fn load_ops_by_kinds(
        &self,
        diff_id: &str,
        kinds: &[&str],
    ) -> Result<Vec<DiffOp>, StoreError> {
        if kinds.is_empty() {
            return Ok(Vec::new());
        }

        let mut sql =
            String::from("SELECT payload_json FROM diff_ops WHERE diff_id = ?1 AND kind IN (");
        for (idx, _) in kinds.iter().enumerate() {
            if idx > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("?{}", idx + 2));
        }
        sql.push_str(") ORDER BY op_idx");

        let mut params_list: Vec<Value> = Vec::with_capacity(1 + kinds.len());
        params_list.push(diff_id.to_string().into());
        for kind in kinds {
            params_list.push(kind.to_string().into());
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_list), |row| row.get::<_, String>(0))?;
        let mut ops = Vec::new();
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            ops.push(op);
        }
        Ok(ops)
    }

    pub fn load_diff_analysis(
        &self,
        diff_id: &str,
        filters: NoiseFilters,
    ) -> Result<DiffAnalysis, StoreError> {
        let mut sheet_name_by_id: std::collections::HashMap<u32, String> =
            std::collections::HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT sheet_id, sheet_name FROM diff_sheets WHERE diff_id = ?1")?;
            let rows = stmt.query_map(params![diff_id], |row| {
                Ok((row.get::<_, i64>(0)? as u32, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (id, name) = row?;
                sheet_name_by_id.insert(id, name);
            }
        }

        #[derive(Debug)]
        struct AggRow {
            kind: String,
            sheet_id: Option<u32>,
            change_kind: Option<String>,
            formula_diff: Option<String>,
            meta_field: Option<String>,
            count: u64,
        }

        let mut rows: Vec<AggRow> = Vec::new();
        {
            // Aggregate at the DB level to avoid loading huge diff ops into Rust for large mode.
            let mut stmt = self.conn.prepare(
                r#"
                SELECT kind, sheet_id, move_id, change_kind, formula_diff, meta_field, COUNT(*) AS n
                FROM (
                  SELECT kind,
                         sheet_id,
                         move_id,
                         json_extract(payload_json, '$.change_kind') AS change_kind,
                         json_extract(payload_json, '$.formula_diff') AS formula_diff,
                         json_extract(payload_json, '$.field') AS meta_field
                  FROM diff_ops
                  WHERE diff_id = ?1
                )
                GROUP BY kind, sheet_id, move_id, change_kind, formula_diff, meta_field
                "#,
            )?;
            let mapped = stmt.query_map(params![diff_id], |row| {
                Ok(AggRow {
                    kind: row.get(0)?,
                    sheet_id: row.get::<_, Option<i64>>(1)?.map(|v| v.max(0) as u32),
                    change_kind: row.get(3)?,
                    formula_diff: row.get(4)?,
                    meta_field: row.get(5)?,
                    count: row.get::<_, i64>(6)? as u64,
                })
            })?;
            for row in mapped {
                rows.push(row?);
            }
        }

        fn bump_severity(counts: &mut SeverityCounts, severity: OpSeverity, n: u64) {
            match severity {
                OpSeverity::High => counts.high = counts.high.saturating_add(n),
                OpSeverity::Medium => counts.medium = counts.medium.saturating_add(n),
                OpSeverity::Low => counts.low = counts.low.saturating_add(n),
            }
        }

        fn bump_category(counts: &mut CategoryCounts, category: OpCategory, n: u64) {
            match category {
                OpCategory::Grid => counts.grid = counts.grid.saturating_add(n),
                OpCategory::PowerQuery => counts.power_query = counts.power_query.saturating_add(n),
                OpCategory::Model => counts.model = counts.model.saturating_add(n),
                OpCategory::Objects => counts.objects = counts.objects.saturating_add(n),
                OpCategory::Other => counts.other = counts.other.saturating_add(n),
            }
        }

        #[derive(Debug, Clone, Copy)]
        enum ChangeKind {
            Added,
            Removed,
            Modified,
            Moved,
        }

        fn classify_change_kind(kind: &str, meta_field: Option<&str>) -> Option<ChangeKind> {
            match kind {
                "SheetAdded" | "RowAdded" | "ColumnAdded" | "NamedRangeAdded" | "ChartAdded"
                | "VbaModuleAdded" | "QueryAdded" => Some(ChangeKind::Added),
                "SheetRemoved" | "RowRemoved" | "ColumnRemoved" | "NamedRangeRemoved"
                | "ChartRemoved" | "VbaModuleRemoved" | "QueryRemoved" => Some(ChangeKind::Removed),
                "BlockMovedRows" | "BlockMovedColumns" | "BlockMovedRect" => {
                    Some(ChangeKind::Moved)
                }
                "RowReplaced"
                | "DuplicateKeyCluster"
                | "RectReplaced"
                | "CellEdited"
                | "SheetRenamed"
                | "NamedRangeChanged"
                | "ChartChanged"
                | "VbaModuleChanged"
                | "QueryRenamed"
                | "QueryDefinitionChanged"
                | "QueryMetadataChanged" => {
                    if kind == "QueryMetadataChanged"
                        && meta_field.is_some_and(|field| field == "LoadToSheet")
                    {
                        Some(ChangeKind::Added)
                    } else {
                        Some(ChangeKind::Modified)
                    }
                }
                #[cfg(feature = "model-diff")]
                "TableAdded" | "ModelColumnAdded" | "RelationshipAdded" | "MeasureAdded" => {
                    Some(ChangeKind::Added)
                }
                #[cfg(feature = "model-diff")]
                "TableRemoved"
                | "ModelColumnRemoved"
                | "RelationshipRemoved"
                | "MeasureRemoved" => Some(ChangeKind::Removed),
                #[cfg(feature = "model-diff")]
                "ModelColumnTypeChanged"
                | "ModelColumnPropertyChanged"
                | "CalculatedColumnDefinitionChanged"
                | "RelationshipPropertyChanged"
                | "MeasureDefinitionChanged" => Some(ChangeKind::Modified),
                _ => None,
            }
        }

        fn classify_category(kind: &str) -> OpCategory {
            if kind.starts_with("Query") {
                return OpCategory::PowerQuery;
            }
            if kind == "CalculatedColumnDefinitionChanged"
                || kind.starts_with("Table")
                || kind.starts_with("ModelColumn")
                || kind.starts_with("Relationship")
                || kind.starts_with("Measure")
            {
                return OpCategory::Model;
            }
            if matches!(
                kind,
                "NamedRangeAdded"
                    | "NamedRangeRemoved"
                    | "NamedRangeChanged"
                    | "ChartAdded"
                    | "ChartRemoved"
                    | "ChartChanged"
                    | "VbaModuleAdded"
                    | "VbaModuleRemoved"
                    | "VbaModuleChanged"
            ) {
                return OpCategory::Objects;
            }
            if matches!(
                kind,
                "SheetAdded"
                    | "SheetRemoved"
                    | "SheetRenamed"
                    | "RowAdded"
                    | "RowRemoved"
                    | "RowReplaced"
                    | "DuplicateKeyCluster"
                    | "ColumnAdded"
                    | "ColumnRemoved"
                    | "BlockMovedRows"
                    | "BlockMovedColumns"
                    | "BlockMovedRect"
                    | "RectReplaced"
                    | "CellEdited"
            ) {
                return OpCategory::Grid;
            }
            OpCategory::Other
        }

        fn classify_severity(
            kind: &str,
            change_kind: Option<&str>,
            formula_diff: Option<&str>,
        ) -> OpSeverity {
            match kind {
                "DuplicateKeyCluster" => OpSeverity::High,
                "QueryAdded" | "QueryRemoved" => OpSeverity::High,
                "QueryDefinitionChanged" => match change_kind.unwrap_or("") {
                    "semantic" => OpSeverity::High,
                    "formatting_only" => OpSeverity::Low,
                    "renamed" => OpSeverity::Low,
                    _ => OpSeverity::Medium,
                },
                #[cfg(feature = "model-diff")]
                "MeasureAdded" | "MeasureRemoved" | "TableAdded" | "TableRemoved" => {
                    OpSeverity::High
                }
                #[cfg(feature = "model-diff")]
                "MeasureDefinitionChanged" | "CalculatedColumnDefinitionChanged" => {
                    match change_kind.unwrap_or("") {
                        "semantic" => OpSeverity::High,
                        "formatting_only" => OpSeverity::Low,
                        "unknown" => OpSeverity::Medium,
                        _ => OpSeverity::Medium,
                    }
                }
                "CellEdited" => match formula_diff.unwrap_or("") {
                    "semantic_change" => OpSeverity::High,
                    "formatting_only" => OpSeverity::Low,
                    "filled" => OpSeverity::Medium,
                    _ => OpSeverity::Medium,
                },
                "SheetRenamed" | "QueryRenamed" => OpSeverity::Low,
                "BlockMovedRows" | "BlockMovedColumns" | "BlockMovedRect" => OpSeverity::Medium,
                "SheetAdded" | "SheetRemoved" => OpSeverity::High,
                "RowAdded" | "RowRemoved" | "RowReplaced" | "ColumnAdded" | "ColumnRemoved"
                | "RectReplaced" => OpSeverity::Medium,
                "NamedRangeAdded" | "NamedRangeRemoved" | "NamedRangeChanged" | "ChartAdded"
                | "ChartRemoved" | "ChartChanged" | "VbaModuleAdded" | "VbaModuleRemoved"
                | "VbaModuleChanged" => OpSeverity::Medium,
                _ => OpSeverity::Medium,
            }
        }

        fn should_skip_row(
            kind: &str,
            change_kind: Option<&str>,
            formula_diff: Option<&str>,
            filters: NoiseFilters,
        ) -> bool {
            if filters.hide_m_formatting_only {
                if kind == "QueryDefinitionChanged" && change_kind == Some("formatting_only") {
                    return true;
                }
            }
            if filters.hide_dax_formatting_only {
                if matches!(
                    kind,
                    "MeasureDefinitionChanged" | "CalculatedColumnDefinitionChanged"
                ) && change_kind == Some("formatting_only")
                {
                    return true;
                }
            }
            if filters.hide_formula_formatting_only {
                if kind == "CellEdited" && formula_diff == Some("formatting_only") {
                    return true;
                }
            }
            false
        }

        let mut analysis = DiffAnalysis::default();
        let mut by_category: std::collections::HashMap<OpCategory, SeverityCounts> =
            std::collections::HashMap::new();
        let mut sheet_map: std::collections::HashMap<String, SheetBreakdown> =
            std::collections::HashMap::new();

        for row in rows {
            let kind = row.kind.as_str();
            let change_kind = row.change_kind.as_deref();
            let formula_diff = row.formula_diff.as_deref();
            let meta_field = row.meta_field.as_deref();

            if should_skip_row(kind, change_kind, formula_diff, filters) {
                continue;
            }

            let is_move = matches!(
                kind,
                "BlockMovedRows" | "BlockMovedColumns" | "BlockMovedRect"
            );
            let effective_count = if is_move && filters.collapse_moves {
                1
            } else {
                row.count
            };
            if effective_count == 0 {
                continue;
            }

            analysis.op_count = analysis.op_count.saturating_add(effective_count);
            if let Some(kind) = classify_change_kind(kind, meta_field) {
                match kind {
                    ChangeKind::Added => {
                        analysis.counts.added =
                            analysis.counts.added.saturating_add(effective_count)
                    }
                    ChangeKind::Removed => {
                        analysis.counts.removed =
                            analysis.counts.removed.saturating_add(effective_count)
                    }
                    ChangeKind::Modified => {
                        analysis.counts.modified =
                            analysis.counts.modified.saturating_add(effective_count)
                    }
                    ChangeKind::Moved => {
                        analysis.counts.moved =
                            analysis.counts.moved.saturating_add(effective_count)
                    }
                }
            }

            let category = classify_category(kind);
            bump_category(&mut analysis.categories, category, effective_count);
            let severity = classify_severity(kind, change_kind, formula_diff);
            bump_severity(&mut analysis.severity, severity, effective_count);
            bump_severity(
                by_category.entry(category).or_default(),
                severity,
                effective_count,
            );

            if let Some(sheet_id) = row.sheet_id {
                let sheet_name = sheet_name_by_id
                    .get(&sheet_id)
                    .cloned()
                    .unwrap_or_else(|| "<unknown>".to_string());
                let entry = sheet_map
                    .entry(sheet_name.clone())
                    .or_insert_with(|| SheetBreakdown {
                        sheet_name,
                        ..SheetBreakdown::default()
                    });
                entry.op_count = entry.op_count.saturating_add(effective_count);
                if let Some(kind) = classify_change_kind(kind, meta_field) {
                    match kind {
                        ChangeKind::Added => {
                            entry.counts.added = entry.counts.added.saturating_add(effective_count)
                        }
                        ChangeKind::Removed => {
                            entry.counts.removed =
                                entry.counts.removed.saturating_add(effective_count)
                        }
                        ChangeKind::Modified => {
                            entry.counts.modified =
                                entry.counts.modified.saturating_add(effective_count)
                        }
                        ChangeKind::Moved => {
                            entry.counts.moved = entry.counts.moved.saturating_add(effective_count)
                        }
                    }
                }
                bump_severity(&mut entry.severity, severity, effective_count);
            }
        }

        // Category breakdown rows.
        let mut breakdown: Vec<CategoryBreakdownRow> = Vec::new();
        for (category, severity) in by_category {
            let total = severity.high + severity.medium + severity.low;
            breakdown.push(CategoryBreakdownRow {
                category,
                total,
                severity,
            });
        }
        breakdown.sort_by_key(|row| match row.category {
            OpCategory::Grid => 0u8,
            OpCategory::PowerQuery => 1u8,
            OpCategory::Model => 2u8,
            OpCategory::Objects => 3u8,
            OpCategory::Other => 4u8,
        });
        analysis.category_breakdown = breakdown;

        // Sort sheets: high severity first, then op count, then alpha.
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

        Ok(analysis)
    }

    pub fn stream_ops<F>(&self, diff_id: &str, mut f: F) -> Result<(), StoreError>
    where
        F: FnMut(DiffOp) -> Result<(), StoreError>,
    {
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM diff_ops WHERE diff_id = ?1 ORDER BY op_idx")?;
        let rows = stmt.query_map(params![diff_id], |row| row.get::<_, String>(0))?;
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            f(op)?;
        }
        Ok(())
    }

    pub fn load_strings(&self, diff_id: &str) -> Result<Vec<String>, StoreError> {
        let json = self
            .conn
            .query_row(
                "SELECT strings_json FROM diff_runs WHERE diff_id = ?1",
                params![diff_id],
                |row| row.get::<_, Option<String>>(0),
            )?
            .ok_or_else(|| StoreError::MissingRun(diff_id.to_string()))?;
        let strings: Vec<String> = serde_json::from_str(&json)?;
        Ok(strings)
    }

    pub fn sheet_id_for_name(&self, diff_id: &str, sheet_name: &str) -> Result<u32, StoreError> {
        let id: Option<i64> = self.conn.query_row(
            "SELECT sheet_id FROM diff_sheets WHERE diff_id = ?1 AND sheet_name = ?2 COLLATE NOCASE",
            params![diff_id, sheet_name],
            |row| row.get(0),
        ).optional()?;
        match id {
            Some(value) => Ok(value as u32),
            None => Err(StoreError::MissingSheet(sheet_name.to_string())),
        }
    }

    fn load_warnings(&self, diff_id: &str) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT text FROM diff_warnings WHERE diff_id = ?1 ORDER BY idx")?;
        let rows = stmt.query_map(params![diff_id], |row| row.get::<_, String>(0))?;
        let mut warnings = Vec::new();
        for row in rows {
            warnings.push(row?);
        }
        Ok(warnings)
    }

    fn load_sheet_summaries(&self, diff_id: &str) -> Result<Vec<SheetSummary>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT sheet_id, sheet_name, op_count, added_count, removed_count, modified_count, moved_count \
             FROM diff_sheets WHERE diff_id = ?1 ORDER BY sheet_name",
        )?;
        let rows = stmt.query_map(params![diff_id], |row| {
            Ok(SheetSummary {
                sheet_id: row.get::<_, i64>(0)? as u32,
                sheet_name: row.get(1)?,
                op_count: row.get::<_, i64>(2)? as u64,
                counts: ChangeCounts {
                    added: row.get::<_, i64>(3)? as u64,
                    removed: row.get::<_, i64>(4)? as u64,
                    modified: row.get::<_, i64>(5)? as u64,
                    moved: row.get::<_, i64>(6)? as u64,
                },
            })
        })?;

        let mut sheets = Vec::new();
        for row in rows {
            sheets.push(row?);
        }
        Ok(sheets)
    }

    pub fn replace_pbip_docs(
        &self,
        diff_id: &str,
        docs: &[CorePbipDocDiff],
        profile: CorePbipProfile,
        profile_summary: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM diff_pbip_docs WHERE diff_id = ?1",
            params![diff_id],
        )?;

        for doc in docs {
            let applied = doc
                .new
                .as_ref()
                .and_then(|s| s.normalization_applied.as_deref())
                .or_else(|| doc.old.as_ref().and_then(|s| s.normalization_applied.as_deref()))
                .unwrap_or(profile_summary);

            let (old_hash, old_error, old_text) = doc
                .old
                .as_ref()
                .map(|s| (Some(s.hash as i64), s.error.clone(), Some(s.normalized_text.clone())))
                .unwrap_or((None, None, None));
            let (new_hash, new_error, new_text) = doc
                .new
                .as_ref()
                .map(|s| (Some(s.hash as i64), s.error.clone(), Some(s.normalized_text.clone())))
                .unwrap_or((None, None, None));

            self.conn.execute(
                "INSERT INTO diff_pbip_docs (diff_id, path, doc_type, change_kind, impact_hint, normalization_profile, normalization_applied,\
                 old_hash, new_hash, old_error, new_error, old_text, new_text) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    diff_id,
                    doc.path,
                    doc.doc_type.as_str(),
                    doc.change_kind.as_str(),
                    doc.impact_hint,
                    profile.as_str(),
                    applied,
                    old_hash,
                    new_hash,
                    old_error,
                    new_error,
                    old_text,
                    new_text,
                ],
            )?;
        }

        Ok(())
    }

    pub fn replace_pbip_entities(
        &self,
        diff_id: &str,
        entities: &[CorePbipEntityDiff],
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM diff_pbip_entities WHERE diff_id = ?1",
            params![diff_id],
        )?;

        for entity in entities {
            let kind = format!("{:?}", entity.entity_kind).to_ascii_lowercase();
            let ptr = entity.pointer.clone();
            let entity_id = format!(
                "doc:{}|kind:{}|ptr:{}|label:{}",
                entity.doc_path,
                kind,
                ptr.as_deref().unwrap_or(""),
                entity.label
            );

            let old_text = entity.old_text.clone();
            let new_text = entity.new_text.clone();

            self.conn.execute(
                "INSERT INTO diff_pbip_entities (diff_id, entity_id, doc_path, entity_kind, label, change_kind, pointer, impact_hint, old_hash, new_hash, old_text, new_text) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    diff_id,
                    entity_id,
                    entity.doc_path,
                    kind,
                    entity.label,
                    entity.change_kind.as_str(),
                    ptr,
                    Option::<String>::None,
                    Option::<i64>::None,
                    Option::<i64>::None,
                    old_text,
                    new_text,
                ],
            )?;
        }

        Ok(())
    }

    pub fn load_pbip_navigator(&self, diff_id: &str) -> Result<NavigatorModel, StoreError> {
        let mut model = NavigatorModel {
            columns: vec![
                "Path".to_string(),
                "Type".to_string(),
                "Change".to_string(),
                "Impact".to_string(),
            ],
            rows: Vec::new(),
        };

        let mut stmt = self.conn.prepare(
            "SELECT path, doc_type, change_kind, impact_hint, old_error, new_error \
             FROM diff_pbip_docs WHERE diff_id = ?1 \
             ORDER BY CASE change_kind WHEN 'modified' THEN 0 WHEN 'added' THEN 1 WHEN 'removed' THEN 2 ELSE 3 END, path",
        )?;
        let rows = stmt.query_map(params![diff_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;

        for row in rows {
            let (path, doc_type, change_kind, impact, old_error, new_error) = row?;
            let doc_path = path.clone();
            let has_error = old_error.as_deref().map(str::trim).filter(|v| !v.is_empty()).is_some()
                || new_error.as_deref().map(str::trim).filter(|v| !v.is_empty()).is_some();
            let impact_display = if has_error {
                match impact.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                    Some(hint) => format!("Error | {hint}"),
                    None => "Error".to_string(),
                }
            } else {
                impact.unwrap_or_default()
            };
            model.rows.push(NavigatorRow {
                target: SelectionTarget {
                    domain: DiffDomain::PbipProject,
                    kind: SelectionKind::Document,
                    id: None,
                    path: Some(doc_path.clone()),
                    pointer: None,
                    label: None,
                },
                cells: vec![
                    path,
                    doc_type.to_ascii_uppercase(),
                    change_kind,
                    impact_display,
                ],
            });

            // Append entities for this document (if any).
            let mut stmt = self.conn.prepare(
                "SELECT entity_id, entity_kind, label, change_kind, pointer \
                 FROM diff_pbip_entities WHERE diff_id = ?1 AND doc_path = ?2 \
                 ORDER BY entity_kind, label",
            )?;
            let rows = stmt.query_map(params![diff_id, doc_path], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?;
            for row in rows {
                let (entity_id, kind, label, change_kind, pointer) = row?;
                let display = format!("  {}: {}", kind, label);
                model.rows.push(NavigatorRow {
                    target: SelectionTarget {
                        domain: DiffDomain::PbipProject,
                        kind: SelectionKind::Entity,
                        id: Some(entity_id),
                        path: Some(doc_path.clone()),
                        pointer,
                        label: Some(label),
                    },
                    cells: vec![display, kind, change_kind, String::new()],
                });
            }
        }

        Ok(model)
    }

    pub fn load_pbip_details(
        &self,
        diff_id: &str,
        target: &SelectionTarget,
    ) -> Result<DetailsPayload, StoreError> {
        if target.domain != DiffDomain::PbipProject {
            return Err(StoreError::InvalidData(
                "load_pbip_details called for non-PBIP domain".to_string(),
            ));
        }

        match target.kind {
            SelectionKind::Document => {
                let Some(path) = target.path.as_deref() else {
                    return Err(StoreError::InvalidData("Missing document path".to_string()));
                };
                let row = self
                    .conn
                    .query_row(
                        "SELECT doc_type, change_kind, normalization_applied, old_error, new_error, old_text, new_text \
                         FROM diff_pbip_docs WHERE diff_id = ?1 AND path = ?2",
                        params![diff_id, path],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, Option<String>>(3)?,
                                row.get::<_, Option<String>>(4)?,
                                row.get::<_, Option<String>>(5)?,
                                row.get::<_, Option<String>>(6)?,
                            ))
                        },
                    )
                    .optional()?;
                let Some((doc_type, change_kind, applied, old_error, new_error, old_text, new_text)) = row else {
                    return Err(StoreError::InvalidData(format!(
                        "Missing PBIP doc details for {path}"
                    )));
                };
                let language = match doc_type.as_str() {
                    "pbir" => "json",
                    "tmdl" => "tmdl",
                    _ => "text",
                }
                .to_string();
                let mut header = format!("Document: {path} ({doc_type}, {change_kind})");
                if let Some(err) = old_error.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                    header.push_str(&format!("\nOld error: {err}"));
                }
                if let Some(err) = new_error.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                    header.push_str(&format!("\nNew error: {err}"));
                }
                Ok(DetailsPayload {
                    target: target.clone(),
                    language,
                    header,
                    old_text,
                    new_text,
                    normalization_applied: Some(applied),
                })
            }
            SelectionKind::Entity => {
                let Some(entity_id) = target.id.as_deref() else {
                    return Err(StoreError::InvalidData("Missing entity id".to_string()));
                };
                let row = self
                    .conn
                    .query_row(
                        "SELECT entity_kind, label, change_kind, old_text, new_text \
                         FROM diff_pbip_entities WHERE diff_id = ?1 AND entity_id = ?2",
                        params![diff_id, entity_id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, Option<String>>(3)?,
                                row.get::<_, Option<String>>(4)?,
                            ))
                        },
                    )
                    .optional()?;
                let Some((kind, label, change_kind, old_text, new_text)) = row else {
                    return Err(StoreError::InvalidData(format!(
                        "Missing PBIP entity details for {entity_id}"
                    )));
                };
                Ok(DetailsPayload {
                    target: target.clone(),
                    language: "json".to_string(),
                    header: format!("{kind}: {label} ({change_kind})"),
                    old_text,
                    new_text,
                    normalization_applied: None,
                })
            }
            _ => Err(StoreError::InvalidData(format!(
                "Unsupported SelectionKind for PBIP: {:?}",
                target.kind
            ))),
        }
    }

    fn apply_schema(conn: &Connection) -> Result<(), StoreError> {
        let schema = include_str!("schema.sql");
        conn.execute_batch(schema)?;
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
        Ok(())
    }
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "".to_string())
}

impl DiffMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffMode::Payload => "payload",
            DiffMode::Large => "large",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "large" => DiffMode::Large,
            _ => DiffMode::Payload,
        }
    }
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Complete => "complete",
            RunStatus::Failed => "failed",
            RunStatus::Canceled => "canceled",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "complete" => RunStatus::Complete,
            "failed" => RunStatus::Failed,
            "canceled" => RunStatus::Canceled,
            _ => RunStatus::Running,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SheetStatsResolved {
    pub sheet_id: u32,
    pub sheet_name: String,
    pub counts: ChangeCounts,
    pub op_count: u64,
}

pub fn resolve_sheet_stats(
    strings: &[String],
    stats: &[SheetStats],
) -> Result<Vec<SheetStatsResolved>, StoreError> {
    let mut resolved = Vec::with_capacity(stats.len());
    for stat in stats {
        let name = strings
            .get(stat.sheet_id as usize)
            .cloned()
            .ok_or_else(|| StoreError::InvalidData("sheet id out of range".to_string()))?;
        resolved.push(SheetStatsResolved {
            sheet_id: stat.sheet_id,
            sheet_name: name,
            counts: stat.counts,
            op_count: stat.op_count,
        });
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_diff::{
        DiffOp, DiffReport, DiffSummary, ExpressionChangeKind, FormulaDiffResult, QueryChangeKind,
        StringId,
    };

    fn run_analysis(ops: Vec<DiffOp>, strings: Vec<String>, filters: NoiseFilters) -> DiffAnalysis {
        let store = OpStore::open_in_memory().expect("store");
        let diff_id = store
            .start_run(
                "old.xlsx",
                "new.xlsx",
                "{}",
                "engine",
                "app",
                DiffMode::Large,
                false,
            )
            .expect("start_run");

        let mut report = DiffReport::new(ops.clone());
        report.strings = strings.clone();

        let (counts, sheet_stats) = store
            .insert_ops_from_report(&diff_id, &report)
            .expect("insert ops");

        let summary = DiffSummary {
            complete: true,
            warnings: Vec::new(),
            op_count: ops.len(),
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        };

        let resolved = resolve_sheet_stats(&strings, &sheet_stats).expect("resolve sheets");
        store
            .finish_run(
                &diff_id,
                &summary,
                &strings,
                &counts,
                &resolved,
                RunStatus::Complete,
            )
            .expect("finish_run");

        store
            .load_diff_analysis(&diff_id, filters)
            .expect("load_diff_analysis")
    }

    #[test]
    fn analysis_filters_formatting_only_power_query_changes() {
        let ops = vec![
            DiffOp::QueryDefinitionChanged {
                name: StringId(1),
                change_kind: QueryChangeKind::FormattingOnly,
                old_hash: 1,
                new_hash: 2,
                semantic_detail: None,
            },
            DiffOp::QueryDefinitionChanged {
                name: StringId(2),
                change_kind: QueryChangeKind::Semantic,
                old_hash: 3,
                new_hash: 4,
                semantic_detail: None,
            },
        ];
        let analysis = run_analysis(
            ops,
            vec!["Sheet1".to_string()],
            NoiseFilters {
                hide_m_formatting_only: true,
                ..NoiseFilters::default()
            },
        );
        assert_eq!(analysis.op_count, 1);
        assert_eq!(analysis.categories.power_query, 1);
        assert_eq!(analysis.severity.high, 1);
    }

    #[test]
    fn analysis_filters_formatting_only_dax_changes() {
        let ops = vec![
            DiffOp::MeasureDefinitionChanged {
                name: StringId(1),
                change_kind: ExpressionChangeKind::FormattingOnly,
                old_hash: 1,
                new_hash: 2,
            },
            DiffOp::MeasureDefinitionChanged {
                name: StringId(2),
                change_kind: ExpressionChangeKind::Semantic,
                old_hash: 3,
                new_hash: 4,
            },
        ];
        let analysis = run_analysis(
            ops,
            vec!["Sheet1".to_string()],
            NoiseFilters {
                hide_dax_formatting_only: true,
                ..NoiseFilters::default()
            },
        );
        assert_eq!(analysis.op_count, 1);
        assert_eq!(analysis.categories.model, 1);
        assert_eq!(analysis.severity.high, 1);
    }

    #[test]
    fn analysis_filters_formatting_only_formula_cell_edits() {
        let sheet = StringId(0);
        let addr = excel_diff::CellAddress::from_indices(0, 0);
        let from = excel_diff::CellSnapshot::empty(addr);
        let to = excel_diff::CellSnapshot::empty(addr);
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
        let analysis = run_analysis(
            ops,
            vec!["Sheet1".to_string()],
            NoiseFilters {
                hide_formula_formatting_only: true,
                ..NoiseFilters::default()
            },
        );
        assert_eq!(analysis.op_count, 1);
        assert_eq!(analysis.categories.grid, 1);
        assert_eq!(analysis.severity.high, 1);
        assert_eq!(analysis.sheets.len(), 1);
        assert_eq!(analysis.sheets[0].sheet_name, "Sheet1");
        assert_eq!(analysis.sheets[0].op_count, 1);
    }

    #[test]
    fn analysis_collapse_moves_counts_distinct_move_ids() {
        let sheet = StringId(0);
        let ops = vec![
            DiffOp::BlockMovedRows {
                sheet,
                src_start_row: 10,
                row_count: 3,
                dst_start_row: 20,
                block_hash: None,
            },
            // Duplicate move (same computed move_id).
            DiffOp::BlockMovedRows {
                sheet,
                src_start_row: 10,
                row_count: 3,
                dst_start_row: 20,
                block_hash: None,
            },
        ];
        let raw = run_analysis(
            ops.clone(),
            vec!["Sheet1".to_string()],
            NoiseFilters::default(),
        );
        assert_eq!(raw.op_count, 2);
        assert_eq!(raw.counts.moved, 2);

        let collapsed = run_analysis(
            ops,
            vec!["Sheet1".to_string()],
            NoiseFilters {
                collapse_moves: true,
                ..NoiseFilters::default()
            },
        );
        assert_eq!(collapsed.op_count, 1);
        assert_eq!(collapsed.counts.moved, 1);
        assert_eq!(collapsed.severity.medium, 1);
    }
}
