use std::collections::HashMap;
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use crate::memory_metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Total,
    Parse,
    MoveDetection,
    Alignment,
    CellDiff,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DiffMetrics {
    pub parse_time_ms: u64,
    pub move_detection_time_ms: u64,
    pub alignment_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub diff_time_ms: u64,
    pub peak_memory_bytes: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
    #[serde(skip)]
    phase_start: HashMap<Phase, Instant>,
}

impl DiffMetrics {
    pub fn start_phase(&mut self, phase: Phase) {
        if matches!(phase, Phase::Total) {
            #[cfg(not(target_arch = "wasm32"))]
            memory_metrics::reset_peak_to_current();
        }
        self.phase_start.insert(phase, Instant::now());
    }

    pub fn end_phase(&mut self, phase: Phase) {
        if let Some(start) = self.phase_start.remove(&phase) {
            let elapsed = start.elapsed().as_millis() as u64;
            match phase {
                Phase::Parse => self.parse_time_ms += elapsed,
                Phase::MoveDetection => self.move_detection_time_ms += elapsed,
                Phase::Alignment => self.alignment_time_ms += elapsed,
                Phase::CellDiff => self.cell_diff_time_ms += elapsed,
                Phase::Total => {
                    self.total_time_ms += elapsed;
                    self.diff_time_ms = self.total_time_ms.saturating_sub(self.parse_time_ms);
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        self.peak_memory_bytes = memory_metrics::peak_bytes();
                    }
                }
            }
        }
    }

    pub fn add_cells_compared(&mut self, count: u64) {
        self.cells_compared = self.cells_compared.saturating_add(count);
    }

    pub fn phase_guard(&mut self, phase: Phase) -> PhaseGuard<'_> {
        PhaseGuard::new(self, phase)
    }
}

pub struct PhaseGuard<'a> {
    metrics: &'a mut DiffMetrics,
    phase: Phase,
}

impl<'a> PhaseGuard<'a> {
    pub fn new(metrics: &'a mut DiffMetrics, phase: Phase) -> Self {
        metrics.start_phase(phase);
        Self { metrics, phase }
    }
}

impl Drop for PhaseGuard<'_> {
    fn drop(&mut self) {
        self.metrics.end_phase(self.phase);
    }
}
