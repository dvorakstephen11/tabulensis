//! PBIP (Power BI Project) support: scan, normalize, and diff text-native artifacts.
//!
//! Iteration 2 introduces PBIP/PBIR/TMDL diffs as a first-class "diff domain". The
//! core principles here are:
//! - Deterministic normalization (stable output on repeated runs)
//! - Graceful degradation (malformed inputs become errors in the report, not panics)
//! - Small, composable types that the desktop/backend can store efficiently

mod diff;
mod normalize;
mod types;

#[cfg(feature = "std-fs")]
mod scan;

pub use diff::{diff_snapshots, PbipDiffReport, PbipEntityDiff, PbipEntityKind};
pub use normalize::{normalize_doc_text, NormalizationApplied, NormalizationError};
#[cfg(feature = "std-fs")]
pub use scan::{snapshot_project_from_fs, PbipScanConfig, PbipScanError};
pub use types::{
    PbipChangeKind, PbipDocDiff, PbipDocRecord, PbipDocSnapshot, PbipDocType,
    PbipNormalizationProfile, PbipProjectSnapshot,
};

#[cfg(test)]
mod tests;
