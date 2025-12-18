use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffOp};
use crate::formula_diff::FormulaParseCache;
use crate::sink::DiffSink;
use crate::string_pool::StringPool;

use super::SheetId;

#[derive(Debug, Default)]
pub(super) struct DiffContext {
    pub(super) warnings: Vec<String>,
    pub(super) formula_cache: FormulaParseCache,
}

pub(super) fn emit_op<S: DiffSink>(
    sink: &mut S,
    op_count: &mut usize,
    op: DiffOp,
) -> Result<(), DiffError> {
    sink.emit(op)?;
    *op_count = op_count.saturating_add(1);
    Ok(())
}

pub(super) struct EmitCtx<'a, S: DiffSink> {
    pub(super) sheet_id: SheetId,
    pub(super) pool: &'a StringPool,
    pub(super) config: &'a DiffConfig,
    pub(super) cache: &'a mut FormulaParseCache,
    pub(super) sink: &'a mut S,
    pub(super) op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    pub(super) fn new(
        sheet_id: SheetId,
        pool: &'a StringPool,
        config: &'a DiffConfig,
        cache: &'a mut FormulaParseCache,
        sink: &'a mut S,
        op_count: &'a mut usize,
    ) -> Self {
        Self {
            sheet_id,
            pool,
            config,
            cache,
            sink,
            op_count,
        }
    }

    pub(super) fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}
