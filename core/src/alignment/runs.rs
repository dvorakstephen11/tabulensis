//! Run-length encoding for repetitive row patterns.
//!
//! Implements run-length compression as described in the unified grid diff
//! specification Section 2.6 (optional optimization). For grids where >50%
//! of rows share signatures with adjacent rows, this provides a fast path
//! that avoids full AMR computation.
//!
//! This is particularly effective for:
//! - Template-based workbooks with many identical rows
//! - Data with long runs of blank or placeholder rows
//! - Adversarial cases designed to stress the alignment algorithm

use crate::alignment::row_metadata::RowMeta;
use crate::workbook::RowSignature;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRun {
    pub signature: RowSignature,
    pub start_row: u32,
    pub count: u32,
}

pub fn compress_to_runs(meta: &[RowMeta]) -> Vec<RowRun> {
    let mut runs = Vec::new();
    let mut i = 0usize;
    while i < meta.len() {
        let sig = meta[i].signature;
        let start = i;
        while i < meta.len() && meta[i].signature == sig {
            i += 1;
        }
        runs.push(RowRun {
            signature: sig,
            start_row: meta[start].row_idx,
            count: (i - start) as u32,
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(idx: u32, hash: u128) -> RowMeta {
        let sig = RowSignature { hash };
        RowMeta {
            row_idx: idx,
            signature: sig,
            hash: sig,
            non_blank_count: 1,
            first_non_blank_col: 0,
            frequency_class: crate::alignment::row_metadata::FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn compresses_identical_rows() {
        let meta = vec![make_meta(0, 1), make_meta(1, 1), make_meta(2, 2)];
        let runs = compress_to_runs(&meta);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].count, 2);
        assert_eq!(runs[1].count, 1);
    }

    #[test]
    fn compresses_10k_identical_rows_to_single_run() {
        let meta: Vec<RowMeta> = (0..10_000).map(|i| make_meta(i, 42)).collect();
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 1, "10K identical rows should compress to a single run");
        assert_eq!(runs[0].count, 10_000, "single run should have count of 10,000");
        assert_eq!(runs[0].signature.hash, 42, "run signature should match input");
        assert_eq!(runs[0].start_row, 0, "run should start at row 0");
    }

    #[test]
    fn alternating_pattern_ab_does_not_overcompress() {
        let meta: Vec<RowMeta> = (0..10_000)
            .map(|i| {
                let hash = if i % 2 == 0 { 1 } else { 2 };
                make_meta(i, hash)
            })
            .collect();
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 10_000, 
            "alternating A-B pattern should produce 10K runs (no compression benefit)");
        
        for (i, run) in runs.iter().enumerate() {
            assert_eq!(run.count, 1, "each run should have count of 1 for alternating pattern");
            let expected_hash = if i % 2 == 0 { 1 } else { 2 };
            assert_eq!(run.signature.hash, expected_hash, "run signature should alternate");
        }
    }

    #[test]
    fn mixed_runs_with_varying_lengths() {
        let mut meta = Vec::new();
        let mut row_idx = 0u32;
        
        for _ in 0..100 {
            meta.push(make_meta(row_idx, 1));
            row_idx += 1;
        }
        for _ in 0..50 {
            meta.push(make_meta(row_idx, 2));
            row_idx += 1;
        }
        for _ in 0..200 {
            meta.push(make_meta(row_idx, 3));
            row_idx += 1;
        }
        for _ in 0..1 {
            meta.push(make_meta(row_idx, 4));
            row_idx += 1;
        }
        
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 4, "should produce 4 runs for 4 distinct signatures");
        assert_eq!(runs[0].count, 100);
        assert_eq!(runs[1].count, 50);
        assert_eq!(runs[2].count, 200);
        assert_eq!(runs[3].count, 1);
    }

    #[test]
    fn empty_input_produces_empty_runs() {
        let meta: Vec<RowMeta> = vec![];
        let runs = compress_to_runs(&meta);
        assert!(runs.is_empty(), "empty input should produce empty runs");
    }

    #[test]
    fn single_row_produces_single_run() {
        let meta = vec![make_meta(0, 999)];
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].count, 1);
        assert_eq!(runs[0].start_row, 0);
        assert_eq!(runs[0].signature.hash, 999);
    }

    #[test]
    fn run_compression_preserves_row_indices() {
        let meta: Vec<RowMeta> = (0..1000u32)
            .map(|i| make_meta(i, (i / 100) as u128))
            .collect();
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 10, "should have 10 runs (one per 100 rows)");
        
        for (group_idx, run) in runs.iter().enumerate() {
            let expected_start = (group_idx * 100) as u32;
            assert_eq!(run.start_row, expected_start, 
                "run {} should start at row {}", group_idx, expected_start);
            assert_eq!(run.count, 100, "each run should have 100 rows");
        }
    }
}
