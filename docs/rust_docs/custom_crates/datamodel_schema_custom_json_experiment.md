# Experiment: `custom-json-schema` (DataModelSchema JSON Parse Without `serde_json::Value`)

Goal: reduce CPU + memory when parsing PBIX/PBIT `DataModelSchema` by avoiding a full `serde_json::Value` tree build. This is scoped to extracting only:
- tables + columns
- measures
- relationships

## Variant Matrix

- A: Baseline (legacy `Value` parser): `serde_json::Value` parse + tree walk (pre-experiment behavior).
- B: Custom (`custom-json-schema`): typed `Deserialize` parse that skips irrelevant fields.

Correctness stance:
- Custom parser **falls back** to the baseline `Value` parser when `model.tables` is missing, preserving legacy “collect measures anywhere” behavior.

## Implementation Notes

- Feature flag: `custom-json-schema` in:
  - `core/Cargo.toml`
  - surfaced through `desktop/backend/Cargo.toml` and `desktop/wx/Cargo.toml` for app A/B
- Code: `core/src/tabular_schema.rs`
  - `parse_data_model_schema_value(...)` = baseline implementation (unchanged logic)
  - `parse_data_model_schema_custom(...)` = custom typed parse
  - `parse_data_model_schema(...)` dispatches by feature flag
- Parity test:
  - when `custom-json-schema` is enabled, compare custom output against the baseline output in `core/src/tabular_schema.rs` tests.

## Perf Harness

Added a deterministic, synthetic perf test that generates a large `DataModelSchema` JSON document and repeatedly parses it via `excel_diff::datamodel_schema_parse_counts(...)`:
- Test: `core/tests/e2e_perf_datamodel_schema_parse.rs`
- Helper: `core/src/lib.rs` (`datamodel_schema_parse_counts`, `#[doc(hidden)]`)
- Command A (legacy baseline; forces `Value` parser):
  - `cargo test -p excel_diff --release --no-default-features --features "excel-open-xml model-diff base64-crate perf-metrics" --test e2e_perf_datamodel_schema_parse -- --ignored --nocapture --test-threads=1`
- Command B (default-on custom parser):
  - `cargo test -p excel_diff --release --features "perf-metrics model-diff" --test e2e_perf_datamodel_schema_parse -- --ignored --nocapture --test-threads=1`

Expected win shape:
- lower `parse_time_ms` on the synthetic schema parse suite
- lower `peak_memory_bytes` (should drop without the `Value` tree allocations)

## Initial Results (2026-02-08)

Single-run A/B on this machine (release build, `schema_bytes=1095820`, `iterations=47`):

| Variant | parse_time_ms | peak_memory_bytes |
| --- | ---:| ---:|
| Baseline (`perf-metrics model-diff`) | 1178 | 10012656 |
| Custom (`perf-metrics model-diff custom-json-schema`) | 366 | 3122828 |

Delta:
- parse time: `-812 ms` (`-68.9%`)
- peak memory: `-6889828 bytes` (`-68.8%`)

Next gate: 5-run alternating A/B to confirm stability and rule out noise.

## 5-run A/B Confirmation (2026-02-08)

Median-of-5 (alternating A/B) on `datamodel_schema_parse`:

| Variant | parse_time_ms | peak_memory_bytes |
| --- | ---:| ---:|
| Baseline (legacy `Value` parser) | 1181 | 10012766 |
| Default-on custom parser | 520 | 3122938 |

Delta:
- parse time: `-661 ms` (`-56.0%`)
- peak memory: `-6889828 bytes` (`-68.8%`)

## Promotion (2026-02-08)

Enabled `custom-json-schema` by default for:
- core crate default features: `core/Cargo.toml`
- desktop app default build: `desktop/wx/Cargo.toml`
- wasm build: `wasm/Cargo.toml`

## Next Steps

1. Keep the feature flag available for rollback; if a real-world schema breaks typed parsing, fallback behavior should preserve correctness but may reduce the win.
2. If we see schema drift in the wild, extend the typed structs conservatively (new optional fields) and keep the parity test.
