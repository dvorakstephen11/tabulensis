use std::path::Path;

use excel_diff::{DiffOp, DiffReport, DiffSummary};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::types::{ChangeCounts, OpIndexFields, SheetStats, accumulate_sheet_stats, op_index_fields};

const SCHEMA_VERSION: i64 = 1;

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

        self.conn.execute("DELETE FROM diff_warnings WHERE diff_id = ?1", params![diff_id])?;
        for (idx, warning) in summary.warnings.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO diff_warnings (diff_id, idx, text) VALUES (?1, ?2, ?3)",
                params![diff_id, idx as i64, warning],
            )?;
        }

        self.conn.execute("DELETE FROM diff_sheets WHERE diff_id = ?1", params![diff_id])?;
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

    pub fn fail_run(&self, diff_id: &str, status: RunStatus, message: &str) -> Result<(), StoreError> {
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
        let mut stats_map: std::collections::HashMap<u32, SheetStats> = std::collections::HashMap::new();

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

        let Some((old_path, new_path, started_at, finished_at, engine_version, app_version, mode, status, trusted, complete, op_count, added, removed, modified, moved)) = row else {
            return Err(StoreError::MissingRun(diff_id.to_string()));
        };

        let warnings = self.load_warnings(diff_id)?;
        let sheets = self.load_sheet_summaries(diff_id)?;

        Ok(DiffRunSummary {
            diff_id: diff_id.to_string(),
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

    pub fn load_sheet_ops(&self, diff_id: &str, sheet_name: &str) -> Result<Vec<DiffOp>, StoreError> {
        let sheet_id = self.sheet_id_for_name(diff_id, sheet_name)?;
        let mut stmt = self.conn.prepare(
            "SELECT payload_json FROM diff_ops WHERE diff_id = ?1 AND sheet_id = ?2 ORDER BY op_idx",
        )?;
        let rows = stmt.query_map(params![diff_id, sheet_id as i64], |row| row.get::<_, String>(0))?;
        let mut ops = Vec::new();
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            ops.push(op);
        }
        Ok(ops)
    }

    pub fn load_ops(&self, diff_id: &str) -> Result<Vec<DiffOp>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT payload_json FROM diff_ops WHERE diff_id = ?1 ORDER BY op_idx",
        )?;
        let rows = stmt.query_map(params![diff_id], |row| row.get::<_, String>(0))?;
        let mut ops = Vec::new();
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            ops.push(op);
        }
        Ok(ops)
    }

    pub fn stream_ops<F>(&self, diff_id: &str, mut f: F) -> Result<(), StoreError>
    where
        F: FnMut(DiffOp) -> Result<(), StoreError>,
    {
        let mut stmt = self.conn.prepare(
            "SELECT payload_json FROM diff_ops WHERE diff_id = ?1 ORDER BY op_idx",
        )?;
        let rows = stmt.query_map(params![diff_id], |row| row.get::<_, String>(0))?;
        for row in rows {
            let payload = row?;
            let op: DiffOp = serde_json::from_str(&payload)?;
            f(op)?;
        }
        Ok(())
    }

    pub fn load_strings(&self, diff_id: &str) -> Result<Vec<String>, StoreError> {
        let json = self.conn.query_row(
            "SELECT strings_json FROM diff_runs WHERE diff_id = ?1",
            params![diff_id],
            |row| row.get::<_, Option<String>>(0),
        )?.ok_or_else(|| StoreError::MissingRun(diff_id.to_string()))?;
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
        let mut stmt = self.conn.prepare(
            "SELECT text FROM diff_warnings WHERE diff_id = ?1 ORDER BY idx",
        )?;
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
