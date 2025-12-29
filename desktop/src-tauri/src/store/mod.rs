mod op_sink;
mod op_store;
mod types;

pub use op_sink::OpStoreSink;
pub use op_store::{
    DiffMode, DiffRunSummary, OpStore, RunStatus, SheetStatsResolved, StoreError,
    resolve_sheet_stats,
};
pub use types::{ChangeCounts, SheetStats};
