Here’s the difficulty picture for the product, from “hardest” down, based on the docs you gave me.

---

## 1. Difficulty rubric

I’ll score every hurdle on four 1–5 axes and sum them (min 4, max 20):

* **C – Conceptual / algorithmic complexity**
  1 = straightforward plumbing; 5 = you’re effectively doing research-level algorithm design.
* **U – Uncertainty / reverse‑engineering / spec risk**
  1 = fully specified & stable; 5 = lots of undocumented behavior / evolving formats.
* **S – System surface area**
  1 = small, isolated component; 5 = cross‑cuts most of the engine and APIs.
* **P – Performance & scale pressure**
  1 = can be slow; 5 = must be extremely fast and memory‑efficient on large files.

**Total difficulty score = C + U + S + P (range 4–20).**

---

## 2. Ranked technical hurdles

### Summary table

| Rank    | Hurdle                                                                             | C | U | S | P | Total  |
| ------- | ---------------------------------------------------------------------------------- | - | - | - | - | ------ |
| 1       | H1. High‑performance 2D grid diff engine (row/col alignment, block moves, DB mode) | 5 | 3 | 5 | 5 | **18** |
| 2       | H2. Streaming, memory‑efficient parsing & IR build (Rust/WASM, 100MB files)        | 4 | 3 | 4 | 5 | **16** |
| 2 (tie) | H4. Semantic M diff engine (step‑aware + AST diff)                                 | 5 | 3 | 4 | 4 | **16** |
| 4       | H3. M language parser & step model                                                 | 4 | 4 | 4 | 3 | **15** |
| 4 (tie) | H9. Robust handling of weird/legacy/future DataMashup/Excel variants               | 3 | 5 | 4 | 3 | **15** |
| 6       | H5. DAX / data‑model parser & semantic diff                                        | 4 | 4 | 3 | 3 | **14** |
| 7       | H6. Excel formula parser & semantic diff                                           | 4 | 3 | 3 | 3 | **13** |
| 8       | H7. PackageParts & embedded OPC/Embedded.Value extraction                          | 3 | 3 | 3 | 3 | **12** |
| 8 (tie) | H10. Cross‑platform packaging & integration (Mac/Win CLI, Web/WASM, Git)           | 3 | 2 | 4 | 3 | **12** |
| 8 (tie) | H11. Comprehensive test harness, fixtures, fuzzing, golden oracles                 | 3 | 2 | 4 | 3 | **12** |
| 8 (tie) | H12. Human‑facing hierarchical diff reporting (DiffOp → UX/API)                    | 3 | 3 | 4 | 2 | **12** |
| 12      | H8. Host container + DataMashup extraction & top‑level MS‑QDEFF framing            | 2 | 2 | 3 | 2 | **9**  |
| 13      | H13. Permission bindings & DPAPI verification                                      | 2 | 3 | 2 | 1 | **8**  |

Below is what each hurdle actually is and why it scored the way it did.

---

## 3. Hurdles in detail (hardest → easiest)

### H1. High‑performance 2D grid diff engine (spreadsheet + database modes) – **18**

**What it is**

* Row‑ and column‑alignment for arbitrary sheets using Hunt–Szymanski–style LCS on row signatures, then similar alignment for columns, plus detection of inserted/deleted rows, moved blocks, and key‑based “database mode” with heuristic key inference. 
* Needs to correctly distinguish “row moved”, “row changed”, “pure insert/delete”, and handle huge ranges without blowing up.

**Why it’s so hard**

* **C=5:** You’re adapting advanced sequence‑alignment algorithms (LCS/HS, Hungarian matching, rolling hashes) to a noisy 2D domain and combining them into a coherent pipeline.
* **U=3:** The math is known, but “what users expect” (e.g. when to call something a move vs delete+add) is not formally specified.
* **S=5:** This sits in the absolute hot path of the product: every workbook diff depends on it, and it also underpins how formula, M, and DAX differences get surfaced.
* **P=5:** It must run near‑linearly on ~100MB workbooks, so any naive O(n²) behavior will be visible as “this tool hangs”.

---

### H2. Streaming, memory‑efficient parsing & IR build – **16**

**What it is**

* Streaming Open XML + DataMashup parsing to build the normalized `Workbook` / `Sheet` / `Grid` / `DataMashup` IR in Rust, and the same engine compiled to WASM for the browser.

**Why it’s hard**

* **C=4:** You’re doing non‑trivial streaming XML/ZIP parsing and incremental IR construction in a low‑level language, balancing safety and speed.
* **U=3:** Libraries exist, but combining them with your specific IR and performance targets (including WASM limitations) is not “cookie cutter”.
* **S=4:** Everything else builds on this IR; if this layer is wrong or slow, the whole product inherits that.
* **P=5:** This is the first major bottleneck for “instant diff on 100MB files”. Mis‑steps here ripple into all higher‑level algorithms.

---

### H4. Semantic M diff engine (step‑aware + AST diff) – **16**

**What it is**

* Given two parsed M queries (AST + step list), align steps, detect added/removed/changed steps, and summarize precise semantic changes (filter conditions, join types, column sets, etc.), with AST diff as a fallback. 

**Why it’s hard**

* **C=5:** You’re designing your own costed sequence‑diff over “steps”, plus tree edit distance over M ASTs, and then mapping that into human‑readable explanations.
* **U=3:** M is quirky; what’s “the same” vs “different” semantically isn’t always obvious, especially with re‑ordered steps and equivalent expressions.
* **S=4:** This is one of the flagship differentiators of the product (semantic diff for Power Query), so it touches API, reports, and UX.
* **P=4:** You’ll potentially run this over dozens of queries per workbook; each diff is small, but users will notice if semantic diffs feel laggy.

---

### H3. M language parser & step model – **15**

**What it is**

* A production‑grade M parser (lexer + grammar) that produces `MModuleAst`, and a normalized step model (`MStep`, `StepKind`, `StepParams`) that ties back to Power Query’s transformation concepts.

**Why it’s hard**

* **C=4:** Writing a robust parser and AST for a full language is non‑trivial, especially one with its own quirks like M.
* **U=4:** While there’s some documentation, a lot of real‑world behavior is implicit in how Power Query emits code; you’ll have to handle edge cases and odd constructs that aren’t well documented. 
* **S=4:** All M‑related features—semantic diffs, query signatures, rename detection—build on this.
* **P=3:** Queries are usually not huge, but you still need to parse them quickly and safely.

---

### H9. Robust handling of weird/legacy/future formats – **15**

**What it is**

* Making the engine resilient to malformed or legacy DataMashup streams, non‑compliant files, and future MS‑QDEFF versions, using tools like binwalk, Kaitai, and property‑based tests.

**Why it’s hard**

* **C=3:** The logic is more about defensive programming than fancy algorithms, but it’s subtle.
* **U=5:** By definition, you’re dealing with undocumented behavior, historical quirks, and future changes that Microsoft hasn’t written yet.
* **S=4:** Error handling and fallback behavior permeate almost every parsing path; get it wrong and you crash on exactly the messy files customers care most about.
* **P=3:** Usually not on the hottest path, but you still must not degrade performance with over‑zealous scanning.

---

### H5. DAX / data‑model parser & semantic diff – **14**

**What it is**

* Parsing the tabular data model (tables, relationships, measures) from Excel/Power BI containers and building ASTs for DAX measures; then diffing them semantically.

**Why it’s hard**

* **C=4:** Similar in difficulty to formula parsing, but DAX has its own semantics and context rules.
* **U=4:** The underlying model and file layout differ between Excel and PBIX/PBIT, and documentation is patchier.
* **S=3:** It’s a later‑phase feature, but critical for “Modern Excel / Power BI” positioning.
* **P=3:** Measure expressions are small, so performance concerns are moderate.

---

### H6. Excel formula parser & semantic diff – **13**

**What it is**

* Parsing cell formulas into ASTs, canonicalizing them, and diffing to distinguish formatting‑only changes from logical changes in formulas. 

**Why it’s hard**

* **C=4:** You need a robust parser that handles all Excel function syntax, references, and edge cases.
* **U=3:** The grammar is mostly stable, but there are locale and version quirks.
* **S=3:** Used across many sheets but localized to the formula layer.
* **P=3:** There can be lots of formulas, but each one is small, so tree‑diff cost is manageable.

---

### H7. PackageParts & embedded OPC/Embedded.Value extraction – **12**

**What it is**

* Interpreting the `PackageParts` section of the DataMashup stream as an inner OPC/ZIP package, reading `/Config/Package.xml`, `/Formulas/Section1.m`, and recursively handling `/Content/{GUID}` mini‑packages used by `Embedded.Value`.

**Why it’s hard**

* **C=3:** Nested ZIP/OPC handling and plumbing isn’t trivial but is well understood.
* **U=3:** Specs + community tooling exist, but embedded content layouts vary in practice.
* **S=3:** This is how you actually get to the M code and embedded queries, so it’s important but somewhat localized.
* **P=3:** Decompression and IO must be efficient, but not algorithmically extreme.

---

### H10. Cross‑platform packaging & integration (Mac/Win, CLI, Web/WASM, Git) – **12**

**What it is**

* Shipping the Rust core as native binaries (Mac/Win) with a CLI, integrating as a difftool/mergetool for Git/SVN/etc., and as a WASM module behind a web viewer.

**Why it’s hard**

* **C=3:** Mainly build‑, tooling‑, and deployment complexity rather than deep algorithms.
* **U=2:** Toolchains are known, but OS‑specific quirks and Git integration behaviors can surprise you.
* **S=4:** This spans your whole delivery story—CI, installers, web bundling, and integrations.
* **P=3:** Runtime performance is mostly governed by the core engine, but packaging can impact startup time and resource use.

---

### H11. Comprehensive test harness, fixtures, fuzzing, golden oracles – **12**

**What it is**

* A multi‑layer test strategy: unit and integration tests per milestone, a Python fixtures repo that generates real Excel/PBIX scenarios, golden comparisons against Data Mashup Cmdlets, property‑based and performance tests. 

**Why it’s hard**

* **C=3:** Designing good tests and fixtures for this many layers is non‑trivial.
* **U=2:** The plan is clearly laid out, but test data will keep evolving as you discover new edge cases.
* **S=4:** This touches every part of the engine and is key to preventing regressions.
* **P=3:** Test suites must stay fast enough for CI, which becomes a real constraint with large fixtures.

---

### H12. Human‑facing hierarchical diff reporting – **12**

**What it is**

* Designing `DiffOp` types, aggregating raw differences into workbook/sheet/query‑level stories, and shaping this into JSON and UI views that feel intuitive (e.g., GitHub‑style diffs for grid and M).

**Why it’s hard**

* **C=3:** The core logic isn’t algorithmically extreme but requires careful structuring and summarization.
* **U=3:** User expectations around what constitutes a “useful explanation” are fuzzy and must be iterated on.
* **S=4:** The reporting model is the contract between engine, CLI, and any web or desktop UI.
* **P=2:** Mostly not performance‑critical compared to parsing and alignment.

---

### H8. Host container + DataMashup extraction & top‑level MS‑QDEFF framing – **9**

**What it is**

* Treating Excel/PBIX as OPC/ZIP containers, finding the `<DataMashup>` part in `customXml` or root `DataMashup` file, base64 decoding where needed, and parsing the top‑level MS‑QDEFF framing (version + four length‑prefixed sections with invariants).

**Why it’s hard**

* **C=2:** Mostly careful I/O and binary parsing.
* **U=2:** MS‑QDEFF is documented; blogs fill in gaps.
* **S=3:** This is the first step for any M‑aware diff; if it fails, the rest can’t run.
* **P=2:** The work is small relative to the rest of the pipeline.

---

### H13. Permission bindings & DPAPI verification – **8**

**What it is**

* Handling the Permission Bindings section (DPAPI‑protected checksum tying permissions to the mashup); on non‑Windows platforms likely treating it as opaque bytes, optionally validating on Windows with DPAPI. 

**Why it’s hard**

* **C=2:** Using DPAPI is straightforward, and a “treat as opaque” policy is simple.
* **U=3:** Exact behavior across Excel versions and failure modes can be a bit underspecified.
* **S=2:** Important mainly for faithfully mirroring Excel’s privacy behavior; orthogonal to diff logic.
* **P=1:** Rarely on a hot performance path.

---

## 4. How to read this ranking

* The **top cluster (H1, H2, H3, H4, H9)** are your “architecture‑defining” problems: grid alignment, streaming parsing, full M understanding, and robustness to weird inputs. Getting these right (and tested) will make almost everything else easier.
* **Middle items (H5–H7, H10–H12)** are substantial, but you can parallelize them once the core IR and diff spine exist.
* **Bottom items (H8, H13)** are important but relatively contained and can be scoped tightly.

If you'd like, next step we can turn this into a phased implementation roadmap (e.g., which hurdles to tackle in what order to de‑risk the project fastest).

---

Last updated: 2025-11-24 19:33:13
