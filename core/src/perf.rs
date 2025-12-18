use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Total,
    Parse,
    MoveDetection,
    Alignment,
    CellDiff,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffMetrics {
    pub move_detection_time_ms: u64,
    pub alignment_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
    #[serde(skip)]
    phase_start: HashMap<Phase, Instant>,
}

impl DiffMetrics {
    pub fn start_phase(&mut self, phase: Phase) {
        self.phase_start.insert(phase, Instant::now());
    }

    pub fn end_phase(&mut self, phase: Phase) {
        if let Some(start) = self.phase_start.remove(&phase) {
            let elapsed = start.elapsed().as_millis() as u64;
            match phase {
                Phase::Parse => {}
                Phase::MoveDetection => self.move_detection_time_ms += elapsed,
                Phase::Alignment => self.alignment_time_ms += elapsed,
                Phase::CellDiff => self.cell_diff_time_ms += elapsed,
                Phase::Total => self.total_time_ms += elapsed,
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
