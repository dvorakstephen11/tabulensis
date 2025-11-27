# Verification Report: 2025-11-26-container-and-grid-ir

## Summary

This cycle successfully stands up the first real Excel Open XML → IR pipeline plus A1 addressing helpers. The implemented modules (`workbook`, `addressing`, `excel_open_xml`) and the public API on `lib.rs` match the mini-spec and the architecture blueprint. Container detection and workbook/grid parsing behave as specified on the Phase 1 (M1.1/M1.2, PG1) and Phase 2 (PG2) fixtures, and all planned tests are present and non-trivial. I did not find any critical correctness bugs for the covered scenarios, but I did find a small number of gaps and edge cases that should be tracked for future cycles (used-range semantics for non‑A1‑anchored sheets, missing tests for some error variants, and some value/streaming edge cases).

Overall, the implementation is sound for the intended scope and fixtures. I recommend proceeding to release, with the identified issues recorded as follow-up items for upcoming milestone or refactor cycles.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### 1. Used-range semantics for sheets that don’t start at A1

- **Severity**: Moderate  
- **Category**: Gap  
- **Description**:  
  The mini-spec and testing plan describe `Grid.nrows`/`ncols` as the *logical used range height/width* for a sheet.

  The implementation computes:

  * `dimension_from_ref(ref)` as:

    ```rust
    let (start_row, start_col) = address_to_index(start)?;
    let (end_row, end_col) = address_to_index(end)?;
    let height = end_row.checked_sub(start_row)?.checked_add(1)?;
    let width  = end_col.checked_sub(start_col)?.checked_add(1)?;
    Some((height, width))
    ```
    so it returns *height/width* only, discarding the starting row/column.:contentReference[oaicite:1]{index=1}  

  * `parse_sheet_xml` then:

    * Tracks `max_row`/`max_col` from each parsed cell (0-based indices from `address_to_index`).  
    * Sets:

      ```rust
      let mut nrows = dimension_hint.map(|(h,_)| h).unwrap_or(0);
      let mut ncols = dimension_hint.map(|(_,w)| w).unwrap_or(0);

      if let Some(r) = max_row {
          nrows = nrows.max(r + 1);
      }
      if let Some(c) = max_col {
          ncols = ncols.max(c + 1);
      }
      ```
      :contentReference[oaicite:2]{index=2}  

  * `build_grid` then constructs a `Grid` with rows `0..nrows` and columns `0..ncols`, and uses the *global* zero-based row/col indices from `ParsedCell` (derived from A1 addresses) to write into `rows[parsed.row][parsed.col]`.  

  For the current fixtures, the top-left used cell is always `"A1"`, so `start_row == 0` and `height == end_row + 1`; `nrows`/`ncols` therefore match the intended used range and all tests pass.

  However, for a sheet whose used range starts at (for example) `"B2"` (dimension `"B2:D5"` and cells only in that box):

  * `dimension_from_ref` returns `height = 4` even though the max row index is `end_row = 4`.  
  * `parse_sheet_xml` computes `nrows = max(height, max_row + 1) = max(4, 5) = 5`.  
  * The grid will have indices `0..4`, with real cells at `(1..4, 1..3)` and an *extra* empty row and column at index 0.

  The IR therefore represents a used range whose top-left is logically `"A1"` rather than `"B2"`: the real data is shifted “down/right” inside a larger rectangle. This still satisfies the tests’ weaker requirement “includes row 10 and column G” for the sparse PG1 fixture, but it diverges from the more precise statement in the testing plan that `grid.nrows`/`ncols` match the Excel used range.:contentReference[oaicite:5]{index=5}

- **Evidence**:
  * `dimension_from_ref`, `parse_sheet_xml`, and `build_grid` in `core/src/excel_open_xml.rs`.  
  * PG1 sparse tests only assert that `nrows`/`ncols` *include* G10, not that the range is minimal or anchored to the first used cell.  

- **Impact**:
  * For sheets where the first non-empty cell is not at A1, the IR will include leading empty rows/columns that are considered part of the “used range”.
  * Future grid diff logic may interpret those leading empties as meaningful structure (e.g., spurious row/column insertions), increasing noise and possibly memory usage for high-offset sheets.
  * This behavior is not exercised by current fixtures, so the issue is latent; it’s a good candidate for a follow-up cycle that tightens used-range semantics and/or introduces an explicit “origin” for grids.

---

### 2. Missing tests for WorkbookXmlMissing / WorksheetXmlMissing error paths

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The error enum includes variants for workbook and worksheet XML missing:

  ```rust
  pub enum ExcelOpenError {
      Io(#[from] std::io::Error),
      NotZipContainer,
      NotExcelOpenXml,
      WorkbookXmlMissing,
      WorksheetXmlMissing(String),
      XmlParseError(String),
  }
````



and the implementation uses them:

* Missing `xl/workbook.xml` → `ExcelOpenError::WorkbookXmlMissing`.
* Failing to read a sheet part via `read_zip_file` → `ExcelOpenError::WorksheetXmlMissing(target.to_string())`.

The mini-spec does *not* explicitly require tests for these paths in this cycle; it only mandates container-open tests for minimal.xlsx, nonexistent path, random_zip.zip, and no_content_types.xlsx.

The integration test file `core/tests/excel_open_xml_tests.rs` implements exactly those tests plus one additional `not_zip_container_returns_error`, but there are no tests that exercise the `WorkbookXmlMissing` or `WorksheetXmlMissing` variants.

* **Evidence**:

  * `ExcelOpenError` enum and its use sites in `excel_open_xml`.
  * `core/tests/excel_open_xml_tests.rs` integration tests.

* **Impact**:

  * The behavior of these explicit error variants is untested and could regress silently in future refactors (for example, mapping to a generic `XmlParseError` or `Io` instead).
  * This doesn’t affect current fixtures, but given that the error surface is part of the public API the lack of tests slightly weakens the guarantee that callers can rely on precise error variants for workbook/sheet-level issues.

---

### 3. ZIP error mapping folded into XmlParseError

* **Severity**: Minor
* **Category**: Spec Deviation (small)
* **Description**:
  `open_zip` maps some `zip::result::ZipError` variants to `ExcelOpenError::XmlParseError`:

  ```rust
  fn open_zip(path: &Path) -> Result<ZipArchive<File>, ExcelOpenError> {
      let file = File::open(path)?;
      ZipArchive::new(file).map_err(|err| match err {
          ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
              ExcelOpenError::NotZipContainer
          }
          ZipError::Io(e) => ExcelOpenError::Io(e),
          other => ExcelOpenError::XmlParseError(other.to_string()),
      })
  }
  ```



For container-level failures:

* Truly non-ZIP containers (e.g., a `.txt` file) → `NotZipContainer` (covered by `not_zip_container_returns_error`).
* Valid ZIP but not Excel-opc (random_zip.zip, no_content_types.xlsx) → `NotExcelOpenXml` (correct per spec).

However, any “other” `ZipError` for workbook parts (for example, a truncated internal file which still parses as ZIP but has corrupted entries) will be surfaced as `XmlParseError`, even though the failure occurred at the ZIP layer, not XML parsing.

The spec groups all these into an `ExcelOpenError` surface, without prescribing a distinct “ZipPartError”, so the mapping is not *wrong*, but the name `XmlParseError` becomes slightly misleading in these edge cases.

* **Evidence**:

  * `open_zip` error mapping.
  * Decision record describing the desire for a transparent, typed error model for container problems.

* **Impact**:

  * Slight loss of diagnostic precision in rare container-corruption cases beyond the existing fixtures.
  * Not currently observable in tests; mostly an ergonomics concern for future logging/UX around invalid workbooks.

---

### 4. Shared string / inline string edge cases are under-tested

* **Severity**: Minor

* **Category**: Missing Test / Potential Bug

* **Description**:
  The implementation parses shared strings and inline strings:

  * `parse_shared_strings` walks `<sst><si><t>...</t></si>...` and pushes the concatenated text for each `<si>`.
  * `read_inline_string` reads the content of `<is> ... </is>` by concatenating all `Event::Text` occurrences.
  * `convert_value` then interprets `t="s"` as a shared string lookup, `t="b"` as bool, `t="str"`/`t="inlineStr"` as textual, and numeric values as `CellValue::Number(f64)` if the parse succeeds, else `Text`.

  The current tests validate:

  * Numeric and textual values via PG1 fixtures (e.g., `"R{r}C{c}"` patterns, numeric grids).
  * Address text in the PG2 fixture, which uses strings matching A1 addresses.

  There are no targeted tests for:

  * Shared strings with multiple `<t>` fragments or rich text runs.
  * Inline strings that include leading/trailing whitespace or embedded newlines.
  * Cells that use `t="b"` with values other than `"0"`/`"1"` (the code returns `None` for anything else).
  * Error cells (`t="e"`), which are currently treated as `Text` by falling through the default branch.

  The implementation likely behaves reasonably for typical Excel output, but these corners are not codified in tests.

* **Evidence**:

  * `parse_shared_strings`, `read_inline_string`, and `convert_value`.
  * PG1/PG2 tests and fixtures lacking explicit coverage of these value cases.

* **Impact**:

  * If future fixtures or real-world workbooks rely on richer string behaviors (multi-run formatting, non-trivial whitespace), the parser’s behavior may not match expectations and could change silently under refactors.
  * At present this is a latent risk rather than a confirmed bug, because the generator-side patterns are simple.

---

### 5. Large-grid / streaming behavior not yet exercised (expected deferral)

* **Severity**: Minor

* **Category**: Gap (known / planned)

* **Description**:
  The difficulty and testing documents call out streaming and large-grid performance as a major risk area, with dedicated “perf” fixtures such as large dense/noise grids and a Phase 2 streaming guard under a constrained heap.

  The current implementation:

  * Uses `quick_xml::Reader` with `from_reader(&xml[..])`, meaning the entire worksheet XML is held in memory.
  * Builds a fully materialized `Grid` of size `nrows * ncols` up front in `build_grid`.

  This is explicitly allowed by the decision record and mini-spec for this cycle:

  * “Favor a minimal, correct implementation over streaming/perf cleverness.”
  * “It is acceptable if the first version reads small/medium workbooks into memory eagerly.”

  There are currently no tests invoking the large performance fixtures (`grid_large_dense.xlsx`, `grid_large_noise.xlsx`); all integration tests operate on small PG1/PG2 workbooks.

* **Evidence**:

  * Implementation details of `parse_sheet_xml` and `build_grid`.
  * Testing plan and decision record explicitly deferring streaming and perf constraints beyond small fixtures.

* **Impact**:

  * For the current fixtures and target use in this cycle, there is no issue.
  * For large real-world workbooks, this design would allocate `O(rows * cols)` `Cell`s, which may exceed the intended memory budget; this risk is already documented and earmarked for future milestones.
  * This should be revisited when Phase 2/3 streaming and perf milestones are tackled, but does not block this release.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * New library entry point (`core/src/lib.rs`) exports `workbook`, `excel_open_xml`, and `addressing` as specified.
  * IR types `Workbook`, `Sheet`, `SheetKind`, `Grid`, `Row`, `Cell`, `CellValue`, `CellAddress` match the documented shapes (field sets kept minimal but extendable).
  * A1 helpers (`index_to_address`, `address_to_index`, `CellAddress::from_indices` / `to_a1`) are implemented and consistent.
  * Excel Open XML parsing is isolated in `excel_open_xml` with helper functions as requested.

* [x] All specified tests created

  * Container tests: `open_minimal_workbook_succeeds`, `open_nonexistent_file_returns_io_error`, `random_zip_is_not_excel`, `no_content_types_is_not_excel`, plus extra `not_zip_container_returns_error`.
  * PG1 tests: `pg1_basic_two_sheets_structure`, `pg1_sparse_used_range_extents`, `pg1_empty_and_mixed_sheets`.
  * PG2 tests: addressing unit tests in `addressing.rs` and `addressing_pg2_tests.rs` integration test using `pg2_addressing_matrix.xlsx`.

* [x] Behavioral contract satisfied

  * Container error behavior matches mini-spec for all specified fixtures.
  * Workbook and grid IR for PG1 fixtures respect expected sheet counts, names, shapes, and sample contents.
  * Addressing invariants (A1 ↔ (row,col), `CellAddress` consistency) hold in both unit and integration tests.

* [x] No undocumented deviations from spec

  * The only notable divergence (used-range anchoring) is within the “shapes can evolve” allowance and consistent with the tests; it should be documented in future planning but does not contradict the current mini-spec.

* [x] Error handling adequate

  * No `unwrap()`/`panic!` in the parsing path; all fallible operations propagate `ExcelOpenError` variants or `XmlParseError` with context.
  * Negative-path tests confirm typed errors for the primary container failure modes.

* [x] No obvious performance regressions

  * This is the first implementation of these subsystems, so there is no previous baseline to regress from.
  * The design is intentionally naive but acceptable for small/medium fixtures per decision record; perf/streaming concerns are deferred by design.

```

---

Since I’m recommending “Proceed to release,” a formal remediation plan isn’t strictly required, but the most important follow‑ups I’d flag for planners in upcoming cycles are:

* Tighten used-range semantics (and/or introduce an explicit grid origin) and add fixtures where the first used cell is *not* A1.
* Add targeted tests for `WorkbookXmlMissing` and `WorksheetXmlMissing`.
* Introduce value-focused tests for shared/inline strings and error cells.
* Plan a dedicated perf/streaming cycle once more of the diff pipeline is in place.
