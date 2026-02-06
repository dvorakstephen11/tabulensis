# Rust Core Performance Experiments (Five New Experiments)

Date: 2026-02-06

This document is a decision-complete plan for five new performance experiments targeting the Rust core (plus e2e/perf tests), grounded in what has already been tried in this repo and what the benchmarks have shown.

## What Was Reviewed (Prior Plans + Benchmark Evidence)

- Custom-crate experiment index: `docs/rust_docs/custom_crates/README.md`
- Active custom XML experiment (with benchmark deltas): `docs/rust_docs/custom_crates/next_experiment_custom_xml.md`
- Prior custom-base64 experiment (outcome + perf harness learnings): `docs/rust_docs/custom_crates/base64_custom_crate_experiment.md`
- Recent perf-cycle deltas/signals: `benchmarks/perf_cycles/*/cycle_delta.md`, `benchmarks/perf_cycles/*/cycle_signal.md`
- Current perf TODO shortlist: `docs/rust_docs/perf_next_todos.md`
- Historical perf plans and findings:
  - `allocation_improvements.md`
  - `parse_time_improvements.md`
  - `addressing_regressions_in_e2e.md`
  - `string_interning_improvement.md`
  - `signature_building_improvements.md`
  - `algorithmic_improvement*.md`

## Key Observations (Why These Experiments)

1. Workbook-open e2e is parse-dominated on recent runs.
   - Example noted in the custom-xml log: in a recent cycle, `pre_e2e.json` had parse share ~99.57% of total.
   - Practical implication: improvements in sheet/sharedStrings parsing and container read behavior can move `total_time_ms` almost 1:1.

2. The `custom-xml` experiment is already a proven win and is the highest-leverage axis to keep pushing.
   - From `docs/rust_docs/custom_crates/next_experiment_custom_xml.md`:
     - Slice 1 (shared strings only), 5-run confirmation: aggregate total median improved by ~7.5%.
     - Slice 2 (worksheet cell/value scan loop), 5-run confirmation: aggregate total/parse improved by ~21% on the `e2e_perf_workbook_open` suite.

3. Identical-workbook behavior changed: fast OpenXML diff can skip parsing and legitimately report 0ms parse/total after ms rounding.
   - `core/tests/e2e_perf_workbook_open.rs` documents this explicitly.
   - Perf interpretation rule: treat `e2e_p5_identical` as a special case; use op counts and/or additional micro metrics if needed.

4. Parse-time can be noisy across runs even at the same commit (environmental variance).
   - See `parse_regression_findings.md`.
   - Practical implication: for parse-heavy changes, use median-of-5 confirmation after a promising median-of-3, and keep pre/post run counts symmetric.

5. Past custom-crate experiments show the right pattern: feature-gate, parity test, measure, and be willing to delete the custom code if it does not win.
   - The base64 experiment ended with removing the custom decoder and keeping only the structural wins; that is the right precedent for how to treat future custom writer/reader work.

## Standard Experiment Protocol (Must Follow)

### A/B/C build matrix (where applicable)

- A: Baseline (default features; existing crates/code paths)
- B: Custom (new behavior behind a feature flag)
- C: Parity (optional but recommended): compile both implementations in test builds and compare outputs side-by-side.

### Measurement policy

If any of the following are touched: parse/open/container/alignment/diff behavior or Rust deps/features, run a full perf cycle.

1. Before: `python3 scripts/perf_cycle.py pre`
2. After:  `python3 scripts/perf_cycle.py post --cycle <cycle_id>`

For parse-oriented changes, do:
- median-of-3 for an initial read, then
- a 5-run confirmation (alternating A/B, same commands, same build flags).

Also run:
- `cargo test -p excel_diff`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`

### Guardrails

- Do not combine multiple experiments in a single perf cycle until each has been measured in isolation.
- If you introduce fixtures or baselines, keep them deterministic and update manifests without using `--clean` unless intended.

---

# Experiment 1: Custom-XML Extension (Numeric Parsing Fast Paths + Default-On Readiness)

## Objective

Build on the already-confirmed `custom-xml` wins by eliminating remaining hot micro-costs in worksheet parsing, especially:
- numeric parsing (common in `e2e_p2_noise`, `e2e_p3_repetitive`),
- shared-string index parsing (`t="s"` cells),
- and any avoidable allocations in value conversion.

Secondarily, make `custom-xml` ready to be evaluated as a default-on backend (still with an easy fallback to quick-xml).

## Why This Is Next (Evidence)

`custom-xml` Slice 2 already improved the aggregate `e2e_perf_workbook_open` median by about 21% (5-run confirmation). That implies the scanner itself is working; remaining regressions or missed wins are likely in value decoding and conversion work.

## Scope / Touchpoints

- `core/src/grid_parser.rs`:
  - `parse_sheet_xml_internal_custom(...)`
  - `parse_cell_custom(...)`
  - `convert_value(...)`
  - Any helper parsing functions used by the custom path
- Feature flag: `custom-xml` in `core/Cargo.toml`

## Implementation Plan (Decision Complete)

1. Add ASCII digit parsers (bytes -> integer) and use them in hot paths.
   - Add helpers (exact signatures):
     - `fn parse_usize_ascii(bytes: &[u8]) -> Option<usize>`
     - `fn parse_u32_ascii(bytes: &[u8]) -> Option<u32>`
   - Behavior:
     - Accept optional leading ASCII whitespace (trim) only if existing behavior trims.
     - Reject empty and any non-digit bytes.
     - Use checked arithmetic for overflow safety.
   - Replace `.parse::<usize>()` in `convert_value` for shared-string indices and any other hot parse points where the source is known ASCII digits.

2. Add a fast numeric parsing ladder for common Excel numeric values.
   - Update `convert_value` numeric branch:
     - If the value bytes are an ASCII integer (optional `-`), parse as i64 and convert to f64 (exact for typical cell ints in fixtures).
     - Else if it is a simple decimal without exponent (one `.`), parse with a small custom decimal parser (cap fractional digits to a safe number, e.g. 18) and convert deterministically.
     - Else fallback to `str::parse::<f64>()` to preserve correctness for exponent/scientific.
   - Keep baseline semantics:
     - Preserve current behavior for empty `<v></v>` (explicit empty cached value) per the Slice 2 fix.

3. Expand parity tests between quick-xml and custom-xml paths.
   - Tests should cover:
     - sharedStrings entities, CDATA, rich-text run flattening.
     - worksheet numeric, boolean, error, inline string, empty `<v></v>`.
     - formula parsing and escaping/unescaping.
   - Parity mechanism:
     - In `cfg(test)`, run both parsers on the same XML fixture and compare produced `Grid`/cell values and any derived metadata (string pool content).

4. Add candidate-specific micro benchmarks (optional but recommended).
   - Add a new perf test target (ignored) focused on worksheet parsing alone:
     - parse a large synthetic sheet XML and report `parse_time_ms` and `allocations_est` (if available).
   - This isolates parse from container I/O variability and makes iteration faster.

## A/B Measurement Plan

- A: `cargo test -p excel_diff --release --features perf-metrics --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`
- B: `cargo test -p excel_diff --release --features "perf-metrics custom-xml" --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`
- Confirm with 5-run alternating A/B.
- Then run a full perf cycle.

## Acceptance Criteria

- Correctness: parity tests pass; no differences on the existing perf e2e fixtures.
- Performance:
  - No regressions relative to current `custom-xml` baseline.
  - `e2e_p2_noise` and `e2e_p3_repetitive` should improve or at least stop being the long tail relative to `e2e_p1_dense`.
- Memory: no stable >5% increase in peak memory.

## Stop / Rollback Conditions

- Any mismatch in parsed grids (beyond known/intentional semantic fixes) blocks promotion; fix correctness first.

---

# Experiment 2: ZIP Entry Lookup Caching (Name -> Index) in `ZipContainer`

## Objective

Reduce overhead of repeated ZIP part access by caching the mapping of part name -> archive index, and preferentially using `by_index` instead of `by_name` in hot paths:
- `read_file_checked`
- `file_fingerprint_checked`

## Why This Is Worth Testing

The OpenXML open/diff path touches many parts (`xl/workbook.xml`, rels, multiple sheets, drawings, charts). Even small overhead per part can become noticeable. This experiment is low-risk and should be measurable via e2e parse time.

## Scope / Touchpoints

- `core/src/container.rs` (`ZipContainer` and `OpcContainer` wrappers)

## Implementation Plan (Decision Complete)

1. Add an index cache to `ZipContainer`.
   - Add fields:
     - `name_to_index: rustc_hash::FxHashMap<String, usize>`
   - Populate immediately after `ZipArchive::new(...)` succeeds, during `open_from_reader_with_limits(...)`:
     - Iterate indices `0..archive.len()`.
     - Read each entry name once and insert into the map.
   - If duplicate names exist (unusual but possible), keep the first occurrence to preserve old behavior as closely as possible (baseline `by_name` would also return the first match).

2. Add helper:
   - `fn index_of(&self, name: &str) -> Option<usize>`

3. Update read/fingerprint methods to use the cache.
   - `read_file_checked(name)`:
     - lookup index; if missing return `ContainerError::FileNotFound`.
     - probe size via `by_index` for limit checks (as today).
     - then read via `by_index` into a vec.
   - `file_fingerprint_checked(name)` similarly.

4. Preserve error mapping and limits.
   - Keep the same `ContainerError` variants and codes.
   - Enforce `max_part_uncompressed_bytes` and `max_total_uncompressed_bytes` as today.

## Measurement Plan

- Full perf cycle pre/post (container behavior change).
- Watch `e2e parse_time_ms` and `total_time_ms`.

## Acceptance Criteria

- No correctness changes (fixtures still parse/diff identically).
- Any stable improvement on e2e parse time, even small (>=1% median), justifies keeping this because it is also a complexity reduction (fewer repeated lookups).

## Stop Conditions

- If zip crate API prevents safe or consistent by-index usage, abandon this experiment rather than shipping behavioral risk.

---

# Experiment 3: Scratch-Buffer Reads and Pre-Sizing for ZIP Part Reads (No Streaming Yet)

## Objective

Reduce allocator churn and transient memory overhead during workbook open by:
- pre-sizing the `Vec<u8>` based on ZIP uncompressed size, and
- reusing scratch buffers across repeated part reads inside `open_workbook_from_container_with_grid_filter`.

This is an intentionally low-risk stepping stone toward true streaming parse.

## Scope / Touchpoints

- `core/src/container.rs` (new `*_into` APIs)
- `core/src/excel_open_xml.rs` (use scratch buffers in the sheet/drawing/chart loops)

## Implementation Plan (Decision Complete)

1. Add a new API in `ZipContainer`:
   - `pub fn read_file_checked_into(&mut self, name: &str, dst: &mut Vec<u8>) -> Result<(), ContainerError>`
   - Behavior:
     - Clear `dst`.
     - Reserve capacity using a safe cap:
       - `reserve = min(size as usize, 64 * 1024 * 1024)` to avoid a single gigantic immediate alloc.
     - Read into `dst`.
   - Keep `read_file_checked(name) -> Vec<u8>` by implementing it as:
     - allocate local vec, call `read_file_checked_into`, return vec.

2. Update `open_workbook_from_container_with_grid_filter` to reuse scratch buffers.
   - Add scratch vecs outside loops:
     - `let mut sheet_buf = Vec::<u8>::new();`
     - `let mut rels_buf = Vec::<u8>::new();`
     - `let mut drawing_buf = Vec::<u8>::new();`
     - `let mut chart_buf = Vec::<u8>::new();`
   - Replace `container.read_file_checked(&target)` patterns with:
     - `container.read_file_checked_into(&target, &mut sheet_buf)?;` then parse from `&sheet_buf`.

3. Keep profiling counters intact.
   - Existing `OpenWorkbookProfile` should still reflect bytes read; make sure it uses `buf.len()` after reading.

## Measurement Plan

- Full perf cycle pre/post.
- Watch:
  - e2e parse time (might improve slightly),
  - peak memory (may become more stable / lower spikes).

## Acceptance Criteria

- No correctness changes.
- Any stable improvement in e2e parse time or peak memory reduction on large fixtures.

## Stop Conditions

- If buffer reuse introduces lifetime/aliasing mistakes, do not proceed; keep changes minimal and obviously correct.

---

# Experiment 4: SharedStrings-Aware Sheet Skipping When `sharedStrings.xml` Changes

## Objective

Currently, if `xl/sharedStrings.xml` differs, the fast OpenXML diff path conservatively parses all matched sheets. The goal is to still skip parsing sheets that provably do not depend on shared strings.

This is explicitly called out as a high-impact TODO in `docs/rust_docs/perf_next_todos.md`.

## Scope / Touchpoints

- `core/src/package.rs`:
  - `compute_sheet_grid_parse_targets(...)`
- Potentially a helper in `core/src/excel_open_xml.rs` or `core/src/grid_parser.rs` for cheap scanning.
- Add a new perf e2e fixture and test entry.

## Correctness Constraints

This must be conservative:
- False positives (parse more than necessary) are acceptable.
- False negatives (skip parsing when it was needed) are correctness bugs.

## Implementation Plan (Decision Complete)

1. Add a cheap sheet "uses shared strings" detector.
   - Function:
     - `fn sheet_uses_shared_strings(xml: &[u8]) -> bool`
   - Conservative detection:
     - Return true if the byte pattern `t=\"s\"` or `t='s'` exists.
     - If the XML is malformed or unusually encoded, return true.

2. Change the "sharedStrings changed" logic in `compute_sheet_grid_parse_targets`.
   - Today:
     - if sharedStrings differs, parse all matched sheets.
   - New behavior:
     - First, compare sheet fingerprints:
       - If `old_fp == new_fp`, the sheet bytes are identical, so it is safe to skip parsing regardless of sharedStrings.
     - If fingerprints differ and sharedStrings differs:
       - Read sheet XML bytes and run `sheet_uses_shared_strings`.
       - If it returns false for both sides, do not force parse due to sharedStrings; instead rely on fingerprints (already different) to decide parse as usual.
       - If it returns true for either side, keep the conservative parse behavior (force parse).

3. Add a targeted e2e fixture and perf test case.
   - Add new fixtures (exact names):
     - `e2e_p6_sharedstrings_changed_numeric_only_a.xlsx`
     - `e2e_p6_sharedstrings_changed_numeric_only_b.xlsx`
   - Construct them so:
     - `sharedStrings.xml` differs across A/B, but the main worksheet is numeric-only and identical (no `t="s"` usage).
     - At least one sheet should remain "fingerprint equal" across A/B to validate that skipping works.
   - Add a new ignored test in `core/tests/e2e_perf_workbook_open.rs`:
     - `e2e_p6_sharedstrings_changed_numeric_only()`

4. Add correctness tests.
   - Unit tests for `sheet_uses_shared_strings` with representative XML snippets.
   - Integration-style tests for parse target computation on a minimal workbook pair.

## Measurement Plan

- Full perf cycle pre/post (fast-diff behavior change).
- Specifically watch:
  - `e2e_p6_*` new test total/parse time.
  - No regressions in existing e2e cases.

## Acceptance Criteria

- No correctness regressions.
- New `p6` shows a significant parse-time reduction vs baseline behavior.

## Stop Conditions

- Any uncertainty about the detector's conservatism: keep behavior conservative (parse) and refine later.

---

# Experiment 5: Custom JSON Lines Writer for `JsonLinesSink` (Avoid `serde_json::to_writer` Per Op)

## Objective

Reduce time spent in op emission (`op_emit_time_ms`) for large diffs by implementing a custom JSONL writer for `DiffOp` and the header, behind a feature flag.

This is a direct continuation of the "custom-json" direction in `docs/rust_docs/custom_crates/custom_crate_code_experiment.md`, but scoped to JSONL emission first (fixed schema, easiest to parity test).

## Scope / Touchpoints

- `core/src/output/json_lines.rs` (sink implementation)
- Add new module:
  - `core/src/output/json_write.rs` (custom string escaping + primitive writers)
- Add new perf test target:
  - `core/tests/perf_jsonl_emit.rs` (ignored)
- New feature flag:
  - `custom-jsonl` in `core/Cargo.toml`

## Implementation Plan (Decision Complete)

1. Add feature flag.
   - `core/Cargo.toml`:
     - `custom-jsonl = []`
   - Default remains `serde_json` path.

2. Implement minimal, correct JSON escaping.
   - In `core/src/output/json_write.rs`:
     - `write_json_string(w: &mut impl Write, s: &str) -> io::Result<()>`
       - Escape `"`, `\\`, and control chars; use `\\u00XX` for bytes < 0x20 not covered by short escapes.
     - Primitive writers:
       - `write_u32`, `write_i64`, `write_bool`, `write_null`, etc.

3. Implement header writer and DiffOp writers.
   - Header must match existing schema and ordering:
     - `{ "kind": "Header", "version": "...", "strings": [...] }`
   - Implement a `DiffOp` JSON writer that:
     - handles every variant,
     - writes stable key order,
     - avoids allocations (write directly to `Write`).

4. Parity tests (mandatory).
   - For a corpus of `DiffOp` values covering all variants:
     - Serialize using serde_json and using custom writer.
     - Parse both lines as `serde_json::Value`.
     - Assert equality of the parsed values.
   - Include tricky strings:
     - quotes, backslashes, newlines, tabs, non-ASCII, and empty strings.

5. Add `perf_jsonl_emit` perf test (ignored).
   - Goal: exercise sink emission without writing to disk.
   - Pattern:
     - Build two large in-memory grids (or reuse an existing perf grid generator).
     - Diff them while writing JSONL to `std::io::sink()`.
     - Emit `PERF_METRIC jsonl_emit ... op_emit_time_ms=... total_time_ms=...` via existing DiffMetrics plumbing.

## Measurement Plan

- A:
  - `cargo test -p excel_diff --release --features perf-metrics --test perf_jsonl_emit -- --ignored --nocapture --test-threads=1`
- B:
  - same command with `--features "perf-metrics custom-jsonl"`
- If results are promising, run a full perf cycle (only if this affects shared metrics materially).

## Acceptance Criteria

- Correctness parity for all DiffOp variants.
- Performance: stable reduction in `op_emit_time_ms` on `perf_jsonl_emit` (target >= 10%).

## Stop Conditions

- If parity is hard due to schema drift or variant complexity, keep it feature-gated and do not promote; do not ship a partially-correct writer.

---

## Suggested Execution Order

1. Experiment 2 (ZIP name->index cache): low risk, broad benefit.
2. Experiment 4 (sharedStrings-aware skipping): likely big win on real-world fast-diff cases.
3. Experiment 1 (custom-xml numeric parsing fast paths): builds directly on proven wins.
4. Experiment 3 (scratch buffer reads): incremental improvement, reduces allocator churn, prepares for streaming.
5. Experiment 5 (custom JSONL): independent; improves large-output paths.

## Assumptions / Defaults

- Default perf-cycle run count is 3; parse-heavy changes require 5-run confirmation.
- Feature flags are preferred for A/B clarity.
- Correctness conservatism wins when uncertainty exists (especially for sharedStrings-related skipping).

