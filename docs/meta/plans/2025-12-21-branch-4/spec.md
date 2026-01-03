## What Branch 4 requires

Branch 4 is about finishing the “object graph” so the diff isn’t only grid cells + Power Query, but also:

* **VBA project modules** (from `xl/vbaProject.bin`) → add/remove/change detection. 
* **Named ranges** (from `<definedNames>` in `xl/workbook.xml`) → add/remove/change detection. 
* **Chart objects** (basic) (from `xl/charts/*.xml`, but sheet-level add/remove requires following drawing relationships) → add/remove/change detection, with a hash of chart XML for “changed”. 
* New `DiffOp` variants for all of the above, and tests proving JSON serialization + no regressions.

## Where the codebase is today (relevant baseline)

* **Workbook IR** currently only contains `sheets: Vec<Sheet>`. 
* **Package IR** is `WorkbookPackage { workbook, data_mashup }`. 
* The OpenXML loader builds a `Workbook` from `xl/workbook.xml`, `xl/sharedStrings.xml`, and `xl/worksheets/*.xml`, returning `Workbook { sheets: ... }`. 
* Workbook relationships parsing for sheet targets is currently worksheet-focused (filters relationship types containing `"worksheet"`). 
* Package diff appends **M diffs** after grid diffs (`report.ops.extend(m_ops)`), and streaming emits M ops after the grid diff finishes.
* CLI/text output partitions ops into “by sheet” + “Power Query” and silently drops non-sheet, non-M ops today. That will hide named ranges and VBA unless updated. 

---

# Detailed implementation plan

## Phase 1 — Data model + `DiffOp` surface area

### 1. Add the new IR types

**1.1 Named ranges in `Workbook` (required by Branch 4)**
Branch 4 explicitly requires adding `NamedRange` to `Workbook`. 

Implementation steps:

1. In `core/src/workbook.rs`, add:

   * `pub struct NamedRange { pub name: StringId, pub refers_to: StringId, pub scope: Option<StringId> }` (as in the plan). 
2. Extend `pub struct Workbook` to include:

   * `pub named_ranges: Vec<NamedRange>` (default empty).
   * (Strongly recommended) `pub charts: Vec<ChartObject>` for chart diffing storage (see below).

Why: parsing happens during open; you need to keep these objects around to diff later (you don’t keep the ZIP container around in memory). Right now `open_workbook_from_container` discards everything but `sheets`. 

**Churn control:**
A field addition will break a lot of test helpers and struct literals that currently do `Workbook { sheets: ... }`. 
To keep the refactor mechanical:

* Derive or implement `Default` for `Workbook`, and update constructions to `Workbook { sheets: ..., ..Default::default() }`.
* Update the `single_sheet_workbook` helper and any common builders first, then let most tests stay unchanged.

**1.2 VBA modules in `WorkbookPackage` (required by Branch 4)**
Branch 4 requires `WorkbookPackage` include VBA modules. 
`WorkbookPackage` is currently only `{ workbook, data_mashup }`. 

Implementation steps:

1. Add the types:

   * `VbaModule { name: StringId, module_type: VbaModuleType, code: String }`
   * `VbaModuleType { Standard, Class, Form, Document }` 
2. Add a new field to `WorkbookPackage`:

   * `pub vba_modules: Option<Vec<VbaModule>>` (use `Option` so `.xlsx` stays `None`).
3. Update `From<Workbook> for WorkbookPackage` to set `vba_modules: None`. 

**Important design choice:** keep `code: String` (not `StringId`) exactly like the plan, so you don’t explode the global `StringPool` / JSON string tables with full module text. 

**1.3 Chart object IR (needed to diff charts)**
Branch 4 provides `ChartInfo` and requires chart add/remove/change detection. 
You’ll need some container to store parsed charts. The cleanest is to attach to `Workbook`:

* `ChartInfo { name: StringId, chart_type: StringId, data_range: Option<StringId> }` 
* `ChartObject { sheet: StringId, info: ChartInfo, xml_hash: u128 }` (you’ll need `sheet` + `hash` to implement the required diff ops).

### 2. Add new `DiffOp` variants

In `core/src/diff.rs`, extend `DiffOp` with the exact variants in Branch 4:

* `VbaModuleAdded { name: StringId }`
* `VbaModuleRemoved { name: StringId }`
* `VbaModuleChanged { name: StringId }`
* `NamedRangeAdded { name: StringId }`
* `NamedRangeRemoved { name: StringId }`
* `NamedRangeChanged { name: StringId, old_ref: StringId, new_ref: StringId }`
* `ChartAdded { sheet: StringId, name: StringId }`
* `ChartRemoved { sheet: StringId, name: StringId }`
* `ChartChanged { sheet: StringId, name: StringId }`

Notes:

* `DiffOp` is `#[serde(tag="kind")]` today, so these new variants automatically serialize as new `"kind"` values in JSON. 
* After adding variants, any *exhaustive* matches in outputs/tests will need updates (text output rendering, JSON-lines tests, etc.). 

---

## Phase 2 — Parsing: populate NamedRanges, Charts, VBA modules at open time

### 3. Named range parsing from `xl/workbook.xml`

You already read `xl/workbook.xml` bytes in `open_workbook_from_container`. 
Today you only parse sheets from it via `parse_workbook_xml`. 

Implementation steps:

1. Add a new parser function (recommended location: `core/src/grid_parser.rs` next to `parse_workbook_xml`):

   * `parse_defined_names(workbook_xml: &[u8], sheets_in_order: &[SheetDescriptor], pool: &mut StringPool) -> Result<Vec<NamedRange>, GridParseError>`

2. Extract each `<definedName>`:

   * `name` attribute
   * optional `localSheetId` attribute (sheet scope)
   * inner text (the “refers_to” string)

3. Map `localSheetId` → sheet name:

   * `localSheetId="0"` corresponds to the **0th `<sheet>` element order** in `workbook.xml`, which should match the order you already return from `parse_workbook_xml`.

4. **Identity / ambiguity handling (important):**

   * Excel allows the same name on different sheets (sheet-scoped names). Your diff ops only carry a single `name: StringId`. 
   * To avoid ambiguous diffs, store a *qualified* name in `NamedRange.name`, e.g.:

     * workbook-scoped: `"SalesData"`
     * sheet-scoped: `"Sheet1!SalesData"` (or with proper quoting if you want)
   * Keep `scope: Some(sheet_name_id)` separately for programmatic access.

5. In `open_workbook_from_container`, call this function and set `Workbook.named_ranges = ...` before returning. 

### 4. Chart object extraction (sheet-level) + chart metadata parsing

Branch 4 wants chart metadata from `xl/charts/*.xml` and sheet-level add/remove/change. 
But chart parts don’t inherently know which sheet they’re on; the sheet association comes from drawings and relationships. The minimum viable traversal is:

**Worksheet** → `<drawing r:id="rIdX">`
**Worksheet rels** (`xl/worksheets/_rels/sheetN.xml.rels`) → target drawing part
**Drawing** (`xl/drawings/drawingM.xml`) → chart references `<c:chart r:id="rIdY">` and object names in `xdr:cNvPr name="Chart 1"`
**Drawing rels** (`xl/drawings/_rels/drawingM.xml.rels`) → target chart part `xl/charts/chartK.xml`
**Chart part** → hash + basic metadata (type + first data range reference)

Implementation steps:

1. Add a **generic relationships parser**:

   * Current workbook rels parsing filters to relationship types containing `"worksheet"`. 
   * For charts you need the raw `Id -> Target` mapping without filtering.
   * Implement `parse_relationships_all(rels_xml: &[u8]) -> Result<HashMap<String, String>, GridParseError>` that reads `<Relationship Id="..." Target="..." Type="...">`.

2. Add a helper to compute “rels part path” for any part:

   * For `xl/worksheets/sheet1.xml` → `xl/worksheets/_rels/sheet1.xml.rels`
   * For `xl/drawings/drawing1.xml` → `xl/drawings/_rels/drawing1.xml.rels`

3. Add a helper to resolve a `Target` against the base part directory:

   * Handle `../drawings/drawing1.xml` properly.
   * Normalize `.` and `..` path segments.

4. Parse each worksheet XML you already read:

   * Find `<drawing>` elements and their `r:id` attribute.
   * If none, the sheet has no embedded drawings/charts → skip.

5. Follow the relationship chain and build `ChartObject` entries:

   * Determine `ChartInfo.name`:

     * Prefer `xdr:cNvPr@name` (object name) from drawing XML
     * Fallback: chart part file name if necessary
   * Determine `chart_type`:

     * Find first `<c:*Chart>` element under plot area (e.g. `barChart`, `lineChart`) and normalize.
   * Determine `data_range`:

     * Grab the first `<c:f>` formula under a series reference (`numRef` / `strRef`) if present.
   * Compute `xml_hash`:

     * Use the same hashing strategy the codebase already uses for other content hashing (xxh3). 

6. Store:

   * Add the resulting objects to `Workbook.charts` (or to `WorkbookPackage` if you choose that layout instead).

**Performance considerations:**

* Cache chart part parses keyed by full part path so multiple references don’t re-read or re-hash the same file.
* Treat missing optional parts defensively:

  * If a sheet has `<drawing r:id="...">` but the rels file is missing, you can either:

    * return an error (strict, likely correct for “corrupt workbook”), or
    * treat as “no charts on this sheet” (best-effort).
  * For Branch 4 acceptance, the important thing is that normal, valid files work.

### 5. VBA module extraction from `xl/vbaProject.bin`

This is the hardest piece: `vbaProject.bin` is an **OLE Compound File** containing streams, and VBA code is **compressed** (MS-OVBA).

Implementation steps:

1. Add optional dependencies gated behind `excel-open-xml` (so `--no-default-features` wasm builds don’t pull in a compound-file parser):

   * A compound-file reader crate (e.g., `cfb`-style).
   * Optional `encoding_rs` for codepage decoding.

2. Implement `open_vba_modules_from_container` (in `core/src/excel_open_xml.rs` or a new `core/src/vba.rs` behind the feature):

   * Read `xl/vbaProject.bin` via the container’s checked read (same safety pattern you already use). 
   * If missing: return `Ok(None)`.

3. Parse the compound file:

   * Locate the `VBA/dir` stream, decompress it (MS-OVBA compression), then parse module metadata records from it to enumerate:

     * module name
     * module stream name
     * code offset
     * module type flags (class/form/document) if present

4. For each module:

   * Read the module stream bytes
   * Skip to “compressed source” using the code offset
   * Decompress the source into bytes
   * Decode into a Rust `String` (codepage-aware if you implement it; otherwise a safe lossy conversion)
   * Intern module name into the `StringPool`
   * Populate `VbaModule { name, module_type, code }`

5. Wire into `WorkbookPackage::open`:

   * After parsing workbook + data mashup, call `open_vba_modules_from_container` and store in `WorkbookPackage.vba_modules`. 

---

## Phase 3 — Diffing: produce new ops and integrate into all diff entry points

### 6. Implement the three new diff passes

Add three pure functions (no IO) that operate on the already-parsed IR.

**6.1 `diff_named_ranges(old: &Workbook, new: &Workbook, pool: &StringPool) -> Vec<DiffOp>`**

* Build sorted maps by a case-insensitive key.
* Emit:

  * `NamedRangeAdded`
  * `NamedRangeRemoved`
  * `NamedRangeChanged { old_ref, new_ref }` when `refers_to` differs. 

**6.2 `diff_charts(old: &Workbook, new: &Workbook, pool: &StringPool) -> Vec<DiffOp>`**

* Key: `(sheet, chart name)` case-insensitive.
* Emit:

  * `ChartAdded`
  * `ChartRemoved`
  * `ChartChanged` when `xml_hash` differs. 

**6.3 `diff_vba_modules(old: Option<&[VbaModule]>, new: Option<&[VbaModule]>, pool: &StringPool) -> Vec<DiffOp>`**

* Key modules by name (case-insensitive).
* Emit:

  * `VbaModuleAdded`
  * `VbaModuleRemoved`
  * `VbaModuleChanged` when normalized code differs (at minimum normalize CRLF to LF so Windows newline changes don’t create false positives). 

### 7. Integrate into `WorkbookPackage::diff_with_pool`

Right now package diff is: grid diff → append M ops → set strings. 

Update plan:

1. Run grid diff (unchanged).
2. Compute object ops:

   * named ranges from `self.workbook.named_ranges`
   * charts from `self.workbook.charts`
   * vba modules from `self.vba_modules`
3. Append them to `report.ops` **before** M ops (recommended order: grid → workbook objects → Power Query).
4. Append M ops (existing behavior).
5. Set `report.strings = pool.strings().to_vec()` (unchanged). 

### 8. Integrate into `WorkbookPackage::diff_streaming_with_pool`

Today streaming does: compute `m_ops`, run streaming grid diff, then emit `m_ops` and increments `summary.op_count` for them. 

Update plan:

1. Compute `object_ops` (named ranges + charts + vba) and `m_ops`.
2. Run streaming grid diff as now.
3. Emit `object_ops` (increment `summary.op_count` for each).
4. Emit `m_ops` (increment `summary.op_count` as today). 

Do the same for database mode streaming if you want object diffs to be present there too (recommended for completeness).

### 9. Update the path-based JSON helpers (`core/src/output/json.rs`)

Those helpers currently:

* open workbooks + data mashup
* run grid diff streaming
* append M ops
* return the report. 

Branch 4 “in its entirety” should make these helpers include the new object diffs too, otherwise users calling these helpers will miss them.

Update plan:

1. Open workbooks as today (once named ranges/charts are included in the workbook IR).
2. Add an `open_vba_modules(path, pool)` helper (feature-gated) that reads `xl/vbaProject.bin` and returns `Option<Vec<VbaModule>>`.
3. After grid diff but before `m_ops`, extend report with:

   * `diff_named_ranges(&wb_a, &wb_b)`
   * `diff_charts(&wb_a, &wb_b)`
   * `diff_vba_modules(&vba_a, &vba_b)`
4. Then append M ops as today. 

---

## Phase 4 — Make the CLI and text outputs actually show these new diffs

Right now, the output layer partitions ops into:

* “sheet ops” (ops that have a `sheet`)
* “Power Query” (ops where `op.is_m_op()`)
  …and drops everything else. 

If you don’t change this, **NamedRange*** and **VbaModule*** ops will never be printed in text/git-diff output (even though they exist in JSON). That undermines “reported”.

Implementation steps:

1. Update `get_sheet_id` to recognize chart ops:

   * `ChartAdded/Removed/Changed` should return `Some(sheet)`.
2. Update `partition_ops` to also collect “workbook-level ops”:

   * ops that are not M ops and have no sheet id (named ranges + vba).
3. Add a new rendered section, e.g.:

   * `@@ Workbook @@`

     * print named range ops + vba ops
   * then sheet sections
   * then `@@ Power Query @@` (existing). 
4. Add formatting in `write_op_diff_lines` / `render_op`:

   * Named range changed should include old/new `refers_to` in verbose mode.
   * VBA module changed prints module name.
   * Chart ops print chart name.

---

# Tests + fixtures plan (ensures acceptance criteria)

## 1) Update existing tests for `Workbook` field changes

Because `Workbook` currently only has `sheets`, adding `named_ranges` will break compilation for many tests and helpers. 
Mechanically update:

* Common helper constructors to include defaults
* Any direct `Workbook { sheets: ... }` uses to `..Default::default()`

## 2) Add fixture generation entries

### Named range fixtures (`.xlsx`)

Add two fixtures:

* `named_ranges_a.xlsx` (baseline)
* `named_ranges_b.xlsx` (added/removed/changed)

Generate with openpyxl:

* workbook-scoped name add/remove
* sheet-scoped name change (so you validate scope handling too)

### Chart fixtures (`.xlsx`)

Add two fixtures:

* `charts_a.xlsx` (one chart)
* `charts_b.xlsx` (change that chart + add another chart)

Generate with openpyxl chart APIs (`LineChart`, `BarChart`, etc.) and `ws.add_chart(...)`.

### VBA fixtures (`.xlsm`)

Openpyxl cannot reliably create a brand-new VBA project, so the simplest reliable path is:

* Check in template `.xlsm` files under `fixtures/templates/` created once (via Excel or a trusted tooling flow), such as:

  * `vba_base.xlsm`
  * `vba_added.xlsm`
  * `vba_changed.xlsm`

Then extend the fixture generator with a “copy template” generator that just copies bytes into `fixtures/generated/`.

Also add `fixtures/generated/*.xlsm` to `.gitignore` so local fixture builds don’t create untracked files.

## 3) Add new Rust tests for Branch 4

### VBA module diff test

* Load `vba_base.xlsm` and `vba_added.xlsm` via `WorkbookPackage::open` (macro-enabled file is still ZIP OpenXML). 
* Assert diff contains:

  * `VbaModuleAdded { name: "Module2" }`
* Reverse diff asserts `VbaModuleRemoved`.
* Base → changed asserts `VbaModuleChanged`.

### Named ranges diff test

* Load named ranges fixtures.
* Assert:

  * `NamedRangeAdded`
  * `NamedRangeRemoved`
  * `NamedRangeChanged` with correct `old_ref/new_ref` strings (resolved from `report.strings`). 

### Chart diff test

* Load chart fixtures.
* Assert (A → B):

  * `ChartChanged` for `Chart 1` on `Sheet1`
  * `ChartAdded` for `Chart 2` on `Sheet1`
* Assert (B → A):

  * `ChartRemoved` for `Chart 2`

### JSON serialization test coverage

There’s already a test that roundtrips each `DiffOp` variant through serde JSON; extend it to include the new ops so Branch 4’s “serialize correctly to JSON” is explicitly covered. 

## 4) Ensure “no regressions”

This is mostly guaranteed if:

* For workbooks without those features, parsing produces empty lists / `None`
* You only append ops when there are real differences

But you still validate by running the full existing suite (pg5 grid diff, m6/m7 M diff tests, streaming tests).

---

# Key risks and how to de-risk them

## VBA parsing complexity

Parsing MS-OVBA correctly is tricky (compound file + compression + record parsing). To keep this shippable:

* Build the parser around a known-good compound file crate (feature-gated).
* Start with the subset needed for Branch 4 tests:

  * enumerate modules
  * extract code
  * classify module type best-effort
* Treat unknown records as “skip” rather than fatal.

## Chart variability across producers

Excel, LibreOffice, and openpyxl can produce slightly different chart XML.
To avoid fragile parsing:

* For `chart_type`, detect by presence of `*Chart` elements under plot area; don’t hardcode deep structures.
* For `data_range`, treat as optional and just grab the first plausible `<c:f>`.

## Output visibility

Without updating the CLI/text partitioning, VBA + named ranges will be “in the report” but invisible in the default human output. The existing partitioning logic proves this will happen. 
So treat output updates as part of “entirety” delivery.

