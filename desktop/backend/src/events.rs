use crossbeam_channel::{Receiver, Sender};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub run_id: u64,
    pub stage: String,
    /// Optional sub-stage emitted by the diff engine (e.g. `parse`, `alignment`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    pub detail: String,
    /// Optional percentage (0-100). Best-effort; callers must not assume monotonicity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
}

pub type ProgressTx = Sender<ProgressEvent>;
pub type ProgressRx = Receiver<ProgressEvent>;
