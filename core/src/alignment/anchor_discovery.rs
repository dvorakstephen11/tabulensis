//! Anchor discovery for AMR alignment.
//!
//! Implements anchor discovery as described in the unified grid diff specification
//! Section 10. Anchors are rows that:
//!
//! 1. Are unique (appear exactly once) in BOTH grids
//! 2. Have matching signatures (content hash)
//!
//! These rows serve as fixed points around which the alignment is built.
//! Rows that are unique in one grid but not the other cannot be anchors
//! since their position cannot be reliably determined.

use std::collections::HashMap;

use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::grid_view::GridView;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub old_row: u32,
    pub new_row: u32,
    pub signature: RowSignature,
}

#[allow(dead_code)]
pub fn discover_anchors(old: &GridView<'_>, new: &GridView<'_>) -> Vec<Anchor> {
    discover_anchors_from_meta(&old.row_meta, &new.row_meta)
}

pub fn discover_anchors_from_meta(old: &[RowMeta], new: &[RowMeta]) -> Vec<Anchor> {
    let mut old_unique: HashMap<RowSignature, u32> = HashMap::new();
    for meta in old.iter() {
        if meta.frequency_class == FrequencyClass::Unique {
            old_unique.insert(meta.signature, meta.row_idx);
        }
    }

    new.iter()
        .filter(|meta| meta.frequency_class == FrequencyClass::Unique)
        .filter_map(|meta| {
            old_unique.get(&meta.signature).map(|old_idx| Anchor {
                old_row: *old_idx,
                new_row: meta.row_idx,
                signature: meta.signature,
            })
        })
        .collect()
}
