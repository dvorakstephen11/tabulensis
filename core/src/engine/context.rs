use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffOp};
use crate::formula_diff::FormulaParseCache;
use crate::sink::DiffSink;
use crate::string_pool::StringPool;

use super::SheetId;

#[derive(Debug, Default)]
pub(crate) struct DiffContext {
    pub(crate) warnings: Vec<String>,
    pub(crate) formula_cache: FormulaParseCache,
}

pub(crate) fn emit_op<S: DiffSink>(
    sink: &mut S,
    op_count: &mut usize,
    op: DiffOp,
) -> Result<(), DiffError> {
    sink.emit(op)?;
    *op_count = op_count.saturating_add(1);
    Ok(())
}

pub(crate) struct EmitCtx<'a, S: DiffSink> {
    pub(crate) sheet_id: &'a SheetId,
    pub(crate) pool: &'a StringPool,
    pub(crate) config: &'a DiffConfig,
    pub(crate) cache: &'a mut FormulaParseCache,
    pub(crate) sink: &'a mut S,
    pub(crate) op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    pub(crate) fn new(
        sheet_id: &'a SheetId,
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

    pub(crate) fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}

