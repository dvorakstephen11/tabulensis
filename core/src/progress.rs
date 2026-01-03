/// Progress reporting for long-running diffs.
///
/// The diff engine may call the callback at throttled intervals with a best-effort percentage in
/// the range `[0.0, 1.0]`. Callers should treat progress as advisory and not assume monotonicity
/// across phases.

pub trait ProgressCallback: Send {
    fn on_progress(&self, phase: &str, percent: f32);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoProgress;

impl ProgressCallback for NoProgress {
    fn on_progress(&self, _phase: &str, _percent: f32) {}
}

