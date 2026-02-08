//! Anchor-Move-Refine (AMR) row alignment algorithm.
//!
//! This module implements a simplified version of the AMR algorithm described in the
//! unified grid diff specification. The implementation follows the general structure:
//!
//! 1. **Row Metadata Collection** (`grid_metadata.rs`, Spec Section 9.11)
//!    - Compute row signatures and classify by frequency (Unique/Rare/Common/LowInfo)
//!
//! 2. **Anchor Discovery** (`anchor_discovery.rs`, Spec Section 10)
//!    - Find rows that are unique in both grids with matching signatures
//!
//! 3. **Anchor Chain Construction** (`anchor_chain.rs`, Spec Section 10)
//!    - Build longest increasing subsequence (LIS) of anchors to preserve relative order
//!
//! 4. **Gap Strategy Selection** (`gap_strategy.rs`, Spec Sections 9.6, 12)
//!    - For each gap between anchors, select appropriate strategy:
//!      Empty, InsertAll, DeleteAll, SmallEdit, MoveCandidate, or RecursiveAlign
//!
//! 5. **Assembly** (`assembly.rs`, Spec Section 12)
//!    - Assemble final alignment by processing gaps and anchors
//!
//! ## Intentional Spec Deviations
//!
//! The current implementation simplifies the full AMR spec in the following ways:
//!
//! - **Global move extraction is bounded**: The implementation performs global
//!   unanchored match extraction with LAP assignment and validation, but caps
//!   candidate counts for determinism and performance.
//!
//! - **RLE fast path**: For highly repetitive grids (>50% compression), the implementation
//!   uses a run-length encoded alignment path (`runs.rs`) that bypasses full AMR.
//!
//! These simplifications are acceptable for most real-world Excel workbooks and keep
//! the implementation maintainable. Future work may implement the full global move
//! extraction if complex reordering scenarios require it.

pub(crate) mod anchor_chain;
pub(crate) mod anchor_discovery;
pub(crate) mod assembly;
pub(crate) mod gap_strategy;
pub(crate) mod lap;
pub(crate) mod move_extraction;
pub(crate) mod runs;

pub(crate) use crate::alignment_types::RowBlockMove;
pub(crate) use assembly::align_meta_with_amr;
pub(crate) use assembly::align_rows_amr_with_signatures_from_views;
#[cfg(feature = "perf-metrics")]
pub(crate) use assembly::align_rows_amr_with_signatures_from_views_with_metrics;
