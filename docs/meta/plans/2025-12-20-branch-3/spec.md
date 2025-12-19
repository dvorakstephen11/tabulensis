## Goal and scope for Branch 3

Branch 3’s scope is: remove panics from production code paths, make malformed/corrupt inputs return **typed, contextual errors**, add **ZIP bomb defenses**, add **error codes**, enforce **`clippy::unwrap_used`**, and add **`cargo-fuzz`** harnesses so random inputs don’t crash the parser/diff engine. 

Below is a concrete implementation plan grounded in your current code structure and the specific hotspots visible in `codebase_context.md` (e.g., panics in the diff entrypoints and unchecked ZIP reads).

---

## Phase 0: Baseline audit (one pass, then keep it “zero-regression”)

### 0.1 Create a “panic/unwrap inventory”

Run exactly the search Branch 3 calls for, and save the output in a short dev note (or a GitHub issue comment), so it’s easy to prove you chased every instance. 

Command (from plan):

* `rg '\.unwrap\(\)|\.expect\(|panic!\(|unreachable!\(' core/src --type rust` 

### 0.2 Categorize each hit

For each match, label it as:

* **Input-driven** (can be triggered by malformed XLSX/XML/ZIP/DataMashup) → must become `Result`/warning, never panic.
* **Invariant-driven** (pure internal logic bug) → still prefer `Result` or a structured “internal error” over panic; if you keep a panic, you must document why and ensure it’s not reachable via user-controlled input. 

---

## Phase 1: Fix the known production panics first (they violate the goal immediately)

These are already visible in your codebase context and are almost certainly reachable today.

### 1.1 `engine::diff_workbooks*` currently panics on `Err`

In `core/src/engine/workbook_diff.rs`, both `diff_workbooks` and `diff_workbooks_streaming` call the `try_` versions and `panic!("{}", e)` on error. 

**Plan:**

1. Keep the `try_` functions as the “truth” (they already return `Result<… , DiffError>`).
2. Change the non-try wrappers to be **non-panicking**:

   * `diff_workbooks(...) -> DiffReport`:

     * On `Ok(report)`: return it.
     * On `Err(e)`: return a **synthetic DiffReport**:

       * `ops = empty`
       * `summary.complete = false`
       * `summary.warnings.push(format!("…"))` (include error code + actionable hint)
       * preserve string pool snapshot if needed (or keep empty string table).
   * `diff_workbooks_streaming(...) -> DiffSummary`:

     * On `Err(e)`: return `DiffSummary { complete: false, warnings: vec![…], op_count: 0, … }`
3. Add a unit test asserting **no panic** from these wrappers when `try_` returns `Err`.

This single change eliminates the most egregious “production code must not panic” violation.

### 1.2 `diff_grids_database_mode` currently panics on `Err`

In `core/src/engine/grid_diff.rs`, `diff_grids_database_mode` calls `try_diff_grids_database_mode_streaming(...)` and then `.unwrap_or_else(|e| panic!("{}", e))`. 

**Plan:**

1. Mirror the workbook approach:

   * On `Err(e)`, return a `DiffReport` that:

     * has `complete=false`
     * includes a warning with the error code + message
     * includes whatever string pool snapshot is appropriate
2. Add unit tests:

   * “limits exceeded” (config `on_limit_exceeded = ReturnError`) should no longer panic; should produce incomplete report with warning.
   * “sink error” scenario (use a sink that returns `Err`) should not panic.

This removes another “panic on input/config” path.

### 1.3 Remove/neutralize the debug-only panic triggered by duplicate sheet identity

You currently have behavior where duplicates cause a debug assertion failure, and there’s even a test that *expects* panic in debug builds.

The duplicates are input-derived (a corrupt workbook can absolutely produce this), so Branch 3 wants **no panics** for malformed inputs. 

**Plan:**

1. In `try_diff_workbooks_streaming`, replace the `debug_assert!(was_unique, …)` on sheet identity inserts with **runtime handling**:

   * Keep “last writer wins” determinism (you already document/expect that behavior in release tests).
   * But add a warning into `ctx.warnings` whenever you detect a collision:

     * include the normalized identity key and both original sheet names if available
     * suggest the file is corrupt / Excel disallows this
2. Update/remove `duplicate_sheet_identity_panics_in_debug` test:

   * New assertion: never panics in either mode; and warning exists in debug and release.
3. Replace `unreachable!()` in the `(None, None)` match arm with a non-panicking fallback:

   * `debug_assert!(false, "…"); continue;`
   * or return a `DiffError::InternalInvariant { … }` (preferred if you want typed reporting), but **do not panic**.

---

## Phase 2: Make ZIP handling robust and ZIP-bomb resistant

Right now, `OpcContainer::read_file` reads the entire decompressed part into memory with no size checks, and `open_from_reader` validates OPC-ness by reading `[Content_Types].xml` (also without limits). 
Branch 3 explicitly requires ZIP entry size validation. 

### 2.1 Add container-level limits (and make them testable)

Implement a small limits struct:

* `max_entries: usize` (prevents pathological “millions of parts” archives)
* `max_part_uncompressed_bytes: u64` (prevents huge single entries)
* `max_total_uncompressed_bytes: u64` (prevents cumulative blowups during parse)

**Where:** `core/src/container.rs` alongside `OpcContainer`. 

**API approach:**

* Add `OpcContainer::open_from_reader_with_limits(reader, limits)` (new).
* Keep `open_from_reader` using `Default::default()` limits for backward compatibility.
* Store limits inside `OpcContainer` (private field), so `read_*` can enforce them.

### 2.2 Change OPC validation to “presence check”, not “read whole file”

Instead of calling `read_file("[Content_Types].xml")` to verify OPC, do:

* `archive.by_name("[Content_Types].xml")` to check existence
* check that entry size is within limit
* do not decompress/read unless needed

This avoids allocating `[Content_Types].xml` at all. 

### 2.3 Provide “checked read” methods used by the XLSX parser

Keep your existing `read_file` if you must, but add and migrate parser code to a safe version, e.g.:

* `read_file_checked(&mut self, name: &str) -> Result<Vec<u8>, ContainerError>`
* `read_file_optional_checked(...) -> Result<Option<Vec<u8>>, ContainerError>`

These methods:

* reject `archive.len() > max_entries`
* inspect `ZipFile::size()` (uncompressed) before reading
* enforce per-part and total limits before calling `read_to_end`

### 2.4 Add new `ContainerError` variants for size/structure failures

Today `ContainerError` is fairly coarse (`Zip(String)`, `NotZipContainer`, `NotOpcPackage`). 
Branch 3 wants actionable context + programmatic handling. 

Add variants such as:

* `TooManyEntries { entries, max_entries }`
* `PartTooLarge { path, size, limit }`
* `ZipRead { path, source }` (so the message includes which part failed)
* (optional) `InvalidZip { message }` vs `NotZipContainer` if you want more precision

Then implement:

* `ContainerError::code() -> &'static str`
* `Display` that includes the part name + what to do next.

---

## Phase 3: Rework `PackageError` into the comprehensive, contextual type Branch 3 describes

Your current `PackageError` mostly wraps lower-level errors and uses “missing workbook/worksheet xml” variants that discard part path context, and in some places it treats *any* ZIP read error as “missing”.

### 3.1 Expand `PackageError` with the variants specified in Branch 3

Branch 3 explicitly proposes variants like:

* `NotAZip(std::io::Error)`
* `MissingPart { path }`
* `InvalidXml { part, error }`
* `UnsupportedFormat { message }` 

**Plan:**

1. Keep existing variants for compatibility (it’s `#[non_exhaustive]` already), but start returning the new, more specific variants from the parsing entrypoints. 
2. Add a `code()` method and constants for each “top-level failure family”.

### 3.2 Fix “missing vs unreadable” semantics in `open_workbook_from_container`

Today, e.g.:

* workbook.xml read failure maps to `WorkbookXmlMissing` regardless of why
* sheet xml read failure maps to `WorksheetXmlMissing` regardless of why 

**Plan:**

1. Use the new `read_file_checked` that returns a structured `ContainerError` that includes the path.
2. Map:

   * `FileNotFound` → `PackageError::MissingPart { path: "xl/workbook.xml".into() }`
   * other read errors → `PackageError::UnsupportedFormat { message: "Failed reading part …: …" }` (or a dedicated `ReadPartFailed { part, source }` variant if you prefer)
3. For sheet targets, the `resolve_sheet_target` output should be included verbatim in the part path you report (e.g., `xl/worksheets/sheet1.xml`). This makes the error immediately actionable.

### 3.3 Add file-path context “when available”

Branch 3 wants file path included when available. 

**Plan options (do both if you want strong UX):**

* **Library:** add `WorkbookPackage::open_path(path)` and/or update the deprecated `open_workbook(path)` wrapper to wrap any `PackageError` inside a `PackageError::WithPath { path, source }` variant. Your deprecated path-based functions exist and are a natural place to do this. 
* **CLI:** you already have the path in hand; ensure every CLI error print includes it. (Even if you rely on anyhow context in CLI, still adding `WithPath` at the library level helps non-CLI callers.)

---

## Phase 4: XML parse errors with part path + line/column

Grid parsing errors currently include `XmlError(String)` but no part path or line/column.
Branch 3 explicitly requires line/column for parse errors and “XML path within the package.” 

### 4.1 Add an XML error helper that captures byte offset and computes line/col

In the parsers that use `quick_xml::Reader`, you can:

* read events
* on `Err(e)`, call `reader.buffer_position()` to get a byte offset
* compute `(line, col)` by scanning `xml_bytes[..offset]` (only on error, so cost is fine)

### 4.2 Introduce a structured parse error variant (non-breaking)

Because `GridParseError` is `#[non_exhaustive]`, add a new variant rather than changing existing ones, e.g.:

* `XmlErrorAt { line: usize, column: usize, message: String }`

Then update:

* `parse_workbook_xml`
* `parse_relationships`
* `parse_shared_strings`
* `parse_sheet_xml`
  to emit `XmlErrorAt` wherever you currently build `XmlError(String)`.

### 4.3 Wrap grid parse errors at the package level with the part path

Even if `GridParseError` doesn’t carry a part name, `open_workbook_from_container` does know it:

* workbook: `xl/workbook.xml`
* rels: `xl/_rels/workbook.xml.rels`
* shared strings: `xl/sharedStrings.xml`
* sheet: resolved target `xl/worksheets/*.xml` 

So when mapping:

* `GridParseError::XmlErrorAt { … }` → `PackageError::InvalidXml { part, error: “…”, line, column }`
* `GridParseError::InvalidAddress(addr)` → `PackageError::UnsupportedFormat { message: format!("Invalid cell address '{addr}' in {part} …") }`
* `SharedStringOutOfBounds(i)` → `PackageError::UnsupportedFormat { message: format!("Shared string index {i} out of bounds while parsing {part} …") }`

This achieves “part path + line/col + actionable suggestion” where it matters most: when opening XLSX packages.

---

## Phase 5: DataMashup robustness + removing remaining unwraps

Branch 3 calls out “unexpected DataMashup formats” and “no crashes.” 
You already have good error modeling in `DataMashupError`, but there’s at least one production `.unwrap()` in `metadata_xml_bytes` (`try_into().unwrap()`), which violates the audit requirement.

### 5.1 Replace `try_into().unwrap()` with checked conversion

In `metadata_xml_bytes`, replace:

* `metadata_bytes[0..4].try_into().unwrap()`
  with:
* a checked conversion that returns `DataMashupError::XmlError(...)` (or a new `DataMashupError::InvalidHeader { … }`).

This is a straightforward “unwrap removal” and should be clippy-clean.

### 5.2 Add part-path context for DataMashup discovery errors

When `open_data_mashup_from_container` scans `customXml/*.xml`, the error should say *which* `customXml/itemN.xml` contained the bad DataMashup. Right now, the error is mostly just `DataMashupError` without guaranteed part context. 

**Plan:**

* Add a `PackageError` wrapper variant like:

  * `PackageError::DataMashupPartError { part: String, source: DataMashupError }`
* When a DataMashup is detected inside a given customXml part, any decode/parse error should be wrapped with that part.

### 5.3 Apply ZIP-bomb protections to inner DataMashup “package parts”

`package_parts` are parsed as another ZIP in `datamashup_package`. If you read those entries without size limits, fuzzing can still crash you with an inner bomb even if the outer XLSX is bounded.

**Plan:**

* Reuse the same limits type (or define a narrower one) for inner package parsing:

  * `max_inner_entries`
  * `max_inner_part_bytes`
  * `max_inner_total_bytes`
* Return a `DataMashupError` variant that clearly says “inner package part too large” with the inner path.

---

## Phase 6: Error codes + Display overhaul (actionable context)

Branch 3 requires:

* error codes as constants
* Display messages that include context and suggestions 

### 6.1 Add a single error-code module

Create `core/src/error_codes.rs` that defines constants like:

* `pub const PKG_NOT_ZIP: &str = "EXDIFF_PKG_001";`
* `pub const PKG_MISSING_PART: &str = "EXDIFF_PKG_002";`
* `pub const PKG_INVALID_XML: &str = "EXDIFF_PKG_003";`
* `pub const PKG_ZIP_PART_TOO_LARGE: &str = "EXDIFF_PKG_004";`
* etc.

### 6.2 Add `code()` on every public error enum

Implement:

* `impl PackageError { pub fn code(&self) -> &'static str { … } }`
* `impl ContainerError { … }`
* `impl GridParseError { … }`
* `impl DataMashupError { … }`
* `impl DiffError { … }`

Even if some are wrappers, ensure codes are stable and map to the most meaningful top-level category.

### 6.3 Make Display consistently include:

For `PackageError` especially:

* code
* file path when present (`WithPath`)
* part path
* line/col (for XML errors)
* suggestion text (one short sentence)

Example style (not literal requirement):

* `[EXDIFF_PKG_003] Invalid XML in 'xl/workbook.xml' at line 12, col 7: … Suggestion: re-save the file in Excel or verify it is a valid .xlsx.`

This meets the “actionable context” requirement. 

### 6.4 Document the codes

Add a user-doc page (e.g., `docs/errors.md`) that lists:

* code
* meaning
* likely cause
* suggested next step

Branch 3 explicitly requires this. 

---

## Phase 7: Tests for corrupt fixtures + “no panic” guarantees

Branch 3 requires tests with corrupt fixtures and “no panics when processing corrupt fixtures.” 
You already have a great testing pattern in `core/tests/excel_open_xml_tests.rs` that generates zips in temp and asserts error variants. 

### 7.1 Add explicit corrupt ZIP tests

Add tests that build and assert:

1. **Truncated ZIP**

   * Create a valid zip via your helper, then truncate file bytes.
   * Expect: `PackageError::NotAZip` (or `ContainerError::NotZipContainer` wrapped) with code.

2. **ZIP bomb defense**

   * Use `open_from_reader_with_limits` with a very small `max_part_uncompressed_bytes`.
   * Create an entry bigger than the limit.
   * Expect: `PartTooLarge` / `ZipPartTooLarge` error variant with the part path.

These directly cover Branch 3’s malformed + zip-bomb deliverables.

### 7.2 Add invalid XML tests with part + line/col assertions

Create minimal XLSX shells with:

* invalid `xl/workbook.xml`
* invalid `xl/worksheets/sheet1.xml`
* invalid `xl/sharedStrings.xml`

Assert:

* error variant is `InvalidXml { part: … }`
* `part` matches exactly
* line/col are present (if you store them) or appear in the Display string (if you encode them)

### 7.3 Add DataMashup malformed-format tests

You already test lots of DataMashup scenarios, but add at least:

* DataMashup element present but base64 decodes to garbage framing
* DataMashup with inner package zip that exceeds limits

Assert no panics and correct error code.

### 7.4 Add “no panic” guard tests

For each corrupt fixture case, wrap the parse call in `std::panic::catch_unwind` and assert it returns normally (Ok/Err), never panics.

This is especially important after you remove the panics in:

* `diff_workbooks*` wrappers 
* `diff_grids_database_mode` 

---

## Phase 8: `clippy::unwrap_used` enforcement (with documented exceptions if truly needed)

Branch 3 requires `clippy::unwrap_used` passes (or exceptions documented). 

### 8.1 Add clippy enforcement in CI

Update your CI workflow to run clippy with:

* `-D clippy::unwrap_used`
* (strongly recommended) `-D clippy::expect_used`

Make sure the command you use matches what you want enforced (workspace-wide vs library-only). The intent in Branch 3 is non-test production code. 

### 8.2 Make the core crate deny unwrap/expect by default

In `core/src/lib.rs`, add crate-level lints:

* deny unwrap/expect
* allow them only under `cfg(test)` if you don’t want to rewrite tests

### 8.3 Fix remaining unwraps in core/src

You already have at least one production unwrap in DataMashup parsing. 
Your audit sweep will likely find others; fix them similarly.

### 8.4 If any unwrap is intentionally kept

Add:

* a local `#[allow(clippy::unwrap_used)]`
* a one-line comment explaining why it can’t be triggered by user input and what invariant it relies on
* and mention it in the “exceptions documented” list

---

## Phase 9: Fuzzing with `cargo-fuzz` (no crashes)

Branch 3 requires fuzzing finds no crashes. 

### 9.1 Add fuzz harnesses (minimum set)

Create a `fuzz/` directory (standard `cargo fuzz init`) and add at least these targets:

1. **Fuzz open workbook from arbitrary bytes**

   * Feed `&[u8]` into `WorkbookPackage::open(Cursor::new(data))`
   * Do not `catch_unwind`—a panic is a crash the fuzzer should catch
   * Use small container limits inside the fuzz target if you added configurable limits, to avoid OOM

2. **Fuzz DataMashup parser**

   * Directly fuzz `parse_data_mashup` and `build_data_mashup` on random bytes, or fuzz the whole XLSX open path (which will reach DataMashup scanning)

3. **Fuzz diff engine on structured small inputs**

   * Interpret the byte stream as:

     * small dimensions
     * a few random populated cells
   * Run `try_diff_workbooks_streaming` and ensure it never panics

### 9.2 Make fuzzing practical

* Keep per-part size limits low in fuzz targets to prevent pathological allocations.
* Consider disabling features that inflate runtime if needed, but ensure the XLSX parsing paths are included.

### 9.3 Optional (recommended): add a CI job that builds fuzz targets

Even if you don’t run fuzzing in CI, at least compiling the fuzz targets prevents bitrot.

---

## Acceptance criteria mapping (how you’ll know Branch 3 is “done”)

### No panics on corrupt fixtures

* Achieved by removing production panics in `diff_workbooks*` and `diff_grids_database_mode`
* Replacing debug assertion behavior on duplicate sheet identity with warnings (not panic)
* Adding explicit corrupt fixture tests + `catch_unwind` guards 

### All errors include actionable context

* `PackageError` includes part path + XML location + suggestion text
* `ContainerError` includes which ZIP entry failed + size limit info
* DataMashup failures identify the `customXml/item*.xml` source part

### `clippy::unwrap_used` passes (or exceptions documented)

* Remove `.unwrap()` like the one in `metadata_xml_bytes`
* Add CI clippy gate + crate-level lint policy

### `cargo-fuzz` finds no crashes

* Fuzz targets for open/parsing + diff
* ZIP entry limits prevent OOM/crash on adversarial “zip bomb” inputs

---

## Suggested implementation order (to keep risk low)

1. **Eliminate production panics** (`diff_workbooks*`, `diff_grids_database_mode`, duplicate sheet identity) so your “must not panic” claim becomes true early.
2. **Add ZIP limits + checked read API** in `OpcContainer` (this is crucial before fuzzing).
3. **Upgrade PackageError mapping** so missing vs unreadable vs invalid XML is correctly distinguished, and errors carry part path.
4. **Add XML line/col** and propagate into `PackageError::InvalidXml`.
5. **Add error codes + docs**. 
6. **Add corrupt fixture tests**, then **turn on clippy gate**, then **add fuzz harnesses**.
