mod op_sink;
mod op_store;
mod types;

pub use op_sink::OpStoreSink;
pub use op_store::{
    resolve_sheet_stats, DiffMode, DiffRunSummary, OpStore, RunStatus, SheetStatsResolved,
    StoreError,
};
pub use types::{ChangeCounts, SheetStats};
