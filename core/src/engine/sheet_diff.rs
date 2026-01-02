use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffReport, DiffSummary};
use crate::progress::ProgressCallback;
use crate::sink::{DiffSink, SinkFinishGuard, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::Sheet;

use crate::diff::SheetId;
use super::context::DiffContext;
use super::grid_diff::try_diff_grids_internal;
use super::hardening::HardeningController;

pub fn diff_sheets(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut sink = VecSink::new();
    match try_diff_sheets_streaming(old, new, pool, config, &mut sink) {
        Ok(summary) => {
            let strings = pool.strings().to_vec();
            DiffReport::from_ops_and_summary(sink.into_ops(), summary, strings)
        }
        Err(e) => {
            let strings = pool.strings().to_vec();
            DiffReport {
                version: DiffReport::SCHEMA_VERSION.to_string(),
                strings,
                ops: sink.into_ops(),
                complete: false,
                warnings: vec![e.to_string()],
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            }
        }
    }
}

pub fn try_diff_sheets(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    let summary = try_diff_sheets_streaming(old, new, pool, config, &mut sink)?;
    let strings = pool.strings().to_vec();
    Ok(DiffReport::from_ops_and_summary(
        sink.into_ops(),
        summary,
        strings,
    ))
}

/// Stream a sheet diff into `sink`.
///
/// Streaming output follows the contract in `docs/streaming_contract.md`.
pub fn diff_sheets_streaming<S: DiffSink>(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> DiffSummary {
    match try_diff_sheets_streaming(old, new, pool, config, sink) {
        Ok(summary) => summary,
        Err(e) => DiffSummary {
            complete: false,
            warnings: vec![e.to_string()],
            op_count: 0,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        },
    }
}

pub fn diff_sheets_streaming_with_progress<S: DiffSink>(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: &dyn ProgressCallback,
) -> DiffSummary {
    match try_diff_sheets_streaming_with_progress(old, new, pool, config, sink, progress) {
        Ok(summary) => summary,
        Err(e) => DiffSummary {
            complete: false,
            warnings: vec![e.to_string()],
            op_count: 0,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        },
    }
}

/// Like [`diff_sheets_streaming`], but returns errors instead of embedding them in the summary.
pub fn try_diff_sheets_streaming<S: DiffSink>(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    let mut op_count = 0usize;
    try_diff_sheets_streaming_with_op_count(old, new, pool, config, sink, &mut op_count, None)
}

pub fn try_diff_sheets_streaming_with_progress<S: DiffSink>(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: &dyn ProgressCallback,
) -> Result<DiffSummary, DiffError> {
    let mut op_count = 0usize;
    try_diff_sheets_streaming_with_op_count(
        old,
        new,
        pool,
        config,
        sink,
        &mut op_count,
        Some(progress),
    )
}

#[allow(clippy::too_many_arguments)]
fn try_diff_sheets_streaming_with_op_count<'p, S: DiffSink>(
    old: &Sheet,
    new: &Sheet,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    op_count: &mut usize,
    progress: Option<&'p dyn ProgressCallback>,
) -> Result<DiffSummary, DiffError> {
    let sheet_id: SheetId = old.name;

    sink.begin(pool)?;
    let mut finish_guard = SinkFinishGuard::new(sink);

    let mut ctx = DiffContext::default();
    let mut hardening = HardeningController::new(config, progress);

    if hardening.check_timeout(&mut ctx.warnings) {
        finish_guard.finish_and_disarm()?;
        return Ok(DiffSummary {
            complete: false,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    }

    try_diff_grids_internal(
        sheet_id,
        &old.grid,
        &new.grid,
        config,
        pool,
        sink,
        op_count,
        &mut ctx,
        &mut hardening,
        #[cfg(feature = "perf-metrics")]
        None,
    )?;

    finish_guard.finish_and_disarm()?;
    let complete = ctx.warnings.is_empty();
    Ok(DiffSummary {
        complete,
        warnings: ctx.warnings,
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
}
