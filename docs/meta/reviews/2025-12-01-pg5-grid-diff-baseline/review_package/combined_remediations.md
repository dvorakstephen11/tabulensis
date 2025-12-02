# Combined Remediation Plans

Branch: `2025-12-01-pg5-grid-diff-baseline`
Generated: 2025-12-01 17:10:42
Total remediation files: 2

---

## [1/2] remediation.md

```markdown
# Remediation Plan: 2025-12-01-pg5-grid-diff-baseline

## Overview

This remediation plan focuses on tightening the PG5 test surface and keeping documentation in sync with the new behavior. The implementation of `diff_grids` itself appears correct and aligned with the mini-spec; the main risks are untested truncation behavior and a stale docs-vs-implementation snapshot that understates current capabilities.

## Fixes Required

### Fix 1: Add explicit tests for row/column truncation

- **Addresses Finding**: 1 — Row/column truncation semantics are untested  
- **Changes**:

  - **File**: `core/tests/pg5_grid_diff_tests.rs`   
  - Add two–three new test cases that mirror the structure of PG5.3/PG5.4 but with *shrinking* shapes:

    1. **Row truncation** (e.g., “pg5_7_grid_diff_row_truncated_row_removed_only”):

       - `GridA`: `nrows = 2`, `ncols = 1`, A1 = 1, A2 = 2.  
       - `GridB`: `nrows = 1`, `ncols = 1`, A1 = 1.  
       - Expectation:
         - `report.ops.len() == 1`.  
         - Single `DiffOp::RowRemoved { sheet: "Sheet1", row_idx: 1, row_signature: None }`.  
         - No `CellEdited` ops.

    2. **Column truncation** (e.g., “pg5_8_grid_diff_column_truncated_column_removed_only”):

       - `GridA`: `nrows = 2`, `ncols = 2`, with non-empty values in both columns.  
       - `GridB`: `nrows = 2`, `ncols = 1`, column A identical to `GridA`’s column A.  
       - Expectation:
         - `report.ops.len() == 1`.  
         - Single `DiffOp::ColumnRemoved { sheet: "Sheet1", col_idx: 1, col_signature: None }`.  
         - No `CellEdited` ops.

    3. (Optional, but useful) **Simultaneous row and column truncation**:

       - `GridA`: small 2×2 (or 3×3) grid.  
       - `GridB`: smaller 1×1 or 2×1 grid where the overlapping 1×1 block is unchanged.  
       - Expectation:
         - One or more `RowRemoved` and/or `ColumnRemoved` ops for the truncated tails.  
         - No `CellEdited` ops.

  - Use the existing `grid_from_numbers` and `single_sheet_workbook` helpers to keep helpers symmetric and concise. :contentReference[oaicite:28]{index=28}  

- **Tests**:

  - New tests should follow the PG-style naming convention and assertion style already used (single-op assertions with explicit pattern matching on the `DiffOp` variant, sheet id, index, and signature).   
  - Ensure these new tests are clearly labeled as part of PG5 in comments or docstrings, so they remain tied to the PG5 milestone.

---

### Fix 2: Add at least one mixed “shape + content” scenario

- **Addresses Finding**: 2 — No tests for mixed “shape + content” changes  
- **Changes**:

  - **File**: `core/tests/pg5_grid_diff_tests.rs` :contentReference[oaicite:30]{index=30}  

  - Add a test that combines tail append/truncate with overlapping edits. For example:

    - **Row append + cell edits in overlap**:

      - `GridA`: 2×2 grid with values 1..4.  
      - `GridB`: 3×2 grid:
        - First two rows same shape; change, say, cell `B1` from 2 → 20 and `A2` from 3 → 30.  
        - Append a third row with any values.  
      - Expectations:
        - Exactly **two** `CellEdited` ops for the changed overlapping cells (`B1`, `A2`).  
        - Exactly **one** `RowAdded` with `row_idx` equal to the appended row index, `row_signature.is_none()`.  
        - No `CellEdited` for the appended row.

    - A similar test could be added for column append + edits if desired, but one mixed scenario is likely sufficient for this cycle.

- **Tests**:

  - Assert:
    - The counts of `CellEdited` vs `Row*`/`Column*` ops match expectations.  
    - Edited addresses match a small set of expected A1-style strings, similar to PG5.5’s set comparison. :contentReference[oaicite:31]{index=31}  

---

### Fix 3: Update docs-vs-implementation analysis for UC-04/UC-05

- **Addresses Finding**: 3 — Documentation snapshot is now stale  
- **Changes**:

  - **File**: `docs/rust_docs/2025-11-30-docs-vs-implementation.md` (in this review package as `2025-11-30-docs-vs-implementation.md`).   

  - Update the status table and narrative for:
    - UC-04 (Row append)  
    - UC-05 (Column append)

  - Suggested adjustments (conceptual, not exact wording):

    - For UC-04 / UC-05:
      - Change “❌ No – reports as CellEdited, not RowAdded/ColumnAdded” to something like:
        - “✅ Partially – for simple spreadsheet-mode tails (append at end with no reordering), `diff_grids` now emits `RowAdded`/`ColumnAdded` ops per PG5. More advanced alignment cases remain unimplemented.”

    - In the “Implementation Status by Specification Part” section, clarify that:
      - The naive cell-by-cell engine has been extended to emit structural row/column ops for tail-only spreadsheet-mode scenarios, but the full alignment pipeline and more complex use cases are still unimplemented.   

- **Tests**:

  - No code tests required, but once updated, this doc should be referenced from future planning documents as the post-PG5 alignment snapshot.

---

### Fix 4 (Process-only): Persist this remediation plan in the repo

- **Addresses Finding**: 4 — Prior remediation history is effectively empty  
- **Changes**:

  - Ensure this remediation plan is saved under the appropriate `docs/meta` or equivalent location (e.g., `docs/meta/remediations/2025-12-01-pg5-grid-diff-baseline.md`) and that `combined_remediations.md` is regenerated to include it. :contentReference[oaicite:34]{index=34}  

- **Tests**:

  - None (process / documentation only).

---

## Constraints

- Do **not** change the public API of `diff_workbooks` or the `DiffOp` enum; existing PG4 JSON shape and round-trip tests must continue to pass unchanged.   
- Do **not** introduce new DiffOp variants or alignment behavior in this cycle; PG5 is explicitly scoped to “naive spreadsheet mode, tails only, no reordering”.   
- Keep test helpers (`grid_from_numbers`, `single_sheet_workbook`) in-memory only and free of Excel parsing dependencies, consistent with the PG5 design.   
- Maintain current performance characteristics (simple O(R×C) overlapping loop + O(R+C) tails); any optimization work should be deferred to later H1-aligned grid-diff milestones.   

---

## Expected Outcome

After remediation:

- PG5 will include explicit, executable tests for both **append** and **truncate** tail scenarios, plus at least one mixed “shape + content” case, fully pinning down the spreadsheet-mode tail semantics outlined in the mini-spec.   
- `2025-11-30-docs-vs-implementation.md` will accurately reflect that simple row/column appends at the tail are now backed by `RowAdded`/`ColumnAdded` ops rather than cell edits, while still clearly calling out the remaining algorithmic gaps.   
- Future changes to `diff_grids` that break truncate semantics or leak cell edits into tail-only regions will be caught early by CI, before more advanced alignment or database-mode work builds on top of PG5.  
- The remediation history for this branch will be preserved, giving future reviewers clear traceability from findings to fixes.
```

---

## [2/2] remediationB.md

```markdown
# Verification Report: excel-diff-engine (current cycle)

## Summary

I was able to review the global test blueprint, difficulty analysis, and product differentiation plan for the Excel Diff engine, but I do **not** have access to the actual codebase snapshot, mini‑spec/decision record, cycle plan, cycle summary, or any test artifacts for this cycle. As a result, I cannot verify that the behavior of the implemented code matches the plan, nor that the tests described in `excel_diff_testing_plan.md` were actually implemented or are passing. Given these missing artifacts, the release should **not** proceed as “verified”; remediation is required to (a) surface the implementation and cycle documents to the reviewer and (b) confirm that the plan’s gating tests are implemented and green.

## Recommendation

[ ] Proceed to release  
[x] Remediation required

## Findings

### 1. Missing implementation and cycle artifacts for this review

- **Severity**: Critical  
- **Category**: Gap / Spec Deviation  
- **Description**:  
  The review instructions state that the following inputs are available: `codebase_context.md`, `cycle_plan.md`, `cycle_summary.txt`, `combined_remediations.md`, and the current codebase snapshot. In this environment, only three files are accessible under `/mnt/data`:  
  - `excel_diff_testing_plan.md`  
  - `excel_diff_difficulty_analysis.md`  
  - `excel_diff_product_differentiation_plan.md`   
  There is no visible `codebase_context.md`, no cycle plan/summary, no remediation history, and no source code. That makes it impossible to check what was *actually* implemented or which tests exist.  
- **Evidence**:  
  - Directory listing of `/mnt/data` contains only the three Excel Diff documentation files.  
  - `excel_diff_testing_plan.md` references `excel_diff_specification.md` and implementation details, but that spec and the code are not available to the reviewer. :contentReference[oaicite:1]{index=1}  
- **Impact**:  
  - Cannot confirm that the implementation matches the mini‑spec or overall specification.  
  - Cannot confirm that this cycle’s scope is correctly implemented.  
  - Cannot confirm that any tests have been added, run, or are passing.  
  - The verification process itself is incomplete and cannot gate a release.

---

### 2. Test implementation status vs. testing plan is unknown

- **Severity**: Critical  
- **Category**: Missing Test / Gap  
- **Description**:  
  `excel_diff_testing_plan.md` defines a rich, phased test strategy across container I/O (M1–M2), grid IR and snapshots (PG1–PG3, PG5–PG6), DataMashup framing (Milestone 3), semantic sections (Milestone 4), domain queries (Milestone 5), MVP DiffOps and grid/M diff (Phase 3), advanced alignment (G1–G13), database-mode diffs (D1–D10), fuzzing, CLI contract tests, performance, and cross‑platform determinism.   
  However, without the codebase and test suite, there is no way to verify which of these tests were actually implemented in this cycle or whether they are passing.  
- **Evidence**:  
  - Testing plan clearly lists numerous *gating* tests (tagged `[G]`) across phases 1–3 and beyond (e.g., basic container open tests, “no DataMashup vs exactly one” extraction, PG5 in‑memory grid diff smoke tests, PG4 DiffOp contract, M6 basic M diffs).   
  - No test files, test logs, or cycle_summary are available to confirm their presence.  
- **Impact**:  
  - It is unknown whether key regressions (e.g., bad DiffOp serialization, mis‑aligned grids, broken DataMashup parsing) would be caught.  
  - The testing plan sets clear release‑gating expectations, but they cannot be enforced or audited in this review, so shipping would be effectively untested from the reviewer’s perspective.

---

### 3. Behavioral contract & DiffOp wire schema not verified

- **Severity**: Critical  
- **Category**: Spec Deviation / Missing Test (verification gap)  
- **Description**:  
  The plan requires a stable `DiffOp` and report schema with round‑trip tests (construct each DiffOp variant, serialize to JSON/wire format, deserialize, and validate equality). :contentReference[oaicite:4]{index=4}  
  This contract is central to the engine’s interoperability with CLI, Git integrations, and future web/WASM UI, but I cannot see any implementation or tests to confirm:  
  - The set of variants implemented matches those specified in the plan.  
  - The JSON shape and enum tags are stable and versioned.  
  - Round‑trip (serialize → deserialize) works for individual DiffOps and the report container.  
- **Evidence**:  
  - The test plan explicitly defines PG4.1–PG4.4 to lock down the DiffOp contract and round‑trip behavior. :contentReference[oaicite:5]{index=5}  
  - No code or tests are visible to validate conformance.  
- **Impact**:  
  - If the implementation deviates (fields renamed, missing variants, inconsistent serialization), downstream tools (CLI, CI integrations, UI) could break silently or misinterpret diffs.  
  - Schema drift without tests could make future compat guarantees impossible, which is flagged as a key risk in the plan.

---

### 4. Container and DataMashup extraction behavior unvalidated

- **Severity**: Critical  
- **Category**: Gap / Missing Test (verification gap)  
- **Description**:  
  Phases 1–2 and Milestone 2 define required behavior for opening Excel containers, detecting ZIP/OPC formats, and extracting DataMashup bytes (`dm_bytes`), including handling of:  
  - Nonexistent files, directories, non‑ZIP inputs, and non‑Excel ZIPs.  
  - Workbooks with no `<DataMashup>` vs a single one vs multiple ones.  
  - Corrupt base64 payloads for `<DataMashup>`.   
  Without code/tests, I cannot confirm:  
  - That the engine surfaces specific, typed errors (e.g., `NotAnExcelZip`, `NotExcelContainer`, `NoDataMashupFound`, `MultipleDataMashups`), rather than panicking or returning generic I/O errors.  
  - That corruption paths are handled defensively.  
- **Evidence**:  
  - M1 and M2 tests are explicitly described in the plan with clear expectations for error kinds and behaviors. :contentReference[oaicite:7]{index=7}  
  - No source/test implementation is available for examination.  
- **Impact**:  
  - Poor or inconsistent error handling here will produce brittle behavior on exactly the “weird/legacy/future” files the difficulty analysis calls out as high‑risk. :contentReference[oaicite:8]{index=8}  
  - If panics or generic I/O errors occur in the wild, they could be mis‑diagnosed as generic engine instability, harming reliability perception.

---

### 5. Grid IR, addressing, and snapshot semantics unverified

- **Severity**: Critical  
- **Category**: Gap / Missing Test (verification gap)  
- **Description**:  
  The plan requires a normalized grid IR and strong invariants around:  
  - Addressing (row/column indices ↔ `"A1"` strings; PG2).  
  - Snapshot payloads for `CellEdited` (values, formulas, and formatting semantics; PG3).  
  - In‑memory grid diff behavior before integrating Excel parsing (PG5).   
  I cannot see whether:  
  - `index_to_address` / `address_to_index` are implemented correctly.  
  - Snapshots include the right fields and equality semantics (format‑only vs value/formula differences) are codified.  
  - The basic small‑grid behaviors (e.g., 1×1 change → single `CellEdited`) are tested.  
- **Evidence**:  
  - PG2–PG3–PG5 in `excel_diff_testing_plan.md` spell out precise test cases for this IR and diff behavior.   
  - No IR code or grid tests are visible.  
- **Impact**:  
  - If addressing is wrong, all downstream diffs (including M/query mapping to sheets) may point at incorrect cells.  
  - If snapshot semantics are incorrect, UI and API consumers may misclassify changes (e.g., formatting vs content).  
  - If basic grid diff semantics are off, higher‑level alignment/perf tests (G8–G13, D1–D10) cannot be trusted even if they superficially pass.

---

### 6. Advanced alignment / Database‑mode behaviors unverified

- **Severity**: Moderate  
- **Category**: Gap / Missing Test (verification gap)  
- **Description**:  
  The plan defines detailed scenarios for spreadsheet‑mode alignment (G1–G13) and database‑mode keyed diffs (D1–D10), including:  
  - Pure content equality, single cell edits, format‑only changes, append/truncate, middle insertion/deletion, row/column/rectangular block moves, and fuzzy LAPJV‑based block move detection.   
  - Keyed table behavior: pure reorders vs structural changes, added/removed rows by key, composite keys, duplicate key clusters, explicit vs metadata/heuristic keys.   
  Without code/tests, I cannot tell whether any of these behaviors are implemented or tested.  
- **Evidence**:  
  - G‑ and D‑series tests are thoroughly specified in the test plan.   
- **Impact**:  
  - If these behaviors are missing or partially implemented, the engine may misreport reorders as mass edits, or misalign database‑like tables, which is a key differentiator vs incumbents highlighted in the product plan.   
  - This is somewhat less immediately critical than basic IR/container correctness, but still high‑priority before claiming competitive parity.

---

### 7. Fuzzing, perf guardrails, and cross‑platform determinism unverified

- **Severity**: Moderate  
- **Category**: Missing Test / Gap  
- **Description**:  
  The plan calls for:  
  - Early DataMashup fuzzing with `cargo-fuzz` targeting the framing parser.  
  - Resource‑constrained streaming tests (100MB XML under 50MB heap, adversarial repetitive grids).  
  - Performance tests (P1, P2) and metrics export (`metrics-export` flag).  
  - Cross‑platform determinism tests across Windows, Linux, and WASM (Milestone 12).   
  Without code/tests, none of these can be confirmed.  
- **Evidence**:  
  - All these are explicitly described as either gating (`[G]`) or important hardening milestones.   
- **Impact**:  
  - Without fuzzing and perf guardrails, the engine may break or exhibit pathological behavior on large or adversarial workloads, contradicting the performance and robustness expectations set in the difficulty and product plans.   

---

### 8. CLI/API contract and UX‑level behaviors unverified

- **Severity**: Moderate  
- **Category**: Missing Test / Gap  
- **Description**:  
  Milestone 11 requires CLI integration tests and API “black‑box” tests that treat the diff engine as a stable interface: given file paths → `DiffReport`, stable JSON schema, and exit codes. :contentReference[oaicite:18]{index=18}  
  Without implementation access, there is no way to verify:  
  - That the CLI exists, uses the agreed flags, and returns the correct exit codes.  
  - That the `DiffReport` schema is versioned and stable.  
- **Evidence**:  
  - CLI/API tests are explicitly outlined in the testing plan. :contentReference[oaicite:19]{index=19}  
- **Impact**:  
  - Even if core algorithms work, missing or unstable CLI/API contracts will block real‑world adoption (Git difftool integration, CI, web clients).

---

## Checklist Verification

Because the implementation, test suite, and cycle documents are not visible, all checklist items remain **unverified**:

- [ ] All scope items from mini-spec addressed  
- [ ] All specified tests created  
- [ ] Behavioral contract satisfied  
- [ ] No undocumented deviations from spec  
- [ ] Error handling adequate  
- [ ] No obvious performance regressions  

The unchecked boxes reflect lack of visibility, not a conclusion that the implementation definitely fails them.

```

