use excel_diff::{ColHash, ColMeta, FrequencyClass, HashStats, RowHash, RowMeta, RowSignature};

fn row_meta(row_idx: u32, hash: RowHash) -> RowMeta {
    RowMeta {
        row_idx,
        signature: hash,
        non_blank_count: 0,
        first_non_blank_col: 0,
        frequency_class: FrequencyClass::Common,
        is_low_info: false,
    }
}

fn col_meta(col_idx: u32, hash: ColHash) -> ColMeta {
    ColMeta {
        col_idx,
        hash,
        non_blank_count: 0,
        first_non_blank_row: 0,
    }
}

#[test]
fn hashstats_counts_and_positions_basic() {
    let h1: RowHash = RowSignature { hash: 1 };
    let h2: RowHash = RowSignature { hash: 2 };
    let h3: RowHash = RowSignature { hash: 3 };
    let h4: RowHash = RowSignature { hash: 4 };

    let rows_a = vec![
        row_meta(0, h1),
        row_meta(1, h2),
        row_meta(2, h2),
        row_meta(3, h3),
    ];
    let rows_b = vec![row_meta(0, h2), row_meta(1, h3), row_meta(2, h4)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);

    assert_eq!(stats.freq_a.get(&h1).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h1).copied().unwrap_or(0), 0);

    assert_eq!(stats.freq_a.get(&h2).copied().unwrap_or(0), 2);
    assert_eq!(stats.freq_b.get(&h2).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_a.get(&h3).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h3).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_a.get(&h4).copied().unwrap_or(0), 0);
    assert_eq!(stats.freq_b.get(&h4).copied().unwrap_or(0), 1);

    assert_eq!(
        stats.hash_to_positions_b.get(&h2).cloned().unwrap(),
        vec![0]
    );
    assert_eq!(
        stats.hash_to_positions_b.get(&h3).cloned().unwrap(),
        vec![1]
    );
    assert_eq!(
        stats.hash_to_positions_b.get(&h4).cloned().unwrap(),
        vec![2]
    );

    let threshold = 1;
    assert!(stats.is_unique(h3));
    assert!(stats.is_common(h2, threshold));
    assert!(!stats.is_rare(h3, threshold));
    assert!(stats.appears_in_both(h3));
    assert!(!stats.appears_in_both(h1));
    assert!(!stats.appears_in_both(h4));
}

#[test]
fn hashstats_rare_but_not_common_boundary() {
    let h: RowHash = RowSignature { hash: 42 };
    let rows_a = vec![row_meta(0, h), row_meta(1, h)];
    let rows_b = vec![row_meta(0, h)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);
    let threshold = 2;

    assert!(stats.is_rare(h, threshold));
    assert!(!stats.is_common(h, threshold));
    assert!(stats.appears_in_both(h));
    assert!(!stats.is_unique(h));
}

#[test]
fn hashstats_equal_to_threshold_behavior() {
    let h: RowHash = RowSignature { hash: 99 };
    let rows_a = vec![row_meta(0, h), row_meta(1, h), row_meta(2, h)];
    let rows_b = vec![row_meta(0, h), row_meta(1, h), row_meta(2, h)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);
    let threshold = 3;

    assert!(stats.is_rare(h, threshold));
    assert!(!stats.is_common(h, threshold));
    assert!(stats.appears_in_both(h));
    assert!(!stats.is_unique(h));
}

#[test]
fn hashstats_empty_inputs() {
    let stats = HashStats::from_row_meta(&[], &[]);
    let dummy_hash: RowHash = RowSignature { hash: 123 };

    assert!(stats.freq_a.is_empty());
    assert!(stats.freq_b.is_empty());
    assert!(stats.hash_to_positions_b.is_empty());

    assert!(!stats.is_unique(dummy_hash));
    assert!(!stats.is_rare(dummy_hash, 1));
    assert!(!stats.is_common(dummy_hash, 0));
    assert!(!stats.appears_in_both(dummy_hash));
}

#[test]
fn hashstats_from_col_meta_tracks_positions() {
    let h1: ColHash = 10;
    let h2: ColHash = 20;
    let h3: ColHash = 30;

    let cols_a = vec![col_meta(0, h1), col_meta(1, h2), col_meta(2, h2)];
    let cols_b = vec![col_meta(0, h2), col_meta(1, h3)];

    let stats = HashStats::from_col_meta(&cols_a, &cols_b);

    assert_eq!(stats.freq_a.get(&h1).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h1).copied().unwrap_or(0), 0);

    assert_eq!(stats.freq_a.get(&h2).copied().unwrap_or(0), 2);
    assert_eq!(stats.freq_b.get(&h2).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_b.get(&h3).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_a.get(&h3).copied().unwrap_or(0), 0);

    assert_eq!(
        stats
            .hash_to_positions_b
            .get(&h2)
            .cloned()
            .unwrap_or_default(),
        vec![0]
    );
    assert_eq!(
        stats
            .hash_to_positions_b
            .get(&h3)
            .cloned()
            .unwrap_or_default(),
        vec![1]
    );
}
