
# Remediation Plan: 2025-11-29-refactor (Minor Follow-ups)

## Overview

The current implementation is safe to release and fulfills the mini-spec. The following items are **non-blocking** quality improvements aimed at tightening the public API and test coverage before more advanced diff algorithms and host scenarios are layered on top.

## Fixes Required

### Fix 1: Align module exports and feature-gating with the spec

* **Addresses Finding**: Finding 1 – Feature-gating mismatch

* **Changes**:

  * **Files**: `core/src/lib.rs`

  * **Proposed adjustments** (one of these strategies; spec and code should agree):

    1. **Preferred (align code to spec)**

       * Remove `#[cfg(feature = "excel-open-xml")]` from:

         * `pub mod container;`
         * `pub mod datamashup_framing;`
         * `pub mod grid_parser;`
         * And their associated `pub use` lines.
       * Keep `excel_open_xml` and `diff_workbooks_to_json` gated, since they are explicitly Excel-specific.

    2. **Alternate (align spec to existing behavior)**

       * If you decide these modules should only ever exist when Excel support is compiled in, update the mini-spec §5.3 to show the `#[cfg(feature = "excel-open-xml")]` attributes and note this in the “Module structure” section as an intentional design choice.

  * Option 1 is a code change; Option 2 is a documentation-only change. Choose based on your intended feature model for PBIX/PBIT and other hosts.

* **Tests**:

  * If you adopt Option 1, consider adding a CI check that runs `cargo check --no-default-features` to ensure the crate compiles and that the public API (including `OpcContainer`, `DataMashupError`, and `GridParseError`) is available without Excel-specific features.

---

### Fix 2: Add tests for `Grid::rows_iter` and `Grid::cols_iter`

* **Addresses Finding**: Finding 2 – Untested iterator helpers

* **Changes**:

  * **Files**:

    * `core/src/workbook.rs` (verify the implementations themselves)
    * `core/tests/sparse_grid_tests.rs` (add tests)

  * **Implementation details**:

    * Confirm the iterator bodies use correct ranges:

      ```rust
      pub fn rows_iter(&self) -> impl Iterator<Item = u32> + '_ {
          0..self.nrows
      }

      pub fn cols_iter(&self) -> impl Iterator<Item = u32> + '_ {
          0..self.ncols
      }
      ```

    * Add tests such as:

      ```rust
      #[test]
      fn rows_iter_covers_all_rows() {
          let grid = Grid::new(3, 5);
          let rows: Vec<u32> = grid.rows_iter().collect();
          assert_eq!(rows, vec![0, 1, 2]);
      }

      #[test]
      fn cols_iter_covers_all_cols() {
          let grid = Grid::new(4, 2);
          let cols: Vec<u32> = grid.cols_iter().collect();
          assert_eq!(cols, vec![0, 1]);
      }
      ```

    * Optionally, add a test that combines iterators with actual cells to ensure they work correctly in typical usage:

      ```rust
      #[test]
      fn rows_iter_and_get_are_consistent() {
          let mut grid = Grid::new(2, 2);
          grid.insert(Cell { row: 1, col: 1, address: CellAddress::from_indices(1, 1),
                             value: Some(CellValue::Number(1.0)), formula: None });

          for r in grid.rows_iter() {
              for c in grid.cols_iter() {
                  let _ = grid.get(r, c); // should not panic; we only expect Some at (1,1)
              }
          }
      }
      ```

* **Tests**:

  * Place the tests in `core/tests/sparse_grid_tests.rs` alongside the existing grid behavior tests to keep the API coverage localized.

---

### Fix 3: Extend JSON non-finite guard tests to cover infinities

* **Addresses Finding**: Finding 3 – Non-finite serialization tests incomplete

* **Changes**:

  * **Files**:

    * `core/tests/output_tests.rs`
    * (No changes needed in `core/src/output/json.rs` unless you want to tweak the error message.)

  * **Implementation details**:

    * Add tests that mirror the NaN test but use `f64::INFINITY` and `f64::NEG_INFINITY`:

      ```rust
      #[test]
      fn serialize_diff_report_infinity_maps_to_serialization_error() {
          let addr = CellAddress::from_indices(0, 0);
          let report = DiffReport::new(vec![DiffOp::cell_edited(
              "Sheet1".into(),
              addr,
              make_cell_snapshot(addr, Some(CellValue::Number(f64::INFINITY))),
              make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
          )]);

          let err = serialize_diff_report(&report).expect_err("Infinity should fail to serialize");
          let wrapped = ExcelOpenError::SerializationError(err.to_string());
          match wrapped {
              ExcelOpenError::SerializationError(msg) => {
                  assert!(
                      msg.to_lowercase().contains("infinity"),
                      "error message should mention infinity for clarity"
                  );
              }
              other => panic!("expected SerializationError, got {other:?}"),
          }
      }

      #[test]
      fn serialize_diff_report_neg_infinity_maps_to_serialization_error() {
          // identical pattern but with f64::NEG_INFINITY
      }
      ```

    * Optionally, add a single test that constructs a `DiffReport` with valid finite numbers to show that `serialize_diff_report` *does* succeed when all values are finite, complementing the failure tests.

* **Tests**:

  * Run the full test suite to ensure no change to behavior other than the strengthened coverage:

    * `cargo test -p core` (or the equivalent command used in the cycle summary).

---

## Constraints

* These are all **non-breaking, additive** changes:

  * No public types need to change shape.
  * No JSON schema changes are implied.
  * No modifications are required to existing fixtures or integration tests.

* The feature-gating adjustment (Fix 1) should be done with awareness of how other crates in the monorepo enable `excel_diff` features, but it can also be deferred if you decide to instead update the spec to match current behavior.

---

## Expected Outcome

After these follow-ups:

1. The public API surface (especially module exports and feature flags) will clearly match the architecture described in the mini-spec and decision record.
2. The `Grid` iteration helpers will be validated and safe to use in future, more advanced diff algorithms.
3. The JSON serialization guard for non-finite numbers will have comprehensive coverage (NaN and both infinities), reducing regression risk around the `SerializationError` path.

None of these are release blockers; they are incremental hardening steps that can be scheduled opportunistically in a later cycle.
