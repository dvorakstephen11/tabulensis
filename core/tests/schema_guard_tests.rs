use excel_diff::{diff_workbooks_to_json, DiffConfig};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod common;
use common::fixture_path;

const EXPECTED_JSON_HASH: &str = "54bb0d6e4fd64b6e3d67c605aa6db56dff67d6157ffdc75d750cda80683b6d4d";

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                if let Some(val) = map.get(&key) {
                    out.insert(key, canonicalize(val));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => {
            let normalized = items.iter().map(canonicalize).collect();
            Value::Array(normalized)
        }
        other => other.clone(),
    }
}

#[test]
fn json_output_schema_hash_guard() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json =
        diff_workbooks_to_json(&a, &b, &DiffConfig::default()).expect("json diff should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let canonical = canonicalize(&value);
    let canonical_json =
        serde_json::to_string(&canonical).expect("canonical json should serialize");

    let digest = Sha256::digest(canonical_json.as_bytes());
    let actual = format!("{:x}", digest);

    assert_eq!(
        actual, EXPECTED_JSON_HASH,
        "update EXPECTED_JSON_HASH when the JSON contract intentionally changes"
    );
}
