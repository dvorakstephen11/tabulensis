# Design Evaluation Report

## Executive Summary

The Excel Diff Engine has a clear architectural spine: a layered pipeline from container → Open XML / DataMashup framing → domain IR (Workbook, Grid, DataMashup) → diff algorithms → JSON output. That structure is consistently reflected in the code and tests and aligns closely with the written specification and testing plan.

The current implementation is a strong “vertical slice” through that architecture: workbook parsing, sparse grid IR, object‑graph and simple grid diffs, database mode keyed diff, DataMashup framing/metadata, and a first semantic M diff are all present and thoroughly tested.   At the same time, several spec’d capabilities are intentionally not yet wired together: Workbook IR doesn’t embed DataMashup, DiffOp doesn’t yet carry M / DAX changes, the unified grid diff algorithm is partially implemented (row/column alignment and block moves) but not yet the full AMR or Patience/Myers cascade, and database mode lacks duplicate‑key clustering.

Overall, this looks like a healthy, evolving architecture: the foundational IR and layering are solid, tests are rich and behavior‑driven, and the code largely speaks idiomatic Rust. Future work is mostly additive rather than corrective: filling in algorithmic sophistication, integrating M diff into the canonical DiffOp stream, and preparing the crate for PBIX/WASM by separating host‑level I/O from pure core logic.

---

## Dimension Evaluations

### 1. Architectural Integrity

**Assessment**: **Strong**

**Evidence**

* The spec’s layered architecture (host container → binary framing → semantic parsing → domain IR → diff) is reflected directly in modules:

  * `container.rs` defines `OpcContainer` and `ContainerError` for generic OPC/ZIP handling. 
  * `excel_open_xml.rs` consumes `OpcContainer` and grid/parser modules to produce a `Workbook`, and separately exposes `open_data_mashup` to retrieve `RawDataMashup`.
  * `datamashup_framing.rs` parses the MS‑QDEFF binary envelope into `RawDataMashup` (version, sections, permission blobs). 
  * `datamashup_package.rs` and `datamashup.rs` lift that into `DataMashup` and `Query` domain types.
  * `workbook.rs`, `grid_view.rs`, and `hashing.rs` provide IR and signatures for the grid diff engine.
  * `engine.rs` orchestrates object‑graph, grid, block‑move, and database‑mode diffs into a `DiffReport`.

* Dependencies flow “downward”:

  * The IR types (`Workbook`, `Grid`, `DataMashup`) do not depend on diff algorithms or container details; they live in `workbook.rs` and `datamashup.rs` with no references back to `engine.rs` or `excel_open_xml.rs`.
  * Diff algorithms (`row_alignment.rs`, `column_alignment.rs`, `database_alignment.rs`) depend only on `Grid`/`GridView` and hashing utilities, as specified in the unified grid diff document.

* The spec’s IR sketch for workbook/grid/query is closely mirrored in code:

  * `Workbook { sheets: Vec<Sheet> }`, `Sheet { name, kind, grid }`, `Grid { nrows, ncols, cells, row_signatures, col_signatures }`, `Cell { row, col, address, value, formula }`.
  * `DataMashup { version, package_parts, metadata, permissions, permission_bindings }` and `Query { name, section_member, expression_m, metadata }` match the spec’s model and invariants (e.g., metadata synthesis when missing).

* The diff pipeline description in the spec (object graph → database mode → grid → semantic diff) matches the shape of `engine::diff_workbooks`, `diff_grids_database_mode`, and `m_diff::diff_m_queries` as separate stages, even though they are not yet fully unified behind `DiffOp`.

**Notable divergences**

* Spec IR vs implementation: the spec’s `Workbook` includes optional `data_model` (DAX) and `mashup` fields, but the concrete `Workbook` struct currently contains only `sheets`.   M data is modeled as a separate `DataMashup` that is obtained via `open_data_mashup`, not attached to `Workbook`. 

* The spec shows `DiffOp` carrying M and DAX changes (`MQueryChange`, `DaxMeasureChange`), but the actual `DiffOp` enum currently covers only sheets, rows/columns, block moves, and cell edits.   M diff uses its own `MQueryDiff` stream.

**Recommendations**

1. **Unify workbook & mashup IR**: extend `Workbook` to optionally carry a `DataMashup` (and, later, `DataModel`), and update `open_workbook` to populate those from the container. This aligns the IR with the spec and makes it easier to diff the whole workbook as a single object graph.

2. **Integrate semantic diffs into `DiffOp`**: add `MQueryChange` (and eventually DAX variants) to `DiffOp` and expose a top‑level `diff_workbooks_with_m` that internally calls `diff_m_queries` and merges those changes into the main `DiffReport`.

3. Keep the current layering discipline as you expand: treat PBIX and DAX as new “domain” modules that plug into the existing container and diff layers, instead of piercing upwards into them.

---

### 2. Elegant Simplicity

**Assessment**: **Strong, with localized pockets of algorithmic density**

**Evidence**

* The core IR is minimal but expressive:

  * `Grid` is sparse (`HashMap<(u32,u32), Cell>`) with explicit `nrows`/`ncols`, and signatures are kept optional and computed on demand via `compute_all_signatures`.
  * `CellSnapshot` is purpose‑built for diffs: it stores address, value, formula, and equality deliberately ignores `addr`, which is captured at the `DiffOp::CellEdited` level.

* Grid diff logic is decomposed into composable pieces:

  * `GridView` describes row and column metadata (hashes, densities, non‑empty counts) and `HashStats` summarises uniqueness/repetition.
  * `row_alignment.rs` and `column_alignment.rs` focus on alignment of a single axis, and `rect_block_move.rs` handles rectangular block detection, rather than one monolithic “grid diff” function.

* The DataMashup pipeline is similarly clean:

  * `datamashup_framing` extracts versioned sections from the binary container, validating base64 and UTF‑16 variants.
  * `datamashup_package` deals with inner OPC packages and section sources. 
  * `build_data_mashup` and `build_queries` transform these into strongly‑typed domain structs, including synthesized metadata when missing and preserving section member order.

* The diff surface is deliberately small and well‑shaped:

  * `DiffOp` variants are tightly focused (sheet add/remove, row/column add/remove with signatures, block moves, cell edits), and `DiffReport` just adds a version field.
  * `output::json` exposes only three main helpers: `serialize_diff_report`, `serialize_cell_diffs`, `diff_workbooks_to_json`, keeping I/O and formatting orthogonal to core algorithms. 

**Where complexity leaks through**

* Row/column alignment and fuzzy block move detection are necessarily intricate and currently live in relatively large functions (`align_row_changes`, `detect_fuzzy_row_block_move`, etc.). The logic blends case handling (single row insert/remove, contiguous blocks) with heuristics driven by hash stats and size thresholds.

* Database mode diff (`diff_table_by_key`) is conceptually simple (keyed map join) but conflates key specification, duplication detection, and fallback error signalling in a single function.

**Recommendations**

1. **Refactor algorithmic “knots” into smaller named steps**
   Split `align_row_changes` and `detect_fuzzy_row_block_move` into clearly named phases, e.g.:

   * anchor discovery
   * gap classification
   * candidate similarity scoring
   * structural vs cell‑level change emission
     This would make the code read more like the unified grid algorithm spec and reduce the need for mental simulation.

2. **Centralize heuristic thresholds/config**
   Extract constants like `MAX_ALIGN_ROWS`, `MAX_FUZZY_BLOCK_ROWS`, and hash repetition cut‑offs into a small config struct or module. This both documents the heuristics and makes them tunable without spelunking through algorithms.

3. **Separate database alignment phases**
   Break `diff_table_by_key` into:

   * key extraction and validation (including duplicate detection),
   * row matching/join,
   * diff emission.
     This would clarify where future duplicate‑key clustering and fuzzy key matching will live.

---

### 3. Rust Idiomaticity

**Assessment**: **Adequate to Strong**

**Evidence**

* Ownership and borrowing are clear:

  * IR types own their data; diff functions receive `&Workbook`, `&Grid`, `&DataMashup` and avoid unnecessary `clone` except when building new maps or vectors.
  * Container and framing layers use owned `Vec<u8>` for binary data but pass `&[u8]` slices into parsers (`parse_data_mashup`, XML readers), which is idiomatic and efficient.

* Error handling embraces `Result` and layered error types:

  * `ContainerError`, `GridParseError`, `DataMashupError`, and `ExcelOpenError` form a clear hierarchy, with `From` conversions and `#[from]` usage via `thiserror`.
  * `ExcelOpenError` is marked `#[non_exhaustive]`, which is a nice forward‑compatibility touch. 

* Types encode useful invariants:

  * `CellAddress` is a dedicated type with A1 parsing and formatting, and `AddressParseError` includes the offending string. Tests assert error messages mention the invalid address.
  * `RowSignature`/`ColSignature` are newtypes over hash values, used in `DiffOp` with `skip_serializing_if` so optional signatures don’t clutter JSON.

* Trait use is purposeful:

  * The code relies primarily on standard traits (`FromStr`, `Serialize`, `Deserialize`, `Error`), using them to integrate with parsing and serde rather than inventing heavy custom trait hierarchies.

**Limitations**

* The “Diffable” abstraction described in the spec (trait implemented by types emitting `DiffOp` streams) has not been introduced. Instead, diff logic is organized around free functions (`diff_workbooks`, `diff_grids_database_mode`, `diff_m_queries`).

* Some code fragments (as rendered in the markdown) show minor style issues (very long functions, combined responsibilities) that could benefit from more granular helpers, but these are more about structure than idiomatic Rust per se.

**Recommendations**

1. **Consider a light‑weight `Diffable` trait only if it reduces coupling**
   For example:

   ```rust
   trait Diffable<Rhs> {
       type Diff;
       fn diff(&self, other: &Rhs) -> Self::Diff;
   }
   ```

   Implemented for `(Workbook, Workbook)`, `(Grid, Grid)`, `(DataMashup, DataMashup)` returning `Vec<DiffOp>` or specialized diff structs. This should be introduced only when there is a clear call‑site benefit; the current free‑function setup is acceptable.

2. **Keep error enums `#[non_exhaustive]` where they cross crate boundaries**
   `ExcelOpenError` already does this; consider doing the same for `DataMashupError` if it’s part of the public API.

3. **Gradually shrink multi‑page functions**
   Refactor alignment and diff orchestrators into smaller, named helper functions; this makes pattern matching clearer and keeps control flow more idiomatic (`match` over small enums vs nested branching).

---

### 4. Maintainability Posture

**Assessment**: **Strong**

**Evidence**

* Module organisation mirrors the conceptual architecture:

  * `workbook.rs`, `addressing.rs`, `hashing.rs`: IR and cross‑cutting primitives.
  * `grid_parser.rs`, `grid_view.rs`, `row_alignment.rs`, `column_alignment.rs`, `database_alignment.rs`, `rect_block_move.rs`: grid processing and diff algorithms.
  * `datamashup_*`, `m_ast.rs`, `m_section.rs`, `m_diff.rs`: M‑language pipeline.
  * `excel_open_xml.rs`, `container.rs`: host/container specifics.

* Tests are extremely well‑structured and serve as living documentation:

  * File names and test names (`pg1_ir_tests`, `pg4_diffop_tests`, `g8_row_alignment_grid_workbook_tests`, `m7_semantic_m_diff_tests`) line up with the testing plan’s phases and scenario labels (G1–G13, D1, M6/M7).
  * Many tests assert high‑level behavior (“no structural row/column ops expected in PG6.4”, “database mode should ignore row order when keyed rows are identical”) rather than internal implementation details, which leaves room to evolve algorithms without rewriting tests.

* Change isolation is good:

  * The M pipeline (`datamashup_*`, `m_ast`, `m_diff`) is almost entirely decoupled from grid diff; you could re‑implement the M parser or AST canonicalization without touching grid code.
  * Likewise, grid alignment algorithms rely on `Grid` and `GridView` signatures; as long as those interfaces remain stable, internal alignment strategies can be swapped out.

* Naming and vocabulary are consistent and domain‑appropriate:

  * “Grid”, “RowAlignment”, “BlockMove”, “DataMashup”, “QueryMetadata”, “SectionMember” all match the specification and documentation, bridging the mental model between docs and code.

* The meta‑programming document describes a deliberate process (planner/implementer/reviewer roles, phased milestones, “golden” fixture sets) that is clearly visible in the code’s test structure and commit organization implied by the phases.

**Recommendations**

1. **Document key invariants inline for complex modules**
   For example, at the top of `row_alignment.rs` and `database_alignment.rs`, spell out the expected complexity bounds, size limits (max rows/columns), and behavior when constraints are exceeded. This gives future maintainers a local rationale for the current heuristics.

2. **Add brief “module overviews” in doc comments where missing**
   `grid_view.rs` and `database_alignment.rs` would benefit from a short prose summary of how they relate back to the unified grid diff algorithm spec and the testing plan.

3. **Preserve the phase labels in tests as the system grows**
   Continue to map new features/tests to explicit milestones; this gives maintainers a temporal as well as structural orientation (“PG7 was where X landed, these are the invariants from that phase”).

---

### 5. Pattern Appropriateness

**Assessment**: **Strong**

**Evidence**

* The architecture favors simple data‑plus‑function patterns rather than heavy abstraction:

  * Domain types (`Workbook`, `Grid`, `DataMashup`, `Query`) are plain structs. Algorithms are free functions in focused modules. This fits Rust’s strengths and avoids over‑engineering.

* Diff representation is a textbook “event stream”:

  * `DiffOp` as a tagged enum and `DiffReport` as a versioned list matches the spec and testing plan’s description of the diff layer. It is the right amount of structure for frontends and CLI consumers.

* View/adapter pattern for grid alignment:

  * `GridView` encapsulates per‑row/per‑column metrics and hash stats without mutating the underlying `Grid`. This is a clean separation between storage and algorithm view, exactly as the unified grid spec suggests.

* Error types reflect layers rather than generic “one big error”:

  * `ExcelOpenError` aggregates container, grid parse, and DataMashup errors while exposing domain‑specific variants like `WorkbookXmlMissing` and `WorksheetXmlMissing { sheet_name }`, which are meaningful to callers. 

* Query diff pattern is deliberately simple:

  * `diff_m_queries` aligns queries by `name` and distinguishes `DefinitionChanged` vs `MetadataChangedOnly` using AST canonicalization and metadata equality. This matches the testing plan’s staged approach (first textual, then semantic diff).

**Recommendations**

1. **Extend the diff “event stream” pattern to M and (later) DAX**
   Move from `MQueryDiff` to a `DiffOp::MQueryChange` variant with appropriate payload. This unifies downstream consumption and keeps the same “events over entities” pattern.

2. **Keep polymorphism concrete where possible**
   As you add new algorithms (e.g., multiple grid strategies, different database key strategies), prefer explicit function composition or small enums over trait objects unless dynamic dispatch truly buys modularity. The current design is nicely concrete; maintaining that will keep performance and readability high.

---

### 6. Performance Awareness

**Assessment**: **Adequate, with solid foundations and room to grow**

**Evidence**

* IR choices support large workbooks:

  * Sparse storage for grids avoids allocating full `nrows * ncols` matrices; only non‑empty cells are stored. This is exactly what you want for 100MB files with large but sparse sheets.
  * Pre‑computed row/column signatures let alignment work in O(R + C) for many cases rather than walking all cells repeatedly.

* Hash‑based heuristics avoid worst‑case behavior:

  * `HashStats` tracks how often each row/column hash occurs; unique hashes are used as anchors and repeated hashes trigger “fallback” behaviors.
  * Row and column alignment impose explicit maximums (e.g., number of rows/columns) for running the more advanced heuristics, falling back to simpler (but slower or more conservative) strategies otherwise.

* Database mode is O(N):

  * `diff_table_by_key` builds keyed maps from the specified key column(s), compares rows by key, and emits diffs ignoring row order. This is linear in the number of rows plus hash map overhead.

* The testing plan explicitly calls out performance and fuzzing milestones (e.g., “100MB under 2s”, DataMashup fuzzing, and golden test sets), which shows performance has been considered architecturally, even if explicit benchmarks are not yet wired into the code.

**Gaps vs spec**

* The unified grid diff spec describes a more sophisticated “Anchor–Move–Refine” pipeline (including Patience diff, Myers diff, and LIS‑based block detection) with tunable cost budgets and streaming‑friendly behavior. The current implementation covers anchors, row/column block moves, and some fuzzy similarity, but the full cascade and cost‑budgeting are not yet present.

* Container and parser layers are still largely “load everything into memory”:

  * `OpcContainer::open` and `open_workbook` use `ZipArchive` and read full XML parts into memory before parsing. There is no streaming of grid rows or M sections yet.

* There is no visible benchmark suite in the Rust tests, nor integration with the performance metrics described in the meta‑programming/testing docs (e.g., no `cargo bench` harness or perf fixtures wired into CI).

**Recommendations**

1. **Incrementally align the grid engine with the full AMR pipeline**
   Introduce a well‑factored representation of alignment phases and incorporate a simple LIS/Patience implementation where the spec calls for it. Keep the current fast‑path heuristics and add a cost budget to bail out on degenerate cases.

2. **Add a dedicated benchmark suite**
   Use `criterion` or similar to benchmark:

   * large sparse grids with modest changes,
   * wide vs tall sheets,
   * many repeated rows/columns.
     Anchor these scenarios to the “100MB under 2s” product objective to guide optimization work.

3. **Prepare for streaming**
   While full streaming XLSX parsing is a larger project, you can:

   * Ensure diff algorithms operate on views/iterators rather than requiring fully materialized vectors where possible.
   * Introduce simple row iterators from the parser that could, later, be fed from a streaming XML reader.

---

### 7. Future Readiness

**Assessment**: **Adequate, with strong algorithmic extensibility and moderate platform readiness**

**Evidence**

* Algorithmic and domain extensibility:

  * The spec clearly anticipates DAX, tables, and PBIX. The existing grid and M diff engines are structured so that additional domains can plug into the same pattern (parse to IR → diff → emit `DiffOp`‑like events).
  * `DiffOp` is `#[non_exhaustive]`, signaling planned extension for new variants. 
  * `QueryChangeKind` already includes a `Renamed` variant that is not yet emitted, giving a forward‑compatibility hook for future rename detection.

* Product roadmap alignment:

  * The product differentiation plan mentions CLI, Git‑friendly JSON, WASM/web, and PBIX support. The current design already exposes a CLI‑friendly JSON contract (`DiffReport`, `CellDiff`) and keeps the core diff engine independent of presentation concerns, which is exactly what you want before adding additional frontends.

* WASM potential:

  * Core diff algorithms and IR are free of OS‑specific APIs; the only strongly host‑bound pieces are `excel_open_xml.rs` and `container.rs` (file I/O and `ZipArchive` usage). In a WASM world, those would naturally be replaced with “open from bytes” APIs while reusing the same IR and diff engine.

**Limitations**

* The missing `mashup`/`data_model` fields on `Workbook` mean there isn’t yet a single “workbook diff” that can carry grid + M + data model changes in one `DiffReport`. Integrating M and DAX later will require some IR and API evolution, though not a full rewrite.

* WASM/web builds will require a careful separation of “pure core” vs “host I/O” crates. Right now everything sits inside one crate; refactoring into `excel_diff_core` (IR+diff) plus `excel_diff_open_xml` (host/container) would simplify cross‑platform work.

**Recommendations**

1. **Plan an IR/API evolution for combined workbook diffs**

   * Add optional `mashup` (and later `data_model`) to `Workbook`.
   * Introduce a new `DiffOp` variant for M queries.
   * Add a “v2” schema version to `DiffReport` once M/DAX diffs become part of the core stream.

2. **Split core vs host crates**

   * Extract IR (`workbook`, `datamashup`), hashing, grid algorithms, `DiffOp`, and `engine` into a `core` crate with no direct file I/O.
   * Keep `excel_open_xml`, `container`, and fixture‑related helpers in a host‑specific crate that depends on `zip`, `std::fs`, etc.
     This refactor will ease WASM and PBIX support later without disturbing core logic.

3. **Design PBIX support as another “container + domain” pair**
   Treat PBIX as:

   * a new `pbix_container` layer parallel to `excel_open_xml`,
   * a `DataModel` IR module mirroring the M pipeline’s structure,
   * a separate set of tests analogous to `data_mashup_tests.rs` and `m*_tests.rs`.

---

## Tensions and Trade-offs

1. **Generality vs near‑term value in grid diff**
   The unified grid spec describes a rich, general alignment strategy with Patience/Myers, LIS, and cost budgets, but the current implementation focuses on the subset required by G1–G13 tests: exact and fuzzy row/column moves and simple structural edits.

   * This is a pragmatic choice: it delivers visible value and supports product demos without committing too early to complex heuristics.
   * The tension is that some “hard” real‑world spreadsheets will still fall back to conservative behavior (many `CellEdited` ops, fewer structural ops) until the fuller pipeline is implemented.

2. **Single cohesive IR vs decoupled M pipeline**
   Keeping `DataMashup` separate from `Workbook` made it easier to build and test the M pipeline independently, especially given its high difficulty rating in the difficulty analysis.

   * This decoupling is great for iteration speed, but later you’ll want a unified view so users can see “sheet + query + data model” changes together. That will require some API surgery.

3. **Strict correctness vs permissive parsing**
   DataMashup permissions and metadata parsing errs on the side of safety: malformed or missing permissions yield defaults, duplicate DataMashup parts are rejected, and metadata length/UTF‑8 issues become errors.

   * This reduces ambiguity and security risk, at the cost of sometimes refusing to diff corrupted but “used in the wild” files. The testing and meta‑programming docs acknowledge this, proposing fuzzing and fixture‑driven coverage for oddball producers (LibreOffice, POI).

---

## Areas of Excellence

1. **IR design for workbooks and grids**
   The `Workbook`/`Sheet`/`Grid`/`Cell` model is crisp and minimal, yet it encodes all the information needed for diffing and hashing. Invariants are documented and reinforced in tests, and `CellSnapshot` is a particularly elegant construct that decouples logical content from address.

2. **DataMashup framing and metadata handling**
   The combination of `datamashup_framing`, `datamashup_package`, and `build_data_mashup` is a clean embodiment of the spec’s guidance for QDEFF parsing. The tests cover base64 whitespace, UTF‑16 variants, duplicate parts, malformed permissions, invalid metadata length prefixes, and orphan entries, making this subsystem feel robust and “done.”

3. **Testing discipline and fixtures**
   The Python fixture generators and the Rust tests form a powerful combination: you can see each spec section reflected in concrete generated workbooks and targeted assertions. Files like `g1_g2_grid_workbook_tests.rs`, `d1_database_mode_tests.rs`, `m6_textual_m_diff_tests.rs`, and `m7_semantic_m_diff_tests.rs` read almost like executable chapters of the architecture document.

4. **DiffOp contract and JSON surface**
   `DiffOp`, `DiffReport`, and JSON serialization tests (PG4) lock down the wire format early, as recommended in the testing plan. The helper `diff_report_to_cell_diffs` and the JSON round‑trip tests ensure the CLI and any future frontends have a stable, documented contract.

---

## Priority Recommendations

1. **Integrate M diff into the canonical diff stream**

   * Extend `DiffOp` with an M‑specific variant and introduce a “v2” `DiffReport` schema when needed.
   * Attach `DataMashup` to `Workbook` and add a top‑level entry point that diffs both grid and mashup into a single report.

2. **Progress the grid diff engine toward the full unified algorithm**

   * Refactor row/column alignment into explicit AMR phases and add LIS/Patience diff where the spec calls for it.
   * Introduce explicit cost budgets and richer `HashStats`‑driven decisions for large/degenerate sheets.

3. **Strengthen database mode for duplicate keys and partial keys**

   * Break `diff_table_by_key` into separate phases and add configurable behavior for duplicate keys (cluster‑based alignment, configurable error vs best‑effort).
   * Expand tests to cover duplicate‑key scenarios and large tables, guided by the difficulty analysis.

4. **Refactor into “core” vs “host” crates**

   * Move IR, hashing, alignment, and `DiffOp` into a `core` crate; keep `excel_open_xml` and `container` in a host‑bound crate.
   * This will make WASM and PBIX support much easier and clarifies responsibilities.

5. **Introduce benchmarks and fuzzing**

   * Implement the DataMashup fuzzing and grid diff perf scenarios described in the testing plan, making them part of CI or at least a documented workflow.

6. **Improve algorithm readability with phase‑level documentation**

   * Add concise comments and documentation at the top of key algorithm modules describing their phases and complexity guarantees, referencing the unified grid diff specification where appropriate.

---

## Conclusion

The Excel Diff Engine is architecturally sound and thoughtfully aligned with its specifications and product vision. The foundational IR is well‑chosen, the layered parsing and diffing pipeline is clear, and the tests provide a rich behavioral map of the system. Complexity has been concentrated where the domain demands it—Excel’s container formats, DataMashup framing, and 2D grid alignment—while the surrounding structures remain simple and comprehensible.

The main work ahead is integrative and incremental rather than corrective: unifying grid and M diffs into a single diff stream, fleshing out the full unified grid algorithm, strengthening database mode for edge cases, and preparing the codebase for PBIX and WASM. If those steps are taken with the same discipline already evident in the IR design and testing strategy, this project is well‑positioned to become a high‑quality, extensible diff engine for the broader Excel/Power BI ecosystem.
