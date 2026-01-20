use crossbeam_channel::{Receiver, Sender};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub run_id: u64,
    pub stage: String,
    pub detail: String,
}

pub type ProgressTx = Sender<ProgressEvent>;
pub type ProgressRx = Receiver<ProgressEvent>;
