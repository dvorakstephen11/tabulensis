//! Anchor-Move-Refine (AMR) row alignment algorithm.
//!
//! This module implements a simplified version of the AMR algorithm described in the
//! unified grid diff specification. The implementation follows the general structure:
//!
//! 1. **Row Metadata Collection** (`row_metadata.rs`, Spec Section 9.11)
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
//! - **No global move-candidate extraction phase**: The full spec (Sections 9.5-9.7, 11)
//!   describes a global phase that extracts out-of-order matches before gap filling.
//!   This implementation instead detects moves opportunistically within gaps via
//!   `GapStrategy::MoveCandidate` and `find_block_move`. This is simpler but may miss
//!   some complex multi-block move patterns that the full spec would detect.
//!
//! - **No explicit move validation phase**: The spec describes validating move candidates
//!   (Section 11) to resolve conflicts. The current implementation accepts the first
//!   valid move found within each gap.
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
pub(crate) mod move_extraction;
pub(crate) mod row_metadata;
pub(crate) mod runs;

pub(crate) use assembly::{RowAlignment, RowBlockMove, align_rows_amr};
