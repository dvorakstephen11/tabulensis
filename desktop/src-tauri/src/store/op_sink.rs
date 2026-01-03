use std::collections::HashMap;

use excel_diff::{DiffError, DiffOp, DiffSink};
use rusqlite::{params, Connection};

use super::types::{ChangeCounts, OpIndexFields, accumulate_sheet_stats, classify_op, op_index_fields, SheetStats};

pub struct OpStoreSink {
    conn: Connection,
    diff_id: String,
    op_idx: u64,
    counts: ChangeCounts,
    sheet_stats: HashMap<u32, SheetStats>,
    committed: bool,
}

impl OpStoreSink {
    pub fn new(conn: Connection, diff_id: String) -> Result<Self, DiffError> {
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        Ok(Self {
            conn,
            diff_id,
            op_idx: 0,
            counts: ChangeCounts::default(),
            sheet_stats: HashMap::new(),
            committed: false,
        })
    }

    pub fn into_parts(self) -> (Connection, ChangeCounts, HashMap<u32, SheetStats>, u64) {
        (self.conn, self.counts, self.sheet_stats, self.op_idx)
    }

    fn insert_op(&mut self, fields: OpIndexFields, payload_json: &str) -> Result<(), DiffError> {
        self.conn
            .execute(
                "INSERT INTO diff_ops (diff_id, op_idx, kind, sheet_id, row, col, row_end, col_end, move_id, payload_json)\
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    self.diff_id,
                    self.op_idx as i64,
                    fields.kind,
                    fields.sheet_id.map(|v| v as i64),
                    fields.row.map(|v| v as i64),
                    fields.col.map(|v| v as i64),
                    fields.row_end.map(|v| v as i64),
                    fields.col_end.map(|v| v as i64),
                    fields.move_id,
                    payload_json,
                ],
            )
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        Ok(())
    }
}

impl DiffSink for OpStoreSink {
    fn begin(&mut self, _pool: &excel_diff::StringPool) -> Result<(), DiffError> {
        Ok(())
    }

    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        if let Some(kind) = classify_op(&op) {
            self.counts.apply(kind);
        }
        accumulate_sheet_stats(&mut self.sheet_stats, &op);

        let fields = op_index_fields(&op);
        let payload_json = serde_json::to_string(&op)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.insert_op(fields, &payload_json)?;

        self.op_idx = self.op_idx.saturating_add(1);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        if self.committed {
            return Ok(());
        }
        self.conn
            .execute_batch("COMMIT")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.committed = true;
        Ok(())
    }
}
