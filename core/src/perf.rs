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

#[derive(Debug, Clone, Default)]
pub struct DiffMetrics {
    pub parse_time_ms: u64,
    pub move_detection_time_ms: u64,
    pub alignment_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
    pub peak_memory_bytes: usize,
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
                Phase::Parse => self.parse_time_ms += elapsed,
                Phase::MoveDetection => self.move_detection_time_ms += elapsed,
                Phase::Alignment => self.alignment_time_ms += elapsed,
                Phase::CellDiff => self.cell_diff_time_ms += elapsed,
                Phase::Total => self.total_time_ms += elapsed,
            }
        }
    }
}
