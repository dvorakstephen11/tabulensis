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

use crate::grid_metadata::{FrequencyClass, RowMeta};
#[cfg(test)]
use crate::grid_view::GridView;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub old_row: u32,
    pub new_row: u32,
    pub signature: RowSignature,
}

#[cfg(test)]
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

pub fn discover_context_anchors(old: &[RowMeta], new: &[RowMeta], k: usize) -> Vec<Anchor> {
    if k == 0 || old.len() < k || new.len() < k {
        return Vec::new();
    }

    fn window_signature(window: &[RowMeta]) -> Option<RowSignature> {
        if window.iter().any(|m| m.is_low_info()) {
            return None;
        }
        let mut acc: u128 = 0x9e37_79b1_85eb_ca87;
        for (idx, meta) in window.iter().enumerate() {
            let mul = 0x1000_0000_01b3u128;
            acc = acc
                .wrapping_mul(mul)
                .wrapping_add(meta.signature.hash ^ ((idx as u128) << 1) ^ 0x517c_c1b7_2722_0a95);
            acc ^= acc >> 33;
            acc = acc.rotate_left(7);
        }
        Some(RowSignature { hash: acc })
    }

    let mut count_old: HashMap<RowSignature, u32> = HashMap::new();
    let mut pos_old: HashMap<RowSignature, u32> = HashMap::new();
    for i in 0..=old.len() - k {
        if let Some(sig) = window_signature(&old[i..i + k]) {
            *count_old.entry(sig).or_insert(0) += 1;
            pos_old.entry(sig).or_insert(old[i].row_idx);
        }
    }

    let mut count_new: HashMap<RowSignature, u32> = HashMap::new();
    let mut pos_new: HashMap<RowSignature, u32> = HashMap::new();
    for i in 0..=new.len() - k {
        if let Some(sig) = window_signature(&new[i..i + k]) {
            *count_new.entry(sig).or_insert(0) += 1;
            pos_new.entry(sig).or_insert(new[i].row_idx);
        }
    }

    let mut anchors = Vec::new();
    for (sig, &new_row) in pos_new.iter() {
        if count_new.get(sig).copied().unwrap_or(0) != 1 {
            continue;
        }
        if count_old.get(sig).copied().unwrap_or(0) != 1 {
            continue;
        }
        if let Some(old_row) = pos_old.get(sig) {
            anchors.push(Anchor {
                old_row: *old_row,
                new_row,
                signature: *sig,
            });
        }
    }

    anchors
}

pub fn discover_local_anchors(old: &[RowMeta], new: &[RowMeta]) -> Vec<Anchor> {
    let mut count_old: HashMap<RowSignature, u32> = HashMap::new();
    for m in old.iter() {
        if !m.is_low_info() {
            *count_old.entry(m.signature).or_insert(0) += 1;
        }
    }

    let mut count_new: HashMap<RowSignature, u32> = HashMap::new();
    for m in new.iter() {
        if !m.is_low_info() {
            *count_new.entry(m.signature).or_insert(0) += 1;
        }
    }

    let mut pos_old: HashMap<RowSignature, u32> = HashMap::new();
    for m in old.iter() {
        if !m.is_low_info() && count_old.get(&m.signature).copied().unwrap_or(0) == 1 {
            pos_old.insert(m.signature, m.row_idx);
        }
    }

    let mut out = Vec::new();
    for m in new.iter() {
        if m.is_low_info() {
            continue;
        }
        if count_new.get(&m.signature).copied().unwrap_or(0) != 1 {
            continue;
        }
        if let Some(old_row) = pos_old.get(&m.signature) {
            out.push(Anchor {
                old_row: *old_row,
                new_row: m.row_idx,
                signature: m.signature,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid_metadata::{FrequencyClass, RowMeta};

    fn meta_from_hashes(hashes: &[u128]) -> Vec<RowMeta> {
        hashes
            .iter()
            .enumerate()
            .map(|(idx, &hash)| {
                let sig = RowSignature { hash };
                RowMeta {
                    row_idx: idx as u32,
                    signature: sig,
                    non_blank_count: 1,
                    first_non_blank_col: 0,
                    frequency_class: FrequencyClass::Common,
                    is_low_info: false,
                }
            })
            .collect()
    }

    #[test]
    fn discovers_context_anchors_when_no_uniques() {
        let old = meta_from_hashes(&[1, 2, 3, 4, 5, 6, 1, 2]);
        let new = meta_from_hashes(&[7, 1, 2, 3, 4, 5, 6, 8]);

        let anchors = discover_context_anchors(&old, &new, 4);
        assert!(!anchors.is_empty(), "should find context anchors");
        let mut rows: Vec<(u32, u32)> = anchors.iter().map(|a| (a.old_row, a.new_row)).collect();
        rows.sort();
        assert!(rows.contains(&(0, 1)));
        assert!(rows.contains(&(1, 2)));
        assert!(rows.contains(&(2, 3)));
    }
}
