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
//! - `amr`: AMR (Adaptive Move Recognition) alignment and decision helpers
//! - `context`: Shared types for diff context and emission

mod amr;
mod context;
mod grid_diff;
mod grid_primitives;
mod move_mask;
mod workbook_diff;

use crate::diff::SheetId;
use context::emit_op;

pub use grid_diff::diff_grids_database_mode;
pub(crate) use grid_diff::try_diff_grids_database_mode_streaming;
pub use workbook_diff::{
    diff_workbooks, diff_workbooks_streaming, try_diff_workbooks, try_diff_workbooks_streaming,
};
