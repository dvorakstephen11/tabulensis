# Custom crate code experiment candidates (Rust)

See also:
- `docs/rust_docs/custom_crates/README.md` for experiment index and status.
- `docs/rust_docs/custom_crates/agentic_experiment_playbook.md` for execution guardrails.

## Scope and method
- Scanned Rust workspace crates: `core`, `cli`, `wasm`, `ui_payload`, `desktop/backend`, `desktop/wx`, `license_client`, `license_service`.
- Focused on hot-path parsing/serialization and widely-used dependencies where a custom, narrower implementation could be faster or smaller.
- References below point to current usage sites.
- Default policy: **always run the full test suite and perf-metrics** (baseline + post-change). Skipping tests/perf is not allowed unless the user explicitly asks to skip.

## Experiment plan (detailed)

### Goals
- Measure **true performance impact** (CPU time + throughput) of custom code vs. the existing crates.
- Measure **memory impact** (peak and steady-state allocations).
- Measure **binary size and dependency footprint** changes.
- Confirm **behavioral equivalence** across realistic and adversarial inputs.

### Profiling gate (pre-rewrite)
- Before implementing a rewrite, capture at least one flamegraph/perf profile on a representative workload.
- Proceed only if the target crate/function is a meaningful contributor (e.g., >=1-2% of total time).

### Baseline and variants (A/B/C)
For each candidate, run the following variants on the same machine and inputs:
- **A: Baseline** – current implementation using the third-party crate.
- **B: Custom** – new custom implementation behind a feature flag.
- **C: Parity tests (optional)** – build with both implementations enabled and run side-by-side tests; keep any runtime fallback strictly `cfg(test)` / dev-only (not for release).

Implementation rules:
- Add **feature flags** per candidate (e.g., `custom-xml`, `custom-zip`, `custom-json`, `custom-base64`, `custom-lru`).
- Make third-party deps **optional** and gate them behind a **baseline feature** (enabled by default) so custom builds can actually remove them from the graph.
- Keep **default** behavior on baseline crates until the experiment concludes.
- Prefer **parity tests** over runtime fallback paths; if you add a fallback for differential testing, keep it test-only.
- Avoid combining multiple custom flags in the same run except for a final “all-on” sanity check.

### Documentation & iteration protocol (required)
Each candidate has a **dedicated experiment doc** that interleaves GPT-5.2-Pro guidance with measured results.

Workflow (repeat until complete):
1) **Start doc with the initial GPT-5.2-Pro response file** (verbatim content).
2) **Establish a baseline** before any code changes:
   - Run full tests and perf-metrics (see “Default test/perf policy” below).
   - Record environment details and raw `PERF_METRIC` lines.
3) **Implement the suggestions** from the current GPT-5.2-Pro response.
4) **Run full tests and perf-metrics again** on the same machine and datasets.
5) **Append results to the experiment doc**:
   - Include command list, pass/fail status, raw metrics, and **delta vs baseline**.
6) **Send updated results + codebase to GPT-5.2-Pro**, save the response to a file.
7) **Append the new GPT-5.2-Pro response file** to the doc and repeat.

Completion criteria:
- A final section in the doc summarizes the decision (ship, keep behind flag, or delete) and the evidence.
- The doc includes **baseline + post-change metrics for each iteration**.

Naming guidance:
- One file per candidate (e.g., `base64_custom_crate_experiment.md`).
- Append each GPT-5.2-Pro response file and each measurement block in chronological order.

### Workloads and datasets
Use deterministic fixture generation and a tiered workload matrix:
- **Correctness fixtures**: `fixtures/manifest_cli_tests.yaml`
- **Performance fixtures**: `fixtures/manifest_perf_e2e.yaml`
- **Release smoke fixtures**: `fixtures/manifest_release_smoke.yaml`
- Add targeted new fixtures for each candidate (see per-candidate plans), using `fixtures/src/generators` and `fixtures/manifest.yaml`.

Workload tiers:
1) **Small** (unit-scale): fast local iteration, per-commit.
2) **Medium** (CI-scale): regular suite, nightly or pre-merge.
3) **Large** (stress/perf): performance runs, manual or scheduled.

### Metrics and instrumentation
Collect **both end-to-end and microbench metrics**:

**End-to-end metrics**
- Workbook open time (parse-only, no diff).
- Full diff time on large grids.
- Output serialization time (JSON/JSONL).
- Peak memory (use `perf-metrics` + `CountingAllocator`).

**Microbenchmarks**
- `parse_sheet_xml`, `parse_shared_strings`.
- ZIP part lookup + read.
- DataMashup base64 decoding.
- JSON Lines emission (`DiffOp` stream).

**Binary size / dependency metrics**
- `strip`ped binary size for `cli` and `desktop_wx`.
- Dependency changes using `cargo tree -i <crate>`.
- Optional: `cargo bloat` if available.

**Build time metrics**
- Clean build time for `cli` and `desktop_wx`.
- Incremental build time for `cli` and `desktop_wx` after a small code change.

**Consistency & correctness metrics**
- Parse success rate (must match baseline).
- Diff output hash (same schema and semantic content).
- Stable error codes for corrupt inputs.

### Statistical rigor
- Use **criterion** for microbenchmarks (existing `core/benches/diff_benchmarks.rs`).
- For end-to-end runs: **5–10 iterations**, discard first run (warm-up), report median and p95.
- Record machine details: CPU model, OS, Rust version, and whether CPU scaling is enabled.

### Automation / runner
- Add a single script or `xtask` that builds each variant with fixed flags/profiles, runs the same workloads, and stores JSON/CSV results.
- Embed feature set, commit hash, and environment details in the output to prevent apples-to-oranges comparisons.
- The runner **must** execute tests and perf-metrics by default; it should require explicit opt-out flags to skip.

### Default test/perf policy (must-follow)
For every iteration (baseline and post-change), run:
- **Tests**: `cargo test -p excel_diff`
- **Perf-metrics**: `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`

Optional but encouraged when relevant:
- Criterion microbench: `cargo bench -p excel_diff <bench_name>`
- Isolated perf tests when allocator noise is a concern (run the perf test as a single target).

### Test suite expansion
Add tests that **force edge conditions** for the custom code:

**XML parsing (custom-xml)**
- Shared strings with mixed `<t>` runs, whitespace, and escaped entities.
- Inline strings with nested tags.
- Large sheets with sparse cells and missing `<dimension>` tags.
- CDATA segments for DataMashup XML.

**ZIP parsing (custom-zip)**
- Stored vs deflated entries.
- Duplicated filenames in central directory.
- Truncated and malformed central directory.
- Nested ZIP (DataMashup) with deep paths and large files.

**JSON output / parsing (custom-json)**
- Roundtrip tests vs `serde_json` for a corpus of `DiffOp` and `DiffReport`.
- Escaping correctness for quotes, newlines, Unicode, and control chars.
- Streaming contract validation (`docs/streaming_contract.md`).

**Base64 (custom-base64)**
- Arbitrary whitespace in base64 payloads.
- Invalid characters and padding errors.
- Very large payloads (multi-MB).

**LRU (custom-lru)**
- Eviction order correctness.
- “Hit” semantics identical to `lru` crate.

### Differential testing (parity)
For each candidate:
- Add **side-by-side tests** that run both implementations on the same input and compare output (or error codes).
- For large outputs, compare hashes or normalized JSON.
- Keep any fallback paths **test-only**; do not ship runtime fallback in release builds.

### Decision criteria
Promote custom code if it meets **all**:
- **No functional regressions** on correctness fixtures.
- **Performance gain** in targeted hot paths (aim ≥10–20% on at least one representative workload).
- **No significant memory regression** (≤5% worse peak).
- **Clear complexity reduction** (fewer deps or materially simpler code).

If results are mixed, keep feature flag off and document follow-up improvements.

### Reporting template
Each candidate’s doc **must** interleave guidance and results:
1) GPT-5.2-Pro response (verbatim)
2) Baseline test + perf results (commands + raw `PERF_METRIC` lines)
3) Implementation summary
4) Post-change test + perf results (commands + raw `PERF_METRIC` lines + delta vs baseline)
5) Follow-up GPT-5.2-Pro response (verbatim)
6) Repeat as needed

For each measurement block, include:
- Workload name
- Baseline median / p95 (if applicable)
- Post-change median / p95 (if applicable)
- % change
- Peak memory delta
- Binary size delta
- Notes / regressions

---

## High-impact candidates

### 1) `quick-xml` -> custom minimal XML scanner
**Where used**
- `core/src/grid_parser.rs` (sheet XML, shared strings, workbook/relationships parsing).
- `core/src/excel_open_xml.rs` (workbook open + chart/drawing parsing).
- `core/src/datamashup_framing.rs` and `core/src/datamashup.rs` (DataMashup XML).
- `desktop/wx/src/xrc_validation.rs` (UI XRC validation; not hot path).

**Why it might be suboptimal here**
- Parsing large sheets (`parse_sheet_xml`) and shared strings uses event-based XML parsing and allocates `String`s for values and formulas per cell. This is likely a top CPU+allocation hotspot when opening big workbooks.
- The parser needs only a small, predictable subset of Excel XML (e.g., `<dimension>`, `<c>`, `<v>`, `<f>`, `<is><t>`). A full XML parser adds overhead for generality (namespaces, uncommon constructs) you may not need.

**Custom replacement direction**
- Implement a small, byte-oriented scanner for the Excel subset:
  - Detect `<c ...>` start tags, parse `r` and `t` attributes, then read child `<v>`/`<f>`/`<is><t>` text.
  - For `sharedStrings.xml`, scan `<si>` and collect `<t>` text runs, concatenate into the pool.
  - Provide minimal XML entity unescape (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`) only where needed.
- Keep `quick-xml` for non-hot-path modules (like XRC validation) or hide behind a feature flag while experimenting.

**Potential benefits**
- Fewer allocations and lower CPU in workbook open, especially on large sheets.
- Opportunity to remove `quick-xml` from `core` if custom parser fully covers Excel parsing.

**Risks / costs**
- Correctness risk with edge-case XML (namespaces, CDATA, odd whitespace, mixed text nodes).
- Need strong test coverage across varied real-world files.

**Experiment plan**
- Create `custom-xml` feature flag and run A/B against baseline.
- Add microbenchmarks for `parse_sheet_xml` and `parse_shared_strings`.
- Expand fixtures: add cases for escaped entities, missing `<dimension>`, and large sparse sheets.
- Add parity tests that run both parsers and compare `Grid` + shared string outputs.
- Use end-to-end workbook open perf runs (`manifest_perf_e2e.yaml`) plus `e2e_perf_workbook_open` test.

---

### 2) `zip` -> custom minimal ZIP reader
**Where used**
- `core/src/container.rs` (workbook ZIP access).
- `core/src/datamashup_package.rs` (nested ZIP parsing inside DataMashup).

**Why it might be suboptimal here**
- `ZipArchive` is general-purpose and uses a `Read + Seek` trait object; `by_name` often implies a search and each part is fully read into a new `Vec<u8>`.
- Nested ZIP handling in DataMashup adds repeated archive parsing overhead.

**Custom replacement direction**
- Implement a minimal ZIP reader limited to deflate/stored entries and central-directory lookup.
- Build a single `HashMap<String, EntryInfo>` index so repeated `read_file_checked()` is O(1).
- Use a streaming inflate (e.g., miniz_oxide) to avoid materializing large buffers when a streaming parser is feasible.
- Keep current ZIP-bomb limits and error codes.

**Potential benefits**
- Faster workbook open and lower overhead on repeated part reads.
- Smaller dependency tree if `zip` is removed from `core`.

**Risks / costs**
- ZIP format edge cases and error handling complexity.
- More code to maintain; careful security auditing required.

**Experiment plan**
- Prototype a read-only ZIP indexer with deflate+stored support; gate with `custom-zip`.
- Add microbenchmarks for repeated `read_file_checked()` calls on common parts.
- Add fixtures for malformed ZIPs, duplicate entries, and truncated central directory.
- Add parity tests on a corpus of fixtures to ensure identical error codes.
- Reuse and expand `core/tests/excel_open_xml_tests.rs` + `core/tests/data_mashup_tests.rs`.

---

### 3) `serde_json` -> custom JSON writer + targeted JSON parser
**Where used**
- Output: `core/src/output/json.rs`, `core/src/output/json_lines.rs`, `cli/src/output/json.rs`, `wasm/src/lib.rs`, `desktop/wx/src/main.rs`.
- DataModelSchema parsing: `core/src/tabular_schema.rs`.

**Why it might be suboptimal here**
- `tabular_schema.rs` loads the entire DataModelSchema into a `serde_json::Value` tree even though only a few fields are used (tables, columns, measures, relationships). This can be memory-heavy.
- Large diff outputs serialize many ops via `serde_json`, which allocates intermediate structures and escapes via a general serializer.

**Custom replacement direction**
- Implement a streaming JSON visitor specifically for DataModelSchema that extracts only the needed fields.
- Implement a specialized JSON writer for `DiffOp` / `DiffReport` / JSONL that writes directly to `Write` without building intermediate values.

**Potential benefits**
- Reduced memory and CPU for both schema parsing and large output generation.
- Potential to remove `serde_json` from `core` output paths if you keep it only in peripheral crates.

**Risks / costs**
- Manual JSON correctness (escaping, numeric formatting, optional fields) needs careful tests.
- If public schema changes, custom parser must be updated.

**Experiment plan**
- Start with JSONL output (`core/src/output/json_lines.rs`), schema fixed by `docs/streaming_contract.md`.
- Add `custom-json` feature and run A/B serialization throughput tests.
- Add roundtrip parity tests against `serde_json` for `DiffOp` and `DiffReport`.
- Add a DataModelSchema streaming parser and validate with targeted fixtures.
- Include output-size and allocation metrics in perf runs.

---

### 4) `base64` -> custom whitespace-ignoring decoder for DataMashup
**Where used**
- `core/src/datamashup_framing.rs` (`decode_datamashup_base64`).

**Why it might be suboptimal here**
- The current path allocates a new `String` by `split_whitespace().collect()` before decoding. That is a full copy of potentially large DataMashup payloads.

**Custom replacement direction**
- Implement a tiny base64 decoder that skips ASCII whitespace while decoding directly into a `Vec<u8>`.
- Keep the existing error mapping (`DataMashupError::Base64Invalid`).

**Potential benefits**
- Small but measurable reduction in allocation and copy time when DataMashup payloads are large.
- Easy and low-risk experiment.

**Risks / costs**
- Must match base64 semantics exactly; needs tests for whitespace cases.

**Experiment plan**
- Add `custom-base64` feature and compare decode throughput on large DataMashup payloads.
- Use `core/tests/data_mashup_tests.rs` plus new whitespace-heavy and invalid cases.
- Ensure parity: decode output bytes must match baseline exactly.

---

### 5) `lru` -> tiny fixed-size cache (desktop backend)
**Where used**
- `desktop/backend/src/diff_runner.rs` (`LruCache` with capacity 4/2).

**Why it might be suboptimal here**
- The cache sizes are tiny and predictable; a general LRU adds dependency cost and overhead.

**Custom replacement direction**
- Implement a simple fixed-size array or `VecDeque`-based LRU specialized for small `N`.

**Potential benefits**
- Smaller dependency tree and simpler code.
- Possibly faster with linear scans at tiny `N`.

**Risks / costs**
- Low; behavior is easy to test.

**Experiment plan**
- Replace `LruCache` with a fixed-size cache behind `custom-lru`.
- Add unit tests for eviction order and cache hits.
- Benchmark a synthetic cache workload (optional; small wins expected).

---

### 6) `globset` / `walkdir` -> custom path matching (low priority)
**Where used**
- `desktop/backend/src/batch.rs` (batch compare include/exclude globs).

**Why it might be suboptimal here**
- If UI-provided globs are simple (e.g., `*.xlsx`, `**/*.xlsx`), globset's regex compilation is more machinery than needed.

**Custom replacement direction**
- Implement a minimal glob matcher (`*`, `?`, `**`) and a custom directory walk.

**Potential benefits**
- Smaller dependency tree; possible speed improvement for huge directory trees.

**Risks / costs**
- Feature regression on complex glob patterns.
- Best only if UI constrains glob syntax.

**Experiment plan**
- Treat as opt-in: add `custom-glob` feature and keep default behavior unchanged.
- Create a compatibility test suite that mirrors the current `globset` behavior on a corpus of patterns.
- Profile only if batch workloads are a measurable bottleneck.

---

## Likely non-candidates (already optimized / risky to replace)
- `xxhash-rust` and `rustc-hash` are already fast in hot paths and likely not worth reimplementing.
- `rayon` is optional and provides well-optimized parallelism; custom thread pools are unlikely to outperform it without significant effort.

## Suggested experiment order
1) `quick-xml` (sheet parsing) and `zip` (container access) – biggest potential wins.
2) `serde_json` output + DataModelSchema parsing – medium wins with larger code changes.
3) `base64` and `lru` – small, low-risk wins.

## Measurement hooks already in the repo
- `core/benches/diff_benchmarks.rs` (criterion)
- `core/tests/e2e_perf_workbook_open.rs`
- `perf-metrics` feature gates (phase timing in core)
