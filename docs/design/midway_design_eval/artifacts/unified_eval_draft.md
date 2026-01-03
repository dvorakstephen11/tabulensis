# Design Evaluation Report: Excel Diff Engine

## Executive Summary

The Excel Diff Engine has the shape of a serious product: a clean vertical stack from host container parsing, through binary framing (`MS‑QDEFF`), into a small, coherent domain IR (`Workbook`, `Grid`, `DataMashup`, `Query`), and out to a stable `DiffOp`-based result surface. The **Grid / GridView** memory split is particularly strong, reflecting the difficulty analysis: the hardest part of the system is the high‑performance 2D grid diff engine, and the architecture correctly concentrates sophistication there.

However, the **algorithmic core of the grid diff** is not yet aligned with the specification and, in key places, is functionally incomplete for common editing patterns. The implementation uses a *heuristic waterfall* (`detect_rect` → row/column block moves → row alignment) instead of the **Anchor–Move–Refine (AMR)** pipeline with run‑length compression and global anchoring described in the unified grid diff spec. Critically, this waterfall is **one‑shot**: if a rect move is detected, the function emits that single move and returns immediately; if a row block move is detected, it emits one move and then falls through to alignment—meaning the engine is fundamentally incapable of describing more than one block move per sheet. Worse, this early return causes **silent data loss**: any edits outside the detected move region are never computed or reported. For example, if a rectangular block is moved and a cell outside that block is edited, the engine emits the move and exits; the cell edit is simply dropped. Additionally, the row‑alignment stage itself is limited to "single‑gap" alignments that can represent at most one contiguous insertion/deletion block. When a sheet contains multiple disjoint edit regions (for example, rows inserted near both the top and the bottom), the alignment fails and the engine falls back to a coarse positional diff. Combined with the absence of the spec‑mandated run‑length encoding (RLE) path for heavily repetitive grids, this forces hard safety caps (`MAX_ALIGN_ROWS = 2_000`, `MAX_ALIGN_COLS = 64`, `MAX_BLOCK_GAP = 32`) inside row alignment. Against the documented target of 50,000+ row sheets with strict performance bounds, that combination effectively classifies the current engine as a **restricted prototype**: architecturally sound but both **scale‑limited** and **semantically noisy** on the workloads it is meant to handle.

Beyond the grid algorithm itself, several systemic issues limit readiness:

* The top‑level API fractures the domain into a Grid path (`Workbook`) and a Query path (`DataMashup`), forcing callers to open and diff files twice and merge results manually.
* The core crate's container layer depends on `std::fs::File`, which prevents compiling the current entry points to WASM but is a small I/O abstraction gap (switching to `Read + Seek`) rather than a deep architectural flaw.
* The engine materializes the entire `DiffReport` as an in‑memory `Vec<DiffOp>`, globally sorts it for deterministic output, and then serializes it to a single JSON string. At the same time, the workbook IR stores text cells as `CellValue::Text(String)` and sheet identifiers as `String` in every diff operation, instead of using the string‑interning model described in the spec. On large, repetitive sheets (for example, a 50,000‑row column of identical status values), this combination can exhaust a browser WASM heap during parsing or result assembly long before alignment has a chance to run.
* Critical algorithm thresholds (e.g., fuzzy similarity, row caps) are hardcoded rather than flowing from a configurable `DiffConfig`.
* Row hashes incorporate the **absolute column index** in their computation (`hash_cell_contribution(col, cell)`), so inserting a single column at position 0 shifts all data and invalidates every row hash in the sheet, causing alignment to fail completely. This architectural coupling means the engine cannot successfully align rows when columns have been inserted or deleted.
* Number hashing uses `f64::to_bits()`, which distinguishes `0.0` from `−0.0` and is sensitive to epsilon drift (e.g., `1.0` vs. `1.0000000000000002`). Excel recalculations routinely cause tiny bit‑level changes; the engine will report these as spurious cell edits or fail to align otherwise‑identical rows.

Taken together, the picture is:

> **A robust, well‑designed chassis (IR, layering, memory model) currently powered by an algorithmic engine that is deliberately capped and must be upgraded early to meet the product’s non‑negotiable performance and robustness goals.**

The rest of this report treats that algorithmic maturity as the central gating issue and structures recommendations accordingly.

---

## Dimension Evaluations

### 1. Architectural Integrity

**Assessment:** **Structurally strong, with a top‑level integration gap**

**Evidence**

* **Vertical layering is disciplined and matches the spec.** The path from bytes to IR is cleanly staged:

  * *Host / container layer*: ZIP/OPC handling and Excel Open XML parsing are confined to container and `excel_open_xml` modules.
  * *Binary framing*: DataMashup’s `MS‑QDEFF` layout (version + four length‑prefixed sections) is handled in framing logic that slices the stream into package parts, permissions, metadata, and bindings with explicit invariants.
  * *Domain IR*: `Workbook`, `Sheet`, `Grid`, `DataMashup`, and `Query` are simple, well‑typed models of the domain, matching the IR design in the spec. 
  * *Diff*: `engine::diff_workbooks` orchestrates object‑graph diff, grid diff, and database mode (keyed) diff over those IR types, then feeds into `DiffOp` and JSON helpers.
    Lower layers do not depend on higher ones; the direction of knowledge is correct.

* **Complexity is concentrated in the right place.** The difficulty analysis identifies the high‑performance 2D grid diff (row/column alignment, moves, DB mode) as the hardest problem (score 18/20), followed by streaming parsing and semantic M diff.  The implementation reflects this: container and framing code are straightforward, while `GridView`, hashing, and alignment logic hold most of the complexity.

* **IR is coherent and domain‑centric.** `DiffOp` encodes semantic operations (sheet add/remove, row/column add/remove, row/column/rect moves, cell edits) rather than a low‑level edit script, closely mirroring the DiffOp taxonomy laid out in the unified grid spec.  M diff likewise operates on `Query` IR, not raw strings.

* **Horizontal integration gap at the package level.** There is no unified `WorkbookPackage` abstraction that models "an Excel file" as users understand it. Instead, consumers must call `open_workbook` and `open_data_mashup` separately, then invoke grid diff and M diff independently and merge results themselves. Database mode is exposed as a separate, opt‑in entry point (`diff_grids_database_mode`) that is exported publicly but never called automatically by `diff_workbooks`—there is no key detection or auto‑switching, so users must explicitly choose database mode. This breaks the illusion of a single, coherent engine and increases surface area.

**Recommendations**

1. **Introduce a first‑class `WorkbookPackage` domain type.**
   This struct should own both the `Workbook` (grid + object graph) and `DataMashup` (queries, metadata), potentially plus later DAX data model. A single `open_package` function should parse all relevant streams once and return this unified object.

2. **Unify diff orchestration behind the package abstraction.**
   Provide a `diff(&self, other: &WorkbookPackage, config: &DiffConfig) -> DiffReport` method that internally orchestrates:

   * object‑graph diff (sheets, tables, named ranges),
   * spreadsheet‑mode diff for non‑keyed regions,
   * database‑mode diff for keyed regions,
   * M query diff,
   * future DAX/model diff.

3. **Make “mode decision” explicit and testable.**
   For mixed sheets (keyed table region + free‑form cells), the spec expects region segmentation and different diff modes within the same sheet. Represent those regions and chosen modes explicitly in IR so they are visible in tests and easy to reason about.

---

### 2. Elegant Simplicity

**Assessment:** **High abstraction fidelity, with localized algorithmic knots**

**Evidence**

* **IR and snapshots are simple and expressive.** The separation of cell identity from cell content (`CellSnapshot` with value + formula) allows the engine to compare content independently of positional shifts, while still preserving enough detail for semantic classification (value vs. type vs. formula changes) per the spec.

* **Diff logic reads as a narrative pipeline.** The grid diff entry point is structured as a sequence of increasingly general strategies:
  exact rectangular block moves → row/column block moves → fuzzy block moves → row alignment → fallback positional diff. This makes the control flow easy to follow and matches how humans would think about “what changed” from the outside.

* **M diff semantics are “explained by tests.”** The combination of M diff tests (textual vs AST vs semantic) practically documents the semantics of `MQueryDiff`: formatting‑only changes are suppressed, metadata‑only changes are isolated, and true semantic changes are flagged as definition changes with AST canonicalization to avoid noise.

* **Local complexity in row alignment hides a functional limitation.** The row alignment implementation is dense: manual index juggling, nested conditionals, and ad‑hoc guardrails. That complexity doesn't come from the problem domain alone; it encodes a "single‑gap only" model where the alignment routines can describe at most one contiguous insertion/deletion block. As soon as there are multiple disjoint edit regions, alignment returns `None` and the engine falls back to positional diff, producing long runs of row additions/removals instead of a clean alignment. This is not just heuristic tuning; it is a capability gap relative to the specification's general sequence‑alignment requirement.

* **Output surface is fragmented.** Grid changes and M query changes are reported via separate types/streams (`DiffOp` vs `MQueryDiff`), forcing consumers to multiplex two logs and reason about ordering themselves. This undermines the otherwise simple story at the IR level.

**Recommendations**

1. **Refactor high‑complexity functions into named phases.**
   Functions such as fuzzy block move detection and row alignment should be decomposed into helpers that mirror spec phases: e.g., `collect_row_meta`, `discover_anchors`, `extract_move_candidates`, `fill_gaps`, `assemble_alignment`. This will make it feasible to swap in AMR without destabilizing the rest of the engine.

2. **Unify output into a single diff stream.**
   Extend `DiffOp` to cover M query events (`MQueryAdded`, `MQueryDefinitionChanged`, etc.) and emit a single ordered stream per workbook pair. Keep dedicated helpers (e.g. `diff_report_to_cell_diffs`, `diff_report_to_m_diffs`) as projections, not separate engines.

3. **Tie code phases explicitly to spec sections.**
   Use doc comments on key algorithms (“Implements Section 30.3 AMR row alignment”) to connect code to specification concepts. This improves legibility and lets new contributors navigate by spec, not by archeology.

---

### 3. Rust Idiomaticity

**Assessment:** **Elite memory and error modeling; I/O abstraction and WASM footprint need cleanup**

**Evidence**

* **Ownership and borrowing are clean.** The engine passes references into heavy structures (`&Sheet`, `&Grid`) and uses views (`GridView<'a>`) that borrow underlying data rather than copying it. This achieves zero‑copy analysis over large grids while keeping lifetimes explicit and safe.

* **Error handling is idiomatic and layered.** The spec explicitly defines layered errors (`ExcelOpenError` wrapping `ContainerError`, `GridParseError`, `DataMashupError`), and the implementation follows that model with `thiserror` enums carrying domain‑specific detail (invalid addresses, truncated MS‑QDEFF sections, bad BOMs, XML errors).

* **`GridView` is an exemplar of idiomatic zero‑copy design.** It reuses underlying `Grid` storage, adds light metadata (row hashes, non‑blank counts), and enables hashing, sorting, and alignment without cloning cell contents, matching the spec's emphasis on sparse representation and metadata‑driven alignment. However, there is a mild **IR layering violation**: `Grid` itself holds optional `row_signatures` and `col_signatures` fields, which are algorithm caches rather than pure domain data. The spec expects `Grid` to be immutable data and `GridView` to hold ephemeral analysis state; currently the fields are unused (since `GridView::from_grid` recomputes everything), but they should be removed to preserve the separation.

* **Cell coordinate storage is triply redundant.** The `Cell` struct stores its position three times: once in the `HashMap<(u32, u32), Cell>` key, once in `Cell.row`/`Cell.col` fields, and once in the nested `Cell.address.row`/`Cell.address.col`. This wastes 16 bytes per cell (the key is unavoidable, but either the flat fields or the address struct could be removed). On a 100K‑cell sheet, this adds ~1.6MB of redundant coordinate data, compounding the memory pressure discussed elsewhere.

* **Core crate currently depends on `std::fs::File`.** The container layer exposes an `open` API that takes a path and internally uses `File`. That's natural for a desktop‑only tool but conflicts with the documented requirement that the core diff engine compile to WASM and accept data via byte buffers or generic streams. Notably, the crate already has feature‑flag scaffolding (`#[cfg(feature = "excel-open-xml")]`) that gates the file‑system‑dependent code, so the core diff algorithms and IR are already decoupled from I/O. The remaining fix is a straightforward signature change (for example, taking `R: Read + Seek`) plus wiring up the existing feature gate, rather than a structural rewrite.

* **Heavy parsing dependencies will matter for WASM size.** Using `quick-xml`, `zip`, and `serde_json` is reasonable for a native CLI but will dominate the binary size of a browser WASM target; they should be measured and, if needed, feature‑gated or swapped for lighter alternatives on that platform.

* **Edition 2024 Rust may affect build stability.** The crate uses `edition = "2024"`, which is not yet stable and requires nightly Rust. For WASM compilation and CI stability goals, this dependency on nightly features is a risk; pinning to edition 2021 may be necessary until 2024 stabilizes.

* **String handling is not yet aligned with the spec's interning model.** `SheetId` is currently a `String`, which gets cloned into every diff operation referencing that sheet, and `CellValue::Text(String)` stores a separate heap allocation per text cell. For large, repetitive sheets (for example, a status column with 50,000 `"Pending"` values), this contradicts the spec's `StringPool`/`StringId` design and risks exhausting WASM memory unless interning is introduced.

**Recommendations**

1. **Abstract I/O behind `Read + Seek`, and isolate `std::fs` behind feature flags.**

   * Refactor `OpcContainer` construction to accept `R: Read + Seek` (or similar) so the core library is oblivious to the source of bytes.
   * Provide convenience constructors in a small host‑specific wrapper crate (`open_from_path(path: &Path)`) that live outside the WASM‑targeted core.
   * Treat this as early tech‑debt cleanup rather than as a major architectural project.

2. **Adopt string interning for sheet names and cell strings.**
   Implement the `StringPool` design from the spec and replace `SheetId = String` with `SheetId = u32` or a newtype, so both IR and DiffOps store compact IDs rather than repeated heap strings. This yields large memory savings and makes equality checks O(1) integer compares.

3. **Eliminate coordinate redundancy in the `Cell` struct.**
   Remove either the `row`/`col` fields or the `address` struct from `Cell`, since coordinates are already stored in the `HashMap` key. If `CellAddress` is needed for downstream consumers, derive it on demand from the key rather than storing it redundantly.

4. **Keep panics and `unwrap` strictly out of critical paths.**
   The existing discipline here is good; maintain it as the algorithm becomes more complex by adding property‑based and fuzz tests around new AMR/RLE logic.

5. **Track and constrain WASM binary size.**
   * Add a WASM profile in CI that builds the core crate, reports binary size, and fails if it exceeds agreed thresholds; use this to drive decisions about optional dependencies (for example, `serde`, XML/ZIP crates) for the browser target.

---

### 4. Maintainability Posture

**Assessment:** **Strong test culture; configurability and algorithm clarity need attention**

**Evidence**

* **Tests and fixtures act as executable specifications.**
  The Python fixture generator and the broad `fixtures/` catalog encode real scenarios: simple edits, row/column block moves, fuzzy block moves, database mode behaviors, and a wide range of M/DataMashup edge cases. Test names and fixture naming conventions mirror the testing plan sections (`G8`–`G13`, `D1`–`D3`, `M6`–`M7`, etc.), giving maintainers a map of supported behaviors. Notably, the manifest already defines 50,000‑row performance fixtures (`p1_large_dense`, `p2_large_noise`), so the testing infrastructure supports scale testing—the bottleneck is purely algorithmic bailout, not test tooling.

* **M front‑end still targets a narrow language subset.** The M parser handles `let ... in` expressions including nested lets (via `let_depth_in_value` tracking during parse), but non‑`let` top‑level expressions—direct record/list literals and other shapes—fall back to opaque token sequences rather than full ASTs. Critically, the `canonicalize_tokens` function is explicitly a no‑op for these `Sequence` nodes (the function body is `let _ = tokens;`), so semantic equivalence checking for non‑`let` expressions reduces to exact token equality. This makes semantic M diffing fragile on real‑world queries that don't follow the `let ... in` pattern until coverage is expanded.

* **Module boundaries are reasonably clean.**
  Parsing and IR construction live in dedicated modules; diff logic is in `engine`, `row_alignment`, `column_alignment`, `rect_block_move`, and `database_alignment`; M parsing and diff live in `m_ast`, `m_section`, `datamashup`, and `m_diff`. That allows focused work within each subsystem.

* **Hard‑coded algorithm constants undermine maintainability.**
  Values like `MAX_ALIGN_ROWS = 2_000`, `MAX_BLOCK_GAP`, and `FUZZY_SIMILARITY_THRESHOLD = 0.80` are embedded as `const` in alignment code. That makes it impossible to tune behavior (e.g., stricter move similarity, lower block size for safety) without code changes, and it diverges from the specification’s assumption of a `DiffConfig` object controlling thresholds.

* **Row alignment code is difficult to safely modify or generalize.**
  Because the alignment logic is a hand‑rolled loop with many interdependent conditions and shared mutable indices, making even small changes (e.g., adjusting an early‑bail condition) risks subtle regressions. The current implementation also bakes in the "single‑gap" assumption (prefix/suffix matches around one edit block), so adding support for multiple disjoint insert/delete regions effectively requires a rewrite toward the phased AMR structure (anchors vs. moves vs. gap handling) rather than incremental tweaks.

**Recommendations**

1. **Introduce a `DiffConfig` struct and thread it through the engine.**

   * Centralize alignment thresholds, move similarity thresholds, bail‑out limits, and cost caps in a single configuration type.
   * Pass `&DiffConfig` to all diff functions, and remove hard‑coded numeric constants from core logic.

2. **Refactor alignment into phase‑specific modules or helpers.**
   Mirror the AMR structure: `anchor_discovery`, `move_extraction`, `gap_strategy_selection`, `alignment_assembly`. Each helper should have narrow responsibilities and unit tests.

3. **Align tests with the final algorithmic model.**
   The existing grid tests are strong; as AMR is implemented, extend them to explicitly assert behavior in the spec’s adversarial and scalability scenarios (e.g., repetitive 50K‑row sheets, “99% blank + inserted block”) and treat those as gatekeepers.

---

### 5. Pattern Appropriateness

**Assessment:** **Sound use of domain patterns; missing a few key abstractions**

**Evidence**

* **Domain‑first modeling.**
  The architecture favors domain types (`Workbook`, `Sheet`, `Grid`, `Query`, `DiffOp`) over generic frameworks. That keeps the system honest about what it’s actually doing and avoids over‑engineering.

* **Error enums express real failure modes.**
  `ExcelOpenError`, `ContainerError`, `DataMashupError`, and M‑specific errors encode understandings like “future MS‑QDEFF version”, “invalid shared string index”, “invalid section member syntax,” etc., rather than collapsing everything into a generic error.

* **Diff representation is a good anti‑corruption layer.**
  `DiffOp` serves as the single, flat enumeration of interesting changes, shielding downstream consumers from the complexities of internal alignment structures and algorithm choices.

* **Missing abstraction for “diffable” entities.**
  Right now, `diff_workbooks` knows about many kinds of work (object graph diff, sheet matching, grid diff, DB mode diff, M diff). That orchestration logic could be simplified with a `Diffable` trait implemented for `Workbook`, `Sheet`, `Grid`, and `Query`. This would make each domain’s diff behavior explicit and reusable.

* **No explicit streaming/output pattern.**
  The spec calls for multiple output formats, including streaming JSON Lines for large diffs. Today the engine always collects a full `DiffReport` in memory and then calls `serde_json::to_string` on it; this monolithic pattern both hides the intended streaming contract and risks memory spikes on very large diffs.

**Recommendations**

1. **Define and implement a `Diffable` trait.**

   ```rust
   pub trait Diffable {
       type Diff;
       fn diff(&self, other: &Self, config: &DiffConfig) -> Self::Diff;
   }
   ```

   Implement this for `Workbook`, `Sheet`, `Grid`, `Query`, and later `DataModel`. `diff_workbooks` should become a thin orchestrator assembling these diffs into a unified `DiffReport`.

2. **Adopt an explicit streaming pattern for output.**
   Implement JSON Lines output as specified (metadata + summary + one operation per line) and provide a streaming serializer that writes operations as they are produced, rather than buffering the entire result.

3. **Encapsulate configuration via builder or profile pattern.**
   Provide ergonomic constructors for `DiffConfig` (e.g., `DiffConfig::fastest()`, `DiffConfig::balanced()`, `DiffConfig::most_precise()`) instead of scattering tuning knobs across the API surface.

---

### 6. Performance Awareness

**Assessment:** **Safety‑conscious but currently non‑scalable; algorithmic upgrade is a release blocker**

**Evidence**

* **Specification sets clear, demanding targets.**
  The unified grid spec calls for near‑linear or O(N log N) behavior on 50K×100 grids, with explicit performance budgets for identical grids, scattered edits, block operations, heavy edits, adversarial repetitive data, and totally different sheets.

* **AMR plus strong defenses against repetition is the designed solution.**
  The spec's AMR algorithm uses row/column metadata, hash frequency classification (unique/rare/common), run‑length compression for repetitive rows, and anchor‑based alignment with LIS plus per‑gap strategies to achieve robust O(R log R) behavior even on adversarial inputs, but the exact choices (strict unique‑row anchoring, full RLE) may need adaptation for Excel's highly repetitive financial models.

* **Implementation relies on greedy heuristics with hard caps and noisy fallback.**
  The current row alignment uses a greedy local strategy and is guarded by `MAX_ALIGN_ROWS = 2_000` and `MAX_ALIGN_COLS = 64`. The column cap is particularly restrictive: financial models routinely have hundreds of columns (monthly projections, scenarios, sensitivity tables), so a 64‑column limit can block alignment on common workloads. Additionally, the caps are inconsistent across modules—`rect_block_move` allows up to 128 columns while row/column alignment caps at 64—creating surprising behavior where certain diff strategies succeed on sheets that others reject. When inputs exceed these caps or when repetition triggers internal safety checks, alignment returns `None` and the engine drops back to a positional diff. A further constraint is `MAX_BLOCK_GAP = 32`, which limits contiguous block insertions/deletions to 32 rows regardless of total sheet size; inserting 50 rows in a 500‑row sheet fails alignment even though both dimensions are well under the row cap. Against the 50K‑row target, these caps and the fallback combine to make the engine both scale‑limited and behaviorally fragile.

* **Row alignment is "single‑gap only."**
  Internally, the row‑alignment path chooses between single‑gap and block‑gap routines that require `prefix_match + suffix_match == shorter_length`. That means the algorithm can express at most one contiguous insertion/deletion region per alignment; any scenario with two or more independent edit blocks fails these preconditions and forces the positional fallback. This is a correctness limitation, not just a performance trade‑off.

* **Move detection is "one‑shot," not iterative, and causes silent data loss.**
  The `diff_grids` function detects at most one structural move (rect, row block, or column block) and then either returns immediately or falls through to alignment. If a user moves two separate row blocks, only the first is detected; the second appears as a large deletion/insertion pair. More critically, when a rectangular block move is detected, the function emits the move and returns immediately—any edits outside that moved region are **silently dropped**, never computed or reported. This is not merely a capability gap; it is a correctness failure where the diff report omits real changes.

* **Row hashes are column‑index‑dependent, breaking alignment on column changes.**
  `hash_cell_contribution(position, cell)` hashes the column index into each cell's contribution to its row hash. Inserting a column at position A causes all existing data to shift (B→C, C→D, etc.), changing the position parameter for every cell and thus invalidating every row hash. Because the engine attempts row alignment before column alignment (`diff_grids` tries row strategies first), any column insertion guarantees a 100% row‑alignment failure, forcing a noisy positional fallback. The spec's dimension‑ordering pre‑pass was designed to avoid exactly this failure mode.

* **Floating‑point hashing uses bitwise comparison, not semantic normalization.**
  `CellValue::Number` hashes via `n.to_bits()`, which treats `0.0` and `−0.0` as distinct values (different bit patterns) and is sensitive to ULP‑level epsilon drift. Excel recalculation can flip least‑significant bits without changing displayed values; the engine will report these as cell edits or fail to match rows that are semantically identical. The spec calls for normalizing numbers to a fixed precision (e.g., 15 significant digits) before comparison.

* **RLE and anchor‑first alignment are missing.**
  Although `GridView` already collects metadata like non‑blank counts and hashes, the alignment code does not yet compress runs of common rows or use unique/rare classification to drive a global anchor chain. This absence is the specific technical reason the cap and "heavy repetition" bail‑out exist: without RLE, grids dominated by blank/template rows push the naive alignment toward O(N²) behavior, so the implementation avoids those cases rather than handling them efficiently.

* **Row hashing currently trades a small but non‑zero correctness risk for speed.**
  Row signatures in spreadsheet‑mode alignment are based on 64‑bit hashes; at the 50K‑row scale (and across a large corpus of workbooks) the birthday bound makes collisions unlikely but not impossible, and any collision becomes a silent false match in alignment. The code explicitly documents this tradeoff (`_HASH_COLLISION_NOTE` cites ~0.00006% collision probability at 2K rows and defers secondary verification to a planned "G8a 50K‑row adversarial" testing phase), indicating conscious deferral rather than oversight. Importantly, database‑mode alignment is immune to this risk: it uses exact `KeyValue` comparison over actual cell contents, not hash‑based matching, so keyed diff at scale does not suffer from collision‑driven false matches.

* **Diff materialization and string handling are WASM‑dangerous.**

  * The engine collects every `DiffOp` into a single `Vec<DiffOp>`, globally sorts it for deterministic output, and then serializes that vector into one JSON string. This double‑buffering and sort step are the primary memory risk for 50K‑row inserts or heavy edit workloads, especially under WASM limits.

  * On the input side, the IR models text cells as `CellValue::Text(String)`, so a column of 50,000 identical `"Pending"` values produces 50,000 separate heap allocations before diffing even begins. Combined with per‑operation `String` sheet IDs in the result, this diverges from the spec's `StringPool` + `StringId` design and makes out‑of‑memory failures likely on large, repetitive sheets unless interning is introduced.

  * Additionally, `grid_parser::parse_sheet_xml` accumulates all parsed cells into a temporary `Vec<ParsedCell>` before building the final `Grid`. This double‑buffering (XML stream → ParsedCell Vec → Grid HashMap) amplifies peak memory usage during load and compounds the WASM heap risk on large sheets.

**Recommendations**

1. **Treat the 2,000‑row cap and single‑gap alignment as non‑negotiable release blockers.**
   Removal of the cap, by implementing AMR + RLE and validating performance on the spec's adversarial test cases, should be the top engineering priority. In parallel, the current single‑gap alignment must be replaced with a general multi‑gap algorithm so that everyday patterns like "insert a few rows in two places" no longer degrade into positional diffs. Until both changes are in place, the engine cannot be considered production‑ready.

2. **Implement AMR row alignment and run‑length compression.**

   * Extend `GridView` metadata to classify rows as unique/rare/common and low‑information as per the spec.
   * Compress runs of common/low‑information rows into logical runs (`RowRun`), ensuring alignment operates over compressed sequences.
   * Implement anchor discovery (unique hashes), LIS to form a global anchor chain, move candidate extraction, move‑aware gap filling, and final alignment assembly as described in AMR.

3. **Switch to a streaming diff pipeline and output formats for large diffs.**
   Refactor the engine to expose an iterator‑ or callback‑based API (for example, `impl Iterator<Item = DiffOp>`) so operations can be produced incrementally, and then layer JSON Lines or other streaming serializers on top. This avoids ever holding both a full `Vec<DiffOp>` and its serialized representation in memory at once.

4. **Intern strings and compact result structures.**

   * Introduce a shared `StringPool` per diff run (or per pair of workbooks).
   * Replace string fields (sheet names, string cell values) with integer IDs inside `DiffOp`.
   * Maintain a mapping in metadata to reconstruct human‑readable names for UI. 

5. **Embed performance regression checks into CI.**
   Add perf tests using the "adversarial repetitive" and "large grid" fixtures from the testing plan, enforcing wall‑clock and memory ceilings (e.g., maximum time and peak heap for a 50K×100 grid).

6. **Upgrade row hashing to 128‑bit for safety at scale.**
   Move spreadsheet‑mode row signatures from 64‑bit to 128‑bit (for example, using `xxh3_128` or `SipHash128`) so that even at 50K‑row scale across large corpora, the probability of a collision‑driven false match is negligible. (Database‑mode keyed alignment already uses exact value comparison and does not require this change.) The existing `_HASH_COLLISION_NOTE` documents this as a conscious tradeoff with deferred verification; the upgrade should land before the planned G8a adversarial testing phase.

7. **Fix column‑dependent row hashing and add dimension pre‑ordering.**

   * Refactor `hash_cell_contribution` to use **content‑only hashing** for row signatures (omitting the column index), or adopt a position‑invariant signature scheme so that row identity is stable across column insertions.
   * Implement the spec's dimension‑ordering pre‑pass: before committing to row‑first alignment, compute column stability; if columns have changed significantly, attempt column alignment first. This prevents the current failure mode where any column insertion causes total row‑alignment collapse.

8. **Normalize floating‑point values before hashing.**

   * Replace `n.to_bits()` with a semantic normalization step (e.g., round to 15 significant digits, canonicalize `−0.0` to `0.0`) so that recalculation noise does not produce spurious diffs.
   * Add tests covering edge cases: signed zeros, epsilon drift, and error values like `NaN`.

---

### 7. Future Readiness

**Assessment:** **Conceptually future‑proof, blocked by platform and algorithm gaps**

**Evidence**

* **IR and spec anticipate DAX, PBIX, and multi‑host deployment.**
  The specification and IR reserve room for DAX measures, tabular relationships, PBIX containers (with and without DataMashup), and cross‑platform integration (CLI, desktop, web/WASM).

* **Testing plan already includes PBIX and WASM guardrails.**
  Early phases of the testing plan call for a WASM build guard and PBIX host support, indicating that portability and host diversity are first‑class goals, not afterthoughts.

* **Current implementation is not yet packaged for WASM, but the gaps are mechanical.**
  The `std::fs::File` dependency in the core container, the current "all `DiffOp`s in a `Vec`" output model, and the heavy parsing dependencies (`quick-xml`, `zip`, `serde_json`) all need cleanup and feature‑gating for a browser target, but none require redesigning the IR or core algorithms.

* **Domain fracture complicates future integration.**
  Keeping `Workbook` and `DataMashup` on separate tracks, plus exposing database mode as a separate API, will make it harder to integrate future DAX/model diff and to reason about complex mixed sheets where queries, tables, and free‑form grid content interact.

**Recommendations**

1. **Make WASM a hard CI gate once I/O and output are refactored.**
   After abstracting I/O and introducing streaming output, add a WASM build and smoke tests (parse representative workbooks, run several diff scenarios) to CI. Fail the pipeline if WASM compilation breaks. Note that the current `edition = "2024"` dependency requires nightly Rust; evaluate whether to pin to edition 2021 for stable WASM toolchain support until 2024 stabilizes.

2. **Unify the package‑level domain now, before DAX/model work.**
   Implement `WorkbookPackage` and a unified `DiffOp` surface before introducing DAX or deeper Power BI support, so those new capabilities extend a coherent model rather than patching around fractures later.

3. **Reserve identifiers and DiffOp variants for future domains.**
   Allocate variant names and IDs for DAX and model‑level changes (e.g., `MeasureEdited`, `RelationshipAdded`) even if they are not yet emitted. This pattern is already partially in place: `QueryChangeKind::Renamed` exists with an explicit "not emitted yet" comment, demonstrating forward‑compatibility thinking. Extend this approach to DAX and model‑level changes to reduce the risk of future breaking changes in serialized diff formats.

---

## Tensions and Trade‑offs

1. **Heuristic, single‑gap implementation vs. spec‑aligned AMR anchors.**
   The current heuristics made it possible to ship a working prototype early and handle some repetitive Excel patterns, but they forced the introduction of a row cap, rely on "single‑gap only" alignment, and diverge from the AMR design. The trade‑off was speed‑to‑first‑diff vs. long‑term algorithmic robustness and predictable semantics. The next iteration needs an AMR‑style, globally anchored pipeline that handles arbitrary multi‑gap edits and has explicit defenses against non‑unique rows; simply dropping in a textbook Patience diff over unique rows would likely regress behavior on financial models.

2. **Simple top‑level API vs. unified domain model.**
   Exposing separate APIs for workbook diff, DataMashup diff, and database mode simplifies each subsystem but shifts integration complexity onto callers. Moving that integration back into the engine will complicate internals slightly but greatly simplify external usage and future evolution.

3. **Memory safety via caps vs. full scalability.**
   The row cap and conservative heuristics prevent catastrophic performance failures, but at the expense of refusing real‑world workloads. Once AMR + RLE are in place, those caps should be replaced with algorithmic guarantees and well‑defined bail‑out behavior, not hard ceilings.

4. **Rich semantic diff vs. configuration complexity.**
   The engine aspires to semantic diff for formulas, M, and eventually DAX, but today only M has an AST‑based semantic gate; Excel formulas are still compared as raw strings. Each new semantic dimension brings new thresholds and knobs (for example, when to treat reference shifts or commutative rewrites as "no‑ops"), so centralizing configuration via `DiffConfig` is key to managing this complexity without overwhelming users.

5. **RLE‑heavy spec vs. simpler frequency‑based defenses.**
   The spec leans on run‑length encoding of repetitive rows to defend against 99%‑blank or highly repetitive grids, while the current implementation uses more direct heuristics. For many Excel workloads, a histogram/frequency‑based treatment of low‑information rows may deliver most of the benefit with less complexity than full RLE diff, so the algorithmic design should keep that trade‑off open rather than treating RLE as mandatory.

6. **Streaming output vs. globally sorted determinism.**
   The spec simultaneously calls for streaming JSON Lines output and for a globally sorted `(op, row, col)` order. The implementation currently chooses determinism by collecting all `DiffOp`s into a `Vec`, sorting, and then serializing, which defeats true streaming and contributes to the memory spike on large diffs. The design needs to decide whether to relax the ordering requirement (for example, "row‑major, stable within a sheet") or to adopt a chunked/merge strategy that preserves both goals without a full in‑memory sort.

7. **Heavyweight dimension pre‑pass vs. optimistic dimension ordering.**
   The spec's dimension‑ordering phase computes similarity for all columns before deciding row‑first vs. column‑first alignment. The implementation instead optimistically attempts row‑first alignment and only falls back when column changes are extreme. For the 90% case of mostly stable columns, this lazy approach is faster and simpler than a full similarity pre‑pass. However, because row hashes currently incorporate absolute column indices, the optimistic approach causes **total alignment failure** on any column insertion, not just graceful fallback. The spec and implementation must converge on an approach that either decouples row hashes from column positions or performs a lightweight dimension check first.

8. **One‑shot move detection vs. iterative subtraction.**
   The current `diff_grids` function detects at most one move and then proceeds to alignment, whereas full accuracy requires iterating: detect a move, emit it, logically remove the moved region, and repeat. The one‑shot approach was presumably chosen for simplicity, but it makes multi‑region edits (moving two paragraphs, reordering three row blocks) fundamentally unrepresentable without a more sophisticated loop. The severity goes beyond incomplete representation: for rectangular block moves specifically, the early return causes **silent data loss**—edits outside the moved region are never computed, making the diff report incorrect rather than merely incomplete.

---

## Areas of Excellence

1. **IR and DiffOp Design**
   The IR is small, semantically meaningful, and stable. `DiffOp` provides a clean representation of structural and cell‑level changes, well suited for both UI and CLI consumption. It maps closely to the operations defined in the algorithm and testing specifications and is already backed by round‑trip serialization tests.

2. **Grid / GridView Memory Architecture**
   The separation of `Grid` (sparse, persistent storage) and `GridView` (ephemeral, metadata‑rich view) is a standout design. It directly addresses the "100MB Excel file expands to 400–1000MB in memory" problem by avoiding dense matrices and operating on metadata and non‑empty cells only. (Note: `Grid` currently contains vestigial `row_signatures`/`col_signatures` fields that blur this boundary; they should be removed to complete the separation.)

3. **DataMashup parsing robustness (with a partial M front‑end)**
   The DataMashup parser correctly implements MS‑QDEFF framing, handles BOM variations, malformed XML, duplicate or missing metadata entries, and odd encodings, with extensive tests and fixtures. Query/metadata joining is treated as a first‑class domain. The current M parser handles `let ... in` expressions including nested lets, but non‑`let` top‑level expressions (direct records, lists, primitives) fall back to opaque token sequences, so broad M syntax coverage remains a high‑risk follow‑up item rather than a solved problem.

4. **Semantic M Diff Gating**
   AST canonicalization and semantic equality checks for M queries provide a clear product advantage: ignoring purely formatting changes while detecting true semantic shifts. The tests around semantic vs. textual changes are thoughtful and align closely with the specification's intent, but their reliability ultimately depends on expanding the M parser beyond the current top‑level `let ... in` happy path. At the moment this semantic gate exists only for M; Excel formulas are still treated as text, so realizing the spec's vision of semantic formula diff will require a comparable Excel formula parser and AST‑based comparison layer.

5. **Testing Discipline and Fixture Strategy**
   The breadth of fixtures and alignment with the testing plan (especially advanced grid alignment and database mode scenarios) show a strong testing culture. The Python generator avoids committing large binary files and makes it easy to evolve fixtures as edge cases are discovered.

6. **Excel‑Compatible Sheet Matching**
   Sheet identity comparison lowercases names before matching (`make_sheet_key`), correctly replicating Excel's case‑insensitive behavior where "Sheet1" and "SHEET1" are the same sheet. This compatibility detail is important for diff accuracy on workbooks round‑tripped through systems that normalize casing.

7. **Edge Case Handling in Output**
   The JSON serializer explicitly checks for and rejects non‑finite numbers (`NaN`, `Infinity`) via `contains_non_finite_numbers`, preventing invalid JSON output from Excel cells containing `#DIV/0!` or `#NUM!` errors that resolve to special float values. The fuzzy block similarity function uses a dual‑metric approach (`jaccard.max(positional_ratio)`) that handles both content‑shuffled and position‑preserved blocks, rather than relying on a single threshold.

---

## Priority Recommendations

Ordered by impact and necessity:

1. **Unlock Scalability and Correctness: Implement AMR‑style multi‑gap alignment and remove hard caps**

   * **Fix silent data loss**: The rectangular block move early return must be changed to continue diffing the remaining grid regions rather than exiting immediately. This is a correctness bug, not a scalability issue, and should be addressed before any performance work.
   * Implement row metadata, hash frequency classification, and an AMR‑style anchor/move/refine row alignment pipeline, but treat the spec's Patience‑diff unique‑row anchoring and RLE scheme as starting points to be validated against real Excel workloads (especially highly repetitive financial models), not as immutable requirements.
   * Replace the current "single‑gap only" alignment with a general multi‑gap algorithm so that common scenarios with multiple disjoint insert/delete regions no longer fall back to positional diff.
   * Experiment with simpler defenses against 99%‑blank or highly repetitive regions (for example, histogram/frequency‑based cancellation of low‑information rows) if full RLE diff proves too complex for the benefit it buys.
   * Validate against adversarial and large‑grid fixtures with explicit time and memory ceilings. (The fixture manifest already defines 50K‑row test cases; the infrastructure is ready.)
   * Remove or significantly raise `MAX_ALIGN_ROWS` (2,000), `MAX_ALIGN_COLS` (64), and `MAX_BLOCK_GAP` (32), treating any remaining caps as temporary safeguards for truly pathological cases only. Harmonize the inconsistent column caps across modules (64 in alignment vs. 128 in rect moves).

2. **Centralize Configuration with a `DiffConfig` and Replace Magic Constants**

   * Introduce `DiffConfig` to hold thresholds (similarity, rare/common frequency boundary, gap sizes, bail‑out limits, performance caps).
   * Pass this config through the entire diff pipeline.
   * Add tests verifying that different configurations produce expected trade‑offs (e.g. more aggressive move detection vs. more conservative behavior).

3. **Refactor I/O and Output for WASM and Large‑Diff Readiness**

   * Refactor container APIs to use generic `Read + Seek` and isolate `std::fs::File` to a host adapter; this is a small, high‑leverage refactor that should land early as tech‑debt cleanup.
   * Refactor the diff engine to emit `DiffOp`s through an iterator or callback so that large diffs can be consumed and serialized incrementally rather than via a monolithic `Vec<DiffOp>`.
   * Decide whether determinism truly requires a globally sorted `(op, row, col)` order; if so, design an external or chunked sorting strategy that works with streaming, and if not, relax the requirement in favor of a streaming‑friendly "row‑major, stable by creation order" guarantee and drop the global in‑memory sort.
   * Implement streaming JSON Lines output (and, optionally, a compact binary format) on top of the iterator, so CLI, server, and WASM callers can start rendering results before the entire diff has been computed.
   * Add a WASM build and a small set of WASM smoke tests to CI, alongside a simple size‑budget check for the core WASM bundle.

4. **Introduce String Interning, Compact Identifiers, and Eliminate Coordinate Redundancy**

   * Implement the `StringPool` mechanism and use interned IDs for sheet names and string cell values in both IR and diff outputs.
   * Update serialization formats to include a string table in metadata, mapping IDs to actual strings.
   * Remove coordinate redundancy from `Cell`: coordinates are stored in the HashMap key, in `Cell.row`/`Cell.col` fields, and again in `Cell.address`. Eliminate two of these three to save 16 bytes per cell.
   * Treat these memory optimizations as part of the P1 WASM‑readiness work rather than optional: without them, large sheets can exhaust the browser heap before diffing begins.

5. **Unify Domain Surfaces via `WorkbookPackage` and a Single `DiffOp` Stream**

   * Add `WorkbookPackage` owning both `Workbook` and `DataMashup` (and eventually `DataModel`).
   * Extend `DiffOp` to include M and future DAX/model operations.
   * Expose a single `diff_packages` API that yields a unified `DiffReport`, with helper projections for “just grid” or “just M” views.

6. **Refactor Alignment Code into Spec‑Aligned Phases**

   * Decompose the current heuristic alignment into helpers that mirror the AMR phases, even before full algorithm replacement.
   * Use those phase boundaries as seams to incrementally swap in spec‑driven pieces (e.g., LIS anchor chain first, then run‑length compression, then move‑aware gap strategies).

7. **Strengthen Observability and Performance Regression Testing**

   * Add timing and memory metrics hooks around diff runs (opt‑in to avoid overhead in critical paths).
   * Create canonical "perf" fixtures and enforce non‑regression on runtime and heap usage in CI.

8. **Harden hashing strategy for correctness.**

   * Upgrade spreadsheet‑mode row signatures from 64‑bit to 128‑bit (for example, `xxh3_128` or `SipHash128`) to make collision‑driven false matches vanishingly unlikely at the 50K‑row scale and across large corpora. (Database‑mode keyed alignment already uses exact value comparison and is immune to hash collisions.)
   * Replace `f64::to_bits()` hashing with semantic normalization (15‑digit precision, canonical signed zero) to prevent recalculation noise from causing spurious diffs.
   * Remove column‑index dependency from row hash computation so that column insertions do not invalidate all row signatures.

9. **Make move detection iterative, not one‑shot.**

   * Refactor `diff_grids` to loop: detect a move, emit it, subtract the moved region from consideration, and repeat until no more moves are found. This allows the engine to describe multiple independent block moves per sheet instead of stopping after the first.

10. **Upgrade the M parser from "happy path" to production coverage.**

   * The parser already handles nested `let ... in` expressions correctly. Expand it to handle non‑`let` top‑level expressions (direct records, lists, primitive expressions) as full ASTs rather than opaque token sequences, and align tests to make unsupported constructs explicit. Until this is done, treat semantic M diff as a high‑risk subsystem that depends on parser coverage work, not as a fully solved strength.

11. **Add an Excel Formula Parser and Semantic Diff Layer**

    * Implement an AST parser for Excel formulas with canonicalization similar to the M pipeline (commutative normalization, case‑insensitive function names, and reference‑shift detection).
    * Thread formula semantics into cell comparison so that purely mechanical shifts and harmless rewrites do not surface as noisy `CellEdited` operations.
    * Align tests with the formula semantic diff spec, starting with small, carefully curated fixtures (commutative rewrites, reference shifts, error values) before scaling out to larger workbooks.

---

## Conclusion

The Excel Diff Engine is already successful in the hard, unglamorous work of taming hostile file formats and building a clean, host‑neutral IR. It has an excellent memory model at the structural level, strong Rust idioms, and an unusually thorough testing and fixture setup. Those are durable assets.

What remains is to **upgrade the algorithmic heart** of the system and address several correctness issues that make the current engine fragile:

* **Fix silent data loss on rect moves**: The current early return after rect move detection drops all edits outside the moved region. This must be fixed before the engine can be considered correct.
* **Multi‑gap alignment**: Replace single‑gap row alignment with a general sequence‑alignment algorithm (AMR with anchors and gap strategies).
* **Iterative move detection**: Make `diff_grids` loop to detect multiple moves rather than stopping after the first.
* **Column‑independent row hashes**: Remove column‑index dependency from row hashing so that column insertions do not invalidate all row signatures; alternatively, implement dimension pre‑ordering.
* **Semantic float comparison**: Normalize floating‑point values before hashing to avoid spurious diffs from recalculation noise.
* **String interning**: Adopt the spec's `StringPool`/`StringId` model to reduce memory pressure.
* **Eliminate coordinate redundancy**: Remove duplicated row/col storage from `Cell` to reduce per‑cell memory overhead.

Fixing these correctness and scalability issues, along with the structural gaps (WASM‑unsafe I/O, monolithic JSON with a global sort, fragmented API), will put the implementation on the same footing as the design documents.

If the team focuses first on lifting the row cap by implementing the specified grid algorithm, replacing the single‑gap alignment with a general sequence alignment, introducing string interning, centralizing configuration, and making the core crate WASM‑ and streaming‑ready, everything else—PBIX support, Excel formula semantics, DAX/model diff, richer UX—will have a trustworthy foundation to build on.
