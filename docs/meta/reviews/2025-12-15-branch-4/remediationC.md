I see a lot of the Branch‑4 refactor is in place and the core direction looks right (StringId/StringPool threading, streaming sink, and the WASM build gate), but it’s **not “all correct” yet** based on what you attached.

## What looks correct

* **String interning is implemented** via `StringId` + `StringPool` (including pre-interning `""`), and workbook IR types now use `StringId` for sheet names and text/error values.
* **Coordinate redundancy is removed**: cells are stored as content-only (`CellContent`) and coordinates live in the grid key, with `insert_cell(...)` as the primary write path. 
* **Streaming output is plumbed**: `DiffSink` exists (with `finish()`), and the engine has streaming diff functions that emit ops through a sink.
* **JSONL sink exists** with a header that includes version + strings table + config.
* **WASM readiness pieces exist in one snapshot**: there is a `wasm_smoke` binary and a GitHub Actions workflow that builds wasm with `--no-default-features` and enforces a 5MB size budget.

## What does not look correct yet

### 1) The latest build log you attached still shows test compilation failures

Your `cycle_summary.txt` still reports a broad set of compile errors caused by **leftover old-API usage in tests**:

* Tests still calling `grid.insert(...)` and/or constructing `CellContent { row, col, address, ... }` (those fields no longer exist; `insert_cell` is the new API).
* Tests still treating sheet names as `String`/`&str` instead of `StringId` (e.g., `assert_eq!(sheet, "Sheet1")`).
* Tests calling internal engine functions without the new required `&mut StringPool` argument (e.g., `engine::diff_workbooks` and `diff_grids_database_mode`).

That’s a “repo doesn’t build cleanly” blocker.

### 2) Missing acceptance tests called out in the spec

Your spec checklist explicitly calls for:

* `core/tests/string_pool_tests.rs` (50k identical string interning)
* `core/tests/streaming_sink_tests.rs` (VecSink vs CallbackSink equality) 

I don’t see those in the tests list in the snapshot that enumerates `core/tests`. 

### 3) Benchmarks look structurally OK, but performance is still multi‑second in some cases

The “most recent” benchmark JSON indicates branch‑4 metadata and includes results, but several scenarios are still in the ~9–11s range (e.g., completely different, adversarial repetitive).
That might be acceptable relative to your current guardrails, but if you’re trying to hit stricter targets from earlier specs, this is not there yet.

## Detailed implementation plan for what’s not correct

### A) Make test code session-aware and stop using old string/Cell APIs

#### A1) Standardize on a single test helper pattern

Pick one of these approaches and apply it consistently:

**Option 1: Use the existing `sid(...)` helper + `with_default_session(...)`**

* Use `sid("Sheet1")` whenever you need a `StringId`.
* Use `with_default_session(|session| session.strings.resolve(id))` when you need to assert on the original string.

This is the least intrusive across the test suite.

**Option 2 (cleaner isolation): Use a fresh `DiffSession` per test**

* Avoid cross-test interning order dependence entirely.
* Build your workbooks using `session.strings.intern("...")`.
* Call `engine::diff_workbooks(..., &mut session.strings, ...)`.

This is more plumbing, but produces stable string tables and reduces “test order matters” risk.

#### A2) Replace `Grid::insert(...)` + `Cell{row,col,address,...}` with `insert_cell(...)`

Representative replacement:

Old code to replace:

```rust
grid.insert(Cell {
    row,
    col,
    address: CellAddress::from_indices(row, col),
    value: Some(CellValue::Text("x".into())),
    formula: None,
});
```

New code to use:

```rust
let x = sid("x");
grid.insert_cell(row, col, Some(CellValue::Text(x)), None);
```

Do this everywhere the cycle summary indicates `insert` and the removed fields (`row`, `col`, `address`) are used.

#### A3) Replace all `String` sheet names with `StringId`

Anywhere you have:

Old code to replace:

```rust
Sheet {
    name: "Sheet1".to_string(),
    grid,
}
```

New code to use:

```rust
Sheet {
    name: sid("Sheet1"),
    grid,
}
```

And anywhere you compare:

Old code to replace:

```rust
assert_eq!(sheet, "Sheet1");
```

New code to use:

```rust
assert_eq!(sheet, sid("Sheet1"));
```

(or resolve through the pool if you specifically want to assert on the string).

This is exactly the class of failures shown in `cycle_summary.txt`.

#### A4) Fix internal engine calls missing `&mut StringPool`

You have two valid patterns:

**Pattern 1: Switch imports to the public wrapper that hides the pool**
Old code to replace:

```rust
use excel_diff::engine::diff_workbooks;

let report = diff_workbooks(&wb_a, &wb_b, &config);
```

New code to use:

```rust
use excel_diff::diff_workbooks;

let report = diff_workbooks(&wb_a, &wb_b, &config);
```

**Pattern 2: Keep engine calls and pass an explicit pool**
Old code to replace:

```rust
use excel_diff::engine::diff_workbooks;

let report = diff_workbooks(&wb_a, &wb_b, &config);
```

New code to use:

```rust
use excel_diff::{engine, DiffSession};

let mut session = DiffSession::new();
let report = engine::diff_workbooks(&wb_a, &wb_b, &mut session.strings, &config);
```

The cycle summary shows this exact missing-arg error in multiple tests (including `amr_multi_gap_tests` and `limit_behavior_tests`).

Apply the same idea to `diff_grids_database_mode(...)`, which now requires `&mut StringPool` too.

#### A5) Update any test constructing `CellValue::Text("...".into())` or `Error("...".into())`

Because `Text`/`Error` now store `StringId`, replace those with interned IDs:

Old code to replace:

```rust
Some(CellValue::Text("old".into()))
```

New code to use:

```rust
Some(CellValue::Text(sid("old")))
```

This is explicitly called out in the failures.

### B) Fix output tests that assumed sheet names were strings

Tests that compare `DiffOp` contents need to either:

1. Compare `StringId`s (using `sid("...")`), or
2. Resolve ids back to strings before asserting.

For example, where you currently have failures like `"Sheet1".into()` (no longer valid), change tests to build expected ops with `sid("Sheet1")` and compare equality, or resolve and compare the resolved string.

If you’re testing JSON output, update assertions to:

* Read `report.strings` (array of strings)
* Interpret each `StringId` as an index into that array

### C) Add the missing acceptance tests from the spec

#### C1) `core/tests/string_pool_tests.rs`

Add a new test file that:

* Interns the same string 50,000 times and asserts the ID never changes
* Asserts pool size is `2` (empty string + the interned string) or `>= 2` depending on what else you intern in the test
* Verifies `resolve(id) == "x"`

This is explicitly required in the spec checklist. 

#### C2) `core/tests/streaming_sink_tests.rs`

Add a test that:

* Builds two small workbooks with a fresh `DiffSession`
* Runs `engine::try_diff_workbooks_streaming(...)` twice:

  * once with `VecSink`
  * once with `CallbackSink` collecting ops into a vec
* Asserts the op vectors are identical (same order, same content)

Again, explicitly required.

### D) Validate the WASM gate end-to-end

If you keep the workflow and smoke binary as shown:

* Ensure `cargo build --target wasm32-unknown-unknown --no-default-features -p excel_diff --bin wasm_smoke --release` succeeds
* Ensure the size check stays under 5MB

Those pieces exist and look aligned with the spec.

### E) Benchmarks housekeeping

Your branch‑4 benchmark JSON is present and includes the key fixtures, but it’s worth ensuring:

* The metadata (branch/commit) matches the code snapshot you’re reviewing
* You regenerate after fixing the test suite so you’re not benchmarking a broken tree

(Your current numbers show multi‑second cases like “completely different” and “adversarial repetitive”.)

---

If you execute the plan in the order above, you’ll get to:

1. clean `cargo test` again (fixing the remaining “old API” stragglers),
2. satisfy the spec’s missing acceptance tests (string pool + streaming sink), and
3. have a reliable wasm compilation gate and size budget check.
