# Design Evaluation Report (Unified)

## Executive Summary

This evaluation examines the Excel Diff Engine codebase, a Rust implementation guided by an ambitious technical specification aiming for high-performance, semantic differentiation of Excel and Power BI artifacts. The architecture is meticulously defined, emphasizing a layered approach (Host Container, Binary Framing, Semantic Parsing, Domain, Diff) designed for extensibility and cross-platform compatibility, including WASM.

The current Rust core is a clean, early-stage skeleton that lines up well with the intended architecture where it is implemented, but it only covers a narrow slice of the full product vision. The codebase cleanly separates container parsing, workbook IR, and diff representation, and the tests strongly encode the invariants of those layers. The IR for workbooks and diffs is simple and well-shaped; the DataMashup top-level framing is carefully implemented with solid error handling and fuzz-style safety checks. The implementation demonstrates excellent Rust idiomaticity, strong error handling, and a well-designed Internal Representation (IR) for grids and diff operations.

However, most of the "hard" parts from the specification and difficulty analysis—hierarchical grid alignment, keyed tabular diff, M/DAX semantic layers, and DiffOp-driven pipelines—are not present yet. Critical gaps exist in the implementation of the core differentiating features: the semantic parsing of Power Query (M) is rudimentary, and the sophisticated diff algorithms detailed in the specification are absent. Instead, there's a deliberately naive, cell-by-cell JSON diff path that bypasses the DiffOp IR entirely. This is acceptable as a stepping-stone but will become architectural debt if it persists.

Crucially, the evaluation identifies significant performance risks—specifically the use of a dense grid representation and a naive diff algorithm—that jeopardize the requirement of "instant diff on 100MB files." Addressing these architectural bottlenecks is the highest priority.

Overall, the architecture is healthy for its current scope: modules map cleanly to the spec's layers, Rust idioms are used well, and tests strongly reinforce contracts. The main risks are forward-compatibility of the DiffOp enum, performance characteristics of the dense grid representation, and the gap between the specified Diffable-based diff pipeline and the current "direct JSON" implementation. By prioritizing these architectural improvements, the project is well-positioned to deliver a differentiated and high-performance product.

---

## Dimension Evaluations

### 1. Architectural Integrity

**Assessment**: Strong for implemented scope, but incomplete vs. spec

**Evidence**

The implementation generally honors the layered architecture specified in `excel_diff_specification.md`, with clean dependency direction, but responsibilities are currently too concentrated and key layers are incomplete.

* The implementation mirrors the spec's layered pipeline:

  * **Host container layer**: `open_workbook` and `open_data_mashup` treat `.xlsx` as ZIP/OPC, validate `[Content_Types].xml`, read workbook XML, and locate `DataMashup` either in `customXml` or via the top-level element, matching the host-container flow in the spec.
  * **Binary framing layer (MS‑QDEFF)**: `parse_data_mashup` decodes the base64 payload, reads `version` and four length-prefixed segments, enforces version==0 and bounds checks, and returns `RawDataMashup { version, package_parts, permissions, metadata, permission_bindings }`—this matches the top-level binary layout described in the spec.
  * **Semantic parsing (grid)**: `parse_workbook_xml` + `parse_sheet_xml` use `quick_xml::Reader` in streaming mode, extract sheet descriptors and the `dimension` ref, then parse `<row>`/`<c>` elements into a list of logical `ParsedCell`s and finally materialize a `Grid`.
  * **Domain layer (IR)**: `Workbook`, `Sheet`, `Grid`, `Row`, `Cell`, `CellValue`, and `CellAddress` form a straightforward IR that matches the spec's workbook/grid structure (minus tables, formats, and data model).
  * **Diff layer**: `DiffOp` and `DiffReport` define the hierarchical diff IR (sheet add/remove, row/column add/remove with signatures, block moves, and `CellEdited` with snapshots), matching the "human-facing hierarchical diff" described in the spec and difficulty analysis.

* **Dependency direction is clean**:

  * `workbook` depends only on `addressing`; `diff` depends on `workbook` for `CellAddress` and `CellSnapshot`.
  * `excel_open_xml` depends on `workbook` and `addressing` but not vice versa.
  * `output::json` depends on `workbook` and `addressing` and is behind the `excel-open-xml` feature for file-based paths.

* **IR Coherence**: The grid IR (`Workbook`, `Sheet`, `Grid`, `Cell`) is coherent and well-structured. The `DiffOp` structure (`diff.rs`) is exceptionally well-designed, anticipating the needs of advanced alignment algorithms.

* Tests are organized around the same layer boundaries: PG1 focuses on IR / grid shape, PG2 on addressing, PG3 on snapshots/serialization, PG4 on `DiffOp` invariants, and dedicated `data_mashup_tests` cover the QDEFF top-level layer.

* **Layer Separation Concerns**: However, `excel_open_xml.rs` is monolithic (1000+ lines), conflating responsibilities across the Host Container (ZIP I/O), Binary Framing (MS-QDEFF parsing via `parse_data_mashup`), and Semantic Parsing (Grid XML interpretation). This concentration hinders architectural clarity and maintainability.

* **Implementation Gaps**: The Power Query IR is underdeveloped, represented only by `RawDataMashup`, lacking the semantic types (Query, MStep) required by the specification. Furthermore, the Diff Layer is currently bypassed; the actual diff implementation resides incorrectly in `output/json.rs` and implements only a naive cell comparison.

* The main structural divergence is the diff flow: the spec calls for a `Diffable`-based pipeline that produces `Vec<DiffOp>`, whereas the current `diff_workbooks_to_json` path computes a flat list of `CellDiff` by scanning the entire grid and does not use `DiffOp` at all.

**Recommendations**

1. **Align the end-to-end diff path with `DiffOp`**: Introduce a simple `diff_workbooks(&Workbook, &Workbook) -> DiffReport` that produces `DiffOp`s, even with a naive algorithm, and have `output::json` convert those ops to JSON instead of rolling its own `CellDiff`. This brings the implementation closer to the spec's architectural "spine".

2. **Decompose `excel_open_xml.rs`**: Split the monolithic parser into smaller, focused modules aligned with architectural layers (e.g., `container.rs`, `grid_parser.rs`, `datamashup_framing.rs`).

3. **Introduce a domain-level `DataMashup` type** that wraps `RawDataMashup` and progressively adds semantic layers (`PackageParts`, `Permissions`, `Metadata`) per the spec, rather than keeping everything at the framing level.

4. **Plan for host polymorphism**: As PBIX support comes online, prefer a `pbix_open` module and possibly a "host container" trait over ad-hoc branching inside `excel_open_xml`, keeping the host layer cleanly separated.

---

### 2. Elegant Simplicity

**Assessment**: Strong

**Evidence**

The codebase manages the inherent complexity of the Excel format reasonably well, with minimal but expressive abstractions.

* **Essential vs Accidental Complexity**: Much of the complexity in `excel_open_xml.rs` is essential, driven by the convoluted nature of the formats. The handling of UTF-16 decoding heuristics for DataMashup XML (`decode_datamashup_xml`) is complex but necessary and handled robustly.

* The workbook IR is minimal but expressive: `Workbook { sheets }`, `Sheet { name, kind, grid }`, `Grid { nrows, ncols, rows }`, with cells holding `(row, col, address, value, formula)` and `CellValue` restricted to `Number(f64)`, `Text(String)`, and `Bool(bool)`. This is exactly the subset needed for early grid diffing without preemptively modeling tables, formats, or data models.

* **Abstraction Fidelity**: The core abstractions (e.g., `CellAddress`, `CellValue`, `DiffOp`) are excellent—clear, precise, and effective. `CellSnapshot` correctly models the data needed for comparison.

  * `CellAddress` is a pair of indices plus A1 conversion, with `FromStr` and `Display` implemented via the `address_to_index` and `index_to_address` helpers. Deserialization validates strings and produces friendly "invalid cell address: {a1}" errors.
  * `CellSnapshot` captures `(addr, value, formula)` but its `Eq` implementation intentionally ignores `addr`, so "same logical content in different positions" compares equal. Tests spell this out and validate JSON round-trips—including tampered addresses—so the behavior is explicit rather than surprising.

* DataMashup framing is handled in a very small, understandable pipeline:

  1. `read_datamashup_text` uses `quick_xml::Reader` to locate a single `<dm:DataMashup>` element, handles UTF‑8 and UTF‑16 encodings, and rejects duplicates.
  2. `decode_datamashup_base64` decodes the text with proper error variants.
  3. `parse_data_mashup` interprets the bytes as `version + 4 length fields + 4 segments` with strict bounds checks.

  Tests then validate "happy path", negative invariants (future version, truncated stream, overflow), and a fuzz-style loop that ensures arbitrary short byte sequences never panic—this keeps complexity at the necessary boundary and nowhere else.

* The grid builder is straightforward: `dimension_from_ref` parses the `ref` like `A1:G10` into `(height, width)`, and `build_grid` pre-allocates an `nrows × ncols` matrix of empty cells and then overlays parsed cells into their positions. This is conceptually simple and matches the mental model of "used range" without introducing sparse data structures yet.

* The JSON diff path is intentionally naive: map sheets by name, compute max extents per pair, iterate `(r, c)` across that rectangle, and compare rendered values; mismatches become `CellDiff { coords, value_file1, value_file2 }`. As a short, readable baseline it's clear and easy to reason about, even though it's not the final algorithm.

* **Dense Grid Representation (CRITICAL)**: The choice of a dense grid representation (`Vec<Vec<Cell>>`) in `workbook.rs` simplifies indexing logic but introduces significant accidental complexity regarding memory management. The `build_grid` function allocates a full matrix based on the used range, which is highly inefficient for sparse files.

**Recommendations**

1. **Transition the `Grid` IR to a sparse representation** to eliminate the accidental complexity of managing memory for empty cells (See Performance Awareness).

2. **Guard against "parallel abstractions"**: retire or clearly relegate the `CellDiff`-based JSON diff to a demo/testing-only module once `DiffOp`-based pipelines land, so there's a single conceptual source of truth for diffs.

3. **Keep semantic richness out of low layers**: resist the temptation to inject higher-level diff concepts (like M-step semantics or DAX metadata) into `excel_open_xml` or `workbook`; instead, add separate domain modules as the spec describes, preserving the current simplicity and separation.

4. **Add minimal comments at module boundaries**, not inside algorithms: short "what this module is" notes for `diff`, `output::json`, and `excel_open_xml` would help future readers keep the narrative without cluttering the already clean code.

---

### 3. Rust Idiomaticity

**Assessment**: Strong

**Evidence**

The codebase speaks fluent, idiomatic Rust, leveraging the language's strengths effectively.

* **Ownership and lifetimes** are straightforward:

  * `open_workbook(path: impl AsRef<Path>) -> Result<Workbook, ExcelOpenError>` consumes a `ZipArchive<File>` and returns a fully owned `Workbook`; there are no `Rc`, `RefCell`, or lifetime gymnastics in public APIs.
  * Parsing functions take `&[u8]` or `&mut ZipArchive<_>` and return owned domain structures, keeping lifetimes local and avoiding hidden borrow complexity.
  * Data flows efficiently from raw bytes through parsing into owned IR structures. There is minimal unnecessary cloning.

* **Error Handling Philosophy**: Errors are treated as values.

  * `ExcelOpenError` is a `thiserror::Error` enum that distinguishes IO, non-ZIP containers, non-Excel packages, missing `workbook.xml`, sheet XML, XML parse errors, and DataMashup-specific failures. Variants carry meaningful data (e.g., `WorksheetXmlMissing { sheet_name }`, `DataMashupUnsupportedVersion { version }`).
  * `open_data_mashup` and friends use `Result<Option<RawDataMashup>, ExcelOpenError>` to distinguish "no mashup present" from "mashup malformed", which matches the spec and testing plan expectations for host/container behavior.
  * The use of `Result` and `Option` is precise throughout the parsing pipeline.

* **Type-driven Design**: The type system is used effectively to encode invariants.

  * `CellAddress` deserialization validates addresses and returns a localized error message that includes the offending string; tests assert that invalid addresses like `"1A"` and `"A0"` are rejected and that the error text mentions both "invalid cell address" and the specific address.
  * The custom `Serialize`/`Deserialize` implementations for `CellAddress` enforce A1 parsing rules during serialization roundtrips, preventing invalid states.
  * `DiffOp::CellEdited` includes a doc comment that spells out invariants: `addr` is canonical, and `from.addr`/`to.addr` must both equal it. Tests build sample `CellEdited` ops and explicitly assert that producers maintain these invariants, acknowledging that they're not enforced by the type system.

* **Serde integration** is clean:

  * `CellAddress` serializes as A1 strings and deserializes with validation. `CellSnapshot` and `CellValue` are `Serialize`/`Deserialize`, and tests round-trip snapshots through JSON, even deliberately tampering the `addr` field to validate the semantics of `Eq`.
  * `DiffOp` uses an internally tagged enum (`#[serde(tag = "kind")]`), aligning with the testing plan's emphasis on a stable wire schema for diff reports.

* **Feature Gating**: Cargo features (`excel-open-xml`) are used correctly to gate I/O dependencies (`zip`, `quick-xml`), supporting the goal of WASM compatibility by allowing core types to exist without I/O dependencies.

* **Standard library and crate usage** is idiomatic: `HashMap`, `Vec`, `Result`, `Option`, `impl AsRef<Path>` are used appropriately; `quick_xml`, `zip`, `base64`, and `thiserror` are used in the way their docs intend.

**Recommendations**

1. **Strengthen type-driven invariants for `DiffOp`**:

   * Introduce a constructor like `DiffOp::cell_edited(sheet: SheetId, addr: CellAddress, from: CellSnapshot, to: CellSnapshot) -> DiffOp` that asserts `from.addr == addr && to.addr == addr`, so it's hard to construct an invalid `CellEdited`. This keeps invariants in one place rather than scattered across tests.

2. **Plan for forward-compatible enums**:

   * Mark `DiffOp` and `ExcelOpenError` as `#[non_exhaustive]` before they are widely consumed, so adding new variants for M/DAX diffs or new error modes doesn't become a breaking change. This is particularly important given the future roadmap.

3. **Introduce the `Diffable` trait once algorithms settle**, following the spec's pattern, to make diff logic composable and testable without entangling everything in free functions. Keep the trait focused and small.

4. **Introduce a dedicated error variant for serialization failures** (e.g., `SerializationError`) instead of reusing `XmlParseError` for JSON errors in `output/json.rs`.

---

### 4. Maintainability Posture

**Assessment**: Strong

**Evidence**

The codebase is supported by an exceptional testing strategy, with clear module boundaries and domain-driven naming.

* **Module boundaries** are small and coherent:

  * `addressing`: just A1 ↔ indices conversions.
  * `workbook`: IR only, no parsing or IO.
  * `diff`: diff IR only, no algorithms.
  * `excel_open_xml`: Excel-specific parsing and DataMashup extraction, behind `excel-open-xml` feature.
  * `output::json`: JSON cell-diff helpers, also gated on the Excel feature for file-based APIs.

  This makes it possible to work on, say, DataMashup semantics without touching grid IR or on diff algorithms without touching ZIP/XML parsing.

* **Change isolation scenarios are clear**:

  * Rewriting the Excel sheet parser largely affects `excel_open_xml` and its tests; the domain IR and `DiffOp` remain untouched.
  * Adding M semantics can proceed by building new modules that consume `RawDataMashup` and eventually adding a `mashup: Option<DataMashup>` field to `Workbook` without breaking existing callers (struct fields are public, so adding more is backward-compatible for users who construct via `open_workbook`).
  * Introducing new diff algorithms can be done by adding a `diff_workbooks` API and leaving `output::json` as a consumer rather than a producer of diffs.

* **Testing as Documentation**: The testing strategy is exemplary.

  * The tests (`core/tests/`) directly mirror the milestones in `excel_diff_testing_plan.md` (e.g., `pg1_ir_tests.rs`, `pg4_diffop_tests.rs`). They serve as excellent documentation of behavior and invariants.
  * PG1 tests define how `Grid` behaves for basic, sparse, empty, and formulas-only sheets: `nrows/ncols` must match the used range, every row must have `ncols` cells, formulas-only sheets must still carry cached values, etc.
  * PG2 tests encode the addressing contract: every textual A1 in the sheet must match `cell.address.to_a1()`, and `address_to_index`/`index_to_address` must be consistent with each other.
  * PG3 tests capture the intended semantics of snapshots and JSON serialization, including equality ignoring address and error messages for invalid addresses.
  * PG4 tests validate `DiffOp` schema and invariants, including JSON representation and the invariants on `CellEdited`.
  * `data_mashup_tests` specify edge behavior for workbooks with and without DataMashup and ensure round-tripping of top-level bytes, plus fuzz-style "never panic" tests for random inputs.

* **Fixture Generation**: The sophisticated Python-based fixture generation system (`fixtures/src/generators/`) ensures deterministic and complex scenarios can be easily created and maintained.

* **Naming Discipline**: Naming is consistently domain-driven: `Workbook`, `SheetKind::Worksheet`, `RawDataMashup`, `RowSignature`, `BlockMovedRows`, and test names like `pg1_sparse_used_range_extents` or `workbook_with_valid_datamashup_parses` all reveal intent clearly.

* **Module Boundary Concern**: While some modules have clear boundaries, maintainability is hampered by the monolithic `excel_open_xml.rs`.

**Recommendations**

1. **(Reiterated) Prioritize the refactoring of `excel_open_xml.rs`** to improve module boundaries.

2. **Document the "PGx" test naming convention** briefly in a top-level crate README or module docs, linking it to the testing plan phases, so new contributors understand the scenario taxonomy.

3. **Keep `workbook` and `diff` modules parsing-free**: as new features (M, DAX, tables) arrive, resist sliding parsing logic into these modules; keep them purely as domain IR so they remain easy to reason about.

4. **Centralize fixture metadata**: eventually follow the testing plan's suggestion of a manifest (`testdata_manifest.yaml`) so tests and planners share the same canonical list of scenarios and their files.

---

### 5. Pattern Appropriateness

**Assessment**: Adequate (minimalistic, with some planned patterns still missing)

**Evidence**

The codebase uses patterns sparingly and appropriately, favoring clarity and direct implementation.

* **Event-driven Parsing**: The choice of `quick-xml` in event-driven (SAX-like) mode is highly appropriate for efficiently parsing potentially large XML files, aligning with the streaming requirements.

* **Enums over object hierarchies**: `DiffOp` is an enum with variants for all the grid-level operations the spec calls out (sheet add/remove, row/column add/remove, block moves, cell edits). This is idiomatic Rust and avoids the complexity of trait-object polymorphism where it's not needed.

* **Error handling pattern**: A single `ExcelOpenError` enum is used consistently across container parsing, XML, and DataMashup framing; this centralizes failure modes and avoids a proliferation of custom error types with little added value. However, `output/json.rs` incorrectly maps `serde_json::Error` to `ExcelOpenError::XmlParseError`.

* **Thin API layer**: `diff_workbooks_to_json` and `serialize_cell_diffs` are simple convenience functions; they do not try to be a full "strategy" layer, which keeps the pattern surface small at this stage.

* **Simplicity**: The implementation is straightforward, relying on standard Rust idioms and the powerful `Serde` framework for serialization. There is no premature abstraction.

* **Some planned patterns from the spec are not yet realized**:

  * The `Diffable` trait (for `Workbook`, `Sheet`, `Grid`, etc.) is specified as the backbone for diff algorithms, but is not present in the code yet; free functions and tests are currently acting as the diff driver.
  * There is no explicit "host abstraction" to unify Excel and PBIX handling yet; `excel_open_xml` is Excel‑specific by design, which is fine for now but will need a pattern later.

**Recommendations**

1. **Introduce `Diffable` as a structuring pattern** when you start implementing real diff algorithms:

   * Keep it simple (`type Diff; fn diff(&self, other: &Self) -> Self::Diff`) as in the spec, and define small `Diff` structs that can be converted into `Vec<DiffOp>`. This makes it easier to test and evolve each layer of the diff pipeline independently.

2. **Mark key enums `#[non_exhaustive]`** early to future-proof the pattern; this is a one-time change that unlocks safe evolution for both `DiffOp` and `ExcelOpenError`.

3. **Keep polymorphism explicit and local**: for host handling, prefer explicit enums or small traits (`enum HostKind { Excel, Pbix }` or `trait HostContainer`) rather than pervasive trait-object use. That keeps the pattern surface understandable and avoids accidental dynamic dispatch overhead.

4. **Introduce a dedicated error variant for serialization failures** (e.g., `SerializationError`) instead of reusing `XmlParseError` for JSON errors.

---

### 6. Performance Awareness

**Assessment**: Concerning

**Evidence**

While the parsing code shows performance awareness, the core IR design and the current diff algorithm conflict with the stated goal of handling 100MB files efficiently (Difficulty H1/H2). The product plan explicitly targets "instant diff" and marketing claims like "Compare 100MB files in under 2 seconds."

* **Positive foundations**:

  * XML parsing is streaming via `quick_xml::Reader`; neither workbook nor DataMashup XML are loaded in full string form. Grid parsing walks the event stream and only materializes the cells it sees.
  * `parse_data_mashup` works on slices and uses offsets rather than copying whole sections into new buffers beyond the required `Vec<u8>` segments in `RawDataMashup`. Error checks prevent runaway reads or panics on malformed streams.

* **Grid Allocation Bottleneck (CRITICAL)**: The dense `Grid` representation will fail on large, sparse workbooks due to massive memory allocation in `build_grid`. This directly threatens the performance goals (H2).

  * `build_grid` always allocates a full dense `nrows × ncols` matrix of `Cell`s, even for sparse sheets. For wide but sparsely populated models, this can be very memory-heavy. The spec suggests streaming signatures and sparse structures for large sheets instead.

* **Data Duplication**: `Cell` owns a `String` for text and `Option<String>` for formula; combined with a copy of the shared string's content in each cell, this duplicates data heavily for common repeated values. A more memory-conscious representation would keep shared strings truly shared and formulas in an interned table.

* **Algorithmic Bottleneck**: The implemented diff algorithm (`compute_cell_diffs`) is a naive O(R*C) positional comparison. It does not implement the sophisticated, near-linear alignment algorithms (Patience Diff, LAPJV) specified in the documentation, which are essential for performance (H1).

  * `diff_workbooks_to_json` is O(S · R · C) over sheets, rows, and columns; it scans every cell in the union of sheet extents, regardless of how many cells actually changed. There's no use of row/column signatures or early exit strategies described in the spec's grid diff section.

* **I/O Strategy**: The current approach reads entire ZIP entries into memory (`read_zip_file`) before parsing. For extremely large XML parts, this may need to evolve to true streaming I/O directly from the ZIP stream.

* The **testing plan** and difficulty analysis both emphasize performance-focused milestones (P1, P2) and metrics collection for parse time per MB, peak memory, and alignment efficiency, but there is no sign of those harnesses or metrics in the current code.

**Recommendations**

1. **CRITICAL: Redesign the `Grid` IR** to use a sparse representation (e.g., `HashMap` or `BTreeMap` based structures) rather than dense `Vec<Vec<Cell>>`. This is critical to address the memory overhead risk (H2) on large, sparse files.

2. **Introduce signatures into the IR**:

   * Add `RowSignature` and `ColSignature` fields (or side tables) to `Grid` as the spec suggests; compute them during parsing, ideally with minimal additional memory. This lays the groundwork for anchor-based alignment without re-parsing the workbook.

3. **Refactor shared strings and formulas to avoid duplication**:

   * Store shared string indices in cells and keep the string table on the workbook or sheet; add lookup utilities to render user-facing text when needed.
   * Consider interning formulas to avoid repeated `String` allocations for identical formulas across many cells.

4. **Begin implementation of the specified high-performance alignment algorithms**: Start implementing the Hybrid Alignment Pipeline (Patience Diff, LCS, Myers/Histogram, LAPJV) as specified, to replace the naive diff in `output/json.rs` and address the core algorithmic risk (H1).

5. **Treat the current JSON diff as "small grid only"**:

   * Clearly document and/or guard it for small sheets, and design the real diff engine around the Hybrid Alignment Pipeline from the spec. Even a partial implementation (row-only) will be a big step toward the performance targets.

6. **Add performance harness hooks early**:

   * Implement the `metrics-export` feature flag and basic `parse_only_large_workbook` benchmarks as described in the testing plan, even before algorithms are fully optimized. This will keep performance visible as the code evolves.

---

### 7. Future Readiness

**Assessment**: Strong

**Evidence**

The architecture provides a solid foundation for the future extensions envisioned in the specification (DAX, PBIX, WASM).

* **Extension Points**: The layered architecture allows for clear extension points for new host containers (PBIX) and semantic parsers (M, DAX). The `DiffOp` structure is robust and extensible.

* The core design is **host-agnostic at the domain layer**:

  * `Workbook` and `DiffOp` know nothing about Excel containers or file IO; they are pure domain structures with standard Serde support. This makes them natural candidates for reuse in a WASM-compiled library.
  * Host-specific functionality is isolated under the `excel-open-xml` feature, which can be disabled for WASM builds or supplemented with PBIX-specific modules later.

* The IR is **extendable**:

  * Adding `data_model: Option<DataModel>` and `mashup: Option<DataMashup>` to `Workbook` matches the spec and is backward-compatible for most users (field addition).
  * New `DiffOp` variants (e.g., `MQueryChanged`, `MeasureDefinitionChanged`, `MetadataChanged`) can be added as long as the enum is made non-exhaustive now, preserving forward compatibility in public APIs.

* **WASM Compatibility**: The codebase is designed with WASM in mind, using compatible dependencies and feature gates.

* **Abstraction Stability**: The core abstractions appear stable and capable of supporting future capabilities, provided the performance concerns regarding the `Grid` IR are addressed.

* The **testing plan** already anticipates PBIX host support (Phase 3.5), DAX/model stubs (DX1), and cross-platform determinism, so the repository layout is already aligned with a multi-host, multi-platform story.

**Recommendations**

1. **Implement the Domain Layer structures for M Queries** (`Query`, `MStep`, ASTs) as defined in the specification (Section 5.3) to enable the implementation of the semantic diff engine (H4).

2. **Formalize `Workbook` as the central domain object**:

   * Add `mashup` and (later) `data_model` fields and define how `open_workbook` populates them as DataMashup and model parsers mature. This avoids creating parallel "ExcelWorkbookWithM" types later.

3. **Design `DiffReport` with extension in mind**:

   * Keep the schema version field and plan for versioned JSON diff outputs; consider including a "kind" field for diff categories (grid, M, DAX, metadata), aligning with the planned hierarchical diff reports.

4. **Sketch host abstractions early**:

   * It may be useful to define an internal trait like `HostWorkbookSource` (with methods to obtain `Workbook`, `RawDataMashup`, and, later, `DataModel`) to reduce duplication when PBIX arrives. Implementations can live in `excel_open_xml` and a future `pbix` module.

5. **Keep WASM in view**:

   * Avoid introducing blocking dependencies (thread-heavy or OS-specific APIs) into the core; keep file IO strictly at the edges and ensure an interface that can accept in-memory buffers (for browser file uploads). The current design already leans this way; just preserve that discipline.

---

## Tensions and Trade-offs

1. **Dense vs. Sparse Grid Representation**
   This is the central tension. The current dense representation favors implementation simplicity (easy indexing) but sacrifices memory efficiency for sparse sheets. The dense `Grid` representation and per-cell JSON diff are beautifully simple but clearly at odds with the "100MB in 2 seconds" ambition. The current code chooses simplicity, which is reasonable for an early milestone, but the tension will grow as larger workbooks and advanced algorithms arrive. This trade-off is currently unbalanced and must be corrected to meet the performance goals.

2. **Canonical diff IR vs. ad-hoc JSON schema**
   `DiffOp` + `DiffReport` are poised to be the canonical description of differences, while `CellDiff` in `output::json` is a separate representation. As soon as you implement real algorithms, one of these will become the "truth" and the other a view; today they coexist without a clear hierarchy. That's a design tension that should be resolved in favor of `DiffOp`.

3. **Monolithic Parser vs. Modularity**
   The consolidation of parsing logic accelerated initial development but has created a maintainability burden. This tension needs to be resolved through refactoring.

4. **Implementation Velocity vs. Algorithmic Complexity**
   The project has prioritized foundational robustness over implementing the complex diff algorithms. This is sound, but the project must now pivot to implementing these algorithms (H1, H4) to deliver its core value proposition.

5. **Current scope vs. full product vision**
   The codebase focuses on container parsing, grid IR, and top-level DataMashup framing—roughly H8 and part of H2 in the difficulty ranking—while leaving H1 (grid diff), H3/H4 (M parsing + diff), and H5/H6 (DAX/formulas) as future work. This is a deliberate "bottom half first" approach, but it means decisions today about IR and enums need to anticipate those future layers.

6. **Domain purity vs. ergonomic APIs**
   Keeping `workbook` and `diff` pure IR modules makes the design clean, but the convenience API `diff_workbooks_to_json(path, path)` ties diffing directly to filesystem and host container details. There's a tension between "ergonomic CLI-level API" and "pure library core"; the next iteration should keep them separate by layering.

---

## Areas of Excellence

1. **Testing Strategy and Execution**
   The alignment between the `excel_diff_testing_plan.md` and the implementation is exceptional. It provides high confidence and clarity. The integration tests (`pg1_*`, `pg2_*`, `pg3_*`, `pg4_*`, JSON diff tests, DataMashup tests) closely track the testing plan and effectively serve as executable documentation of scenario behavior. This alignment between docs and tests is a strong foundation for the meta-programming process described in the meta-planning doc.

2. **DataMashup Extraction Robustness**
   The handling of the MS-QDEFF container and the complex heuristics required for UTF-16 detection and decoding in `customXml` parts (`excel_open_xml.rs`) demonstrates excellent defensive programming. The framing code for `RawDataMashup` is small, well-factored, and very well tested: it handles multiple encodings, rejects malformed XML and binary layouts, enforces version and length invariants, and even includes fuzz-style tests to guarantee no panics. This is exactly the kind of defensive programming you want at a difficult boundary layer.

3. **`DiffOp` and `CellAddress` Design**
   The structures in `diff.rs` and `workbook.rs` are clean, well-serialized (leveraging Serde effectively), and handle subtle invariants robustly. `DiffOp` and `DiffReport` are thoughtfully designed to cover the major grid-level operations, with room for row/column signatures and block move hashes. The comments on `CellEdited` and the associated tests show careful thinking about logical vs. positional equality and how that will inform future UX.

4. **Workbook IR and Addressing**
   The `Workbook`/`Sheet`/`Grid` model plus `CellAddress` and `CellSnapshot` give a crisp mental picture of a workbook as a 2D grid of typed values and formulas, with A1 addressing thoroughly tested. It's easy to visualize and reason about.

5. **Rust Idiomaticity**
   The use of error handling, ownership, and the type system is consistently idiomatic and effective.

---

## Priority Recommendations

### High Priority

1. **Redesign Grid Representation for Performance**
   Transition the `Grid` IR from a dense (`Vec<Vec<Cell>>`) to a sparse representation. This is critical to address the memory overhead risk (H2) on large, sparse files.

2. **Make `DiffOp` the Canonical Diff Representation**
   * Implement a `diff_workbooks(&Workbook, &Workbook) -> DiffReport` entry point, even with a simple algorithm that just walks aligned rows/columns and emits `CellEdited`/`RowAdded`/`RowRemoved`.
   * Refactor `diff_workbooks_to_json` to consume `DiffReport` rather than recomputing diffs. This aligns implementation with the spec's hierarchy and removes parallel diff representations.

3. **Refactor Monolithic Parser**
   Split `excel_open_xml.rs` into smaller, focused modules aligned with architectural layers (Container, Framing, Semantic Parsing).

4. **Harden Enums for Future Growth**
   * Mark `DiffOp` and `ExcelOpenError` as `#[non_exhaustive]`.
   * Add constructor functions for critical variants like `CellEdited`, enforcing invariants around addresses. This is cheap now and expensive later.

5. **Introduce Lightweight Performance Primitives**
   * Add row/column signatures to `Grid` and compute them in the parser.
   * Start a bench/harness with `metrics-export` for parsing and diffing large synthetic workbooks (P1/P2 scenarios).
   * Document the current JSON diff as intended for small sheets and begin designing the Hybrid Alignment Pipeline.

### Medium Priority

6. **Implement Advanced Diff Algorithms**
   Begin implementing the Hybrid Alignment Pipeline (Patience Diff, LCS, LAPJV) as specified, to replace the naive diff in `output/json.rs` and address the core algorithmic risk (H1).

7. **Implement M Query Domain Layer**
   Introduce the `Query` and M AST structures as defined in the specification to enable semantic diffing (H4), a key product differentiator.

8. **Introduce Domain-level `DataMashup` and Host Abstractions**
   * Wrap `RawDataMashup` into a richer `DataMashup` type matching the spec's `PackageParts`, `Permissions`, and `Metadata`.
   * Plan a PBIX parsing module that reuses the same framing logic, as the testing plan suggests.

9. **Add High-level Crate Documentation**
   * Provide a crate-level overview that mirrors the spec's layer diagram and maps modules (`excel_open_xml`, `workbook`, `diff`, `output`) to those layers.
   * Briefly explain the PGx test convention and how it maps to the testing plan phases.

10. **Plan for M/DAX Extension Points**
    * Extend `Workbook` with optional `mashup` (and later `data_model`) fields.
    * Reserve `DiffOp` variants or sub-enums for M and DAX diffs so that future work fits naturally into the existing IR.

---

## Conclusion

The Excel Diff Engine is architecturally sound in its high-level design and demonstrates high quality in its Rust implementation and testing strategy. The foundation is well-laid for the ambitious goals of semantic diffing and cross-platform execution. The code nails the lower layers—container parsing, workbook IR, DataMashup framing, and diff representation—and aligns well with the direction laid out in the specification, testing plan, and difficulty analysis. The code uses Rust idioms effectively, separates concerns cleanly, and encodes its invariants in both types and tests.

The main gaps are where you'd expect for a system at this stage: there is no real diff engine yet, performance considerations are more potential than reality, and semantic layers (M, DAX, metadata) are still just plans in the documents rather than code. None of these are structural defects; they are simply the next steps.

The primary challenge lies in evolving the implementation to meet the demanding performance requirements, specifically by addressing the critical bottlenecks in grid allocation and implementing the specified advanced diff algorithms. If you now focus on (1) unifying around `DiffOp` as the canonical diff representation, (2) hardening enums and IR for future growth, and (3) seeding performance primitives (signatures, metrics, streaming patterns), this architecture will be well positioned to absorb the high-difficulty work—grid alignment, M/DAX semantics, PBIX support—without needing a redesign.

By prioritizing these architectural improvements, the project is well-positioned to deliver a differentiated and high-performance product.

