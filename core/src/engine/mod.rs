//! Core diffing engine for workbook comparison.
//!
//! Provides the main entry point [`diff_workbooks`] for comparing two workbooks
//! and generating a [`DiffReport`] of all changes.
//!
//! ## Module Structure
//!
//! - `workbook_diff`: Workbook-level diff orchestration and sheet enumeration
//! - `grid_diff`: Grid diffing pipeline, cell comparison, and positional diff
//! - `move_mask`: Move detection with region masks and SheetGridDiffer
//! - `sheet_diff`: Sheet-level leaf diff entry points
//! - `amr`: AMR (Adaptive Move Recognition) alignment and decision helpers
//! - `context`: Shared types for diff context and emission

mod amr;
mod context;
mod grid_diff;
mod grid_primitives;
mod hardening;
mod move_mask;
mod sheet_diff;
mod workbook_diff;

pub use grid_diff::{
    diff_grids, diff_grids_database_mode, diff_grids_streaming, diff_grids_streaming_with_progress,
    try_diff_grids, try_diff_grids_database_mode_streaming, try_diff_grids_streaming,
    try_diff_grids_streaming_with_progress,
};
pub use sheet_diff::{
    diff_sheets, diff_sheets_streaming, diff_sheets_streaming_with_progress, try_diff_sheets,
    try_diff_sheets_streaming, try_diff_sheets_streaming_with_progress,
};
pub use workbook_diff::{
    diff_workbooks, diff_workbooks_streaming, diff_workbooks_streaming_with_progress,
    diff_workbooks_with_progress, try_diff_workbooks, try_diff_workbooks_streaming,
    try_diff_workbooks_streaming_with_progress, try_diff_workbooks_with_progress,
};
