use excel_diff::StringPool;

#[test]
fn intern_50k_identical_strings_returns_same_id() {
    let mut pool = StringPool::new();
    let first_id = pool.intern("repeated_string");

    for _ in 1..50_000 {
        let id = pool.intern("repeated_string");
        assert_eq!(id, first_id, "interning same string must return same id");
    }

    assert!(
        pool.len() >= 2,
        "pool should have at least 2 entries (empty string + our string)"
    );
    assert!(
        pool.len() <= 3,
        "pool should not grow beyond initial strings"
    );

    assert_eq!(pool.resolve(first_id), "repeated_string");
}

#[test]
fn intern_distinct_strings_returns_different_ids() {
    let mut pool = StringPool::new();

    let id_a = pool.intern("alpha");
    let id_b = pool.intern("beta");
    let id_c = pool.intern("gamma");

    assert_ne!(id_a, id_b);
    assert_ne!(id_b, id_c);
    assert_ne!(id_a, id_c);

    assert_eq!(pool.resolve(id_a), "alpha");
    assert_eq!(pool.resolve(id_b), "beta");
    assert_eq!(pool.resolve(id_c), "gamma");
}

#[test]
fn empty_string_is_pre_interned() {
    let pool = StringPool::new();

    assert!(pool.len() >= 1, "pool should have at least empty string");
    assert_eq!(pool.resolve(excel_diff::StringId(0)), "");
}

#[test]
fn resolve_returns_original_string() {
    let mut pool = StringPool::new();

    let test_cases = vec![
        "hello",
        "world",
        "with spaces",
        "with\nnewline",
        "unicode: 日本語",
        "",
    ];

    for s in &test_cases {
        let id = pool.intern(s);
        assert_eq!(pool.resolve(id), *s);
    }
}

#[test]
fn into_strings_returns_all_interned() {
    let mut pool = StringPool::new();

    pool.intern("first");
    pool.intern("second");
    pool.intern("third");

    let strings = pool.into_strings();

    assert!(strings.contains(&"".to_string()));
    assert!(strings.contains(&"first".to_string()));
    assert!(strings.contains(&"second".to_string()));
    assert!(strings.contains(&"third".to_string()));
    assert_eq!(strings.len(), 4);
}
