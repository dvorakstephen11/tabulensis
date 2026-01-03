## Phase 7 goals

Phase 7 is explicitly about three things: **mechanically enforcing architecture boundaries**, **cleaning up the error taxonomy**, and **strengthening the parse → IR boundary** so parsing stays “pure IR” and doesn’t embed diff assumptions. 

This aligns with the existing “bytes → meaning → IR → operations” spine already called out in the design evaluation (container safety → semantic parsing → compact IR → diff engine emitting `DiffOp`s). 

It also matters right now because the project is already a multi-host, multi-crate product surface (core + CLI + wasm/web + desktop), and further domain expansion (model/PBIX depth) is one of the areas most likely to create gradual layer leakage if boundaries aren’t locked in.

---

## Current codebase reality that Phase 7 must address

### 1. Parse/open code currently depends “upward” on diff and package

`PackageError` (defined in the Open XML parsing layer) currently has a `Diff(#[from] crate::diff::DiffError)` variant and even returns a diff error code (`DIFF_INTERNAL_ERROR`) via `code()`.

That creates exactly the cross-layer coupling Phase 7 is trying to prevent: *open/parse error types depending on diff/runtime error types*. 

Separately, `excel_open_xml.rs` imports VBA types from `crate::package` (`VbaModule`, `VbaModuleType`).
That means parsing depends on a higher-level orchestration module, which is also an “upward dependency.”

### 2. The `PackageError::Diff` coupling isn’t theoretical; it’s used for propagation

The JSON diff helper path (`diff_workbooks_to_json` / `diff_workbooks`) uses `try_diff_workbooks_streaming(...)?` while returning `PackageError`, which is only possible because `DiffError` is convertible into `PackageError` via that `Diff` variant.

So removing `PackageError::Diff` requires a real refactor of these “open + diff + serialize” convenience flows.

### 3. “Artifact IR” isn’t fully an IR layer yet (VBA types live in `package.rs`)

`VbaModuleType` and `VbaModule` are defined in `core/src/package.rs`.
But they’re used by:

* the Open XML parsing code (to output the modules),
* and the diff layer (object diff) to produce diff ops.

That’s a strong signal these types should live in the **IR/artifacts layer**, not in orchestration.

### 4. This is already a recognized pressure point

The design evaluation explicitly recommends enforcing architectural boundaries mechanically (subcrates or stricter discipline) to prevent gradual layer leakage as more domains land. 

---

## Target architecture contract for Phase 7

Before changing code, Phase 7 should lock in a clear dependency contract that matches how the system already *wants* to work:

**Container/I-O safety → Parse → IR → Diff engine → Output adapters → Hosts**

Concretely, for the existing code surface described in the design evaluation: container isolation in `container.rs`, parsing in `excel_open_xml.rs` + `grid_parser.rs`, DataMashup framing vs domain building split, diff orchestration centralized in `engine/mod.rs`. 

### “Allowed dependency direction” (practical rules)

1. **Parse code may depend on IR types**, but IR must not depend on parse/diff/output.
2. **Diff engine may depend on IR types**, but must not depend on parse/container.
3. **Output adapters may depend on both parse + diff + IR**, because they’re integration glue.
4. **Host crates (CLI/wasm/desktop) should only touch stable public entry points** (e.g., `WorkbookPackage`, `DiffReport`, `DiffOp`, config).

Phase 7’s implementation should make violations of (1) and (2) hard to do accidentally.

---

## Implementation plan

### Workstream A: Mechanically enforce architecture boundaries

Phase 7 explicitly calls out “subcrates or stricter `pub(crate)` discipline” for mechanical enforcement.
Given Rust’s intra-crate visibility rules, **true mechanical enforcement** is strongest with **crate boundaries**. The plan below is staged to get immediate wins (remove known upward deps) and then lock it in mechanically.

#### A1. Identify and label the layer membership of existing modules

Create a short internal “layer mapping” (even as a comment block in `core/src/lib.rs` or a `core/src/architecture.md`) listing which files belong to each layer. This is not Phase 8 “full docs”; it’s an implementation aid so the refactor doesn’t drift.

Example mapping grounded in current structure:

* **container layer**: `core/src/container.rs` 
* **parse layer**: `core/src/excel_open_xml.rs`, `core/src/grid_parser.rs`, `core/src/datamashup_framing.rs` 
* **IR layer**: `core/src/workbook.rs` (+ new artifact IR module for VBA)
* **diff layer**: `core/src/engine/*`, `core/src/diff.rs`, `core/src/object_diff.rs`, `core/src/m_diff/*`
* **output adapters**: `core/src/output/json.rs`, etc. 
* **orchestration**: `core/src/package.rs` (but this will lose “IR type ownership”). 

Deliverable: a committed layer map that engineers can refer to during the refactor.

#### A2. Remove existing “upward” dependencies first

This is the lowest-risk step and also the prerequisite for clean crate extraction:

* Parsing must not import from `package` (move VBA types out; see Workstream C).
* Parsing must not depend on `diff` via error types (remove `PackageError::Diff`; see Workstream B).

This ensures the next step (subcrates) is mostly moving code, not untangling logic mid-flight.

#### A3. Introduce mechanical enforcement

Pick one (recommended) and still do the CI guardrail in A4.

**Recommended approach: split core into internal subcrates (strongest enforcement).**

A realistic, minimal-churn decomposition:

1. `excel_diff_ir`

   * Owns pure data structures: `Workbook`, `Sheet`, `Grid`, `NamedRange`, `ChartObject`, string IDs/pool types, and **VBA artifact IR** moved out of `package.rs`.
   * No zip reading, no XML parsing, no diff logic.

2. `excel_diff_parse`

   * Owns container and parsing: `container.rs`, `excel_open_xml.rs`, `grid_parser.rs`, `datamashup_framing.rs`, etc.
   * Depends on `excel_diff_ir`.
   * Owns open/parse errors (the cleaned-up `PackageError` and friends), but **not diff errors**.

3. `excel_diff_engine`

   * Owns diff engine and diff ops: engine module, diff types/errors, object diff, m diff, model diff (feature-gated).
   * Depends on `excel_diff_ir` (and optionally on domain-building modules if they’re kept separate).

4. Keep the existing `excel_diff` crate as a façade

   * Re-export public API so hosts (CLI/wasm/desktop) don’t need to be rewritten immediately.
   * Keep `WorkbookPackage` orchestration here (or move to a small `excel_diff_package` crate later if it grows).

This immediately prevents “parse depending on engine” and “engine depending on parse” because Cargo won’t allow cyclic dependencies.

**Fallback approach: stay single-crate, but add a mechanical CI guardrail** (works even if you later do the split)

Implement a tiny “arch guard” check in CI that fails if forbidden imports appear in certain files (e.g., `excel_open_xml.rs` importing `crate::engine`, `crate::diff`, or `crate::package`). This isn’t as strong as subcrates, but it’s still “mechanical” and catches regressions early.

Deliverables:

* Either: workspace updated with the internal crates and re-exports preserved.
* And regardless: CI guardrail preventing re-introduction of upward dependencies.

#### A4. Host compatibility verification matrix

Because the project is multi-host, Phase 7 should include a concrete “must compile and run tests” checklist for each consumer surface:

* **core crate tests** (including output/json tests that currently use `diff_workbooks_to_json`).
* **CLI** build + tests (imports `PackageError`, `DiffError`, etc).
* **wasm** build (uses `WorkbookPackage::open` + `pkg.diff`).
* **desktop** build (maps `PackageError` into UI payload).

This is part of being “grounded in reality”: phase 7 isn’t done until each host still builds cleanly with the same feature sets.

---

### Workstream B: Clean up error taxonomy and remove `PackageError::Diff`

Phase 7 explicitly requires splitting open/parse errors from diff/runtime errors, reducing reliance on `PackageError::Diff`.

#### B1. Redefine `PackageError` as strictly “open/parse” (and path context)

In `core/src/excel_open_xml.rs` (or the future `excel_diff_parse` crate), update `PackageError` so it:

* Keeps variants that represent I/O/container failures (`ContainerError`), XML parsing failures (`GridParseError`), DataMashup framing failures (`DataMashupError`), missing parts, invalid XML, unsupported format, etc.
* Keeps the path wrapper (`WithPath`) and helper `with_path(...)` since that’s a useful open/parse concern.
* **Removes**:

  * `Diff(#[from] crate::diff::DiffError)`
  * and its `code()` mapping to diff codes. 

Deliverable: `PackageError` no longer imports or references `crate::diff::DiffError`.

#### B2. Refactor the JSON convenience flow so it no longer relies on `From<DiffError> for PackageError`

Today, `output/json.rs` uses `try_diff_workbooks_streaming(...)?` and therefore depends on `PackageError::Diff`.

You have two viable, grounded options. The plan should pick one explicitly and include tests.

**Option B2a (recommended for stability): treat diff-runtime failures as report warnings in this legacy “open+diff+json” helper**
This matches how the “legacy API” already behaves for structured errors like limit exceeded: it converts the error into a warning and returns an incomplete report rather than failing the call.

Implementation shape in `output/json.rs`:

* Keep using the streaming sink pattern so string interning continues to work across grid ops + object ops + M ops.
* Replace the `?` propagation with local handling:

  * call `try_diff_workbooks_streaming(...)`
  * on `Err(DiffError)`, **discard any partial sink output** and convert the error into a `DiffSummary` warning (or reuse the same warning string format the engine uses), then continue assembling object/M ops.

Key detail: **discard partial grid ops on failure** (don’t risk returning a report with “half a grid diff” while `complete=false`). This is important because `try_diff_workbooks_streaming` can fail due to limit-exceeded behaviors configured as errors, which may happen after emitting some ops.

**Option B2b: introduce a new “workflow error” wrapper type**
If you want the helper to remain fallible on diff-runtime failures, create a new error enum at the orchestration/output layer (not in parse) such as:

* `DiffWorkbooksError::Open(PackageError)`
* `DiffWorkbooksError::Diff(DiffError)`
* `DiffWorkbooksError::Serialize(serde_json::Error)`

Then update the helper signature accordingly.

This is “purest taxonomy”, but it risks more downstream signature churn; Option B2a avoids that while still removing the layer violation.

Deliverable: `PackageError::Diff` is no longer needed by any code path; the JSON helper compiles without that conversion.

#### B3. Audit other `?`-based conversions and remove any remaining diff→package coupling

Do a focused search for:

* `?` sites where the error type is `PackageError` and the callee returns `DiffError`
* `From<DiffError> for PackageError` reliance

Based on the context, the main coupling is the output/json convenience path.
But Phase 7 should enforce this audit as an explicit checklist item so no hidden conversion remains.

#### B4. Ensure error codes remain coherent and stable

After removing `PackageError::Diff`, re-check:

* `PackageError::code()` never returns diff-domain codes.
* `DiffError::code()` remains the only place producing `DIFF_*` codes.

This matters because desktop uses `PackageError::code()` to create UI error payloads.

---

### Workstream C: Strengthen parse → IR boundaries for artifacts

Phase 7 calls out “make parse output pure IR for artifacts (charts, defined names, etc.) so parsing doesn’t embed diff assumptions.”

In the current code, *most* of this is already true for charts and named ranges because those are represented as IR structs in `workbook.rs`.
The clear outlier is VBA modules.

#### C1. Move VBA module types into the IR layer

Move `VbaModuleType` and `VbaModule` out of `core/src/package.rs` into a dedicated IR module (for example `core/src/ir/vba.rs`, or `core/src/vba.rs` if you’re keeping a flat layout).

Update:

* `excel_open_xml.rs` to import from the IR module rather than `crate::package`.
* `object_diff.rs` to import from IR rather than `crate::package`.
* `package.rs` to *use* the IR types rather than *define* them.

This is a direct, concrete improvement to the parse→IR boundary, and it also reduces the “upward dependency creep” pressure by removing a real parse→package edge.

#### C2. Preserve the public API surface at the crate root

Right now, `excel_diff` re-exports `VbaModule` and `VbaModuleType` from `package` at the root.

After the move:

* Continue re-exporting these types from the crate root so external users aren’t forced to chase paths.
* Keep `WorkbookPackage` unchanged in surface behavior (it can still hold `Option<Vec<VbaModule>>`), just sourced from IR.

Deliverable: internal layering is fixed without unnecessary public API churn.

#### C3. Quick audit of other artifact outputs for “diff assumptions”

Do a one-pass audit of parse output types for:

* “derived” fields that only exist to help diff (hashes, pre-normalized strings, etc.)
* mixing of “presentation choices” into parse (like truncation, UI caps)

For example, charts currently carry an `xml_hash` and a compact `ChartInfo`.
That can be acceptable IR (a stable fingerprint can be part of representation), but Phase 7 should at least confirm:

* parsing is not emitting `DiffOp`s,
* parsing is not applying diff-specific heuristics,
* and diff-specific decisions stay in `object_diff`/engine.

Deliverable: written audit notes listing any remaining questionable fields and whether they’re accepted as IR or moved to diff.

---

## Test plan and acceptance criteria

### What to add or update in tests

1. **Compile-time guarantees**

   * If you do subcrates: the crate graph itself is the guarantee.
   * If you don’t: add the CI “arch guard” to fail on forbidden imports.

2. **Regression tests for the JSON convenience path**

   * Ensure `diff_workbooks_to_json(...)` still succeeds on normal cases and still returns `PackageError` for open/parse failures.
   * Add a new test case that triggers a structured diff-runtime failure (e.g., limit-exceeded configured as error) and verifies:

     * the JSON helper returns **a report with `complete=false` and a warning**, not an error.
     * no partial grid ops leak into the output (op list should be empty or consistent with the summary).

     This pattern is already tested at the engine layer for “legacy API converts structured error to warning.”

3. **VBA refactor tests**

   * Add/adjust a unit test ensuring `open_vba_modules_from_container` still returns modules and that the diff layer still produces the expected VBA-related `DiffOp`s.

### Phase 7 exit criteria

Phase 7 is complete when all of the following are true:

* **No upward dependencies remain** in the hot spots identified:

  * `excel_open_xml.rs` no longer depends on `crate::diff` or `crate::package` for core types/errors.
* **`PackageError` is open/parse only**

  * `PackageError::Diff` is gone.
  * `PackageError::code()` never returns `DIFF_*` codes.
* **A mechanical guard exists**

  * Either via subcrates or CI enforcement.
* **VBA artifact IR lives in the IR layer**

  * `VbaModule` types are owned by IR and imported by parse and diff.
* **All hosts still build**

  * CLI, wasm, desktop compile with their existing feature sets and still work end-to-end.

---

## Practical sequencing

To keep the refactor controlled and avoid “half-migrated” states, the cleanest order is:

1. **Move VBA types to IR** (removes parse→package dependency).
2. **Remove `PackageError::Diff`** and refactor output/json convenience flow to stop relying on it.
3. **Add mechanical enforcement** (subcrates preferred) and wire re-exports so hosts keep compiling.
4. **Add/adjust the targeted tests** to ensure behavior stays coherent when diff-runtime issues occur.

This sequencing is directly aligned with the Phase 7 goals and fixes the concrete coupling points present in the codebase today.
