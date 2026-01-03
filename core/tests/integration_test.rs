use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up one level from 'core', then into 'fixtures/generated'
    d.push("../fixtures/generated");
    d.push(filename);
    d
}

#[test]
fn test_locate_fixture() {
    let path = get_fixture_path("minimal.xlsx");
    // This test confirms that the Rust code can locate the Python-generated fixtures
    // using the relative path strategy from the monorepo root.
    assert!(
        path.exists(),
        "Fixture minimal.xlsx should exist at {:?}",
        path
    );
}
