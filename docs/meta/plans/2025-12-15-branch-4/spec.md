Below is a concrete, code-oriented implementation plan that will complete **Branch 4: WASM & Memory Readiness** as written in `next_sprint_plan.md` , using the actual current code structure described in `codebase_context.md` (core crate layout, current IR types, container API, JSON output, etc.). 

I’m also calling out a few parts of Branch 4 that are internally inconsistent or likely to cause pain, and I’m folding better guidance directly into the plan (without expanding scope beyond Branch 4).

---

## Key issues in Branch 4 that should be corrected up front

### 1) The WASM CI snippet is currently wrong (package name + release artifact)

Branch 4.5’s YAML references `-p excel_diff_core` and checks `target/.../release/excel_diff_core.wasm`, but the actual package name in `core/Cargo.toml` is `excel_diff`  and (as a library) it will not reliably emit a `*.wasm` artifact unless you build a **bin** or **cdylib** target. 

**Fix in plan:** add a tiny wasm-only **bin target** (e.g. `src/bin/wasm_smoke.rs`) and size-check that output; also build with `--release` if you’re checking a `release/` artifact.

### 2) `DiffSink::finish(self)` conflicts with `&mut S` usage

Branch 4.4 defines `finish(self)` while `diff_workbooks_streaming` takes `sink: &mut S`. You can’t call `finish(self)` on a borrowed sink. 

**Fix in plan:** either remove `finish` from the trait (most flexible) or make it `finish(&mut self)` and keep it optional.

### 3) StringId correctness across two workbooks requires a shared string pool (or remapping)

Branch 4.1 requires `CellValue::Text(StringId)` and `SheetId = StringId`. 
If workbook A and workbook B were interned into **different** pools, then “same text” would have different IDs and equality/hashing would break badly (especially in alignment).

**Fix in plan:** make diffing always happen under a **single shared `StringPool`**, and parse both workbooks using that pool.

### 4) Streaming + “header line contains string table”

Branch 4.4’s “streaming output” is compatible with emitting a header first **only if** the string table is known before ops start. 
If you build the string table lazily from ops, you can’t do that without either buffering ops or emitting incremental dictionary updates.

**Fix in plan:** because we’re sharing a `StringPool` during parsing, the pool is known after parsing and before diffing. That supports “header first” cleanly (even if the table is large). If later you want to shrink the table to “strings referenced by ops only,” that’s a separate optimization.

---

## Implementation plan by Branch 4 section

### 4.1 String interning

#### A. Add `StringId` and `StringPool` (new file)

Create `core/src/string_pool.rs` and export from `lib.rs`.

**New file: `core/src/string_pool.rs`**

```rust
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StringId(pub u32);

#[derive(Debug, Default)]
pub struct StringPool {
    strings: Vec<String>,
    index: FxHashMap<String, StringId>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut p = Self::default();
        p.intern("");
        p
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.index.get(s) {
            return id;
        }
        let id = StringId(self.strings.len() as u32);
        let owned = s.to_owned();
        self.strings.push(owned.clone());
        self.index.insert(owned, id);
        id
    }

    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }
}
```

> Note: this is the straightforward spec version. If later you need to eliminate the “double string storage” in `index`, you can replace the map with a hash-bucket design, but don’t do that in this branch unless memory profiling proves it’s required. 

#### B. Update `CellValue` to use `StringId`

Current `CellValue` uses `Text(String)`/`Error(String)` (see `workbook.rs`). 
Change it to `Text(StringId)` and `Error(StringId)`. Also keep your existing `Bool` variant name unless you want to do a repo-wide rename; Branch 4’s spec uses `Boolean`, but that’s not required for the memory goal. 

**Replace this snippet in `core/src/workbook.rs`:** 

```rust
pub enum CellValue {
    Blank,
    Number(f64),
    Text(String),
    Bool(bool),
    Error(String),
}
```

**With:**

```rust
use crate::string_pool::StringId;

pub enum CellValue {
    Blank,
    Number(f64),
    Text(StringId),
    Bool(bool),
    Error(StringId),
}
```

#### C. Thread `StringPool` through parsing and diff

This is the architectural requirement that keeps IDs consistent.

**Add a “session/context” that owns the pool**:
Create `core/src/session.rs` (or put into `excel_open_xml.rs` if you prefer), with a minimal type:

```rust
use crate::string_pool::StringPool;
use crate::workbook::Workbook;

pub struct DiffSession {
    pub strings: StringPool,
}

impl DiffSession {
    pub fn new() -> Self {
        Self { strings: StringPool::new() }
    }

    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    pub fn strings_mut(&mut self) -> &mut StringPool {
        &mut self.strings
    }
}
```

Now adjust parsing entrypoints to accept `&mut StringPool` and return workbooks whose `StringId`s refer into that pool.

---

### 4.2 Eliminate coordinate redundancy (and intern formulas)

The current `Cell` stores coordinates redundantly and `Grid` stores `FxHashMap<(u32,u32), Cell>`. 
Branch 4.2 wants to store only `(row,col)` in the map key and keep only the content in the value, plus intern formulas. 

#### A. Update `Cell` -> `CellContent` and `Grid` storage

**Replace this snippet in `core/src/workbook.rs`:** 

```rust
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub cells: FxHashMap<(u32, u32), Cell>,
}
```

**With:**

```rust
use crate::string_pool::StringId;

pub struct CellContent {
    pub value: Option<CellValue>,
    pub formula: Option<StringId>,
}

pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub cells: FxHashMap<(u32, u32), CellContent>,
}

impl Grid {
    pub fn insert_cell(&mut self, row: u32, col: u32, value: Option<CellValue>, formula: Option<StringId>) {
        self.cells.insert((row, col), CellContent { value, formula });
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&CellContent> {
        self.cells.get(&(row, col))
    }
}
```

#### B. Add `CellAddress::from_coords(row, col)`

You already have `CellAddress::from_indices(row, col)` in `workbook.rs`. 
Add a compatibility alias method for Branch 4.2 naming. 

**Replace this snippet in `core/src/workbook.rs`:** 

```rust
impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> Self {
        Self { row, col }
    }
}
```

**With:**

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

#### C. Refactor all code that accessed `cell.row` / `cell.col`

This is mechanical but wide. Do it systematically:

* Replace iterations like `for cell in grid.cells.values()` that use `cell.row`/`cell.col`
* With `for ((row, col), cell) in grid.cells.iter()`

Hotspots in this repo (based on current modules listed in `codebase_context.md`): 

* `grid_view.rs` (builds per-row/col views)
* `column_alignment.rs`, `row_alignment.rs`
* `database_alignment.rs`
* `engine.rs` and helpers (e.g., `grids_non_blank_cells_equal` currently reads `cell.row`/`cell.col`) 
* `signature_tests.rs` and any tests creating `Cell { row, col, address, ... }`

#### D. Update signature computations in `workbook.rs`

`compute_all_signatures` currently buckets cells into rows by reading `cell.row` and stores `&Cell`. 
Update to bucket using the HashMap key and store `&CellContent`.

**Before (representative):** 

```rust
for cell in self.cells.values() {
    if cell.row < self.nrows && cell.col < self.ncols {
        row_cells[cell.row as usize].push(cell);
    }
}
```

**After:**

```rust
for ((row, col), cell) in self.cells.iter() {
    if *row < self.nrows && *col < self.ncols {
        row_cells[*row as usize].push((*col, cell));
    }
}
```

Then update any downstream hashing that expected `&Cell` to accept `&CellContent` (and pass col separately if needed).

---

### 4.1 + 4.2 together: Update parsing to intern strings and store `CellContent`

Parsing is in `excel_open_xml.rs` / `grid_parser.rs`. 
You must modify the parser so it can be invoked twice (old + new) against the **same** pool.

#### A. Shared strings parsing: return `Vec<StringId>`

Currently `parse_shared_strings` builds `Vec<String>`. 
Change it to take `pool: &mut StringPool` and return `Vec<StringId>`.

**Replace this signature in `core/src/excel_open_xml.rs`:** 

```rust
fn parse_shared_strings(xml: &[u8]) -> Result<Vec<String>, ExcelOpenError>
```

**With:**

```rust
use crate::string_pool::{StringId, StringPool};

fn parse_shared_strings(xml: &[u8], pool: &mut StringPool) -> Result<Vec<StringId>, ExcelOpenError>
```

Implementation: whenever you parse a `<t>` node, call `pool.intern(text)` and push the id.

#### B. Sheet XML parsing: intern inline strings and formulas

In `grid_parser.rs`, `convert_value` currently returns `CellValue::Text(String)` and formulas are `Option<String>`. 
Change it to accept `pool: &mut StringPool`, use `shared_strings: &[StringId]`, and return interned IDs.

**Replace this signature:** 

```rust
fn convert_value(cell_type: &str, raw: &str, shared_strings: &[String]) -> Option<CellValue>
```

**With:**

```rust
use crate::string_pool::StringPool;

fn convert_value(cell_type: &str, raw: &str, shared_strings: &[StringId], pool: &mut StringPool) -> Option<CellValue>
```

Then:

* For shared string cells: `Text(shared_strings[idx])`
* For inline/str: `Text(pool.intern(raw))`
* For errors: `Error(pool.intern(raw))`

For formulas in `parse_sheet_xml`:

* Parse the formula text
* Intern: `let formula_id = pool.intern(&formula_text);`
* Store: `formula: Some(formula_id)`

And insert into the grid via `grid.insert_cell(row, col, value, formula)`.

#### C. Sheet names: intern and store as `SheetId`

Current engine uses `Sheet.name: String` and `SheetId = String` for ops. 
Branch 4.1 wants `SheetId = StringId`. 
So:

* In the workbook model (`Sheet` struct), store `name: StringId`.
* Keep a helper for sorting by name lexicographically via pool resolution.

---

### 4.1: Update DiffOp / DiffReport serialization to include string table

Current `diff.rs` defines `pub type SheetId = String;` and `DiffReport { version, ops, complete, warnings, metrics }`. 
Branch 4 requires `SheetId = StringId`, ops referencing ids, and serialization including a string table in metadata. 

#### A. Update `SheetId`

**Replace in `core/src/diff.rs`:** 

```rust
pub type SheetId = String;
```

**With:**

```rust
use crate::string_pool::StringId;

pub type SheetId = StringId;
```

#### B. Update DiffOp to use ids

Every variant that currently contains `sheet: SheetId` will now store a `StringId` (u32). That’s a structural change but straightforward.

#### C. Add string table to DiffReport

Add a field like:

* `pub strings: Vec<String>`

and set it from the pool used during parsing.

**Important guidance (superior to the plan as written):**

* Do **not** clone strings if you can avoid it.
* Prefer: construct the pool in the top-level diff pipeline and *move* `pool.strings` into the report when you’re producing a full `DiffReport`.
* For truly streaming sinks (JSON lines), you can write the table once from the pool before emitting ops, and never allocate a second copy.

Concretely:

* Keep `StringPool` as the authority.
* For `DiffReport`, store `Vec<String>` and build it either by cloning or (preferred) moving at the top-level wrapper (where you own the pool).

---

### 4.4 Streaming output (DiffSink + streaming engine)

Your current engine returns `DiffReport` and pushes ops into a `Vec`. 
Branch 4.4 requires a sink-based pipeline that can stream ops without materializing. 

#### A. Define `DiffSink` (fixing the finish-signature issue)

**Branch spec (inconsistent):** has `finish(self)` but uses `&mut sink`. 

Use this instead:

```rust
pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;
}
```

If you want a finalization hook, add it as optional:

```rust
pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;
    fn finish(&mut self) -> Result<(), DiffError> { Ok(()) }
}
```

#### B. Implement sinks

* `VecSink { ops: Vec<DiffOp> }` for compatibility
* `JsonLinesSink<W: Write>` for streaming
* `CallbackSink<F>` for embedding into host apps

#### C. Refactor engine functions to emit rather than return Vec

Do this in the smallest number of steps:

1. Add a new internal entrypoint in `engine.rs`:

```rust
pub fn try_diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError>
```

2. Change the top-level `diff_workbooks` wrapper to:

* Build a `VecSink`
* Call `try_diff_workbooks_streaming(...)`
* Create `DiffReport` from `VecSink.ops` (+ string table + warnings/metrics)

3. Refactor leaf functions incrementally:

* Convert any function returning `Vec<DiffOp>` into `fn(..., sink: &mut impl DiffSink) -> Result<(), DiffError>`
* Replace `ops.push(...)` with `sink.emit(...) ?`

Start with the highest-volume emission points:

* `positional_diff` (cell edits)
* row/col inserted/deleted emission
* move emissions

This preserves correctness and buys streaming memory wins quickly.

#### D. Ordering guarantees (deterministic-by-construction)

Branch 4.4’s ordering resolution is: row-major within sheet, sheet-name order across sheets, no global sort. 

Implementation rules:

* When iterating sheets: create a sorted list of sheet IDs by `pool.resolve(sheet_id)` lexicographic order.
* Within a sheet:

  * Emit structural ops in increasing index order (rows inserted/deleted, cols inserted/deleted)
  * Emit move ops in deterministic order (sort move list by `(from, to, count)` or by old region start)
  * Emit cell edits by `for row in 0..min_rows { for col in 0..min_cols { ... } }` (already row-major if you do it in nested loops)

Add a test that compares ops produced by streaming vs vec sink for exact equality including order, *unless* you intentionally change ordering (Branch spec says order may differ). 
Given Branch 4 explicitly defines ordering, it’s better to make them match exactly and test for exact equality.

---

### 4.4: JsonLinesSink (including optional header line)

Implement `JsonLinesSink` in `core/src/output/json.rs` or a new module `core/src/output/json_lines.rs` under `output/`.

Recommended behavior:

* A `begin(&StringPool, &DiffReportMetadata)` method writes one header JSON line:

  * includes `version`
  * includes `strings` table (the pool’s vector)
* Then each op is one line.

Because the string pool is known after parsing both workbooks, header-first is feasible without buffering. 

---

### 4.3 Abstract I/O to Read + Seek (OpcContainer)

Current `OpcContainer` is hard-wired to `ZipArchive<File>` and `open(path)`. 
Branch 4.3 requires `open_from_reader<R: Read + Seek>` and gating path-based open behind `std-fs`. 

#### A. Refactor `OpcContainer` to accept any reader

To avoid turning the entire crate generic, use a boxed trait object for the archive reader.

**Replace in `core/src/container.rs`:** 

```rust
pub struct OpcContainer {
    pub(crate) archive: ZipArchive<File>,
}

impl OpcContainer {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ContainerError> {
        let file = File::open(path)?;
        let archive = ZipArchive::new(file)?;
        Ok(Self { archive })
    }
}
```

**With:**

```rust
use std::io::{Read, Seek};
use zip::ZipArchive;

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct OpcContainer {
    pub(crate) archive: ZipArchive<Box<dyn ReadSeek>>,
}

impl OpcContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(reader: R) -> Result<Self, ContainerError> {
        let archive = ZipArchive::new(Box::new(reader))?;
        Ok(Self { archive })
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ContainerError> {
        let file = std::fs::File::open(path)?;
        Self::open_from_reader(file)
    }
}
```

Then update all call sites (notably `excel_open_xml.rs`) to use `open_from_reader` / `open_from_path`. 

#### B. Add `std-fs` feature flag

In `core/Cargo.toml` add:

* `std-fs = []`
* Make it part of default features for native builds if desired.

---

### 4.5 WASM build gate (CI + smoke test)

Branch 4 requires “core crate compiles to WASM,” a size budget check, and a smoke test. 
As noted earlier, the provided YAML has multiple issues. Here’s the superior, workable approach.

#### A. Add a wasm smoke binary

Create: `core/src/bin/wasm_smoke.rs`

This must compile with `--no-default-features` (so avoid `excel_open_xml`, avoid filesystem). It should:

* Construct two tiny `Workbook`s programmatically
* Run `diff_workbooks_streaming` with a `CallbackSink` that just counts ops
* Return success

This validates: wasm compilation + core algorithm works + no accidental `std::fs` pulls.

#### B. Add `.github/workflows/wasm.yml`

Key changes vs the plan:

* Use correct package: `-p excel_diff` (from `core/Cargo.toml`) 
* Build `--release`
* Build a bin target so you get `*.wasm`
* Size check the actual produced file

Example (conceptual):

```yaml
- name: Build wasm smoke
  run: cargo build --release --target wasm32-unknown-unknown --no-default-features -p excel_diff --bin wasm_smoke
  working-directory: core

- name: Check size
  run: |
    SIZE=$(stat -c%s core/target/wasm32-unknown-unknown/release/wasm_smoke.wasm)
    if [ $SIZE -gt 5000000 ]; then
      echo "WASM size $SIZE exceeds 5MB limit"
      exit 1
    fi
```

#### C. About “parse small workbook” in wasm smoke test

Branch 4.5 says “parse small workbook, run diff.” 
That’s a bad fit with `--no-default-features` (you lose `excel_open_xml`), and it will bloat the wasm binary with zip/xml parsing dependencies.

**Superior guidance:** the wasm smoke test should build the IR in memory. Keep parsing tests native.

If you really want a wasm parsing test later, do a second CI job that builds with `--features excel-open-xml` but without `std-fs`, and feed bytes via `open_from_reader(Cursor::new(bytes))`. That’s extra and not required to complete Branch 4’s core readiness.

---

## Tests and validation checklist (maps to Branch 4 acceptance criteria)

### 1) String interning correctness test (50K identical strings)

Add `core/tests/string_pool_tests.rs`:

* Create pool
* Intern `"x"` 50,000 times
* Assert all ids are identical
* Assert `pool.len()` is `2` if you pre-interned `""` (empty string) or `1` if you didn’t. 

### 2) Coordinate redundancy compile-time + runtime checks

* Update IR tests that currently assert `cell.address` fields (because those fields will no longer exist).
* Replace with computed `CellAddress::from_coords(row, col)`.

### 3) Core crate compiles to wasm

* CI job does `cargo build --target wasm32-unknown-unknown --no-default-features -p excel_diff --bin wasm_smoke` 

### 4) Streaming output works without full materialization

* Create a test that diffs two moderately sized in-memory workbooks and runs:

  * VecSink: collects ops
  * CallbackSink: counts ops
* Assert count matches, and (preferably) ops equality in order. 

### 5) WASM size budget check

* Enforce <5MB on `wasm_smoke.wasm` in release. 
  If you can’t hit this immediately, the *first* levers are:
* `panic = "abort"` in `[profile.release]`
* `opt-level = "z"`
* LTO enabled

### 6) “50K row workbook parseable without OOM in 256MB WASM heap”

In practice you can’t reliably enforce this in CI without a wasm runtime + memory instrumentation, but you can:

* Add a native perf-ish test that constructs a 50K x N grid (with repeated strings) and ensures it finishes within some memory/time envelope (best effort).
* Document how to run a browser test with a 256MB wasm memory limit.

---

## Practical migration strategy (so this branch doesn’t explode in rework)

Because the `Cell` -> `CellContent` refactor touches many modules, do the work in this order:

1. Land `StringPool` and `StringId` (no call-site changes yet).
2. Change IR (`CellValue`, `CellContent`, `Grid`) and fix compilation (no behavior changes).
3. Update parser to populate new IR.
4. Update engine to accept pool for resolving low-info logic and to emit ops using the new `SheetId`.
5. Add DiffSink + VecSink and switch engine internals from “push vec” to “emit sink.”
6. Add JsonLinesSink and wasm job last (so you’re not debugging wasm while refactoring IR).

---

## What I would *not* do in this branch (even though it’s tempting)

* Don’t try to optimize the `StringPool` index to avoid storing keys twice unless profiling shows it matters; the big win is eliminating per-cell string clones. 
* Don’t attempt to “shrink string table to only referenced strings” unless you also introduce a dictionary-update streaming format (it becomes a separate design decision).
* Don’t make `OpcContainer` generic over `R` unless you’re willing to propagate generics through a large portion of the parsing surface. Boxing the reader keeps the refactor contained.

---

If you want, I can follow this plan one step further and produce a **file-by-file “edit script”** (ordered list of exact functions to touch + search patterns like `cell.row`, `CellValue::Text(`, `SheetId`, etc.), but the above is already sufficient to implement Branch 4 end-to-end against the current repo structure.
