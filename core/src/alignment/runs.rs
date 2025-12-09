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
            start_row: start as u32,
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
}
