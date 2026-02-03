# Base64 Custom Crate Experiment (Combined)

## Table of Contents
- [custom_crate_code_experiment.md](#customcratecodeexperiment-md)
- [base64_custom_crate_code_experiment.md](#base64customcratecodeexperiment-md)
- [base64_custom_crate_code_experiment_part_2.md](#base64customcratecodeexperimentpart2-md)
- [final_base64_custom_crate_experiments.md](#finalbase64customcrateexperiments-md)
- [perf_findings_custom_base64.md](#perffindingscustombase64-md)

---

# custom_crate_code_experiment.md

# Custom crate code experiment candidates (Rust)

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


---

# base64_custom_crate_code_experiment.md

## Improvements I’d make to the experiment plan

A lot of your plan is already solid (A/B/C variants, tiered workloads, criterion for microbenches, explicit dependency + binary-size checks).  
Here are the main “make this easier to trust / easier to execute” improvements I’d add:

1. **Make dependency-footprint measurements “real” by ensuring the crate actually leaves the graph.**
   Your plan includes `cargo tree -i <crate>` and binary-size deltas , but for many candidates you won’t see meaningful changes unless you:

   * make the third-party dependency `optional = true`, and
   * gate it behind a “baseline feature” (enabled by default), and
   * ensure your **custom** configuration *does not* enable that baseline feature.
     Otherwise Cargo will still build/track the dependency even if your code path no longer references it.

2. **Add a “profiling sanity check” gate before implementing a rewrite.**
   You note “hot-path parsing/serialization” , but I’d formalize: *before* rewriting, capture at least one flamegraph/perf profile of the representative workload, and require that the target crate/function shows up meaningfully. This prevents investing in a rewrite whose ceiling is <1–2%.

3. **Prefer “parity tests” over runtime fallback for C.**
   You already call out C as “for differential testing only” . I’d go further:

   * keep C as *tests-only* (or at least `cfg(test)` / dev-only code),
   * and implement “side-by-side” parity tests that exercise both implementations on the same inputs (you already intend this) .
     Runtime fallback paths are easy to accidentally ship and tend to complicate code/branching in hot paths.

4. **Add one more metric: compile time / incremental build time.**
   Since part of the motivation is “simpler and smaller,” tracking “time to build `cli` + `desktop_wx`” can be valuable alongside binary size. It’s often where removing dependencies pays off the most.

5. **Automate the A/B/C runner (so you don’t accidentally compare apples to oranges).**
   Your plan requires multiple iterations, warm-up discard, median + p95, and recording environment details . I’d strongly recommend a single command (script or `xtask`) that:

   * builds each variant with fixed flags/profiles,
   * runs the same workload set,
   * stores JSON/CSV results with the exact feature set and commit hash embedded.

---

## Candidate to rewrite first: `base64` for DataMashup decoding

This is the cleanest low-risk candidate you identified: `decode_datamashup_base64` currently does a full-copy `String` allocation via `split_whitespace().collect()` before decoding, which can be expensive for multi‑MB DataMashup payloads. 
You also already scoped a focused replacement: a strict, whitespace-ignoring decoder that preserves the existing `DataMashupError::Base64Invalid` mapping. 

Below is a **drop-in custom replacement** that:

* decodes *standard* Base64 (RFC 4648 alphabet, `=` padding),
* ignores ASCII whitespace while decoding (space, tab, CR, LF, VT, FF),
* enforces **canonical padding** and rejects non-canonical trailing bits (matching `base64::engine::general_purpose::STANDARD`’s strict behavior),
* and maps errors to your existing error code path.

It is also structured so you can:

* run **baseline** using the `base64` crate,
* run **custom** using your decoder,
* run **parity tests** with both enabled.

---

## Code: custom base64 decoder + wiring

### 1) `core/Cargo.toml` changes

Make `base64` optional so you can actually measure dependency footprint deltas (and keep it enabled by default for baseline behavior). Your current `core/Cargo.toml` includes `base64 = "0.22"` unconditionally. 

Replace that part with:

```toml
# core/Cargo.toml

[features]
# add base64-crate by default so normal builds keep working
default = ["excel-open-xml", "std-fs", "vba", "dpapi", "base64-crate"]

# baseline implementation (third-party crate)
base64-crate = ["dep:base64"]

# custom implementation (this experiment)
custom-base64 = []

# ... existing features unchanged ...
excel-open-xml = []
vba = ["dep:ovba"]
std-fs = []
perf-metrics = []
dev-apis = []
model-diff = []
legacy-api = []
parallel = ["dep:rayon"]
dpapi = []

[dependencies]
# was: base64 = "0.22"
base64 = { version = "0.22", optional = true }

# ... rest unchanged ...
```

### 2) Update dependents that use `default-features = false`

Because `ui_payload` and `desktop_backend` depend on `excel_diff` with `default-features = false`, they must explicitly enable *some* base64 provider now.

* `ui_payload/Cargo.toml` currently: 

```toml
excel_diff = { path = "./core", default-features = false, features = ["excel-open-xml"] }
```

Change to:

```toml
excel_diff = { path = "./core", default-features = false, features = ["excel-open-xml", "base64-crate"] }
```

* `desktop/backend/Cargo.toml` currently: 

```toml
excel_diff = { path = "././core", default-features = false, features = ["excel-open-xml", "vba"] }
```

Change to:

```toml
excel_diff = { path = "././core", default-features = false, features = ["excel-open-xml", "vba", "base64-crate"] }
```

(If any other workspace crates also use `default-features = false` on `excel_diff`, they’ll need the same update.)

### 3) Add the custom decoder module

Create a new file:

#### `core/src/custom_base64.rs`

```rust
//! Minimal, strict Base64 decoder for Tabulensis.
//!
//! Intended for hot paths where the input may contain ASCII whitespace (line breaks, indentation).
//!
//! Behavior matches a strict RFC 4648 "standard" Base64 decoder with canonical padding:
//! - Alphabet: A-Z a-z 0-9 + /
//! - Padding: '=' (canonical only; rejects '=' in the first 2 positions of a quad)
//! - Rejects non-canonical trailing bits when padding is present
//! - Ignores ASCII whitespace: space, tab, CR, LF, VT, FF

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodeError {
    InvalidByte(u8),
    InvalidLength,
    InvalidPadding,
    InvalidTrailingBits,
    TrailingData,
}

#[inline]
fn is_ascii_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
}

/// Returns the Base64 sextet value (0..=63), or 64 for '=', or None for invalid bytes.
#[inline]
fn decode_sextet(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        b'=' => Some(64),
        _ => None,
    }
}

/// Decode Base64 while ignoring ASCII whitespace.
///
/// This is intentionally strict:
/// - Requires canonical padding (length divisible by 4 after whitespace removal).
/// - Rejects non-canonical trailing bits when '=' padding is present.
///
/// On success returns the decoded bytes.
pub(crate) fn decode_standard_ws(input: &str) -> Result<Vec<u8>, DecodeError> {
    let bytes = input.as_bytes();

    // Rough prealloc: includes whitespace but avoids realloc in most realistic inputs.
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);

    let mut quad = [0u8; 4];
    let mut q_len: usize = 0;
    let mut finished = false;

    for &b in bytes {
        if is_ascii_ws(b) {
            continue;
        }

        let v = decode_sextet(b).ok_or(DecodeError::InvalidByte(b))?;

        if finished {
            return Err(DecodeError::TrailingData);
        }

        quad[q_len] = v;
        q_len += 1;

        if q_len == 4 {
            let a = quad[0];
            let b = quad[1];
            let c = quad[2];
            let d = quad[3];

            // '=' is not allowed in the first two positions.
            if a == 64 || b == 64 {
                return Err(DecodeError::InvalidPadding);
            }

            if c == 64 {
                // Must be "xx=="
                if d != 64 {
                    return Err(DecodeError::InvalidPadding);
                }

                // Canonical trailing bits: low 4 bits of b must be zero.
                if (b & 0x0F) != 0 {
                    return Err(DecodeError::InvalidTrailingBits);
                }

                out.push((a << 2) | (b >> 4));
                finished = true;
            } else if d == 64 {
                // Must be "xxx="
                // Canonical trailing bits: low 2 bits of c must be zero.
                if (c & 0x03) != 0 {
                    return Err(DecodeError::InvalidTrailingBits);
                }

                out.push((a << 2) | (b >> 4));
                out.push(((b & 0x0F) << 4) | (c >> 2));
                finished = true;
            } else {
                // Full quad: "xxxx"
                out.push((a << 2) | (b >> 4));
                out.push(((b & 0x0F) << 4) | (c >> 2));
                out.push(((c & 0x03) << 6) | d);
            }

            q_len = 0;
        }
    }

    // Must end on a quad boundary.
    if q_len != 0 {
        return Err(DecodeError::InvalidLength);
    }

    Ok(out)
}
```

### 4) Wire it into `datamashup_framing.rs`

In `core/src/lib.rs`, add the module behind the feature:

```rust
// core/src/lib.rs

#[cfg(feature = "custom-base64")]
mod custom_base64;
```

Now update `core/src/datamashup_framing.rs` where `decode_datamashup_base64` is defined. Your current implementation uses `split_whitespace().collect()` and `STANDARD.decode(...)`. 

Replace the decoding section with this feature-switched implementation:

```rust
// core/src/datamashup_framing.rs

#[cfg(all(not(feature = "custom-base64"), not(feature = "base64-crate")))]
compile_error!(
    "No Base64 backend selected. Enable feature \"base64-crate\" (default) or \"custom-base64\"."
);

#[cfg(feature = "base64-crate")]
use base64::engine::general_purpose::STANDARD;
#[cfg(feature = "base64-crate")]
use base64::Engine;

#[cfg(feature = "custom-base64")]
use crate::custom_base64;

// Baseline: third-party base64 crate (current behavior).
#[cfg(feature = "base64-crate")]
fn decode_datamashup_base64_crate(text: &str) -> Result<Vec<u8>, DataMashupError> {
    // DataMashup base64 often contains line breaks; strip whitespace first.
    let cleaned: String = text.split_whitespace().collect();
    STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| DataMashupError::Base64Invalid)
}

// Custom: strict decoder that ignores ASCII whitespace while decoding.
#[cfg(feature = "custom-base64")]
fn decode_datamashup_base64_custom(text: &str) -> Result<Vec<u8>, DataMashupError> {
    custom_base64::decode_standard_ws(text).map_err(|_| DataMashupError::Base64Invalid)
}

pub fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, DataMashupError> {
    #[cfg(feature = "custom-base64")]
    {
        return decode_datamashup_base64_custom(text);
    }

    #[cfg(not(feature = "custom-base64"))]
    {
        return decode_datamashup_base64_crate(text);
    }
}
```

### 5) Add/extend tests for whitespace + parity

In `core/src/datamashup_framing.rs`’s existing tests module, add:

```rust
#[test]
fn decode_datamashup_base64_ignores_ascii_whitespace() {
    // "QQ==" -> b"A"
    let text = "  Q Q==\n\t\r";
    let out = decode_datamashup_base64(text).unwrap();
    assert_eq!(out, b"A");
}

#[cfg(all(feature = "custom-base64", feature = "base64-crate"))]
#[test]
fn custom_base64_matches_base64_crate_for_various_lengths() {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    // Deterministic "pseudo-random" bytes without adding a rand dependency.
    fn make_bytes(len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len);
        let mut x: u32 = 0x1234_5678;
        for _ in 0..len {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            v.push((x >> 24) as u8);
        }
        v
    }

    for &len in &[0usize, 1, 2, 3, 10, 57, 58, 59, 60, 128, 1024] {
        let data = make_bytes(len);
        let b64 = STANDARD.encode(&data);

        // Insert whitespace to simulate DataMashup formatting.
        let mut spaced = String::with_capacity(b64.len() + (b64.len() / 10) + 8);
        for (i, ch) in b64.chars().enumerate() {
            if i % 60 == 0 {
                spaced.push('\n');
            }
            spaced.push(ch);
            if i % 13 == 0 {
                spaced.push(' ');
            }
        }

        let a = decode_datamashup_base64_crate(&spaced).unwrap();
        let b = decode_datamashup_base64_custom(&spaced).unwrap();

        assert_eq!(a, data);
        assert_eq!(b, data);
        assert_eq!(a, b);
    }
}
```

---

## How you run the A/B/C variants for this candidate

* **A (baseline)**: default features (includes `base64-crate`)
  `cargo test -p excel_diff`
  `cargo bench -p excel_diff`

* **B (custom, no base64 crate in the graph)**:
  `cargo test -p excel_diff --no-default-features --features "excel-open-xml std-fs vba dpapi custom-base64"`
  (Adjust the feature list to match what you need enabled in that build.)

* **C (parity tests)**: enable both so you can compare implementations side-by-side
  `cargo test -p excel_diff --features "custom-base64"`
  (Default still includes `base64-crate`, so parity tests compile/run.)

This aligns with your A/B/C approach and parity testing goals.  

---

If you want, after you run this once, the next best “small and safe” follow-up is the `lru` replacement in `desktop/backend/src/diff_runner.rs` (tiny cache sizes, easy to validate, immediate dependency removal).


---

# base64_custom_crate_code_experiment_part_2.md

Here’s my take after reading the updated code + the base64 findings.

## What looks solid in the experiment setup

* You kept the experiment cleanly feature-gated (`base64-crate` vs `custom-base64`), with a compile-time “must pick one backend” guard.  
* You added both:

  * a Criterion microbench (`datamashup_base64_decode`) and
  * a perf-metrics harness (`e2e_perf_datamashup_decode`)
    and the report captures how to run both A/B. 
* You also added a parity test that compares outputs when both backends are enabled, which is exactly the kind of safety net you want during these rewrites. 

Overall: the *mechanics* of the experiment are in good shape.

## What the base64 results are telling you

On the current fixture set, the custom decoder is **much slower**:

* Criterion: ~+76–93% time regression (throughput drops from ~605–629 MiB/s to ~338–340 MiB/s). 
* Perf-metrics decode loop: 17ms → 21ms over 100 iterations (~23% slower in that run), but with **lower peak memory** (34503 → 29675 bytes, ~14% less). 

That’s consistent with what you’d expect: the `base64` crate is already very optimized, while the custom decoder is scalar + branchy. Beating it on throughput will likely require lookup tables + chunking and/or SIMD, which is a big complexity jump.

So: **as a performance win, custom-base64 isn’t there yet.** As a **memory/alloc reduction** idea, it’s directionally correct.

## Biggest “make this experiment more successful” change: fix the *bigger* copy first

Right now `read_datamashup_text()` clones the entire `<DataMashup>...</DataMashup>` content into `found_content`:

```rust
found_content = Some(content.clone());
```

That means you temporarily hold **two copies** of the (potentially huge) base64 payload in memory, and you also pay the full copy cost. 

This is likely *much* larger than the `split_whitespace().collect()` allocation you targeted in the base64 plan. 

### Patch: move the string instead of cloning

In `core/src/datamashup_framing.rs`, change the `End(DataMashup)` handler to:

```rust
found_content = Some(std::mem::take(&mut content));
```

This preserves the “detect duplicate DataMashup elements later” behavior (because `found_content.is_some()` still trips), but avoids the huge clone. 

If you only make one change before the next bench run, make this one.

## Second high-value change: keep the `base64` crate, but stop building a `String` just to strip whitespace

Your baseline crate backend currently does:

```rust
let cleaned: String = text.split_whitespace().collect();
STANDARD.decode(cleaned.as_bytes())
```

That’s a full pass + allocation + copy into a UTF-8 `String`. 

You can get most of the memory benefit you were aiming for **without** paying the throughput hit of the custom decoder, by stripping ASCII whitespace into a `Vec<u8>` (or doing a fast-path when there is no whitespace at all).

### Patch: “hybrid” crate backend (fast path + byte-strip)

Suggested replacement for `decode_datamashup_base64_crate`:

* **Fast path**: if there’s no ASCII whitespace, decode directly from `text.as_bytes()` (no intermediate allocation).
* **Slow path**: copy only non-whitespace bytes into a `Vec<u8>` and decode that.

Also, this can fix the “dead_code when `custom-base64` is enabled” warning you saw, by compiling the crate-backend function only in `test` builds when `custom-base64` is enabled. 

**Drop-in code (core/src/datamashup_framing.rs):**
*(Citations omitted inside code block per tooling rules; see surrounding text for sources.)*

```rust
// Only compile the crate backend when:
// - custom-base64 is NOT enabled (normal baseline build), OR
// - we're in unit tests (parity tests may call the crate backend)
#[cfg(all(feature = "base64-crate", any(test, not(feature = "custom-base64"))))]
use base64::engine::general_purpose::STANDARD;
#[cfg(all(feature = "base64-crate", any(test, not(feature = "custom-base64"))))]
use base64::Engine;

// Baseline: third-party base64 crate, but avoid allocating a cleaned String.
#[cfg(all(feature = "base64-crate", any(test, not(feature = "custom-base64"))))]
fn decode_datamashup_base64_crate(text: &str) -> Result<Vec<u8>, DataMashupError> {
    let bytes = text.as_bytes();

    // Fast path: no ASCII whitespace -> decode directly, zero extra allocation.
    if !bytes.iter().any(|b| b.is_ascii_whitespace()) {
        return STANDARD
            .decode(bytes)
            .map_err(|_| DataMashupError::Base64Invalid);
    }

    // Slow path: strip ASCII whitespace into a byte buffer (cheaper than split_whitespace + String).
    let mut cleaned = Vec::with_capacity(bytes.len());
    for &b in bytes {
        if !b.is_ascii_whitespace() {
            cleaned.push(b);
        }
    }

    STANDARD
        .decode(&cleaned)
        .map_err(|_| DataMashupError::Base64Invalid)
}
```

This directly targets the “full copy of potentially large DataMashup payloads” concern called out in your plan. 

Even if you ultimately keep the base64 crate, this makes the *baseline* meaningfully better and raises the bar for a custom rewrite (which is good science).

## Third: your perf-metrics decode harness is currently *too capped* to be very informative for timing

In `iterations_for_payload`, you compute iterations to reach 50 MiB but then cap it at 100:

```rust
let mut iterations = (target_bytes / decoded).max(1) as usize;
if iterations > 100 { iterations = 100; }
```



But your largest fixture is currently tiny (4828 chars → 3620 decoded bytes). 
With the cap, you’re only decoding ~362 KB total, which makes timing more sensitive to noise and overhead.

### Patch: raise the cap (or cap by a larger number)

A simple improvement is to cap at something like 50,000 (still safe, but now you’ll actually approach the target bytes for small fixtures):

```rust
if iterations > 50_000 {
    iterations = 50_000;
}
```

Or remove the cap entirely and rely on `target_bytes` (but that can go huge for very small decoded sizes). 

This doesn’t change correctness—just makes the printed `parse_time_ms` a more meaningful signal.

## Fourth: your fixture set doesn’t yet match the plan’s “multi‑MB base64 payload” assumption

Your plan explicitly calls out “very large payloads (multi‑MB)” for base64. 
But the harness report shows the largest payload in fixtures right now is only ~4.8 KB of base64 text. 

That mismatch matters because:

* the “extra pass / extra allocation” penalties you’re chasing are much more visible at MB scale,
* and decode throughput is often measured more reliably with larger buffers.

So to increase experiment success odds, I’d add **one intentionally large DataMashup payload fixture** (or a synthetic bench input) before iterating further on custom-base64.

A practical approach:

* generate a large byte blob (e.g., 5–50 MiB),
* base64-encode it,
* insert line breaks every ~76 chars + some indentation spaces,
* feed that into the decode benches directly (even without going through a full `.xlsx` fixture).

## What I’d do next (base64-specific)

Given the current measurements:

1. Keep `base64-crate` as the default backend. 
2. Apply the two “cheap wins” above:

   * remove the `content.clone()` in `read_datamashup_text` 
   * change whitespace removal to a byte-strip + fast path in the crate backend 
3. Re-run the same A/B benches/harness.
4. Only if you still want a custom decoder after that:

   * switch the custom decoder to a 256-entry decode table (avoid `match` per byte),
   * reduce branches and per-byte state updates,
   * and only then consider SIMD (which is where `base64` crates often win).

## One meta-change to make the overall “custom crates” experiment more successful

This base64 run is a good reminder: **don’t just replace crates—first make sure you’re benchmarking the best use of the crate.**

For base64, the biggest “crate-related” cost may not be base64 decoding at all—it’s the *prep work* (`split_whitespace` + `String` allocation) and even your own extra copies (`content.clone()`).  
Optimizing those first can get you 80% of the benefit with 5% of the risk.

If you want, I can also suggest the **next best candidate** to rewrite based on “high likelihood of win” given what’s in your plan (my vote would be the tiny `lru` replacement for dependency reduction, or the `serde_json` JSONL writer for throughput).


---

# final_base64_custom_crate_experiments.md

Based on what you measured, the “custom base64 decoder” itself isn’t paying for its complexity yet — but the experiment *did* uncover some very real wins around how the decoding is integrated.

## What your results are actually saying

### 1) Throughput: custom decoder loses on realistic sizes

In the Criterion microbench, the custom decoder is **much slower on small/medium payloads** (e.g. ~+82–87% time / ~-45–46% throughput). On the **8 MiB synthetic** payload it’s roughly tied with the crate decoder (no statistically significant difference). 

That pattern is exactly what you’d expect if:

* the `base64` crate is doing very optimized decode (tables, unrolled loops, maybe SIMD), and
* your custom decoder is scalar + branchy, so it has higher per-byte overhead, but the “extra preprocessing copy” in the baseline can dominate for large payloads. 

### 2) Memory: the win is real, but it’s mostly about avoiding extra copies

Your perf-metrics decode loop shows **~20% slower** but **~26% lower peak memory** (43,288,732 → 31,993,541 bytes, about **10.8 MiB saved**). 

That memory savings is very plausibly *not* because “custom decode is more memory efficient,” but because it avoids building intermediate “cleaned” buffers and/or duplicate strings.

### 3) End-to-end: no clear win

Workbook-open and large-grid perf deltas look mixed and mostly noise-level; there’s no consistent speed advantage attributable to custom base64. 

## The big insight: you get most of the value by optimizing the *plumbing*, not replacing base64

Your second pass changes are exactly the right direction:

* **Kill the huge clone** in `read_datamashup_text` by moving out the string (`std::mem::take`). That prevents holding two copies of a potentially giant payload. 
* **Keep the base64 crate but stop allocating a whitespace-stripped `String`** (`split_whitespace().collect()`). Your “fast path + byte-strip into `Vec<u8>`” baseline does that. 
* **Fix the benchmark harness** so it actually exercises multi‑MiB payloads and meaningful iteration counts (synthetic 8 MiB fallback + higher cap). 

Those changes raise the baseline and make the experiment “real.”

### One more cheap win I’d add (likely larger than base64 itself)

Right now your XML extraction path does something like:

* unescape -> **alloc a new `String`** via `into_owned()`
* then `push_str` into `content` (another copy)

For base64 payloads (which almost never contain XML escapes), you can avoid the intermediate allocation entirely by pushing the borrowed data from the `Cow<str>`.

This is in the code you shared (same pattern appears in your DataMashup readers). 

Replace this:

```rust
let text = t
    .unescape()
    .map_err(|e| DataMashupError::XmlError(e.to_string()))?
    .into_owned();
content.push_str(&text);
```

With this:

```rust
let text = t
    .unescape()
    .map_err(|e| DataMashupError::XmlError(e.to_string()))?;
content.push_str(text.as_ref());
```

That’s a pure win: fewer allocations and fewer full-buffer copies, independent of which base64 backend you use.

## Should you roll back the custom base64 code?

If your goals are **performance and simplicity**, then yes: I would **not** keep shipping the custom decoder.

Here’s the decision logic, grounded in your numbers:

* You do not have a consistent throughput win (you have a big regression on common small/medium cases). 
* The only consistent benefit was memory in the isolated decode loop, but you’ve already identified baseline changes that likely recover most of that memory benefit without sacrificing throughput. 
* Beating `base64` crate on throughput usually requires decode tables + heavy loop tuning and/or SIMD, which is a big complexity jump that cuts against the “simpler codebase” goal. 

### My recommended “keep what matters” approach

1. **Keep the baseline improvements** (they’re excellent):

   * `std::mem::take` instead of cloning the DataMashup content 
   * byte-strip + fast path baseline decoder with the `base64` crate 
   * improved perf harness (synthetic payload + iteration policy) 
2. **Make the production path just use `base64-crate`.**
3. Either:

   * **Delete `custom-base64`** entirely (max simplicity), or
   * keep it only as an experiment branch / test-only build (so it doesn’t add maintenance or “what backend are we on?” complexity in real builds).

Given your stated intent (“performance and/or simplicity”), I’d lean toward **deleting it** once you’ve re-run the benches after the baseline cleanups and confirmed the memory gap largely closes.

## If you *do* want to keep pursuing a custom base64 decoder

Then treat it as a deliberate optimization project, not “simplify the codebase” work:

* Switch to a **256-entry decode table** to remove the `match` per byte (big branch reduction). 
* Reduce state updates/branches in the inner loop.
* Only then consider SIMD (and at that point you’re basically rebuilding what the `base64` crate already provides). 

Unless you have a hard requirement to remove the dependency entirely, this is unlikely to be the best use of effort.

## How to get the most out of the “custom crates” strategy overall

The best parts of your methodology are already in your plan:

* **Profiling gate:** don’t rewrite unless you can prove the target is a meaningful contributor (e.g. >=1–2% of total time). 
* **A/B/C variants + parity tests:** you already structured this well. 
* **Make dependency removal real:** optional deps behind baseline features so the custom build actually drops the crate from the graph. 
* **Track build time** (clean + incremental) alongside runtime; that’s often where “fewer deps” pays off. 
* **Automate A/B/C** so you can trust comparisons. 

And base64 specifically taught the key meta-lesson: **first optimize how you use the crate** (unnecessary copies, allocations, data flow), *then* consider replacement. 

## Where I’d go next after base64

If you want “high likelihood of win” rewrites that align with simplicity:

* The tiny **`lru`** use (capacity 4/2) is a great candidate: easy to test, likely simplifies deps, low risk. 
* A **custom JSONL writer / targeted JSON parser** can be a real performance and memory win if you’re currently building large `serde_json::Value` trees or doing lots of serialization overhead. 

If you want, I can sketch a concrete “ship/no-ship” rubric for each rewrite (required speedup, max allowed regression, required memory/binary-size delta, required test coverage) so you can make these decisions quickly and consistently.


---

# perf_findings_custom_base64.md

# Custom Base64 Rewrite: Test + Performance Findings

## Scope
This report captures the post-cleanup run after keeping the integration wins, removing the custom decoder, and adding a targeted XML-extraction perf test. It includes isolated perf-metrics runs to reduce cross-test allocator noise.

## Environment
- Repo: `excel_diff`
- Workspace path: `/home/dvorak/repo/agent_hub_repos/excel_diff`
- Date run: 2026-02-02 (local)

## Changes Applied (Keep What Matters)
- Removed the `custom-base64` feature and the custom decoder implementation.
- Kept the baseline improvements:
  - `read_datamashup_text` moves the DataMashup text instead of cloning.
  - Base64 crate backend uses a fast path + byte-strip slow path (no `split_whitespace().collect()` string).
  - Perf harness keeps the synthetic 8 MiB fallback and higher iteration cap.
- Reduced XML extraction allocations by pushing unescaped `Cow<str>` directly (no intermediate `String` copy).
- Added `e2e_perf_datamashup_text` to isolate the XML extraction path.

## Commands Run
- Tests:
  - `cargo test -p excel_diff`
- Perf-metrics (isolated, one test at a time):
  - `cargo test -p excel_diff --features perf-metrics --test e2e_perf_datamashup_decode -- --ignored --nocapture`
  - `cargo test -p excel_diff --features perf-metrics --test e2e_perf_datamashup_text -- --ignored --nocapture`
  - `cargo test -p excel_diff --features perf-metrics --test e2e_perf_workbook_open -- --ignored --nocapture`
  - `cargo test -p excel_diff --features perf-metrics --test perf_large_grid_tests -- --ignored --nocapture`

## Test Results
- `cargo test -p excel_diff`: PASS
- All isolated perf-metrics tests: PASS

### Warnings Observed
- `core/tests/pg4_diffop_tests.rs`: unused `mut` (existing warning)
- `core/tests/perf_large_grid_tests.rs`: unused import `DiffConfigBuilder` (perf-metrics build)

## Current Perf-Metrics Results (Isolated Runs)
### DataMashup decode loop
- fixture: `synthetic_datamashup_8mib`
- parse_time_ms: 1386
- peak_memory_bytes: 43,288,732

### DataMashup XML extraction (new test)
- test: `datamashup_text_extract`
- iterations: 4
- payload_chars: 11,626,319
- parse_time_ms: 218
- peak_memory_bytes: 69,779,305

### Workbook open perf (peak_memory_bytes)
- e2e_p1_dense: 1,289,823,494
- e2e_p2_noise: 1,189,751,164
- e2e_p3_repetitive: 869,489,399
- e2e_p4_sparse: 1,041,624,745
- e2e_p5_identical: 805,169,683

### Perf-large grid tests (peak_memory_bytes)
- perf_50k_adversarial_repetitive: 1,161,524,565
- perf_50k_99_percent_blank: 1,739,181,708
- perf_50k_identical: 1,766,245,486
- perf_50k_dense_single_edit: 1,766,245,633
- perf_50k_completely_different: 1,766,245,633
- perf_50k_alignment_block_move: 1,766,245,633

## Memory Comparison vs Previous Baseline (Combined Run)
Previous baseline is the last recorded base64-crate run before isolated testing.

### DataMashup decode loop
- previous: 43,288,732
- current: 43,288,732
- delta: 0 (0.00%)

### Workbook open perf (peak_memory_bytes)
- e2e_p1_dense: -134,308,119 (-9.43%)
- e2e_p2_noise: -134,223,296 (-10.14%)
- e2e_p3_repetitive: -560,560 (-0.06%)
- e2e_p4_sparse: -17,345,493 (-1.64%)
- e2e_p5_identical: +0 (+0.00%)

### Perf-large grid tests (peak_memory_bytes)
- perf_50k_adversarial_repetitive: -352,627,692 (-23.29%)
- perf_50k_99_percent_blank: +3,242,443 (+0.19%)
- perf_50k_identical: +1,308,124 (+0.07%)
- perf_50k_dense_single_edit: +1,308,124 (+0.07%)
- perf_50k_completely_different: +1,308,124 (+0.07%)
- perf_50k_alignment_block_move: +1,308,124 (+0.07%)

Note: these deltas are influenced by test isolation; allocator state and ordering materially affect the global peak counter.

## What Actually Improved (and Why)
- **XML extraction allocations reduced**: `unescape()` now feeds `Cow<str>` directly into the DataMashup text buffer. This removes a full payload-sized allocation and copy for unescaped text. The new `datamashup_text_extract` perf test isolates that path.
- **DataMashup text cloning removed**: `std::mem::take` avoids holding two copies of the base64 payload.
- **Base64 preprocessing optimized**: the fast path avoids an intermediate cleaned `String`, and the slow path strips whitespace into a byte buffer. This reduces allocations when the payload has no whitespace and lowers copy cost when it does.

## Regressions / Risks Still Present
- **No custom decoder memory win**: removing `custom-base64` intentionally drops the lower peak memory seen in the decode-loop when that backend was used. This is a trade-off for simplicity and throughput.
- **Global peak memory is noisy**: small changes (and test ordering) can swing `peak_memory_bytes` by tens or hundreds of MB in grid-heavy tests. Isolated runs are more reliable than combined runs for comparison.

## Overall Conclusion
- The “keep what matters” changes are in place and still provide the biggest structural wins (fewer copies, fewer allocations).
- Isolated perf-metrics runs show no new regressions tied to these changes; differences are mostly allocator/ordering noise.
- The DataMashup decode loop peak memory is unchanged vs the last base64-crate baseline; the XML extraction path now has a dedicated perf test for tracking future improvements.


---

Note: The original files will be at C:\Users\dvora\repo\excel_diff\docs\legacy_docs (they are not moved by this change).
