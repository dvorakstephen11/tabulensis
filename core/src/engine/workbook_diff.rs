use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary};
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::sink::{DiffSink, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{Sheet, SheetKind, Workbook};
use crate::progress::ProgressCallback;

use std::collections::HashMap;
#[cfg(feature = "perf-metrics")]
use std::mem::size_of;

use super::context::DiffContext;
use super::grid_diff::try_diff_grids;
use super::hardening::HardeningController;
use super::{SheetId, emit_op};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

fn make_sheet_key(sheet: &Sheet, pool: &StringPool) -> SheetKey {
    SheetKey {
        name_lower: pool.resolve(sheet.name).to_lowercase(),
        kind: sheet.kind.clone(),
    }
}

fn sheet_kind_order(kind: &SheetKind) -> u8 {
    match kind {
        SheetKind::Worksheet => 0,
        SheetKind::Chart => 1,
        SheetKind::Macro => 2,
        SheetKind::Other => 3,
    }
}

pub fn diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    match try_diff_workbooks(old, new, pool, config) {
        Ok(report) => report,
        Err(e) => {
            let strings = pool.strings().to_vec();
            DiffReport {
                version: DiffReport::SCHEMA_VERSION.to_string(),
                strings,
                ops: Vec::new(),
                complete: false,
                warnings: vec![e.to_string()],
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            }
        }
    }
}

pub fn diff_workbooks_with_progress(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    progress: &dyn ProgressCallback,
) -> DiffReport {
    match try_diff_workbooks_with_progress(old, new, pool, config, progress) {
        Ok(report) => report,
        Err(e) => {
            let strings = pool.strings().to_vec();
            DiffReport {
                version: DiffReport::SCHEMA_VERSION.to_string(),
                strings,
                ops: Vec::new(),
                complete: false,
                warnings: vec![e.to_string()],
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            }
        }
    }
}

pub fn diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> DiffSummary {
    match try_diff_workbooks_streaming(old, new, pool, config, sink) {
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

pub fn diff_workbooks_streaming_with_progress<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: &dyn ProgressCallback,
) -> DiffSummary {
    match try_diff_workbooks_streaming_with_progress(old, new, pool, config, sink, progress) {
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

pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    #[allow(unused_mut)]
    let mut summary = try_diff_workbooks_streaming(old, new, pool, config, &mut sink)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(metrics) = summary.metrics.as_mut() {
        metrics.op_buffer_bytes = estimate_op_buffer_bytes(&sink);
    }
    let strings = pool.strings().to_vec();
    Ok(DiffReport::from_ops_and_summary(
        sink.into_ops(),
        summary,
        strings,
    ))
}

pub fn try_diff_workbooks_with_progress(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    progress: &dyn ProgressCallback,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    #[allow(unused_mut)]
    let mut summary =
        try_diff_workbooks_streaming_with_progress(old, new, pool, config, &mut sink, progress)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(metrics) = summary.metrics.as_mut() {
        metrics.op_buffer_bytes = estimate_op_buffer_bytes(&sink);
    }
    let strings = pool.strings().to_vec();
    Ok(DiffReport::from_ops_and_summary(
        sink.into_ops(),
        summary,
        strings,
    ))
}

pub fn try_diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    try_diff_workbooks_streaming_impl(old, new, pool, config, sink, None)
}

pub fn try_diff_workbooks_streaming_with_progress<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: &dyn ProgressCallback,
) -> Result<DiffSummary, DiffError> {
    try_diff_workbooks_streaming_impl(old, new, pool, config, sink, Some(progress))
}

fn try_diff_workbooks_streaming_impl<'p, S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: Option<&'p dyn ProgressCallback>,
) -> Result<DiffSummary, DiffError> {
    let mut hardening = HardeningController::new(config, progress);
    #[cfg(feature = "perf-metrics")]
    let mut metrics = {
        let mut m = DiffMetrics::default();
        m.start_phase(Phase::Total);
        m.start_phase(Phase::Parse);
        m
    };
    hardening.progress("parse", 0.0);

    sink.begin(pool)?;

    let mut ctx = DiffContext::default();
    let mut op_count = 0usize;

        if hardening.check_timeout(&mut ctx.warnings) {
            #[cfg(feature = "perf-metrics")]
            {
                metrics.end_phase(Phase::Parse);
                metrics.end_phase(Phase::Total);
                apply_accounted_peak(&mut metrics, old, new, pool);
            }
            sink.finish()?;
            return Ok(DiffSummary {
                complete: false,
            warnings: ctx.warnings,
            op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: Some(metrics),
        });
    }

    let mut old_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let key = make_sheet_key(sheet, pool);
        if let Some(previous) = old_sheets.insert(key.clone(), sheet) {
            ctx.warnings.push(format!(
                "duplicate sheet identity in old workbook: '{}' ({:?}); \
                 later definition '{}' overwrites earlier one '{}'. The file may be corrupt.",
                key.name_lower,
                key.kind,
                pool.resolve(sheet.name),
                pool.resolve(previous.name)
            ));
        }
    }

    let mut new_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let key = make_sheet_key(sheet, pool);
        if let Some(previous) = new_sheets.insert(key.clone(), sheet) {
            ctx.warnings.push(format!(
                "duplicate sheet identity in new workbook: '{}' ({:?}); \
                 later definition '{}' overwrites earlier one '{}'. The file may be corrupt.",
                key.name_lower,
                key.kind,
                pool.resolve(sheet.name),
                pool.resolve(previous.name)
            ));
        }
    }

    let mut all_keys: Vec<SheetKey> = old_sheets
        .keys()
        .chain(new_sheets.keys())
        .cloned()
        .collect();
    all_keys.sort_by(|a, b| match a.name_lower.cmp(&b.name_lower) {
        std::cmp::Ordering::Equal => sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind)),
        other => other,
    });
    all_keys.dedup();

    hardening.progress("parse", 1.0);
    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Parse);
    }

    for key in all_keys {
        if hardening.check_timeout(&mut ctx.warnings) {
            break;
        }

        match (old_sheets.get(&key), new_sheets.get(&key)) {
            (None, Some(new_sheet)) => {
                emit_op(
                    sink,
                    &mut op_count,
                    DiffOp::SheetAdded {
                        sheet: new_sheet.name,
                    },
                )?;
            }
            (Some(old_sheet), None) => {
                emit_op(
                    sink,
                    &mut op_count,
                    DiffOp::SheetRemoved {
                        sheet: old_sheet.name,
                    },
                )?;
            }
            (Some(old_sheet), Some(new_sheet)) => {
                let sheet_id: SheetId = old_sheet.name;
                try_diff_grids(
                    sheet_id,
                    &old_sheet.grid,
                    &new_sheet.grid,
                    config,
                    pool,
                    sink,
                    &mut op_count,
                    &mut ctx,
                    &mut hardening,
                    #[cfg(feature = "perf-metrics")]
                    Some(&mut metrics),
                )?;
                if hardening.should_abort() {
                    break;
                }
            }
            (None, None) => {
                debug_assert!(false, "sheet key in all_keys but not in either map");
                continue;
            }
        }
    }

    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Total);
        apply_accounted_peak(&mut metrics, old, new, pool);
    }
    sink.finish()?;
    let complete = ctx.warnings.is_empty();
    Ok(DiffSummary {
        complete,
        warnings: ctx.warnings,
        op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: Some(metrics),
    })
}

#[cfg(feature = "perf-metrics")]
fn estimate_workbook_bytes(workbook: &Workbook) -> u64 {
    let sheet_bytes: u64 = workbook
        .sheets
        .iter()
        .map(|sheet| sheet.grid.estimated_bytes())
        .sum();
    let named_ranges = workbook.named_ranges.len() as u64 * size_of::<crate::workbook::NamedRange>() as u64;
    let charts = workbook.charts.len() as u64 * size_of::<crate::workbook::ChartObject>() as u64;
    sheet_bytes.saturating_add(named_ranges).saturating_add(charts)
}

#[cfg(feature = "perf-metrics")]
fn estimate_grid_storage_bytes(workbook: &Workbook) -> u64 {
    workbook
        .sheets
        .iter()
        .map(|sheet| sheet.grid.estimated_bytes())
        .sum()
}

#[cfg(feature = "perf-metrics")]
fn estimate_alignment_buffer_bytes(
    old: &Workbook,
    new: &Workbook,
    pool: &StringPool,
) -> u64 {
    let mut old_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        old_sheets.insert(make_sheet_key(sheet, pool), sheet);
    }

    let mut new_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        new_sheets.insert(make_sheet_key(sheet, pool), sheet);
    }

    let mut max_estimate = 0u64;
    for (key, old_sheet) in &old_sheets {
        if let Some(new_sheet) = new_sheets.get(key) {
            let estimate = super::hardening::estimate_advanced_sheet_diff_peak(
                &old_sheet.grid,
                &new_sheet.grid,
            );
            max_estimate = max_estimate.max(estimate);
        }
    }

    max_estimate
}

#[cfg(feature = "perf-metrics")]
fn estimate_op_buffer_bytes(sink: &VecSink) -> u64 {
    (sink.op_capacity() as u64).saturating_mul(size_of::<DiffOp>() as u64)
}

#[cfg(feature = "perf-metrics")]
fn apply_accounted_peak(
    metrics: &mut DiffMetrics,
    old: &Workbook,
    new: &Workbook,
    pool: &StringPool,
) {
    let grid_storage_bytes = estimate_grid_storage_bytes(old)
        .saturating_add(estimate_grid_storage_bytes(new));
    let string_pool_bytes = pool.estimated_bytes();
    let alignment_buffer_bytes = estimate_alignment_buffer_bytes(old, new, pool);

    metrics.grid_storage_bytes = grid_storage_bytes;
    metrics.string_pool_bytes = string_pool_bytes;
    metrics.alignment_buffer_bytes = alignment_buffer_bytes;

    let estimated = estimate_workbook_bytes(old)
        .saturating_add(estimate_workbook_bytes(new))
        .saturating_add(string_pool_bytes);
    if estimated > metrics.peak_memory_bytes {
        metrics.peak_memory_bytes = estimated;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_kind_order_ranking_includes_macro_and_other() {
        assert!(
            sheet_kind_order(&SheetKind::Worksheet) < sheet_kind_order(&SheetKind::Chart),
            "Worksheet should rank before Chart"
        );
        assert!(
            sheet_kind_order(&SheetKind::Chart) < sheet_kind_order(&SheetKind::Macro),
            "Chart should rank before Macro"
        );
        assert!(
            sheet_kind_order(&SheetKind::Macro) < sheet_kind_order(&SheetKind::Other),
            "Macro should rank before Other"
        );
    }
}
