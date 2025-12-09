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
