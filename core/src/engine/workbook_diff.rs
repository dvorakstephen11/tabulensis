use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary};
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::sink::{DiffSink, SinkFinishGuard, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{Sheet, SheetKind, Workbook};
use crate::progress::ProgressCallback;

use std::collections::{HashMap, HashSet};
#[cfg(feature = "perf-metrics")]
use std::mem::size_of;

use super::context::{DiffContext, emit_op};
use super::grid_diff::try_diff_grids_internal;
use super::hardening::HardeningController;
use crate::diff::SheetId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetIdKey {
    id: u32,
    kind: SheetKind,
}

fn make_sheet_key(sheet: &Sheet, pool: &StringPool) -> SheetKey {
    SheetKey {
        name_lower: pool.resolve(sheet.name).to_lowercase(),
        kind: sheet.kind.clone(),
    }
}

fn sheet_name_lower(sheet: &Sheet, pool: &StringPool) -> String {
    pool.resolve(sheet.name).to_lowercase()
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

/// Stream a workbook diff into `sink`.
///
/// Streaming output follows the contract in `docs/streaming_contract.md`.
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

/// Like [`diff_workbooks_streaming`], but returns errors instead of embedding them in the summary.
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
    let mut finish_guard = SinkFinishGuard::new(sink);

    let mut ctx = DiffContext::default();
    let mut op_count = 0usize;

    if hardening.check_timeout(&mut ctx.warnings) {
        #[cfg(feature = "perf-metrics")]
        {
            metrics.end_phase(Phase::Parse);
            metrics.end_phase(Phase::Total);
            apply_accounted_peak(&mut metrics, old, new, pool);
        }
        finish_guard.finish_and_disarm()?;
        return Ok(DiffSummary {
            complete: false,
            warnings: ctx.warnings,
            op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: Some(metrics),
        });
    }

    let mut old_sheets_by_name: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let key = make_sheet_key(sheet, pool);
        if let Some(previous) = old_sheets_by_name.insert(key.clone(), sheet) {
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

    let mut new_sheets_by_name: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let key = make_sheet_key(sheet, pool);
        if let Some(previous) = new_sheets_by_name.insert(key.clone(), sheet) {
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

    let mut id_counts_old: HashMap<u32, usize> = HashMap::new();
    for sheet in &old.sheets {
        if let Some(id) = sheet.workbook_sheet_id {
            *id_counts_old.entry(id).or_insert(0) += 1;
        }
    }
    for (id, count) in id_counts_old.iter() {
        if *count > 1 {
            ctx.warnings.push(format!(
                "duplicate workbook sheetId in old workbook: id={}, falling back to name-based matching for those sheets.",
                id
            ));
        }
    }

    let mut id_counts_new: HashMap<u32, usize> = HashMap::new();
    for sheet in &new.sheets {
        if let Some(id) = sheet.workbook_sheet_id {
            *id_counts_new.entry(id).or_insert(0) += 1;
        }
    }
    for (id, count) in id_counts_new.iter() {
        if *count > 1 {
            ctx.warnings.push(format!(
                "duplicate workbook sheetId in new workbook: id={}, falling back to name-based matching for those sheets.",
                id
            ));
        }
    }

    let mut old_by_id: HashMap<SheetIdKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let Some(id) = sheet.workbook_sheet_id else {
            continue;
        };
        if id_counts_old.get(&id) != Some(&1) {
            continue;
        }
        let key = SheetIdKey {
            id,
            kind: sheet.kind.clone(),
        };
        old_by_id.insert(key, sheet);
    }

    let mut new_by_id: HashMap<SheetIdKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let Some(id) = sheet.workbook_sheet_id else {
            continue;
        };
        if id_counts_new.get(&id) != Some(&1) {
            continue;
        }
        let key = SheetIdKey {
            id,
            kind: sheet.kind.clone(),
        };
        new_by_id.insert(key, sheet);
    }

    struct SheetEntry<'a> {
        old: Option<&'a Sheet>,
        new: Option<&'a Sheet>,
        by_id: bool,
        sort_name_lower: String,
        kind: SheetKind,
        id: Option<u32>,
    }

    let mut entries: Vec<SheetEntry<'_>> = Vec::new();
    let mut consumed_old: HashSet<*const Sheet> = HashSet::new();
    let mut consumed_new: HashSet<*const Sheet> = HashSet::new();

    let mut id_keys: HashSet<SheetIdKey> = HashSet::new();
    id_keys.extend(old_by_id.keys().cloned());
    id_keys.extend(new_by_id.keys().cloned());

    for key in id_keys {
        let old_sheet = old_by_id.get(&key).copied();
        let new_sheet = new_by_id.get(&key).copied();
        if let Some(sheet) = old_sheet {
            consumed_old.insert(sheet as *const Sheet);
        }
        if let Some(sheet) = new_sheet {
            consumed_new.insert(sheet as *const Sheet);
        }

        let sort_name_lower = if let Some(new_sheet) = new_sheet {
            sheet_name_lower(new_sheet, pool)
        } else {
            sheet_name_lower(
                old_sheet.expect("id entry must have old or new sheet"),
                pool,
            )
        };
        let kind = new_sheet
            .map(|sheet| sheet.kind.clone())
            .unwrap_or_else(|| old_sheet.expect("entry has sheet").kind.clone());
        entries.push(SheetEntry {
            old: old_sheet,
            new: new_sheet,
            by_id: true,
            sort_name_lower,
            kind,
            id: Some(key.id),
        });
    }

    let mut name_keys: Vec<SheetKey> = old_sheets_by_name
        .keys()
        .chain(new_sheets_by_name.keys())
        .cloned()
        .collect();
    name_keys.sort_by(|a, b| match a.name_lower.cmp(&b.name_lower) {
        std::cmp::Ordering::Equal => sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind)),
        other => other,
    });
    name_keys.dedup();

    for key in name_keys {
        let old_sheet = old_sheets_by_name
            .get(&key)
            .copied()
            .filter(|sheet| !consumed_old.contains(&(*sheet as *const Sheet)));
        let new_sheet = new_sheets_by_name
            .get(&key)
            .copied()
            .filter(|sheet| !consumed_new.contains(&(*sheet as *const Sheet)));
        if old_sheet.is_none() && new_sheet.is_none() {
            continue;
        }
        let sort_name_lower = if let Some(new_sheet) = new_sheet {
            sheet_name_lower(new_sheet, pool)
        } else {
            sheet_name_lower(
                old_sheet.expect("name entry must have old or new sheet"),
                pool,
            )
        };
        let kind = new_sheet
            .map(|sheet| sheet.kind.clone())
            .unwrap_or_else(|| old_sheet.expect("entry has sheet").kind.clone());
        entries.push(SheetEntry {
            old: old_sheet,
            new: new_sheet,
            by_id: false,
            sort_name_lower,
            kind,
            id: None,
        });
    }

    entries.sort_by(|a, b| match a.sort_name_lower.cmp(&b.sort_name_lower) {
        std::cmp::Ordering::Equal => {
            let kind_cmp = sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind));
            if kind_cmp != std::cmp::Ordering::Equal {
                return kind_cmp;
            }
            match (a.by_id, b.by_id) {
                (true, true) => a.id.cmp(&b.id),
                (false, false) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
            }
        }
        other => other,
    });

    hardening.progress("parse", 1.0);
    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Parse);
    }

    for entry in entries {
        if hardening.check_timeout(&mut ctx.warnings) {
            break;
        }

        match (entry.old, entry.new) {
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
                if entry.by_id {
                    let old_lower = sheet_name_lower(old_sheet, pool);
                    let new_lower = sheet_name_lower(new_sheet, pool);
                    if old_lower != new_lower {
                        emit_op(
                            sink,
                            &mut op_count,
                            DiffOp::SheetRenamed {
                                sheet: new_sheet.name,
                                from: old_sheet.name,
                                to: new_sheet.name,
                            },
                        )?;
                    }
                }

                let sheet_id: SheetId = if entry.by_id {
                    new_sheet.name
                } else {
                    old_sheet.name
                };
                try_diff_grids_internal(
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
                debug_assert!(false, "entry without old or new sheet");
                continue;
            }
        }
    }

    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Total);
        apply_accounted_peak(&mut metrics, old, new, pool);
    }
    finish_guard.finish_and_disarm()?;
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
            let estimate = crate::memory_estimate::estimate_advanced_sheet_diff_peak(
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
