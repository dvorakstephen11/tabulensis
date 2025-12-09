pub(crate) mod anchor_chain;
pub(crate) mod anchor_discovery;
pub(crate) mod assembly;
pub(crate) mod gap_strategy;
pub(crate) mod move_extraction;
pub(crate) mod row_metadata;
pub(crate) mod runs;

pub(crate) use assembly::{align_rows_amr, RowAlignment, RowBlockMove};
