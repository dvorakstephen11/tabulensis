//! Row metadata and frequency classification for AMR alignment.
//!
//! Implements row frequency classification as described in the unified grid diff
//! specification Section 9.11. Each row is classified into one of four frequency classes:
//!
//! - **Unique**: Appears exactly once in the grid (highest anchor quality)
//! - **Rare**: Appears 2-N times where N is configurable (can serve as secondary anchors)
//! - **Common**: Appears frequently (poor anchor quality)
//! - **LowInfo**: Blank or near-blank rows (ignored for anchoring)

use std::collections::HashMap;

use crate::config::DiffConfig;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrequencyClass {
    Unique,
    Rare,
    Common,
    LowInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RowMeta {
    pub row_idx: u32,
    pub signature: RowSignature,
    pub hash: RowSignature,
    pub non_blank_count: u16,
    pub first_non_blank_col: u16,
    pub frequency_class: FrequencyClass,
    pub is_low_info: bool,
}

impl RowMeta {
    pub fn is_low_info(&self) -> bool {
        self.is_low_info || matches!(self.frequency_class, FrequencyClass::LowInfo)
    }
}

pub fn frequency_map(row_meta: &[RowMeta]) -> HashMap<RowSignature, u32> {
    let mut map = HashMap::new();
    for meta in row_meta {
        *map.entry(meta.signature).or_insert(0) += 1;
    }
    map
}

pub fn classify_row_frequencies(row_meta: &mut [RowMeta], config: &DiffConfig) {
    let freq_map = frequency_map(row_meta);
    for meta in row_meta.iter_mut() {
        if meta.frequency_class == FrequencyClass::LowInfo {
            continue;
        }

        let count = freq_map.get(&meta.signature).copied().unwrap_or(0);
        let mut class = match count {
            1 => FrequencyClass::Unique,
            c if c == 0 => FrequencyClass::Common,
            c if c <= config.rare_threshold => FrequencyClass::Rare,
            _ => FrequencyClass::Common,
        };

        if (meta.non_blank_count as u32) < config.low_info_threshold || meta.is_low_info {
            class = FrequencyClass::LowInfo;
            meta.is_low_info = true;
        }

        meta.frequency_class = class;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(row_idx: u32, hash: u128, non_blank: u16) -> RowMeta {
        let sig = RowSignature { hash };
        RowMeta {
            row_idx,
            signature: sig,
            hash: sig,
            non_blank_count: non_blank,
            first_non_blank_col: 0,
            frequency_class: FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn classifies_unique_and_rare_and_low_info() {
        let mut meta = vec![
            make_meta(0, 1, 3),
            make_meta(1, 1, 3),
            make_meta(2, 2, 1),
        ];

        let mut config = DiffConfig::default();
        config.rare_threshold = 2;
        config.low_info_threshold = 2;

        classify_row_frequencies(&mut meta, &config);

        assert_eq!(meta[0].frequency_class, FrequencyClass::Rare);
        assert_eq!(meta[1].frequency_class, FrequencyClass::Rare);
        assert_eq!(meta[2].frequency_class, FrequencyClass::LowInfo);
    }
}
