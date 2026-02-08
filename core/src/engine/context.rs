use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffOp};
use crate::formula_diff::FormulaParseCache;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::sink::DiffSink;
use crate::string_pool::StringPool;

use super::hardening::HardeningController;
use crate::diff::SheetId;

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

pub(super) struct EmitCtx<'a, 'p, S: DiffSink> {
    pub(super) sheet_id: SheetId,
    pub(super) pool: &'a StringPool,
    pub(super) config: &'a DiffConfig,
    pub(super) cache: &'a mut FormulaParseCache,
    pub(super) sink: &'a mut S,
    pub(super) op_count: &'a mut usize,
    pub(super) warnings: &'a mut Vec<String>,
    pub(super) hardening: &'a mut HardeningController<'p>,
    #[cfg(feature = "perf-metrics")]
    pub(super) metrics: Option<&'a mut DiffMetrics>,
}

impl<'a, 'p, S: DiffSink> EmitCtx<'a, 'p, S> {
    pub(super) fn new(
        sheet_id: SheetId,
        pool: &'a StringPool,
        config: &'a DiffConfig,
        cache: &'a mut FormulaParseCache,
        sink: &'a mut S,
        op_count: &'a mut usize,
        warnings: &'a mut Vec<String>,
        hardening: &'a mut HardeningController<'p>,
        #[cfg(feature = "perf-metrics")] metrics: Option<&'a mut DiffMetrics>,
    ) -> Self {
        Self {
            sheet_id,
            pool,
            config,
            cache,
            sink,
            op_count,
            warnings,
            hardening,
            #[cfg(feature = "perf-metrics")]
            metrics,
        }
    }

    pub(super) fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        if self.hardening.check_op_limit(*self.op_count, self.warnings) {
            return Ok(());
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = self.metrics.as_deref_mut() {
            let _guard = m.phase_guard(Phase::OpEmit);
            return emit_op(self.sink, self.op_count, op);
        }
        emit_op(self.sink, self.op_count, op)
    }
}
