```markdown
# Verification Report: 2025-12-04b-g1-g2-grid-diff

## Summary

This branch wires two new tiny Excel fixtures (`equal_sheet_{a,b}.xlsx`, `single_cell_value_{a,b}.xlsx`) into the Python fixture generator, adds them to the manifest, and introduces a new Rust integration test module `core/tests/g1_g2_grid_workbook_tests.rs` that exercises `diff_workbooks` via `open_workbook` for the G1 (identical 5×5 sheet) and G2 (single literal change at `Sheet1!C3`: 1.0 → 2.0) scenarios. The implementation matches the mini-spec’s scope: the manifest entries and generators line up with the plan, the tests assert the intended DiffOp behavior, and the core diff engine and IR types are unchanged. All existing tests (PG1–PG6, PG4, engine, datamashup, Excel-open-XML) plus the two new G1/G2 tests pass. I did not find any correctness issues or coverage gaps serious enough to block release; the remaining items are minor test/implementation polish opportunities and one small doc/parameter drift in the Python generator.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### Finding 1: G2 test asserts “no formula change” but not explicitly “no formula at all”

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The behavioral contract section for G2 describes a scenario where the only logical difference between the two workbooks is a single **literal value** change at `Sheet1!C3` and explicitly states that formulas should be identical and (for this fixture) typically `None`. The concrete example uses assertions that both `from.formula` and `to.formula` are `None`. :contentReference[oaicite:0]{index=0}  

  The actual G2 test in `core/tests/g1_g2_grid_workbook_tests.rs` asserts that the `CellEdited` op has the right sheet, address, and values, and then asserts only that `from.formula == to.formula`, not that they are both `None`. :contentReference[oaicite:1]{index=1}  

  Existing PG3 snapshot tests already verify that value-only cells parsed from Excel yield snapshots with `formula == None`. :contentReference[oaicite:2]{index=2} Combined with the way the `SingleCellDiffGenerator` writes the fixture (pure values, no formulas), this strongly implies formulas are in fact `None` here, but the property is not re-asserted in the new integration test.
- **Evidence**:  
  - G2 behavioral contract & example assertions in the cycle plan. :contentReference[oaicite:3]{index=3}  
  - Implementation of `g2_single_cell_literal_change_produces_one_celledited` test. :contentReference[oaicite:4]{index=4}  
  - PG3 snapshot tests confirming formula handling from Excel (`pg3_value_and_formula_cells.xlsx`). :contentReference[oaicite:5]{index=5}  
- **Impact**:  
  If a future regression caused `open_workbook` (or `CellSnapshot::from_cell`) to start populating formulas on what should be value-only cells for this fixture, the new G2 test would still pass as long as both sides shared the same (incorrect) formula string. That would be a subtle IR drift not directly caught by G1/G2. However, given the strong coverage in PG3 and the very simple fixture generator, this is a low-risk gap.

---

### Finding 2: IR shape for new fixtures (5×5 grid) is not asserted directly

- **Severity**: Minor  
- **Category**: Missing Test / Gap  
- **Description**:  
  The mini-spec’s IR section states that for these new fixtures the workbook parser should produce a single `Sheet { name: "Sheet1", kind: Worksheet, grid: Grid { nrows = 5, ncols = 5, ... } }`. :contentReference[oaicite:6]{index=6}  

  The new G1/G2 tests use `open_workbook` and `diff_workbooks` and assert on the resulting `DiffReport.ops` but never inspect `workbook.sheets[0].grid.nrows` / `ncols` for these specific Excel files. :contentReference[oaicite:7]{index=7}  

  There *are* existing PG1/PG3 tests that assert grid shapes and cell properties for other fixtures (e.g., PG1 basic grids, PG3 “Types” sheet), proving that the Excel→IR path produces correct `Grid` dimensions and cell addresses in those scenarios. :contentReference[oaicite:8]{index=8} But for the new G1/G2 fixtures the shape expectation (5×5) is only implied via how the generator is configured, not re-pinned at the Rust IR level.
- **Evidence**:  
  - IR expectations for G1/G2 in the cycle plan. :contentReference[oaicite:9]{index=9}  
  - G1/G2 test module, which asserts only on diff behavior, not grid shape. :contentReference[oaicite:10]{index=10}  
  - Existing PG1/PG3 tests that already exercise grid dimensions and snapshot contents for other fixtures. :contentReference[oaicite:11]{index=11}  
- **Impact**:  
  If a future change caused the grid parser to miscompute dimensions for these fixtures (e.g., expand to 6×6 while leaving the populated region intact), `diff_workbooks` would still produce the same empty/single diff for G1/G2 as long as both sides shared the same mistaken shape, and the current tests would pass. That would violate the more precise IR expectation in the mini-spec but not the observable diff behavior. Because other fixtures already exercise grid sizing and these fixtures are small and simple, this is a documentation/coverage nicety rather than a blocker.

---

### Finding 3: `BasicGridGenerator` ignores the `sheet` argument used in the new G1 manifest

- **Severity**: Minor  
- **Category**: Spec Deviation / Gap  
- **Description**:  
  The mini-spec’s fixture manifest sketch for G1 uses a `sheet: "Sheet1"` argument for the `basic_grid` generator. :contentReference[oaicite:12]{index=12} The actual updated `fixtures/manifest.yaml` follows that sketch for `g1_equal_sheet`. :contentReference[oaicite:13]{index=13}  

  However, the `BasicGridGenerator` implementation always titles the sheet `"Sheet1"` and completely ignores any `sheet` argument in `args`. :contentReference[oaicite:14]{index=14} For G1 this is harmless because the argument and the hard-coded default match (`"Sheet1"`), and the mini-spec only requires these fixtures to have that name. The G2 scenario uses `SingleCellDiffGenerator`, which *does* respect its `sheet` argument and is used consistently. :contentReference[oaicite:15]{index=15}  
- **Evidence**:  
  - Spec/plan snippet showing `sheet: "Sheet1"` in the G1 manifest example. :contentReference[oaicite:16]{index=16}  
  - Manifest entry for `g1_equal_sheet` using `sheet: "Sheet1"`. :contentReference[oaicite:17]{index=17}  
  - `BasicGridGenerator` using `ws.title = "Sheet1"` unconditionally. :contentReference[oaicite:18]{index=18}  
  - `SingleCellDiffGenerator` respecting `sheet` in its args. :contentReference[oaicite:19]{index=19}  
- **Impact**:  
  For this branch’s fixtures there is no functional problem: both the hard-coded title and the manifest arg are `"Sheet1"`, and the new G1/G2 tests assert on that name. :contentReference[oaicite:20]{index=20} However, the manifest and spec give the impression that `sheet` is a meaningful parameter for `basic_grid`. If someone later tries to reuse `basic_grid` with a different sheet name, they may be surprised. This is a minor doc/implementation drift worth cleaning up eventually but not a blocker for the current release.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - New manifest scenarios `g1_equal_sheet` and `g2_single_cell_value` are present with the expected generators and arguments. :contentReference[oaicite:21]{index=21} :contentReference[oaicite:22]{index=22}  
  - Four generated fixture filenames (including `single_cell_value_{a,b}.xlsx`) are listed under `fixtures/generated`. :contentReference[oaicite:23]{index=23}  
  - New test module `core/tests/g1_g2_grid_workbook_tests.rs` exists and exercises `open_workbook` + `diff_workbooks`. :contentReference[oaicite:24]{index=24}  
  - Core engine APIs (`diff_workbooks`, `diff_grids`, reexports in `lib.rs`) match the surfaces described in the plan and remain unchanged in structure. :contentReference[oaicite:25]{index=25} :contentReference[oaicite:26]{index=26}  

- [x] All specified tests created  
  - The two Rust tests called out in Section 5.2 of the mini-spec (G1 empty diff, G2 single `CellEdited`) are implemented with the intended assertions, plus an extra negative check that G2 does not emit row/column structure ops. :contentReference[oaicite:27]{index=27} :contentReference[oaicite:28]{index=28}  

- [x] Behavioral contract satisfied  
  - **G1**: `diff_workbooks(equal_sheet_a, equal_sheet_b)` yields an empty `DiffReport.ops`, consistent with both PG5 in‑memory tests and the new Excel-backed G1 test. :contentReference[oaicite:29]{index=29} :contentReference[oaicite:30]{index=30}  
  - **G2**: `diff_workbooks(single_cell_value_a, single_cell_value_b)` yields exactly one `CellEdited` op at `"Sheet1!C3"` with `from.value = 1.0` and `to.value = 2.0`, and no row/column structure ops, matching the contract and validating that Excel parsing + grid diff integrate correctly. :contentReference[oaicite:31]{index=31} :contentReference[oaicite:32]{index=32}  

- [x] No undocumented deviations from spec  
  - Core engine behavior (`diff_workbooks`, `diff_grids`), DiffOp invariants (`DiffOp::CellEdited` helper, snapshot semantics), and JSON shapes are unchanged and still covered by PG4, PG5, and engine tests. :contentReference[oaicite:33]{index=33} :contentReference[oaicite:34]{index=34} :contentReference[oaicite:35]{index=35}  
  - The only discrepancies are minor: the G2 test checks formula equality instead of explicitly asserting `None`, and `BasicGridGenerator` ignores the `sheet` arg. Both are behaviorally benign for this branch and documented in the findings above.

- [x] Error handling adequate  
  - No new error-handling code paths were introduced; all changes are in tests and fixture manifest wiring. Existing error-handling tests for Excel containers, datamashup framing, and Excel-open-XML remain green. :contentReference[oaicite:36]{index=36}  

- [x] No obvious performance regressions  
  - No changes were made to the diff algorithm’s complexity or memory behavior. G1/G2 fixtures are tiny 5×5 workbooks, and the tests exercise only single diff calls. The plan explicitly notes that fixture sizes are small and no perf work is in scope for this branch. :contentReference[oaicite:37]{index=37}  

---

### Prior Remediation History

The `combined_remediations.md` file contains only an empty placeholder remediation document and no concrete outstanding action items relevant to this branch. :contentReference[oaicite:38]{index=38} There are therefore no prior remediation points that needed to be re‑verified here.

```
