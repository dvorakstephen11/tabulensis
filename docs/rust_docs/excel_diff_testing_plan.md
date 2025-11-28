Here's a concrete, end-to-end testing blueprint reorganized into interleaved vertical-slice phases (per test_reorganization.md). The phases advance container handling, grid parsing, and M parsing together so we surface the grid alignment/memory risks early while still aiming at semantic M-query diffing.

For each phase:
* What the Rust side does
* What to test (unit/integration/property/perf)
* How the Python fixture repo participates

### Priority tags

Use `[G]` for release-gating tests, `[H]` for hardening/nice-to-have, `[E]` for exploratory/fuzz cases, and `[RC]` for resource-constrained guardrails (explicit memory/time ceilings). Phases 1–3 default to `[G]` unless a test is explicitly tagged otherwise; later phases call out `[H]/[E]/[RC]` inline.

### Phase index (map)

| Phase | Focus | IDs | Key risks exercised |
| ----- | ---------------------------- | ------------------- | --------------------- |
| 0 | Harness & fixtures | - | Tooling only |
| 1 | Containers, basic grid IR, WASM build guard | M1–M2, PG1 | H2, H8, WASM |
| 2 | IR semantics (snapshots, M) + streaming budget | PG2–PG3, PG5–PG6, M3–M5 | H2, H3, H7, H8, H9 |
| 3 | MVP diff slice + early DataMashup fuzzing | PG4, M6, G1–G7 | H1, H3, H4, H9 |
| 3.5 | PBIX host support | PBIX-1 | H8, H9 |
| 4 | Advanced alignment & DB mode (incl. adversarial grids) | M7, G8-G13, D1-D10 | H1, H4 |
| 5 | Polish, perf, metrics | M8–M12, P1–P2 | H2, H9, H10, H11, H12 |
| 6 | DAX/model stubs (post-MVP) | DX1 | Data model / DAX |

### MVP readiness

| Capability | Must work before MVP | Can land just before release | Post-MVP |
| ------------------------------- | --------------------------- | ------------------------------ | -------- |
| Excel grid diff | Yes |  |  |
| Excel DataMashup + M diff | Yes |  |  |
| PBIX with DataMashup |  | Yes |  |
| PBIX without DataMashup (tabular model) |  |  | Yes |
| DAX / data model diff |  |  | Yes |

---

## 0. Test harness & repo layout

Before milestones, define the basic shape:

* **Rust repo (`excel-diff-core`)**

  * `src/…` – core parser + diff engine
  * `tests/…` – integration tests that open real `.xlsx`/`.pbix`
  * `fixtures/…` – copied/generated Excel / PBIX files
  * Optional: `testdata_manifest.yaml` – list of scenarios and file paths (consumed by both Python and Rust)

* **Python fixtures repo (`excel-fixtures`)**

  * Python scripts that:

    * Create new Excel/PBIX files from scratch
    * Clone and mutate base fixtures for variant tests
  * Writes to a shared checked‑out directory like `../excel-diff-core/fixtures/generated/…`
  * You can drive it by a manifest keyed by scenario IDs so LLM‑generated Python has a simple contract.

On CI: a setup step runs “generate fixtures” (Python) before `cargo test`. Locally you can run that occasionally or on demand.

--- 

---

### Metrics export for planner agents

To keep the AI “planner” loop data-driven instead of parsing text logs:

* Add a harness feature flag `metrics-export` that writes `target/metrics/current_run.json` after test/bench runs.
* Capture at least:

  * `duration_micros` or `time_ms_per_mb`
  * `alloc_bytes` / `peak_memory_usage_kb` (via `dhat` or a custom allocator wrapper)
  * `aligned_row_ratio` (aligned rows / total rows) for grid tests
* In CI, publish `current_run.json` as an artifact and optionally diff it against a `baseline.json` to spot perf regressions automatically.

---

---

## How the Python fixture repo fits in

To make this all workable with LLM-authored Python:

1. **Define a simple manifest schema** (YAML/JSON) in the Rust repo:

   ```yaml
   - id: m_add_query
     description: "Query added to workbook B"
     kind: excel_pair
     a: "fixtures/generated/m_add_query_a.xlsx"
     b: "fixtures/generated/m_add_query_b.xlsx"
   - id: metadata_simple
     kind: excel_single
     path: "fixtures/generated/metadata_simple.xlsx"
   ```

2. **Python script responsibilities**

   * Reads the manifest.
   * For each entry:

     * If files missing or `--force` flag set, (re)generate them.
   * Generation patterns per scenario:

     * `excel_single`: create workbook with specified queries/settings.
     * `excel_pair`: start from a base version and apply mutations to create B.

3. **Rust tests**

   * Use the same manifest for test discovery:

     * e.g., a helper `load_fixture_pair("m_add_query")` that returns two paths.
   * This keeps Rust tests declarative and stable even as the Python side evolves.

---

---

### Real-world corpus ("Museum of Horrors")

* Maintain an ingested set of non-synthetic workbooks (`fixtures/real_world/**`) from open datasets (Excel from LibreOffice/Apache POI/Office versions).
* Use corpus entries as seeds for fuzzing (Phase 3) and regression tests; never rely solely on Python-generated fixtures.
* Track provenance/allowlist via a small manifest so CI can download or cache them without bloating the repo.

---

---

## Phase 1 - Unified Container (Skeleton)

Goal: build the shared workbook skeleton. Combine the original container-open tests (M1/2) with basic sheet discovery (PG1) so both M and grid data have a home in the same IR.

### WASM compatibility guard ([G], Phase 1)

* CI step: `cargo check --target wasm32-unknown-unknown --no-default-features`.
* Purpose: fail fast if a dependency drags in host-only I/O, threads, or libc assumptions.
* Keep the core parsing/diff crates `no_std`-friendly where feasible; gate any host adapters behind feature flags.
* A deeper wasm smoke test (headless) lands in Phase 2; this Phase 1 gate is a build-only sentinel.

### Milestone 1 – “Hello bytes”: core binary & Excel container I/O

**Rust capability**

* Open a file path.
* Detect/validate that a `.xlsx` is a ZIP container.
* Read raw parts (no DataMashup yet).

**Tests**

### 1.1 Basic file opening

**Goal:** ensure you never regress on simple I/O (paths, errors, cross‑platform).

* **Unit tests (Rust)**

  * `open_file_success`: open a tiny text file in `fixtures/smoke/hello.txt`, assert bytes length > 0.
  * `open_nonexistent_file`: ensure you return a well‑typed error (not panic) with a useful error kind (e.g., `NotFound`).
  * `open_directory_instead_of_file`: error path is correct.

No Python needed here; just check in tiny fixtures.

### 1.2 "Is this a ZIP?" tests

**Goal:** robustly recognize an OPC/ZIP file (Excel / PBIX) vs random inputs.

* **Unit tests ([G] unless noted)**

  * `detect_zip_excel_file`: open `minimal.xlsx` from fixtures; verify that your ZIP detection passes.
  * `reject_non_zip`: pass a `.txt` or random bytes, assert you get a specific error (`NotAnExcelZip`).
  * `[H] reject_non_excel_zip`: feed a ZIP without `[Content_Types].xml` (not an OPC container), assert you get a `NotExcelContainer`-style error rather than a generic failure.

* **Python fixtures**

  * Script to generate `minimal.xlsx` with no Power Query at all (simple workbook with one sheet, a few constant cells).
  * `[H] random_zip.zip`: empty ZIP with a dummy text file to exercise the non-Excel ZIP case.

---

### Milestone 2 - Host container + DataMashup bytes (Excel first)

Now you implement the **host container layer** for Excel: find the `<DataMashup>` part in Excel and base64-decode it. PBIX host support moves to Phase 3.5 so you can harden the Excel path first.

**Rust capability**

* For `.xlsx/.xlsm/.xlsb`:

  * Open `[Content_Types].xml`
  * Iterate `/customXml/item*.xml`
  * Find root element `<DataMashup xmlns="http://schemas.microsoft.com/DataMashup">`
  * Base64-decode its text -> `dm_bytes`

* Keep the parser reusable so the same logic works for `.pbix/.pbit` once enabled in Phase 3.5.

**Tests**

### 2.1 “No DataMashup” vs “Exactly one”

* **Fixtures (Python)**

  * `no_power_query.xlsx`: vanilla workbook.
  * `single_mashup.xlsx`: workbook with a single trivial Power Query.
  * `two_mashups.xlsx`: contrived workbook with two `customXml` entries having `<DataMashup>` (should be invalid, but good robustness test).

  Python would use `openpyxl`/Excel COM or a pre‑baked file copied from a template where you manually created 1–2 Power Queries.

* **Integration tests (Rust)**

  * `extract_mashup_none`: from `no_power_query.xlsx`, your API should clearly report “no DataMashup found” with a benign error variant.
  * `extract_mashup_single`: from `single_mashup.xlsx`, you get non‑empty `dm_bytes`.
  * `extract_mashup_multiple`: from `two_mashups.xlsx`, you either:

    * Choose one deterministically and log/warn, or
    * Return a “multiple DataMashup parts” error. Decide behaviour and codify with the test.

### 2.2 Base64 correctness & corruption

* **Fixtures**

  * `corrupt_base64.xlsx`: same as `single_mashup.xlsx` but you byte‑flip part of the `<DataMashup>` text (Python can open the ZIP, edit that XML string).

* **Tests**

  * `corrupt_base64_errors`: ensure you detect invalid base64 and surface a clear error, not garbage bytes.

---

### PG1 – Workbook → Sheet → Grid IR sanity

Goal: the parser yields a correct `Workbook / Sheet / Grid / Row / Cell` structure from real Excel files. 

### Fixtures

1. `pg1_basic_two_sheets.xlsx`

   * Sheet `Sheet1`: 3×3 block (A1:C3) of constants (numbers + text).
   * Sheet `Sheet2`: 5×2 block (A1:B5) of constants.

2. `pg1_sparse_used_range.xlsx`

   * Sheet `Sparse`:

     * A1 and B2 non‑empty.
     * G10 non‑empty (forces used range to extend).
     * Entire row 5 and column D completely empty in between (to test “holes”).

3. `pg1_empty_and_mixed_sheets.xlsx`

   * Sheet `Empty`: completely empty.
   * Sheet `ValuesOnly`: 10×10 constants.
   * Sheet `FormulasOnly`: 10×10 simple formulas referencing `ValuesOnly`.

4. (Optional later) `pg1_merged_and_hidden.xlsx`

   * A few merged cells over A1:B2, some hidden rows/columns.
   * Purely to codify whatever IR policy you choose for merged/hidden cells (even if policy is “we ignore merges in Grid”).

### Tests

**PG1.1 – Basic workbook structure**

* Open `pg1_basic_two_sheets.xlsx`.
* Assert:

  * `Workbook.sheets.len() == 2`.
  * Sheet names `"Sheet1"` and `"Sheet2"` in order.
  * `Sheet.kind` for both is `Worksheet`.
  * `Sheet1.grid.nrows == 3`, `ncols == 3`.
  * `Sheet2.grid.nrows == 5`, `ncols == 2`.

**PG1.2 – Sparse used range → grid extents**

* Open `pg1_sparse_used_range.xlsx`.
* On `Sparse`:

  * `grid.nrows` and `grid.ncols` match Excel’s used range (should include row/col of G10).
  * `Row.index` values run from 0..(nrows‑1) with no gaps.
  * Cells:

    * There is a non‑empty cell object at `A1`, `B2`, `G10`.
    * All cells outside the used range are either absent or represented as “empty” according to your IR policy (test codifies which).

**PG1.3 – Empty vs non‑empty sheets**

* Open `pg1_empty_and_mixed_sheets.xlsx`.
* Assert:

  * `Empty.grid.nrows == 0` and `ncols == 0`; empty sheets standardize to a 0x0 used range (matching Excel's empty used range semantics).
  * `ValuesOnly.grid.nrows == 10`, `ncols == 10`.
  * `FormulasOnly.grid.nrows == 10`, `ncols == 10`.
  * For `ValuesOnly`, at least one cell has non‑`None` `value` and `formula == None`.
  * For `FormulasOnly`, at least one cell has `formula.is_some()` and `value.is_some()` (parsed formula + cached result).

**PG1.4 – (Optional) Merged / hidden policy**

* Open `pg1_merged_and_hidden.xlsx`.
* Decide your IR rules (e.g., “only top‑left cell in a merged range exists; others are logically empty”).
* Assert that the `Grid` representation of merged and hidden regions matches that rule exactly.

---

---

## Phase 2 - Basic Parsing (Parallel M + Grid)

Goal: parse binaries/XML into Rust IR for both domains and validate memory model early. Bring up addressing, snapshots, and the DataMashup framing/metadata (M3/4/5) alongside grid parsing (PG2/3/5/6).

### Streaming memory budget guard ([RC], Phase 2)

* Fixture: Python generates a ~100MB XML/Excel file with simple rows (e.g., repeating `<row><c><v>1</v></c></row>`).
* Test harness: run the parser under a 50MB heap limit (custom `GlobalAlloc`, `cap`, or OS-level limit) and assert the parse completes.
* Fails the phase if the process OOMs—this proves streaming rather than DOM loading.

### WASM smoke test (headless) ([G], Phase 2)

* Add `wasm-bindgen-test` dev-dependency and a `tests/wasm_smoke.rs` that parses a tiny embedded `.xlsx` byte array.
* Run `wasm-pack test --headless` (or equivalent webdriver runner) in CI for this single case.
* Purpose: ensure the parsing core remains pure and browser-safe beyond the Phase 1 build-only gate.

### PG2 – Addressing and index invariants

Goal: row/column indices, numeric coordinates, and `"A1"` addresses are wired up and consistent everywhere. 

### Fixtures

1. **No Excel file needed** for pure address helpers – use in‑memory tests.

2. `pg2_addressing_matrix.xlsx`

   * Single sheet `Addresses`:

     * Cells populated specifically at:

       * A1, B2, C3, Z1, Z10, AA1, AA10, AB7, AZ5, BA1, ZZ10, AAA1.
     * Each cell’s *text* equals its address (`"A1"`, `"B2"`, etc.) so you can cross‑check easily.

### Tests

**PG2.1 – index_to_address small grid**

* Pure unit tests on helper functions:

  * (0,0) → `"A1"`
  * (0,25) → `"Z1"`
  * (0,26) → `"AA1"`
  * (0,27) → `"AB1"`
  * (0,51) → `"AZ1"`
  * (0,52) → `"BA1"`

**PG2.2 – Round‑trip address_to_index**

* For a list of addresses: `["A1","B2","Z10","AA1","AA10","AB7","AZ5","BA1","ZZ10","AAA1"]`

  * `address_to_index(addr)` → `(r,c)`
  * `index_to_address(r,c)` → `addr`
  * Assert round‑trip equality.

**PG2.3 – IR cells carry correct addresses**

* Open `pg2_addressing_matrix.xlsx`.
* Iterate over all non‑empty cells in the grid:

  * For each cell `c`:

    * Assert `c.address` string equals the text value stored in that cell.
    * Assert converting `(c.row, c.col)` back to an address matches `c.address`.
* This proves the IR’s row/col indices and its notion of `CellAddress` are consistent with the Excel layout.

---

### PG3 – Cell snapshots and comparison semantics

Goal: define and verify what a “cell snapshot” (the payload inside `CellEdited`) contains and how equality works. 

### Fixtures

1. **In‑memory only**: construct `Grid` objects with hand‑built `Cell` values.

2. `pg3_value_and_formula_cells.xlsx`

   * Sheet `Types`:

     * A1: number `42`.
     * A2: text `"hello"`.
     * A3: boolean `TRUE`.
     * A4: empty cell.
     * B1: formula `=A1+1`.
     * B2: formula `="hello" & " world"`.
     * B3: formula that returns boolean (e.g., `=A1>0`).

### Tests

**PG3.1 – Snapshot from basic value cells**

* In‑memory unit tests:

  * Build a `Cell` with `value = Number(42)`, no formula, default format.
  * Call whatever function/materialization builds a `CellSnapshot`.
  * Assert:

    * Snapshot row/col/address match the cell.
    * Snapshot’s value kind is numeric, with 42.
    * Snapshot formula field is `None`.
* Repeat for text and boolean.

**PG3.2 – Snapshot from formula cells**

* Open `pg3_value_and_formula_cells.xlsx`.
* For cells B1, B2, B3:

  * Build snapshots.
  * Assert:

    * `formula` field in snapshot contains the original formula text or AST reference.
    * `value` field contains the displayed value (e.g., `43` for B1, `"hello world"` for B2, `TRUE` for B3).
    * Address/row/col are correct.

**PG3.3 – Snapshot equality semantics**

* Define (in prose, not code) your equality rule for snapshots: e.g.,

  * Equal if type + value + formula text + relevant format fields are equal; ignore volatile formatting like “last calc timestamp”.
* Unit tests:

  * Build pairs of snapshots:

    * Same value, same formula, identical format → `equal`.
    * Same formula text, different numeric result (e.g., different cached value) → either `not equal` or “equal if ignoring cache” depending on your policy (test codifies choice).
    * Same value, trivial formatting difference (if you intend to ignore some formatting) → test either equality or inequality as you intend.

**PG3.4 – Snapshot stability across parse round‑trip**

* Open `pg3_value_and_formula_cells.xlsx`, build snapshots, serialize them (e.g., to JSON) and immediately deserialize.
* Assert the deserialized snapshot equals the original snapshot under your equality rule.

---

### PG5 – In‑memory grid diff smoke tests (no Excel)

Goal: prove the core grid‑diff logic produces the right `DiffOp`s on tiny, hand‑built `Grid`s before you involve Excel parsing, row alignment, keys, etc.

### Fixtures

No files – build `Grid`/`Row`/`Cell` objects entirely in memory.

You’ll want a helper to construct small grids like:

* `grid_1x1(value)`
* `grid_2x2(values)`
* `grid_nxm(...)`

### Tests

**PG5.1 – 1×1 identical grids → empty diff**

* Build `GridA` with one row, one cell = number 1.
* Build `GridB` as an exact clone.
* Run “grid diff” that assumes both grids use `Spreadsheet Mode` and *no* row/column reordering.
* Assert the diff op list is empty.

**PG5.2 – 1×1 value change → single CellEdited**

* Same as PG5.1, but `GridB` cell = 2.
* Assert:

  * Exactly one `DiffOp`.
  * It is `CellEdited` for address `"A1"`.
  * `from` snapshot value is 1, `to` is 2.

**PG5.3 – 1×2 row appended at end**

* `GridA`: one row, one column: A1=1.
* `GridB`: two rows, one column: A1=1, A2=2.
* Run diff.
* Assert:

  * A single `RowAdded` (or contiguous block add) representing row index 1.
  * No `CellEdited` on A1.

**PG5.4 – 2×1 column appended at end**

* `GridA`: one column, two rows (A1,A2).
* `GridB`: two columns, two rows (A1,A2 plus B1,B2).
* Assert:

  * `ColumnAdded` for the new column index.
  * No `CellEdited` on existing cells.

**PG5.5 – Same shape, multiple cell edits, no structure change**

* `GridA`: 3×3 with values 1..9.
* `GridB`: same except three scattered cells changed.
* Assert:

  * Exactly three `CellEdited` ops, with correct addresses and values.
  * No `RowAdded/Removed` or `ColumnAdded/Removed`.

**PG5.6 – Degenerate grids**

* Empty vs empty: both `GridA` and `GridB` have `nrows=0`, `ncols=0`. → diff is empty.
* Empty vs 1×1: treat this as a `RowAdded` + `ColumnAdded` or as a single `CellAdded` depending on your chosen semantics; tests must pin down that choice so later code doesn’t quietly drift.

These tests ensure the diff implementation works in a vacuum before you plug in row/column alignment, keys, or Excel parsing.

---

### PG6 – Object graph vs grid responsibilities

Goal: prove that sheet‑level/object‑graph diff and grid diff are cleanly separated: adding/removing/renaming sheets doesn’t accidentally emit row/column/cell ops.

### Fixtures

1. `pg6_sheet_added_{a,b}.xlsx`

   * A:

     * Sheet `Main`: 5×5 block of constants.
   * B:

     * Sheet `Main`: identical contents.
     * Sheet `NewSheet`: small 3×3 block.

2. `pg6_sheet_removed_{a,b}.xlsx`

   * Reverse of above (B has only `Main`; A has `Main`+`OldSheet`).

3. `pg6_sheet_renamed_{a,b}.xlsx`

   * A: sheet `OldName` with some content.
   * B: same sheet renamed to `NewName`, identical grid.

4. `pg6_sheet_and_grid_change_{a,b}.xlsx`

   * A: sheet `Main` with 5x5 grid, sheet `Aux` untouched.
   * B:

     * `Main`: same size but a couple of cell edits.
     * `Aux`: unchanged.
     * New sheet `Scratch`: some constants.

5. `pg6_renamed_and_changed_{a,b}.xlsx`

   * A: sheet `Summary` with a small data block.
   * B: same sheet renamed to `P&L` with a couple of cell edits inside.

### Tests

**PG6.1 – Sheet addition doesn’t trigger grid ops on unchanged sheets**

* Diff `pg6_sheet_added_a.xlsx` vs `pg6_sheet_added_b.xlsx`.
* Assert:

  * Exactly one `SheetAdded` (`"NewSheet"`).
  * No `Row*` / `Column*` / `Cell*` ops for `Main`.

**PG6.2 – Sheet removal symmetrical**

* Diff `pg6_sheet_removed_a.xlsx` vs `pg6_sheet_removed_b.xlsx`.
* Assert:

  * Exactly one `SheetRemoved` (`"OldSheet"`).
  * No grid ops for `Main`.

**PG6.3 – Sheet rename vs remove+add**

Depending on your object graph diff design: 

* If you support `SheetRenamed`:

  * Diff `pg6_sheet_renamed_a.xlsx` vs `pg6_sheet_renamed_b.xlsx`.
  * Assert:

    * One `SheetRenamed { from: "OldName", to: "NewName" }`.
    * No grid ops.
* If you *don’t* support renames (treat as remove+add):

  * Assert:

    * One `SheetRemoved("OldName")`, one `SheetAdded("NewName")`.
    * Still no grid ops.
* Either way, this proves rename/add/remove does not cascade into bogus cell diffs.

**PG6.4 ? Sheet & grid changes composed cleanly**

* Diff `pg6_sheet_and_grid_change_a.xlsx` vs `_b.xlsx`.
* Assert:

  * `SheetAdded("Scratch")`.
  * For `Main`: whatever `CellEdited`/`Row*`/`Column*` ops are appropriate for the few cell tweaks.
  * No grid ops for `Aux`.
* This ensures that the object-graph layer and grid layer both fire, but in a controlled, separable way.

**PG6.5 ? Rename plus grid edits semantics**

* Diff `pg6_renamed_and_changed_a.xlsx` vs `_b.xlsx`.
* Assert (choose and codify one policy):

  * Preferred: `SheetRenamed("Summary" -> "P&L")` plus the specific `CellEdited` ops for the changed cells.
  * If you treat renames as remove+add: `SheetRemoved("Summary")` + `SheetAdded("P&L")` plus the cell edits for that new sheet.
* No other grid ops on unchanged sheets; this nails down rename semantics when content also changes.

### Milestone 3 – MS‑QDEFF top‑level framing

Now you parse `dm_bytes` into the `Version + 4 length‑prefixed sections` structure. 

**Rust capability**

* Given `dm_bytes`:

  * Read `Version` (u32 LE)
  * Read 4 length fields (u32 LE each)
  * Slice buffer into: `package_bytes`, `permissions_bytes`, `metadata_bytes`, `permission_bindings_bytes`
  * Enforce invariants (version, sizes, no overflow).

**Tests**

### 3.1 “Happy path” framing

* **Fixture source**

  * You can generate `dm_bytes` synthetically in Rust for unit tests (bypass Excel) since this is pure binary. For integration tests, use `single_mashup.xlsx` and run host‑extraction first.

* **Unit tests**

  * `parse_minimal_zero_lengths`: construct a `dm_bytes` buffer with:

    * `Version = 0`
    * All length fields = 0
    * Total size = 4 + 4*4
    * Assert you get empty slices for all four sections and no error.
  * `parse_basic_nonzero_lengths`: synthetic buffer where each section is e.g., `b"AAAA"`, `b"BBBB"`, etc. Assert all slices match exactly.

### 3.2 Invariant tests (negatives)

From your blueprint: version must be 0 for now, lengths must fit within the buffer. 

* **Unit tests**

  * `reject_future_version`: set `Version = 1`, assert either a `FutureVersion` warning/error depending on your policy.
  * `reject_truncated_stream`: create buffer where `PackagePartsLength` is larger than actual bytes; ensure `BoundsError` not panic.
  * `reject_overflow_sum`: craft lengths whose sum > buffer; parser must fail gracefully.

### 3.3 Round‑trip framing

* **Integration tests**

  * Extract `dm_bytes` from a real workbook, parse into sections, then re‑emit the header (keeping section bytes identical) and confirm it equals the original `dm_bytes`. This proves your slicing is correct and non‑destructive.

Python is not needed for these tests beyond providing the original workbook.

---

### Milestone 4 – Semantic sections: PackageParts / Permissions / Metadata / Bindings

Now you implement the **semantic layer** per the blueprint: treat `PackageParts` as ZIP, parse Permissions XML, Metadata XML, and treat Permission Bindings as opaque. 

**Rust capability**

* From the four slices, you can construct:

```text
struct DataMashup {
    version: u32,
    package_parts: PackageParts,
    permissions: Permissions,
    metadata: Metadata,
    permission_bindings_raw: Vec<u8>,
}
```



### 4.1 PackageParts / OPC tests

**Fixtures (Python)**

* `one_query.xlsx`: workbook with a single query `Section1/Foo`.
* `multi_query_with_embedded.xlsx`: workbook:

  * Several queries.
  * At least one `Embedded.Value` using `/Content/{GUID}` (e.g., referencing a small static table). 

Python can create these by:

* Recording Excel macros or using COM automation to add Power Queries; or
* If that’s too annoying, start from a manually built template `.xlsx` checked into the fixtures repo and only tweak cells, not the mashup.

**Tests**

* `package_parts_contains_expected_entries`:

  * Open `one_query.xlsx`
  * Ensure:

    * `/Formulas/Section1.m` exists
    * `/Config/Package.xml` exists
    * `/Content/…` is empty
* `embedded_content_detection`:

  * Open `multi_query_with_embedded.xlsx`
  * Ensure:

    * `/Content/{GUID}` entries are found
    * Each can be opened as a nested OPC package
    * Their own `/Formulas/Section1.m` is non‑empty

### 4.2 Permissions XML tests

**Fixtures**

* `permissions_defaults.xlsx`: workbook with Power Query and default privacy/firewall settings.
* `permissions_firewall_off.xlsx`: workbook where you explicitly set “Ignore privacy level checks” in Power Query.

**Tests**

* `permissions_parsed_flags`:

  * Confirm `FirewallEnabled` and `WorkbookGroupType` (or equivalent) flip between the two fixtures.
  * If XML is missing or malformed, default values are correct (design tests for that too, e.g., by corrupting the Permissions bytes manually).

### 4.3 Metadata XML tests

Per blueprint, Metadata XML has `LocalPackageMetadataFile` with `Formulas` entries keyed by `SectionName/FormulaName`, plus load destinations etc. 

**Fixtures**

* `metadata_simple.xlsx`: 2 queries:

  * `Section1/LoadToSheet` → load to table on sheet.
  * `Section1/LoadToModel` → load only to data model.
* `metadata_query_groups.xlsx`: queries organized in folders (Power Query groups).
* `metadata_hidden_queries.xlsx`: some queries are connection‑only / not loaded.

**Tests**

* `metadata_formulas_match_section_members`:

  * Parse Metadata XML.
  * Parse `Section1.m` into members.
  * Assert number of `ItemType=Formula` entries equals number of shared members in `Section1.m` (minus known oddities like step entries) – your blueprint already calls this out as an invariant. 
* `metadata_load_destinations`:

  * For each query, assert load settings from Metadata (sheet vs model vs both) match what you manually configured when creating the fixtures.
* `metadata_groups`:

  * Assert group hierarchy (e.g., “Inputs/DimTables”) is correctly mapped.

### 4.4 Permission Bindings

At this stage you don’t need to decrypt/validate; you just surface presence. 

* `permission_bindings_present_flag`:

  * On a normal workbook, `has_bindings` should be true if the field is non‑empty.
* `permission_bindings_missing_ok`:

  * On a contrived `dm_bytes` with zero‑length bindings, ensure you handle gracefully.

---

### Milestone 5 – Domain layer: M queries & metadata API

Here you build an ergonomic API like:

```text
struct Query {
    name: String,           // "Section1/Foo"
    section_member: String, // "Foo"
    expression_m: String,   // raw M code for this query
    metadata: QueryMetadata
}
```



**Rust capability**

* Split `Section1.m` into named members.
* Associate each member with its metadata entry by `SectionName/FormulaName`.
* Produce an ordered `Vec<Query>` and top‑level `Metadata` object.

**Tests**

### 5.1 Section1.m splitting

**Fixtures**

* `section_single_member.m` (unit test only).
* `section_multiple_members.m` with:

  * `section Section1;`
  * `shared Foo = ...;`
  * `shared Bar = ...;`
  * Private member `Baz = ...;`

**Unit tests**

* `parse_single_member_section`:

  * Ensure you get one `section_member: "Foo"`, `expression_m` contains exactly that body.
* `parse_multiple_members`:

  * Shared members recognized as queries; private ones either included/excluded depending on API design. Tests codify that decision.
* `tolerate_whitespace_comments`:

  * Add comments and random blank lines; ensure splitting still works.

### 5.2 Query–metadata join

**Integration tests (using real Excel)**

* `metadata_join_simple`:

  * From `metadata_simple.xlsx`, ensure:

    * Query list has two queries: `Section1/LoadToSheet`, `Section1/LoadToModel`.
    * Each has metadata where `load_to_sheet`, `load_to_model` flags match expectations.
* `metadata_join_url_encoding`:

  * Create a query with special characters in the name that require URL encoding (`"Query with space & #"`).
  * Ensure you correctly map metadata `ItemPath` to section member after decoding.

### 5.3 Domain invariants

* `query_names_unique`:

  * Ensure `name` is unique within a DataMashup instance.
* `metadata_orphan_entries`:

  * Build a fixture where metadata lists a `Section1/Nonexistent` formula (edit XML manually). Your parser should:

    * Either drop it with a warning, or
    * Surface it as an “orphan” entry; the test should assert your chosen behaviour.

---

---

## Phase 3 - MVP Diff (Vertical Slice)

Goal: first end-to-end diff slice. Define the DiffOp contract, ship textual M diffs, and land the simplest spreadsheet-mode diffs (G1?G7) so the product can compare real files even before advanced alignment.

### Early DataMashup fuzzing (bit-flip) [E?G if stable]

* Move fuzz hardening up from later phases: add a `cargo-fuzz` target that mutates length prefixes and payload bytes of the `DataMashup` framing parser.
* Seed corpus with valid headers extracted from golden fixtures plus a few “weird” real-world workbooks (LibreOffice/Apache POI outputs).
* Success: no panics; parser returns `Result::Err` for corrupted streams. Run for at least 1 hour in CI/nightly.

### PG4 – DiffOp plumbing & wire contract

Goal: all `DiffOp` variants exist, serialize properly, and can be round‑tripped independently of any real diff algorithm.

### Fixtures

No Excel fixtures required – this is purely type‑level / serialization testing.

### Tests

**PG4.1 – Construct each DiffOp variant**

* Unit tests that *manually* construct:

  * `CellEdited { sheet, addr, from: snapshot1, to: snapshot2 }`
  * `RowAdded { sheet, row_idx, row_signature, … }`
  * `RowRemoved { … }`
  * `ColumnAdded`, `ColumnRemoved`, `BlockMovedRows`, `BlockMovedColumns` (or whatever set you decide).
* For each, assert that:

  * Mandatory fields are present and non‑default.
  * Optional fields behave as intended (e.g., `block_hash` maybe `None`).

**PG4.2 – JSON (or wire‑format) shape**

* For each DiffOp instance from PG4.1:

  * Serialize to your wire format (probably JSON for CLI/web).
  * Assert (as string or as parsed JSON object) that:

    * Enum tags are as documented (e.g., `"kind": "CellEdited"` or similar).
    * Sheet identifiers, addresses, and snapshot payloads appear under the expected keys.
    * No extraneous / internal fields leak through.

**PG4.3 – Round‑trip stability**

* Deserialize the JSON back to a `DiffOp`.
* Assert:

  * The variant type is identical.
  * Key fields (`sheet`, `addr`, `from`, `to`) match the original.
* This codifies the contract between engine and frontends/CLI that the meta‑process relies on.

**PG4.4 – DiffOp list / report container**

* Construct a “fake report” with a small vector/list of DiffOps mixing several variants.
* Serialize and deserialize the whole collection.
* Assert order and contents are preserved.
* This is the object you’ll later stream out of the end‑to‑end diff pipeline, so locking it early avoids painful schema changes later.

---

### Milestone 6 – Textual M diff engine (first working diff)

Before AST semantic diffing, you’ll probably stand up a simpler text‑level diff for M code so you can ship something sooner and then harden it.

**Rust capability**

* Given two `DataMashup` domain objects:

  * Align queries by `name` (Section1/QueryName).
  * Report:

    * Added queries
    * Removed queries
    * Changed queries (text diff on `expression_m`)
    * Metadata‑only changes

Think of a `MQueryDiff` type like:

```text
enum QueryChangeKind {
    Added,
    Removed,
    Renamed { from: String, to: String },
    DefinitionChanged,
    MetadataChangedOnly
}
```

(You might not support rename yet; tests can initially treat renames as removed+added, then evolve.)

**Tests**

### 6.1 Basic M diffs

**Fixtures (Python)**

* `m_add_query_{a,b}.xlsx`:

  * A: one query `Foo`.
  * B: queries `Foo`, `Bar` (same result data).
* `m_remove_query_{a,b}.xlsx`:

  * Reverse of the above.
* `m_change_literal_{a,b}.xlsx`:

  * A: `Foo` with `= 1`.
  * B: `Foo` with `= 2`.
* `m_metadata_only_change_{a,b}.xlsx`:

  * A: `Foo` loads to sheet.
  * B: `Foo` load destination is model only; M code identical.

**Integration tests**

For each `{a,b}` pair:

* Run full pipeline: open Excel → extract DataMashup → domain queries → diff.
* Assert:

  * `m_add_query`: exactly one diff with kind `Added` for `Bar`.
  * `m_remove_query`: one `Removed` for `Bar`.
  * `m_change_literal`: single `DefinitionChanged` for `Foo`; optionally assert the text diff shows changed literal.
  * `m_metadata_only_change`: `MetadataChangedOnly` for `Foo` (no `DefinitionChanged`).

### 6.2 Renames (optional first pass)

**Fixture**

* `m_rename_query_{a,b}.xlsx`:

  * A: query `Foo` with body `= 1`.
  * B: query `Bar` with identical body and metadata.

**Test**

* Initially, your engine may report `Removed(Foo)` and `Added(Bar)`. Codify that behaviour in tests.
* Later, when you add rename detection (e.g., based on identical expression + metadata), update the test to assert a single `Renamed { from: "Foo", to: "Bar" }`.

### 6.3 Embedded contents

**Fixture**

* `m_embedded_change_{a,b}.xlsx`:

  * Both versions have same top‑level queries.
  * Only change is inside `/Content/{GUID}/Formulas/Section1.m` for an embedded mini‑mashup used by `Embedded.Value`.

**Tests**

* Decide domain model:

  * Either treat embedded contents as separate “queries” or as an attribute of the parent query.
* Then assert:

  * A diff exists precisely at the embedded content location.
  * No spurious changes reported on unrelated queries.

---

These assume you already have `Workbook` → `Sheet` → `Grid` IR and basic DiffOp types like `CellEdited`, `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, `BlockMoved`.

Each milestone has:

* **Core capability** — what the engine must correctly do.
* **Fixture sketch** — what kind of workbook(s) to generate.
* **Checks** — what the diff output should look like.

---

### G1 – Identical sheet → empty diff

**Core capability**

The grid diff engine can compare two small, identical sheets and produce **no grid‑level DiffOps**.

**Fixture sketch**

* `equal_sheet_{a,b}.xlsx`

  * One worksheet, 5×5 grid, simple constants (numbers, strings).
  * B is a byte‑for‑byte copy of A.

**Checks**

* Diff emits **no** `RowAdded/Removed`, `ColumnAdded/Removed`, or `CellEdited`.
* Any workbook/sheet‑level “compared these two sheets” metadata is present, but **diff list is empty**.

---

### G2 – Single cell literal change

**Core capability**

Detect one changed value in an otherwise identical grid and surface a single `CellEdited` op.

**Fixture sketch**

* `single_cell_value_{a,b}.xlsx`

  * A: 5×5 grid, constants.
  * B: identical except `C3` changes from `1` to `2`.

**Checks**

* Exactly one `CellEdited` for `C3`.
* Old/new value snapshots correct; no row/column structure ops emitted.

---

### G3 – Single cell formula change (same value vs different value)

**Core capability**

Distinguish **formula change** from **value change**, using formula ASTs where available.

**Fixture sketch**

* `single_cell_formula_same_result_{a,b}.xlsx`

  * A: `C3 = A3 + B3`.
  * B: `C3 = B3 + A3` (commutative, same result for all current inputs).
* `single_cell_formula_diff_result_{a,b}.xlsx`

  * A: `C3 = SUM(A1:A10)`.
  * B: `C3 = AVERAGE(A1:A10)`.

**Checks**

* For “same result” pair: canonicalized ASTs equal → either:

  * No `CellEdited` at all, **or**
  * A dedicated “FormulaFormattingOnly” flag, but *not* a semantic change.
* For “different result” pair: `CellEdited` at `C3` with a **formula‑change detail**, not just a raw “value changed”.

---

### G4 – Format‑only change vs content change

**Core capability**

Ensure pure formatting edits don’t show up as logical changes when configuration says “ignore formatting”.

**Fixture sketch**

* `format_only_{a,b}.xlsx`

  * A: 5×5 numbers, default formatting.
  * B: same values & formulas, but some cells bold, different colors, different number formats.

**Checks**

* With “ignore formatting” option on:

  * No grid diff ops at all.
* With “show formatting” option on:

  * Diffs are typed as “format changed” (whatever format DiffOps you define), but **no row/column/cell content ops**.

---

### G5 – Multiple independent cell edits in a fixed grid

**Core capability**

Handle multiple scattered cell edits without collapsing them into bogus row/column structure changes.

**Fixture sketch**

* `multi_cell_edits_{a,b}.xlsx`

  * A: 20×10 grid of constants.
  * B: 5–10 cells scattered across different rows and columns changed (mix of numbers and strings).

**Checks**

* Diff lists exactly those addresses as `CellEdited` (correct old/new values).
* No `RowAdded/Removed` or `ColumnAdded/Removed` for this scenario.

---

### G6 – Simple row append / truncate (bottom of sheet)

**Core capability**

Detect rows appended or removed **at the end** of a sheet and surface explicit row add/remove ops.

**Fixture sketch**

* `row_append_bottom_{a,b}.xlsx`

  * A: rows 1–10 with simple sequential IDs in column A.
  * B: rows 1–12 (rows 11–12 newly added).
* `row_delete_bottom_{a,b}.xlsx`

  * A: rows 1–12.
  * B: rows 1–10.

**Checks**

* Append case: two `RowAdded` ops for logical row indices 11 and 12; no spurious `CellEdited`.
* Delete case: two `RowRemoved` ops for 11 and 12.

---

### G7 – Simple column append / truncate (right edge)

**Core capability**

Symmetric to G6 but for columns.

**Fixture sketch**

* `col_append_right_{a,b}.xlsx`

  * A: columns A–D filled.
  * B: A–F (E and F new).
* `col_delete_right_{a,b}.xlsx`

  * A: columns A–F.
  * B: columns A–D.

**Checks**

* Append: `ColumnAdded` for E, F; no cell‑by‑cell updates.
* Delete: `ColumnRemoved` for E, F.

---

--- 

## Phase 3.5 - PBIX host support (post-Excel)

Goal: reuse the Excel DataMashup parser for `.pbix/.pbit` containers once framing/metadata (Milestones 3–5) are stable. This is release-gating for PBIX, but `[H]` for the Excel-first MVP.

### PBIX presence/absence

**Fixtures (Python) [H]**

* `legacy.pbix`: a small PBIX with queries (includes a `DataMashup` file).
* `enhanced_metadata.pbix`: PBIX where Power BI no longer stores `DataMashup` (tabular model only).

**Integration tests ([H] until PBIX ships)**

* `extract_mashup_pbix_legacy`: confirm you find `DataMashup` and produce bytes via the shared Excel parser.
* `extract_mashup_pbix_enhanced`: return a structured domain error like `NoDataMashupUseTabularModel` (tabular-only path), not a panic.

---

## Phase 4 - Algorithmic Heavy Lifting

Goal: tackle the hardest algorithms. Add semantic M AST diffing (M7), advanced spreadsheet alignment/moves (G8?G12), and the database-mode keyed diffs (D1?D10) that exercise H1/H4 risks.

### Milestone 7 - Semantic (AST) M diffing

This is where your differentiator shows up: semantic AST diffing rather than raw text diff, using a hybrid **GumTree + APTED** strategy.

**Rust capability**

* Parse `expression_m` into AST.
* Normalize irrelevant differences:

  * Whitespace
  * Comments
  * Possibly step order when it is semantically irrelevant (careful here).
* Implement hybrid AST differencing: **GumTree** for scalable move/rename detection, **APTED** for exact edits on small/medium ASTs and unmatched sub-forests.
* Compare ASTs for semantic equality / produce a semantic diff with move awareness.

**Tests**

### 7.1 Formatter round-trip is a no-op

**Fixtures**

* `m_formatting_only_{a,b}.xlsx`:

  * A: ugly M with no newlines.
  * B: the same query run through a different pretty-printer (re-indented, newlines/comments shuffled), identical semantics.
* `m_formatting_only_{b_variant}.xlsx`:

  * Same as B but with a single identifier tweak (e.g., `Table1` -> `Table2`) for a negative-control diff.

**Tests**

* Parse A vs B to AST and diff: **zero** semantic DiffOps allowed (formatting-only changes must not surface).
* Parse A vs `b_variant`: expect a single semantic change (identifier rename or load target) to be reported.

### 7.2 Reordering non‑dependent steps

If you choose to treat some reorders as semantic‑no‑ops, you need tests for it.

* `m_step_reorder_{a,b}.xlsx`:

  * A & B differ only by the order of independent `let` bound steps.
* Assert semantic diff is empty.

### 7.3 Specific semantic changes

Test cases that map directly onto user‑visible Power Query edits:

* `m_filter_added_{a,b}.xlsx`:

  * A: query without filter step.
  * B: same query with a `Table.SelectRows` filter on a column (“Region = EMEA”).
* `m_column_removed_{a,b}.xlsx`:

  * A: includes a `RemovedOtherColumns` step keeping `["A","B","C"]`.
  * B: keeps `["A","B"]` only.

Tests should assert:

* There is a `DefinitionChanged` with structured detail, e.g.,

  * Change type: “step added” with name and signature.
  * Or “filter predicate changed on column Region from `<> null` to `= "EMEA"`”.

You don’t have to implement the reporting format yet, but the test can at minimum assert that:

* Exactly one “semantically significant” change is reported for the query.
* The description mentions the step’s name or type.

---

### 7.4 Hybrid AST Strategy Validation

**Fixtures**
* `m_ast_deep_skewed_{a,b}.xlsx`: (APTED test) Deeply nested IFs (~500 nodes) designed to trigger Zhang-Shasha worst cases. B has a minor edit.
* `m_ast_large_refactor_{a,b}.xlsx`: (GumTree test) Large query (> 3000 nodes). A large expression block is moved from one `let` binding to another in B.
* `m_ast_wrap_unwrap_{a,b}.xlsx`: (Precision test) A function is wrapped (e.g., `Value(X)` -> `Table.Buffer(Value(X))`).

**Tests**
* `apted_robustness`: Diff `m_ast_deep_skewed`. Assert APTED completes quickly (O(N^3)) and finds the minimal edit.
* `gumtree_moves_and_scale`: Diff `m_ast_large_refactor`. Assert the engine completes quickly (validating scale) and explicitly reports the change as a `Move` operation (validating semantics).
* `precision_wrap`: Diff `m_ast_wrap_unwrap`. Assert the diff reports an `Insert(Table.Buffer)` node with `Value(X)` as a child, demonstrating structural understanding rather than a text replacement.

### Spreadsheet-Mode advanced alignment (G8?G12)

#### G8a - Adversarial repetitive patterns [RC] [CRITICAL UPDATE]

**Core capability**

Validate that the Hybrid Alignment Pipeline (Patience/Myers/Histogram) avoids the O(N^2 log N) pathology of Hunt-Szymanski on repetitive data.

**Fixture sketch**

* `adversarial_grid_repetitive_{a,b}.xlsx`:

  * 50,000 rows. 99% of rows are identical (e.g., blank rows). A few unique rows exist as headers/footers.
  * B is similar to A but with a block of 1000 blank rows inserted in the middle.

**Checks**

* The Patience pass must correctly anchor the unique rows.
* Wrap diff in a strict timeout (e.g., < 1 second). Assert the engine does not exhibit super-linear performance degradation.
* Assert the diff correctly identifies the 1000 `RowAdded` ops.

#### G8 – Single row insert/delete in the middle (row alignment)

**Core capability**

Use row signatures / LCS so inserting or deleting a single row in the **middle** of the sheet doesn’t mark everything below as changed.

**Fixture sketch**

* `row_insert_middle_{a,b}.xlsx`

  * A: rows 1–10 with an ID column and stable content.
  * B: identical except a new row inserted between 5 and 6.
* `row_delete_middle_{a,b}.xlsx`

  * A: rows 1–10.
  * B: identical except row 6 removed.

**Checks**

* Exactly **one** `RowAdded` or `RowRemoved` at the appropriate logical position.
* Rows below the insertion/deletion (7–10) are aligned correctly — no `CellEdited` / phantom changes.

---

#### G9 – Single column insert/delete in the middle (column alignment)

**Core capability**

Column‑signature LCS alignment works just like row alignment when a column is inserted or removed in the middle.

**Fixture sketch**

* `col_insert_middle_{a,b}.xlsx`

  * A: columns A–H with stable header row and data.
  * B: new column inserted between C and D.
* `col_delete_middle_{a,b}.xlsx`

  * A: A–H.
  * B: without column D.

**Checks**

* One `ColumnAdded` / `ColumnRemoved`.
* Cells in columns after the insertion/deletion line up correctly and only show diffs where actual content changed.

---

#### G10 – Contiguous block of rows inserted / deleted

**Core capability**

Treat a **block of adjacent rows** as a block add/remove, not as a mixture of adds and edits.

**Fixture sketch**

* `row_block_insert_{a,b}.xlsx`

  * A: rows 1–10.
  * B: an additional block of rows 4–7 inserted, with distinctive content.
* `row_block_delete_{a,b}.xlsx`

  * A: rows 1–10.
  * B: rows 4–7 removed.

**Checks**

* Either:

  * One `BlockAddedRows { start_row, count }` / `BlockRemovedRows`, **or**
  * A sequence of four `RowAdded`/`RowRemoved` with contiguous indices and no spurious `CellEdited`.
* Rows outside the block perfectly aligned.

---

#### G11 – Block move (rows) detection

**Core capability**

Detect when a contiguous block of rows has **moved** rather than been removed and re‑added, using block hashing / similarity.

**Fixture sketch**

* `row_block_move_{a,b}.xlsx`

  * A: rows 1–20 with a distinctive 4‑row block (e.g., rows 5–8 tagged “BLOCK”).
  * B: same data, but that 4‑row block moved to rows 13–16 with identical content.

**Checks**

* Diff emits a single `BlockMovedRows { from: 5..8, to: 13..16 }` (or equivalent), **not** 4 removes + 4 adds.
* No `CellEdited` inside the moved block.

---

#### G12 – Column / rectangular block move

**Core capability**

Same as G11, but for columns and 2D rectangular blocks.

**Fixture sketch**

* `column_move_{a,b}.xlsx`

  * A: columns A–H, with column C clearly distinguishable.
  * B: column C moved to position F.
* `rect_block_move_{a,b}.xlsx`

  * A: a 3×3 data block at (rows 3–5, cols B–D).
  * B: same block moved to a new location but unchanged internally.

**Checks**

* `BlockMovedColumns` or equivalent for column move.
* For rectangle: a single rectangular move op (or combination of row/column moves) and no `CellEdited` in the block.

---

#### G13 - Fuzzy Move Detection (LAPJV)

**Core capability**

Validate that the LAPJV solver correctly identifies blocks that were moved and subsequently edited.

**Fixture sketch**

* `grid_move_and_edit_{a,b}.xlsx`:

  * A distinctive 10-row block is moved from the top to the bottom.
  * In B, 2 cells inside the moved block are edited.

**Checks**

* The engine must report a `BlockMoved` operation (not Delete+Insert).
* The engine must also report the 2 `CellEdited` operations within the moved block.

---

These milestones exercise the **keyed, database‑mode diff** where row order is irrelevant and rows are matched on primary keys.

---

### D1 – Keyed equality (no differences)

**Core capability**

When a sheet/table is in Database Mode with a known key, identical data produces **no row/cell diffs**, regardless of row order.

**Fixture sketch**

* `db_equal_ordered_{a,b}.xlsx`

  * A: table with columns `[ID, Name, Amount]`, IDs 1..10.
  * B: same rows, same order.
* `db_equal_reordered_{a,b}.xlsx`

  * B: same rows but randomly permuted.

**Checks**

* In both pairs, diff yields no `RowAdded/Removed` and no `CellEdited`.
* In the reordered pair, engine successfully matches by key instead of row index.

---

### D2 – Single keyed row added / removed

**Core capability**

Treat new/missing keys as single row add/remove events.

**Fixture sketch**

* `db_row_added_{a,b}.xlsx`

  * A: IDs 1..10.
  * B: IDs 1..11 (new ID = 11).
* `db_row_removed_{a,b}.xlsx`

  * A: IDs 1..11.
  * B: IDs 1..10.

**Checks**

* Exactly one `RowAdded` / `RowRemoved` with key = 11.
* No cell edits on shared keys.

---

### D3 – Keyed row updated (non‑key column changes)

**Core capability**

Detect changes to non‑key fields on an existing key as **cell edits**, not new rows.

**Fixture sketch**

* `db_row_update_{a,b}.xlsx`

  * Same set of IDs.
  * For ID = 7, `Amount` changes from 100 to 120; all other columns equal.

**Checks**

* One `CellEdited` (for that row’s `Amount`), keyed to ID = 7.
* No row add/remove events.

---

### D4 – Pure reorder vs structural diff

**Core capability**

Prove reordering alone is ignored, but reordering *plus* changes still surfaces changes correctly.

**Fixture sketch**

* `db_reorder_only_{a,b}.xlsx`

  * A and B identical except random row order.
* `db_reorder_and_change_{a,b}.xlsx`

  * Same as above, plus one record’s `Amount` changed.

**Checks**

* Reorder‑only: empty diff.
* Reorder+change: diff only lists cell edits on changed keys; no structural ops.

---

### D5 – Composite primary key

**Core capability**

Support multi‑column keys and match rows correctly on their combination.

**Fixture sketch**

* `db_composite_key_{a,b}.xlsx`

  * Key is `[Country, CustomerID]`.
  * Add a row that creates a new `[Country, CustomerID]` pair; change one non‑key column for an existing pair.

**Checks**

* New combination → `RowAdded`.
* Existing combination with changed non‑key field → `CellEdited`.
* No false matches when only one part of composite key matches.

---

### D6 – Duplicate key clusters surfaced explicitly

**Core capability**

Detect duplicate keys on either side and expose them as a special case (cluster diff) rather than silently mis‑matching.

**Fixture sketch**

* `db_duplicate_keys_{a,b}.xlsx`

  * A: ID 5 appears twice with slightly different data.
  * B: ID 5 appears twice with a different pair of rows.

**Checks**

* Engine surfaces a `DuplicateKeyCluster { key: 5, left_rows, right_rows }` or equivalent.
* Within that cluster, small Hungarian / best‑match logic can be tested (e.g., each left row paired with the closest right row).
* No assumption that there is a single canonical row for that key.

---

### D7 – User‑provided key vs metadata vs heuristic

**Core capability**

Respect **explicit key choice** over metadata or heuristic inference.

**Fixture sketch**

* `db_key_priority_{a,b}.xlsx`

  * Sheet with columns `[RowID, CustomerID, Name]`.
  * Metadata suggests table key is `RowID`.
  * User specifies key = `CustomerID`.

**Checks**

* Diff uses `CustomerID` as key:

  * Reordering by `RowID` does not cause diffs.
  * Changes that only affect `RowID` (with constant customer) are not treated as row identity changes.
* A separate test where user *doesn’t* supply key checks that metadata key is used instead.

---

### D8 – Simple heuristic key inference (unambiguous)

**Core capability**

Infer a key when neither user nor metadata provides one, using uniqueness / null‑rate heuristics.

**Fixture sketch**

* `db_infer_key_simple_{a,b}.xlsx`

  * Columns `[Index, Name, Amount]`.
  * `Index` is 1..N, unique, no blanks. Other columns have duplicates.
  * B: reorders rows, changes one `Amount`.

**Checks**

* Engine chooses `Index` as inferred key; reorder only yields no diffs, changed `Amount` yields a single `CellEdited`.
* Expose inferred key in diff report (for debug / transparency).

---

### D9 – Ambiguous key inference → safe fallback

**Core capability**

When no clear key exists (e.g., all columns have duplicates or nulls), fall back safely (e.g., row‑index or spreadsheet‑mode diff) and clearly report that key inference failed.

**Fixture sketch**

* `db_infer_key_ambiguous_{a,b}.xlsx`

  * Small table where all columns have many duplicates and nulls.
  * B: reorder rows.

**Checks**

* Engine reports “no reliable key” / Database Mode fallback.
* Diff behavior is documented:

  * Either treat as Spreadsheet Mode (so reorder shows as row moves), **or**
  * Treat row index as implicit key, so reorder shows as structure diff.
* The important part is: **no bogus “no diff”** when data has truly changed; and behavior is deterministic.

---

### D10 – Mixed sheet: database table region + free‑form grid

**Core capability**

Handle a sheet that has a structured, keyed table region (Database Mode) plus free‑form cells around it (Spreadsheet Mode) and diff each region appropriately.

**Fixture sketch**

* `db_mixed_sheet_{a,b}.xlsx`

  * Table `Sales` (keyed by `OrderID`) in range `B3:F100`.
  * Free‑form commentary / formulas above and to the right.
  * B: changes some table rows and some standalone cells, reorders the table rows.

**Checks**

* Table region:

  * Reorder ignored; only true data changes surfaced as keyed row/cell diffs.
* Free‑form region:

  * Spreadsheet Mode semantics: cell and possibly row/column structure diffs.
* No cross‑contamination where free‑form cells get treated as part of the table, or vice versa.

---

---

## Phase 5 - Polish & Production

Goal: polish and production hardening. Cover workbook-level M scenarios, fuzzing/regressions, performance suites (including grid perf P1/P2), and product-level CLI/API contracts.

### Milestone 8 – Workbook‑level M diff scenarios (end‑to‑end)

Now you move from micro‑cases to realistic, multi‑query workbooks – the kind of thing you’d demo.

**Rust capability**

* Given two Excel files, produce a full “M diff report” at workbook scope:

  * Query additions/removals/renames
  * Per‑query semantic diffs
  * Metadata diffs (load destinations, groups, privacy flags)
  * Summary stats (“3 queries changed, 2 added, 1 removed”)

**Fixtures (Python)**

Here your Python repo shines. Define named scenarios and let Python build both A and B:

1. **“ETL Pipeline with staging + facts + dimensions”**

   * A: 5–10 queries:

     * `StgSales`, `StgCustomers`, `DimDate`, `DimCustomer`, `FactSales`.
   * B: modifications:

     * Add a new dimension query (`DimProduct`).
     * Change filter in `StgSales` to drop test data.
     * Change group of `DimCustomer` (“Dimensions / Customers” -> “Master Data”).

2. **“Broken but structurally similar pipeline”**

   * A & B have same queries and steps, but in B:

     * One key step changes join type from Inner to LeftOuter.

3. **“Query load changes only”**

   * A: some queries load to sheet; B: those toggled to model only.

4. **“Mixed Excel+PBIX sample set”**

   * A: `.xlsx`; B: `.pbit` with equivalent queries (for future cross‑host consistency tests).

**End‑to‑end tests**

* `pipeline_scenario_detailed_diff`:

  * Run full diff on scenario (1).
  * Assert high‑level expectations:

    * `added_queries` count is 1; name `DimProduct`.
    * `changed_queries` includes `StgSales` with at least one semantic diff.
    * Group membership diff reported for `DimCustomer`.
* `join_type_change_surface_cause`:

  * Scenario (2): ensure diff explanation clearly indicates join type change (not just “Text changed”).
* `load_settings_only`:

  * Scenario (3): check that queries appear as `MetadataChangedOnly` without `DefinitionChanged`.

These tests double as validation that your earlier, more granular tests actually compose correctly.

---

### Milestone 9 – Fuzzing, golden tests, and regression harness

Once the basics work, you want to be very hard to break.

### 9.1 Golden oracles (Data Mashup Explorer / Cmdlets)

Your parser blueprint already suggests using Ben Gribaudo’s tools as oracles. 

* **Offline golden files**

  * For a subset of fixtures, check in:

    * The output of `Export-DataMashup -Raw` (Section1.m) and `Export-DataMashup -Item Metadata`.
  * Write tests that:

    * Parse workbook with Rust.
    * Serialize its internal representation back to JSON.
    * Compare against golden JSON (loose comparison to allow for field ordering differences).

This catches subtle schema interpretation bugs.

### 9.2 Property‑based tests on the binary framing

From your blueprint’s own suggestions: test invariants like length sums. 

* **Property tests**

  * Randomly generate valid `dm_bytes` headers and random body slices; ensure your parser never panics and always:

    * Either returns consistent slices or
    * Flags a specific error for invalid cases.
  * Randomly corrupt lengths, bytes, etc., and assert you get facing errors.

### 9.3 Differential fuzzing vs binwalk / OPC library

Use a binwalk‑like scan as a sanity check that your `PackageParts` slice really contains a ZIP with entries where you think they are. 

* For randomly selected fixtures:

  * Run your framing parser.
  * Independently scan `dm_bytes` with binwalk or a Rust ZIP library to find PK signatures.
  * Assert that your `package_parts` slice covers exactly the region that contains the primary ZIP.

### 9.4 Regression harness

Whenever you find a real‑world workbook that breaks something:

* Drop it into `fixtures/bugs/{issue-id}`.
* Add:

  * A focused unit/integration test describing the failure.
  * If useful, also a simplified hand-crafted reconstruction for faster tests.
* This gradually builds a "museum" of bad but realistic inputs.

### 9.5 Grid diff property/fuzz tests

**Property tests [E]**

* Generate small random grids, apply random edit sequences (cell edits, row/column inserts or deletes, simple block moves).
* Diff original vs mutated and assert:

  * Every emitted `DiffOp` matches an applied mutation; no "extra" ops.
  * Applying the diff ops back to A reconstructs B for simple cases.
* Keep grid sizes tiny for CI but run enough seeds to shake out alignment bugs, mirroring the DataMashup framing property tests.

---

### Milestone 10 – Performance & scalability testing

This ties into the “Compare 100MB in under 2 seconds” claim in your product plan. 

**Fixtures (Python)**

Generate large synthetic workbooks:

* Many queries (hundreds).
* Deep `let` chains.
* Large embedded contents.

Python can automate:

* Cloning a base query and tweaking a literal per copy.
* Creating minimal data tables to keep file size realistic but manageable.

**Perf tests**

* Use a non‑test harness (e.g., `cargo bench` or a small benchmark binary) rather than unit tests, but keep them well‑defined:

  * `parse_only_large_workbook`: time to open Excel → DataMashup → domain.
  * `diff_large_workbooks`: time to diff 2 large but similar workbooks.
* Track:

  * Total time.
  * Peak memory (if feasible in your environment).
* When `metrics-export` is enabled, emit `target/metrics/current_run.json` with `parse_time_ms_per_mb`, `peak_memory_usage_kb`, and `alignment_efficiency_score` for P1/P2 scenarios.
* Store baselines in CI; fail if regressions exceed a threshold.

---

### Milestone 11 – Product‑level tests: CLIs, APIs, and UX contracts

From the competitive analysis, your product should support CLI/Git and later Web/WASM. 

**CLI tests**

Assuming a CLI like:

```text
excel-diff m-diff old.xlsx new.xlsx --format json
```

* **Integration tests**

  * Run the binary against multiple `{a,b}` pairs from the earlier scenarios.
  * Assert:

    * Exit code 0 on success.
    * JSON parses successfully.
    * Schema fields (`queries_changed`, `queries_added`, etc.) match what the library already asserts.

**API contract tests**

If your Rust crate exposes a public API (used by e.g. GUI or WASM):

* Add tests that treat it as a black box:

  * Given two file paths → returns `DiffReport` object.
  * Serialize `DiffReport` and ensure it’s stable (versioned). This matters for web clients and future backward compatibility.

---

### Milestone 12 - Cross-platform determinism

Goal: enforce identical diff output across supported platforms/builds (Windows/Linux native + WASM).

**Tests ([H] now, [G] before multi-platform release)**

* Run the same small canonical fixture suite (one per major scenario) on native Windows, native Linux, and headless WASM in CI.
* Assert the JSON diff output is identical (or matches after a stable canonicalization); fail on drift.
* Capture artifacts on mismatch to debug float/order issues; keep the same wire schema for all targets.

---

You already have a general perf milestone, but two grid-focused ones are worth calling out separately, because the H1 difficulty item explicitly calls out "high-performance 2D grid diff" as one of the hardest problems.

---

### P1 – Large dense sheet, minimal changes

**Core capability**

Row/column alignment and cell diff remain near‑linear on a large dense sheet when only a few cells change.

**Fixture sketch**

* `grid_large_dense_{a,b}.xlsx`

  * A: 50k rows × 50 columns with synthetic but deterministic data.
  * B: identical except for 50 random `CellEdited` changes.

**Checks**

* Measured time and memory within agreed budget (record baseline in CI).
* Diff only lists those ~50 `CellEdited` ops; no spurious structure changes.

---

### P2 – Large sheet with random noise (worst‑case alignment)

**Core capability**

Even when row signatures collide and there is lots of noise, the Hunt–Szymanski alignment doesn’t degenerate to catastrophic behavior.

**Fixture sketch**

* `grid_large_noise_{a,b}.xlsx`

  * A: large sheet with pseudo‑random data.
  * B: another large sheet with different random data of the same shape.

**Checks**

* Runtime stays within acceptable multiple of P1 (record in perf suite).
* Diff primarily reports per‑cell edits; no pathological explosion in DiffOps or runtime.



---

## Phase 6 - DAX/model stubs (post-MVP placeholder)

Goal: anchor the later DAX/data-model diff work so the plan stays aligned with the architecture while staying explicitly post-MVP.

### DX1 - Measure parsing smoke test

**Fixtures (Python) [H, post-MVP]**

* `dax_measures_{a,b}.pbix` or `.xlsx` with a minimal tabular model containing two measures (e.g., `TotalSales` = SUM, `AvgSales` = AVERAGE); in B change one measure definition.
* Keep the fixture tiny; static checked-in files are fine if automating tabular model generation is painful.

**Tests ([H], post-MVP)**

* Parse measures into an AST and assert a simple change (SUM -> AVERAGE or literal tweak) surfaces as a structured `DefinitionChanged` (or equivalent) on that measure.
* Tag the scenario as post-MVP so it does not block Excel/M release trains but keeps DAX/data-model semantics on the radar.

---

Last updated: 2025-11-25 14:04:00
