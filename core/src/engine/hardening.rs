use crate::config::DiffConfig;
use crate::progress::ProgressCallback;
use crate::workbook::{Cell, Grid};
use std::mem::size_of;
use std::time::{Duration, Instant};

const BYTES_PER_MB: u64 = 1024 * 1024;

const PROGRESS_MIN_DELTA: f32 = 0.01;
const TIMEOUT_CHECK_EVERY_TICKS: u64 = 256;

pub(super) struct HardeningController<'a> {
    start: Instant,
    timeout: Option<Duration>,
    max_memory_bytes: Option<u64>,
    aborted: bool,
    warned_timeout: bool,
    warned_memory: bool,
    progress: Option<&'a dyn ProgressCallback>,
    last_progress_phase: Option<&'static str>,
    last_progress_percent: Option<f32>,
    timeout_tick: u64,
}

impl<'a> HardeningController<'a> {
    pub(super) fn new(config: &DiffConfig, progress: Option<&'a dyn ProgressCallback>) -> Self {
        Self {
            start: Instant::now(),
            timeout: config
                .timeout_seconds
                .map(|secs| Duration::from_secs(secs as u64)),
            max_memory_bytes: config
                .max_memory_mb
                .map(|mb| (mb as u64).saturating_mul(BYTES_PER_MB)),
            aborted: false,
            warned_timeout: false,
            warned_memory: false,
            progress,
            last_progress_phase: None,
            last_progress_percent: None,
            timeout_tick: 0,
        }
    }

    pub(super) fn should_abort(&self) -> bool {
        self.aborted
    }

    pub(super) fn check_timeout(&mut self, warnings: &mut Vec<String>) -> bool {
        if self.aborted {
            return true;
        }
        let Some(timeout) = self.timeout else {
            return false;
        };

        self.timeout_tick = self.timeout_tick.saturating_add(1);
        let should_check = self.timeout_tick == 1 || self.timeout_tick % TIMEOUT_CHECK_EVERY_TICKS == 0;
        if !should_check {
            return false;
        }

        if self.start.elapsed() < timeout {
            return false;
        }

        self.aborted = true;
        if !self.warned_timeout {
            self.warned_timeout = true;
            warnings.push(format!(
                "timeout after {} seconds; diff aborted early; results may be incomplete",
                timeout.as_secs()
            ));
        }
        true
    }

    pub(super) fn memory_guard_or_warn(
        &mut self,
        estimated_extra_bytes: u64,
        warnings: &mut Vec<String>,
        context: &str,
    ) -> bool {
        let Some(limit) = self.max_memory_bytes else {
            return false;
        };

        if estimated_extra_bytes <= limit {
            return false;
        }

        if !self.warned_memory {
            self.warned_memory = true;
            warnings.push(format!(
                "memory budget exceeded in {context} (estimated ~{} MB > limit {} MB); falling back to positional diff; results may be incomplete",
                bytes_to_mb_ceil(estimated_extra_bytes),
                bytes_to_mb_ceil(limit),
            ));
        }

        true
    }

    pub(super) fn progress(&mut self, phase: &'static str, percent: f32) {
        let Some(callback) = self.progress else {
            return;
        };

        let mut clamped = if percent.is_finite() { percent } else { 0.0 };
        if clamped < 0.0 {
            clamped = 0.0;
        } else if clamped > 1.0 {
            clamped = 1.0;
        }

        let should_emit = match (self.last_progress_phase, self.last_progress_percent) {
            (Some(last_phase), Some(last_percent)) if last_phase == phase => {
                clamped == 0.0
                    || clamped == 1.0
                    || clamped < last_percent
                    || (clamped - last_percent) >= PROGRESS_MIN_DELTA
            }
            _ => true,
        };

        if !should_emit {
            return;
        }

        self.last_progress_phase = Some(phase);
        self.last_progress_percent = Some(clamped);
        callback.on_progress(phase, clamped);
    }
}

pub(super) fn estimate_gridview_bytes(grid: &Grid) -> u64 {
    let nrows = grid.nrows as u64;
    let ncols = grid.ncols as u64;
    let cell_count = grid.cell_count() as u64;

    let row_view_bytes = nrows.saturating_mul(size_of::<crate::grid_view::RowView<'static>>() as u64);
    let row_meta_bytes = nrows.saturating_mul(size_of::<crate::grid_view::RowMeta>() as u64);
    let col_meta_bytes = ncols.saturating_mul(size_of::<crate::grid_view::ColMeta>() as u64);

    let cell_entry_bytes = cell_count
        .saturating_mul(size_of::<(u32, &'static Cell)>() as u64)
        .saturating_mul(5)
        .saturating_div(4);

    let build_row_counts_bytes = nrows
        .saturating_mul(size_of::<u32>() as u64)
        .saturating_add(nrows.saturating_mul(size_of::<Option<u32>>() as u64));
    let build_col_counts_bytes = ncols
        .saturating_mul(size_of::<u32>() as u64)
        .saturating_add(ncols.saturating_mul(size_of::<Option<u32>>() as u64));
    let build_hashers_bytes =
        ncols.saturating_mul(size_of::<xxhash_rust::xxh3::Xxh3>() as u64);

    row_view_bytes
        .saturating_add(row_meta_bytes)
        .saturating_add(col_meta_bytes)
        .saturating_add(cell_entry_bytes)
        .saturating_add(build_row_counts_bytes)
        .saturating_add(build_col_counts_bytes)
        .saturating_add(build_hashers_bytes)
}

pub(super) fn estimate_advanced_sheet_diff_peak(old: &Grid, new: &Grid) -> u64 {
    let base = estimate_gridview_bytes(old).saturating_add(estimate_gridview_bytes(new));
    let max_rows = old.nrows.max(new.nrows) as u64;
    let max_cols = old.ncols.max(new.ncols) as u64;

    let alignment_overhead = max_rows
        .saturating_add(max_cols)
        .saturating_mul(size_of::<u32>() as u64)
        .saturating_mul(8);

    base.saturating_add(alignment_overhead)
}

fn bytes_to_mb_ceil(bytes: u64) -> u64 {
    bytes
        .saturating_add(BYTES_PER_MB.saturating_sub(1))
        .saturating_div(BYTES_PER_MB)
}

