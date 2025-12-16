Below is a detailed, execution-ready implementation plan that will make the uploaded `spec.md` and **Branch 4 (WASM & Memory Readiness)** complete, using the repository’s current structure and the concrete problems visible in `cycle_summary.txt` / `codebase_context.md`.

---

## The core invariants Branch 4 needs you to lock in

1. **A single shared `StringPool` must be used for both workbooks being diffed.**
   If A and B are interned into different pools, equal text will have different IDs and comparisons (alignment, equality, hashing, output) will be wrong.

2. **Coordinates are not stored in cells anymore; they live in the grid key.**
   The grid is keyed by `(row, col)` and the cell payload is just “what’s in the cell” (value + formula), while addresses are derived on demand.

3. **Streaming output means: no “collect and global-sort.”**
   Determinism must come from construction: sheets emitted in sorted-name order, and within a sheet, row-major emission.

4. **WASM gate requires a bin target for size checks.**
   The branch YAML in `next_sprint_plan.md` is wrong (package name and artifact). The practical fix is a tiny `wasm_smoke` binary compiled with `--no-default-features`.

---

## Where the repo already is (so you don’t redo work)

From `codebase_context.md`, the following Branch 4 foundations already exist:

* `StringPool` / `StringId` exist and are the basis for `SheetId`.
* `std-fs` feature exists and is in `default`. 
* `OpcContainer` has been refactored in the direction Branch 4 wants (reader-based + path wrapper under `std-fs`).
* The big remaining pain is that **tests (and benches) still reference pre-Branch-4 APIs** (strings as `String`, `Cell` containing coords/address, etc.). `cycle_summary.txt` shows lots of these exact compile failures.
* Benchmarks are also still using the old `grid.insert(Cell { row, col, address, value: Text(String) })` shape and will need to be updated too.

So the plan below focuses on: (A) finishing the migration (tests/benches), (B) implementing streaming sinks + engine streaming, and (C) adding the WASM CI gate + smoke bin.

---

## Phase 1: Make the branch compile cleanly (tests + benches)

### 1.1 Add the tiny ergonomics shim: `CellAddress::from_coords`

`spec.md` calls out updating tests to compute addresses using `CellAddress::from_coords(row, col)` (because `cell.address` fields disappear).

If you don’t already have it, add an alias that matches the spec.

**Replace (in `core/src/workbook.rs`, inside `impl CellAddress`)**

```rust
impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> Self {
        Self { row, col }
    }
}
```

**With**

```rust
impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    pub fn from_coords(row: u32, col: u32) -> Self {
        Self::from_indices(row, col)
    }
}
```

This is small, but it prevents dozens of repetitive test edits from turning ugly.

---

### 1.2 Stop fighting the thread-local session: add a test helper to intern strings

Most failures are caused by tests comparing `StringId` to `"Sheet1"` or constructing `CellValue::Text("x".into())`.

Add a helper in `core/tests/common/mod.rs` (or similar) so test code stays readable:

**Create (or add) in `core/tests/common/mod.rs`**

```rust
use excel_diff::{StringId, with_default_session};

pub fn sid(s: &str) -> StringId {
    with_default_session(|session| session.strings_mut().intern(s))
}
```

Then systematically convert tests to use `sid("Sheet1")`, `sid("old")`, etc.

This avoids needing to “resolve back to string” for most asserts. (Resolve is still useful for debugging.)

---

### 1.3 Update all test patterns that rely on the old IR

Here are the mechanical rewrite rules that fix almost all compile failures shown in `cycle_summary.txt`:

#### A) Fix sheet name comparisons

**Before**

```rust
assert_eq!(sheet, "Sheet1");
```

**After**

```rust
assert_eq!(sheet, common::sid("Sheet1"));
```

This is required across `pg4_diffop_tests.rs`, `pg6_object_vs_grid_tests.rs`, `limit_behavior_tests.rs`, etc.

#### B) Fix `diff_workbooks` / `diff_grids_database_mode` imports

Some tests are calling the *engine* functions directly (which now require a pool argument) instead of the crate-root wrappers. `cycle_summary.txt` shows `diff_workbooks` being resolved to `core/src/engine.rs:59` and failing due to missing `&mut StringPool`.

Two viable fixes:

* Prefer: **use the crate-root wrapper** `excel_diff::diff_workbooks(&a, &b, &config)` (no pool argument).
* If the test truly needs engine-level APIs, then call via a session:

  ```rust
  excel_diff::with_default_session(|s| {
      excel_diff::engine::diff_workbooks(&a, &b, s.strings_mut(), &config)
  })
  ```

Likewise for `diff_grids_database_mode(..., pool, config)` (missing pool is explicitly called out in the errors).

#### C) Replace `grid.insert(Cell { row, col, address, ... })`

The benches still do this (and some tests likely do too).

Rewrite to the new “payload-only” insertion API.

**Before**

```rust
grid.insert(Cell {
    row,
    col,
    address: CellAddress::from_indices(row, col),
    value: Some(CellValue::Number(123.0)),
    formula: None,
});
```

**After**

```rust
grid.insert_cell(row, col, CellValue::Number(123.0), None);
```

And for text:

**Before**

```rust
value: Some(CellValue::Text("hello".into())),
```

**After**

```rust
value: CellValue::Text(common::sid("hello")),
```

#### D) Replace direct `cell.address` assertions

**Before**

```rust
assert_eq!(cell.address, CellAddress::new("A1"));
```

**After**

* If you have `(row, col)`:

  ```rust
  assert_eq!(CellAddress::from_coords(row, col).to_a1(), "A1");
  ```
* Or if iterating through a grid map entry:

  ```rust
  for ((row, col), _cell) in sheet.grid.cells.iter() {
      let addr = CellAddress::from_coords(*row, *col);
      // assert on addr
  }
  ```

(Adapt to your actual API for rendering A1 strings.)

#### E) Update `CellSnapshot::from_cell(...)` call sites

`cycle_summary.txt` shows `CellSnapshot::from_cell` now takes `(row, col, &CellContent)` but tests still pass a single `&Cell`.

Rewrite:

**Before**

```rust
let snap = CellSnapshot::from_cell(cell);
```

**After**

```rust
let snap = CellSnapshot::from_cell(row, col, cell_content);
```

(Or prefer a helper in tests if repeated.)

---

### 1.4 Update serialization tests to include the new `strings` field

There are tests asserting top-level JSON keys on serialized reports (example in `codebase_context.md` expects only `complete`, `ops`, `version`, `metrics`). With the string table added, those must be updated to include `"strings"`.

So update expected key lists like:

**Before**

```rust
let expected = ["complete", "ops", "version", "metrics"];
```

**After**

```rust
let expected = ["complete", "ops", "strings", "version", "metrics"];
```

---

### 1.5 Fix benchmarks: make them compile under Branch 4 IR

Bench code in `core/benches/diff_benchmarks.rs` is still using the old IR and is guaranteed to break once you run `cargo bench`.

Two concrete strategies:

#### Strategy A (recommended): switch benches to numeric-only payloads

This avoids interning millions of unique bench strings like `"NEW_ROW_42"`.

* Replace all `CellValue::Text(format!(...))` with a numeric value that still distinguishes the region, e.g.:

  ```rust
  grid_b.insert_cell(row, col, CellValue::Number((row as f64) * 1e6 + col as f64), None);
  ```

#### Strategy B: if you want text benches, intern via a local pool

If you keep text:

* Create a `StringPool` in the bench
* Intern once per unique string
* Use those `StringId`s in `CellValue::Text(id)`

But note this can dominate runtime/memory and make the benchmark less about diffing and more about string allocation.

---

## Phase 2: Implement streaming output (Branch 4.4)

Branch 4.4 requires a `DiffSink` abstraction, `VecSink`, `CallbackSink`, and `JsonLinesSink`, plus refactoring the engine to emit ops through the sink and *not* do a global sort.

### 2.1 Add a sink error pathway to `DiffError`

The spec’s sink APIs return `Result<(), DiffError>`, but your current `DiffError` is oriented around alignment limits and doesn’t cover I/O/serialization failures.

Minimal, low-coupling fix: add a string-based sink error variant.

**Replace (in `core/src/diff.rs`)**

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DiffError {
    #[error(
        "diff limits exceeded on sheet {sheet:?}: rows={rows}, cols={cols} (limits: rows={limit_rows}, cols={limit_cols})"
    )]
    LimitsExceeded {
        sheet: SheetId,
        rows: u32,
        cols: u32,
        limit_rows: u32,
        limit_cols: u32,
    },
}
```

**With**

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DiffError {
    #[error(
        "diff limits exceeded on sheet {sheet:?}: rows={rows}, cols={cols} (limits: rows={limit_rows}, cols={limit_cols})"
    )]
    LimitsExceeded {
        sheet: SheetId,
        rows: u32,
        cols: u32,
        limit_rows: u32,
        limit_cols: u32,
    },

    #[error("sink error: {message}")]
    SinkError { message: String },
}
```

This lets sinks map `std::io::Error` / `serde_json::Error` to `DiffError::SinkError { message }` without forcing `diff.rs` to depend on writer/serde types.

---

### 2.2 Add `DiffSink`, `VecSink`, `CallbackSink`

**Create file: `core/src/sink.rs`**

```rust
use crate::diff::{DiffError, DiffOp};

pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;

    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
    }
}

pub struct VecSink {
    ops: Vec<DiffOp>,
}

impl VecSink {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn into_ops(self) -> Vec<DiffOp> {
        self.ops
    }
}

impl DiffSink for VecSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        self.ops.push(op);
        Ok(())
    }
}

pub struct CallbackSink<F: FnMut(DiffOp)> {
    f: F,
}

impl<F: FnMut(DiffOp)> CallbackSink<F> {
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F: FnMut(DiffOp)> DiffSink for CallbackSink<F> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        (self.f)(op);
        Ok(())
    }
}
```

This matches Branch 4.4’s deliverables while also fixing the `finish(self)` vs `&mut sink` mismatch called out in `spec.md`.

Then export it from `lib.rs`:

**Replace (in `core/src/lib.rs`, module list area)**

```rust
pub mod diff;
pub mod engine;
pub mod workbook;
```

**With**

```rust
pub mod diff;
pub mod engine;
pub mod sink;
pub mod workbook;
```

(Keep your existing exports; this is just showing where to add `sink`.)

---

### 2.3 Add `JsonLinesSink` (streaming writer sink)

Add a new output module and wire it up.

**Replace (in `core/src/output/mod.rs`)**

```rust
pub mod json;
```

**With**

```rust
pub mod json;
pub mod json_lines;
```

**Create file: `core/src/output/json_lines.rs`**

```rust
use crate::diff::{DiffError, DiffOp};
use crate::sink::DiffSink;
use crate::string_pool::StringPool;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct JsonLinesHeader<'a> {
    kind: &'static str,
    version: &'a str,
    strings: &'a [String],
}

pub struct JsonLinesSink<W: Write> {
    w: W,
    wrote_header: bool,
    version: &'static str,
}

impl<W: Write> JsonLinesSink<W> {
    pub fn new(w: W) -> Self {
        Self {
            w,
            wrote_header: false,
            version: "1",
        }
    }

    pub fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        if self.wrote_header {
            return Ok(());
        }

        let header = JsonLinesHeader {
            kind: "Header",
            version: self.version,
            strings: pool.strings(),
        };

        serde_json::to_writer(&mut self.w, &header)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.w
            .write_all(b"\n")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;

        self.wrote_header = true;
        Ok(())
    }
}

impl<W: Write> DiffSink for JsonLinesSink<W> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        serde_json::to_writer(&mut self.w, &op)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.w
            .write_all(b"\n")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.w
            .flush()
            .map_err(|e| DiffError::SinkError { message: e.to_string() })
    }
}
```

This matches the “header then op-per-line” streaming format discussed in `spec.md`, without buffering ops.

---

### 2.4 Refactor the engine to stream ops (and keep old APIs via `VecSink`)

Branch 4.4 requires: “refactor engine to emit through sink rather than return Vec; wrapper uses VecSink.” 

#### A) Add a streaming entrypoint

In `core/src/engine.rs`, add:

* `try_diff_workbooks_streaming(old, new, pool, config, sink) -> Result<DiffSummary, DiffError>`
* `diff_workbooks_streaming(...) -> DiffSummary` (panic-on-error wrapper, mirroring `diff_workbooks`)

You’ll also need a small `DiffSummary` type (can live in `diff.rs` next to `DiffReport`).

**Create (in `core/src/diff.rs`, near `DiffReport`)**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSummary {
    pub complete: bool,
    pub warnings: Vec<String>,
    pub op_count: usize,
}
```

#### B) Implement streaming by moving “push ops” into `sink.emit(op)?`

Mechanically:

* Change functions that currently accept `ops: &mut Vec<DiffOp>` to accept `sink: &mut impl DiffSink`.
* Replace `ops.push(x)` with `sink.emit(x)?`.
* Replace `ops.extend(vec)` with a loop that emits in a defined order.

#### C) Preserve deterministic ordering by construction

Per Branch 4.4: 

* Across sheets: sorted name order (you already have a `SheetKey` with `name_lower`; keep/ensure sort).
* Within a sheet: row-major emission.
* No global final sort step.

Concretely, ensure:

* row/col added/removed indices are emitted sorted increasing
* moves (block moved) are emitted sorted by `(from_start, from_end, to_start, to_end)` or equivalent stable key
* cell edits emitted row-major

#### D) Keep backward compatible `diff_workbooks` returning `DiffReport`

Implement existing non-streaming API by using `VecSink` and then building the report.

**Replace (conceptually, in `try_diff_workbooks`)**

```rust
let mut ops = Vec::new();
... ops.push(...)
Ok(DiffReport::new(ops))
```

**With**

```rust
let mut sink = crate::sink::VecSink::new();
let summary = try_diff_workbooks_streaming(old, new, pool, config, &mut sink)?;
let ops = sink.into_ops();

let mut report = DiffReport::new(ops);
report.complete = summary.complete;
report.warnings = summary.warnings;
report.strings = pool.strings().to_vec();
Ok(report)
```

This satisfies Branch 4.4’s “VecSink wrapper” requirement and sets you up for true streaming sinks.

---

### 2.5 Add the Branch 4 validation tests for streaming + interning

`spec.md` includes a tight validation checklist that maps to Branch 4 acceptance criteria.

Implement these tests:

#### A) `core/tests/string_pool_tests.rs` (50K identical strings)

* Intern `"x"` 50,000 times
* Assert all IDs equal
* Assert pool size is 2 if you pre-intern empty string, else 1

#### B) Streaming vs VecSink equivalence test

* Construct two moderate in-memory workbooks
* Run:

  * streaming engine with `VecSink` (collect ops)
  * streaming engine with `CallbackSink` (count ops)
* Assert counts match, and preferably ops match in order.

#### C) JsonLinesSink structure test

* Build pool + workbooks
* `sink.begin(pool)` writes first line, parse as JSON, assert `kind == "Header"` and `strings` includes expected sheet name
* Emit one op, parse that line as `DiffOp` JSON

---

## Phase 3: Add the WASM build gate (Branch 4.5)

Branch 4.5’s YAML in `next_sprint_plan.md` is wrong for your repo (wrong package name and wrong artifact assumption). `spec.md` already points out the correct approach: build a wasm-only bin target and size-check it.

### 3.1 Add `core/src/bin/wasm_smoke.rs`

This must compile with `--no-default-features` and must not pull filesystem or Excel parsing (keep it IR-only).

**Create file: `core/src/bin/wasm_smoke.rs`**

```rust
use excel_diff::config::DiffConfig;
use excel_diff::sink::CallbackSink;
use excel_diff::string_pool::StringPool;
use excel_diff::workbook::{CellValue, Grid, Sheet, SheetKind, Workbook};

fn main() {
    let mut pool = StringPool::new();

    let sheet = pool.intern("Sheet1");
    let mut a = Grid::new(2, 2);
    let mut b = Grid::new(2, 2);

    a.insert_cell(0, 0, CellValue::Number(1.0), None);
    b.insert_cell(0, 0, CellValue::Number(2.0), None);

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sheet,
            kind: SheetKind::Worksheet,
            grid: a,
        }],
    };

    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sheet,
            kind: SheetKind::Worksheet,
            grid: b,
        }],
    };

    let cfg = DiffConfig::default();
    let mut count = 0usize;
    let mut sink = CallbackSink::new(|_op| count += 1);

    let _ = excel_diff::engine::diff_workbooks_streaming(&wb_a, &wb_b, &pool, &cfg, &mut sink);

    let _ = count;
}
```

Notes:

* This assumes you expose `engine::diff_workbooks_streaming` and it takes an immutable pool reference (ideal for this use case).
* If your streaming API signature still takes `&mut StringPool`, just pass `&mut pool`.

Also: the code intentionally doesn’t print anything, because runtime execution isn’t the goal here; the compile + size gate is.

---

### 3.2 Add `.github/workflows/wasm.yml`

Follow the “superior workable approach” in `spec.md`: correct package name, build a bin, build release, check the `.wasm` output.

**Create file: `.github/workflows/wasm.yml`**

```yaml
name: WASM Build Gate

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  wasm-build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install wasm target
        run: rustup target add wasm32-unknown-unknown

      - name: Build wasm smoke (no default features)
        run: cargo build --release --target wasm32-unknown-unknown --no-default-features -p excel_diff --bin wasm_smoke
        working-directory: core

      - name: Check wasm size budget
        run: |
          SIZE=$(stat -c%s core/target/wasm32-unknown-unknown/release/wasm_smoke.wasm)
          echo "wasm_smoke.wasm size: $SIZE bytes"
          if [ $SIZE -gt 5000000 ]; then
            echo "WASM size $SIZE exceeds 5MB limit"
            exit 1
          fi
```

`spec.md` suggests a 5MB budget; your branch acceptance text mentions 2MB in one place and 5MB in another, so pick 5MB initially (and tighten later once you see real numbers).

### 3.3 If size fails: apply the “first levers”

Per `spec.md`, the first size levers are:

* `panic = "abort"`
* `opt-level = "z"`
* LTO

Best practice: add a custom profile (so you don’t regress native release perf), and build wasm_smoke with `--profile wasm-release`.

---

## Phase 4: Final “Definition of Done” checklist for Branch 4

This is the concrete completion checklist that matches the branch acceptance criteria in `next_sprint_plan.md`.

1. **String interning correctness**

* `string_pool_tests.rs` passes (50K identical strings give identical IDs) 

2. **Coordinate redundancy eliminated**

* No tests reference `cell.row`, `cell.col`, `cell.address` fields on the payload
* Address-based tests use `CellAddress::from_coords` 

3. **Streaming output**

* `DiffSink`, `VecSink`, `CallbackSink`, `JsonLinesSink` exist
* Engine can stream ops without materializing a `Vec<DiffOp>` (except via VecSink wrapper)
* Deterministic order: sheet-name order, then row-major within sheets; no global sort

4. **WASM build in CI + size gate**

* `wasm_smoke` compiles for `wasm32-unknown-unknown` with `--no-default-features`
* CI enforces size budget

5. **Benches compile**

* `cargo bench` works (bench code updated to `insert_cell` and interned strings as needed)

6. **Large workbook memory story**

* You likely can’t enforce “50K rows parseable in 256MB WASM heap” in CI right now, but you can:

  * add a native perf-ish test constructing a 50K x N grid with repeated strings
  * document how to run a browser test with a 256MB cap
