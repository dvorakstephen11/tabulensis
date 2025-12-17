I compared the **current** `codebase_context.md` snapshot against Branch 7 as described in `spec.md` (parser + AST, canonicalization, shift-equivalence, semantic formula diff classification + integration, and the safety/perf guardrails). The Branch 7 work appears **implemented end-to-end**, and the Branch 7 test suite is green in `cycle_summary.txt`.

Below is the concrete checklist-style verification, mapped to the code and tests you provided.

---

## 1) Branch 7.1 Formula AST + parser

### What the spec requires

`spec.md` calls for a lightweight Excel formula parser + AST that can handle common syntax (A1, R1C1, arrays, structured refs/table refs, sheet/file prefixes, etc.).

### What your codebase has

* A full formula AST exists (e.g., `FormulaExpr`, `CellReference`, `RangeReference`, row/col ref forms, unary/binary operators, Excel error literals, etc.).
* Parser coverage includes the “varied syntaxes” explicitly called out in Branch 7 (examples from the test suite include):

  * whitespace/case in functions: `=sum( A1 , B1 )`
  * quoted sheet names + absolute refs: `\'My Sheet\'!$A$1`
  * workbook prefixes: `[Book1.xlsx]Sheet1!A1`
  * array constants: `{1,2;3,4}`
  * structured refs (captured/treated as name-like refs): `Table1[Column1]`
  * R1C1 and offset R1C1: `R1C1`, `R[2]C[-3]` 
* The parser test run is passing (`f7_formula_parser_tests`). 

✅ Result: **Branch 7.1 implemented.**

---

## 2) Branch 7.2 Canonicalization

### What the spec requires

Canonicalization should normalize “formatting-only” differences, including:

* case normalization (function names, named refs)
* commutative argument ordering for commutative functions
* commutative reordering for commutative binary ops
* stable range endpoint ordering

### What your codebase has

* Canonicalization logic implements:

  * `SUM/AND/OR/MAX/MIN` argument sorting
  * `+` and `*` operand ordering
  * uppercasing of function names and `NamedRef`
  * range endpoint normalization 
* Canonicalization tests exist and pass (`f7_formula_canonicalization_tests`), including:

  * commutative function sorting
  * commutative binary op normalization
  * range endpoint canonicalization
  * structured refs parse + canonicalize 

✅ Result: **Branch 7.2 implemented.**

---

## 3) Branch 7.3 “Filled down” / shift-equivalence detection

### What the spec requires

A function that detects whether two formulas are semantically the same modulo a row/column shift (for filled/copied formulas), shifting **relative** references while leaving absolute refs unchanged.

### What your codebase has

* A shift engine that rewrites references under a configurable shift mode (relative-only vs all), including shifting inside ranges, function calls, unary/binary ops, and arrays.
* `formulas_equivalent_modulo_shift(...)` is implemented exactly in the intended style: shift one side in `RelativeOnly` mode, canonicalize both, then compare. 
* Shift tests exist and pass (`f7_formula_shift_tests`) for:

  * filled-down formulas matching under row shift
  * mismatched refs not matching under zero shift

✅ Result: **Branch 7.3 implemented.**

---

## 4) Branch 7.4 Semantic formula diff integration

### What the spec requires

* Add `FormulaDiffResult` classification.
* Thread it into `DiffOp::CellEdited` as a backward-compatible field (`#[serde(default)]`).
* Implement formula diff classification:

  * `Unchanged / Added / Removed`
  * `FormattingOnly` when canonical forms match
  * `Filled` when shift-equivalent under row/col shift
  * `SemanticChange` when both parse but not equivalent
  * `TextChange` when parsing fails or semantic diff disabled

### What your codebase has

**4.1 FormulaDiffResult + DiffOp schema update**

* `FormulaDiffResult` exists with the expected variants and default behavior. 
* `DiffOp::CellEdited` includes `formula_diff` with `#[serde(default)]`, preserving backward compatibility for older JSON that lacks the field. 
* The `DiffOp::cell_edited(...)` helper takes `formula_diff` explicitly (forcing call sites to consciously classify). 

**4.2 Diff logic with caching + feature flag**

* `core/src/formula_diff.rs` provides:

  * `FormulaParseCache` caching parsed and canonical ASTs by `StringId`
  * `diff_cell_formulas_ids(...)` that:

    * returns early for `old == new`
    * returns `TextChange` immediately when semantic diff is disabled (so no parsing work is done in that mode)
    * otherwise parses (via cache), checks canonical equality (`FormattingOnly`), checks shift-equivalence (`Filled`), else `SemanticChange`
    * returns `TextChange` when parsing fails 

**4.3 Engine integration with row/col shift propagation**

* The engine wires a `formula_cache` into diff context and computes `formula_diff` at `CellEdited` emission time using `(row_shift, col_shift)` derived from alignment/mapping.

✅ Result: **Branch 7.4 implemented.**

---

## 5) Performance & safety checks from spec.md

The “guardrails” in `spec.md` are present:

* **Caching**: parsing (and canonicalization) is cached by `StringId` via `FormulaParseCache`.
* **No parsing when disabled**: semantic diff returns `TextChange` before any parse attempts when `enable_formula_semantic_diff` is false.
* **Schema backward compatibility**: `formula_diff` is `#[serde(default)]`.

✅ Result: **Spec.md safety/perf requirements are met.**

---

## 6) Tests & JSON contract updates

* Branch 7 focused tests all pass: parser, canonicalization, shift tests, and the integration tests that check:

  * formatting-only vs text-change respects the feature flag
  * filled-down detection when a row shift is involved 
* JSON shape tests for `CellEdited` were updated to include `formula_diff` (so the serialized contract is now asserted). 

✅ Result: **Branch 7 tests + JSON contract checks are in place and green.**

---

# Conclusion

Based on the current `codebase_context.md` and the Branch 7 definition-of-done in `spec.md`, the codebase **does fully implement Branch 7**, and I do **not** see any missing or incorrect requirements that still need implementation.

---

## Optional follow-ups (not required for “Branch 7 complete”)

These are not omissions relative to `spec.md`’s done criteria, but they may be worth considering:

1. **Canonicalization sort-key performance**
   Your canonical argument ordering uses a stable debug-string sort key (correct, but allocates). If you start seeing perf regressions on very large / highly repetitive formulas, swapping to a hash-based key would reduce allocations. This is explicitly called out as an optional upgrade in the spec narrative. 

2. **Broader commutative function coverage**
   If you care about treating additional commutative functions (e.g., `PRODUCT`) as formatting-only, expand the commutative set. Right now it’s scoped to the core set.

If you want, I can outline an exact patch plan for either of those optimizations (with replace-vs-new code blocks in the format you prefer), but they’re not required to claim Branch 7 is implemented.
