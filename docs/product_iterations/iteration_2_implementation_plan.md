# Iteration 2 Implementation Plan: PBIP / PBIR / TMDL

Date: 2026-02-08

This plan expands Iteration 2 ("PBIP/PBIR/TMDL support") from `product_roadmap.md` into an implementation sequence that prioritizes:

- Robustness: deterministic output, graceful degradation, strong error surfaces.
- Elegant simplicity: a small number of stable concepts, minimal coupling between layers.
- Architectural excellence: crisp boundaries, reusable primitives, and a UI that scales to new diff domains without rework.

Related:

- Iteration 2 readiness: `docs/product_iterations/iteration_2_readiness.md`

---

## Executive Summary

Iteration 2 adds a new diff domain: **Power BI source-controlled projects** (PBIP) composed of text-native artifacts (PBIR JSON, TMDL).

The highest-leverage design is to make the desktop app a **domain-agnostic shell** plus **domain-specific adapters**:

1. Shell: run/progress, preset/profile selection, exports, recents, stable result tabs.
2. Domain adapters: input pickers, navigator model, details renderer, explain builder, noise/normalization filters.

The Iteration 2 MVP should ship as **document-first** (changed files list + high-quality normalized diff) and then evolve into **entity-aware navigation** (pages/visuals/measures) without changing the shell.

---

## Goals (What "Done" Means)

### User-facing outcomes

- Users can diff **two PBIP project folders** (old/new) locally in the desktop app.
- The app shows a robust **Summary** (counts and highlights) and a usable **Details** view (normalized diff).
- The app provides an **Explain** view that outputs PR-review language (best-effort at first).
- Output is deterministic: repeated runs on the same inputs produce the same normalized representations and the same diffs.

### Engineering outcomes

- A new diff domain can be added without editing the Excel-specific UI logic in a fragile way.
- Desktop/backend storage is resilient for large projects and does not require loading entire projects into memory to show summaries.
- We have fixtures + tests that protect normalization rules and prevent regressions.

---

## Non-Goals (For Iteration 2 MVP)

- A full three-way merge UI.
- “Perfect” semantic mapping of every PBIR/TMDL construct.
- Completing every Power BI edge case up front.

Instead: build the scaffolding so that improvements are incremental and isolated.

---

## Product Tenets (UI/UX + Architecture)

These are the rules that keep Iteration 2 and beyond from turning into a pile of special-cases.

### Tenet A: A Stable Shell

The desktop app keeps the same high-level flow across domains:

- Inputs -> Compare -> Progress -> Results
- Results tabs:
  - Summary (what changed)
  - Details (show me the diff)
  - Explain (translate into PR language)

Domain-specific views must fit into these stable "contracts".

### Tenet B: Domain-Specific Navigator + Viewer

The "left panel" and "Details rendering" must be domain owned:

- Excel domain: sheets/cells/grid preview.
- PBIP domain: documents/entities/source preview.

The shell should not assume "sheet/cell" concepts.

### Tenet C: One Cross-Domain Selection Model

Everything the user can click becomes a **SelectionTarget**:

- Excel: sheet, cell, range
- PBIP: document, entity, location pointer

SelectionTarget is the only thing that flows from Navigator -> Details/Explain.

### Tenet D: Deterministic Normalization First

For text-native artifacts, normalization quality dominates product quality.

- If normalization is stable and correct, the diff is stable and useful.
- If normalization is noisy, everything else fails (UI included).

### Tenet E: Incremental Depth

Ship value early, then deepen semantics:

1. Document-first diffs (fast shipping, Git-native)
2. Entity-aware navigation (pages/visuals/measures)
3. Domain affordances (noise filters tuned for Power BI)

Each step should be an additive layer, not a rewrite.

---

## Architecture Blueprint

### Overview (Layers)

- `core/` (engine primitives):
  - parsing + normalization + diff algorithms for PBIP artifacts
  - domain-neutral diff result structures where possible
- `desktop/backend/` (orchestration + storage):
  - “run a diff job” and store results for later viewing
  - DB-side aggregation for large projects
- `ui_payload/` (protocol / DTO):
  - domain identifiers, selection targets, navigator models, summaries
- `desktop/wx/` (shell UI):
  - stable compare workflow
  - domain plugin points (inputs, navigator, details renderer)

### New Core Concepts (Keep This Small)

1. **DiffDomain**
   - `ExcelWorkbook`
   - `PbipProject`

2. **InputPair**
   - `FilePair { old, new }`
   - `DirPair { old, new }`

3. **SelectionTarget** (domain-neutral envelope)
   - `domain: DiffDomain`
   - `kind: SelectionKind` (Document, Entity, Location, Sheet, Cell, ...)
   - `id/path/pointer` fields depending on kind (keep as strings, do not leak domain structs into UI)

4. **NavigatorModel**
   - minimal, UI-ready rows for "left panel" (table or tree)
   - each item carries a `SelectionTarget`

5. **DetailsPayload**
   - text to render, plus metadata (mime-like: json, tmdl, text; left/right sides; optional patch)

Everything else can evolve behind those five boundaries.

---

## UI/UX Design (Iteration 2)

### Compare Screen (Inputs)

Add an explicit domain choice (or an auto-detect with explicit override):

- Mode selector: `Workbook` vs `PBIP Project`
- Inputs switch based on domain:
  - Workbook: file pickers (existing)
  - PBIP: directory pickers

Keep:

- Compare / Cancel
- Preset + profile selection (domain-specific options live behind the same UI)
- Status/progress surfaces

### Results Layout (Stable)

Keep the existing "shell" results layout:

- Navigator panel on left
- Result tabs on right

Change behavior per domain:

- Excel: Navigator = Sheets; Details includes Grid preview tab
- PBIP: Navigator = Documents initially; Details shows normalized source diff; Grid preview hidden

### Navigator: Iteration 2 MVP

Document list (table), sorted for review:

- Path (relative to project root)
- Type (PBIR/TMDL/Other)
- Change kind (Added/Removed/Modified)
- Impact hint (best-effort: "Report", "Model", "Visuals", "Measures")

Selection:

- click row -> updates Details and Explain to that document

### Details: Iteration 2 MVP

Prioritize speed and clarity over fancy UI:

- side-by-side normalized content (old vs new)
- optional inline diff highlight later (but keep payload contracts ready for it)

Minimum: show the normalized old/new texts and a small header:

- Document path
- Normalization profile (what rules were applied)

### Explain: Iteration 2 MVP

Best-effort, deterministic, PR language:

- “Report changed: 3 pages modified, 7 visuals updated” (when possible)
- Otherwise: “Document X changed: N structural changes detected”

Explain should never crash. If rules cannot be applied, it shows a clear fallback.

### UI Rules (Prevent “Excel UI Leak”)

- Rename Excel-specific shell labels to generic where appropriate:
  - “Sheets panel” -> “Navigator”
  - “Grid” tab -> “Preview” (domain can decide what preview means or disable it)
- Any Excel-only UI affordance must be guarded by `DiffDomain::ExcelWorkbook`.

---

## Data Model and Protocol Changes (`ui_payload/`)

Plan changes so that older payloads remain valid.

### Additions

- `DiffDomain` enum
- `SelectionTarget` struct
- `NavigatorModel` (table rows first; tree later)
- `DetailsPayload` (old/new text, language tag, optional patch metadata)
- PBIP-specific normalization config:
  - ignore ordering rules
  - GUID normalization policy
  - ignore paths (like `*.pbi/` caches or known generated folders)

### Versioning

- Add a `payload_version` field for domain payloads (or reuse existing app/engine versioning) so we can evolve normalization without breaking older stored diffs.

---

## Desktop Backend: Orchestration + Storage

### Diff execution

Introduce a domain dispatch layer (simple pattern matching is fine):

- If `DiffDomain::ExcelWorkbook`, run existing workflow.
- If `DiffDomain::PbipProject`, run PBIP workflow:
  - scan -> normalize -> diff -> summarize -> store

### Storage strategy (Robustness)

Avoid "store everything" and prefer "store what we need for UI":

- Store:
  - per-document change kind
  - stable hashes of normalized content
  - small, UI-useful snippets and summary aggregates
  - full normalized text only when needed (or store lazily/on-demand)

For MVP simplicity:

- Store full normalized text for changed documents, but cap size and degrade gracefully:
  - if too large, store only hashes + a limited excerpt and show "too large to display" with export option.

### DB aggregation for large projects

For PBIP, a "large mode" analog should exist from the beginning:

- Summary aggregation should be DB-first (counts by doc type, change kind).
- Entity extraction can be deferred/lazy in early milestones.

---

## Normalization: Robust and Deterministic

### PBIR (JSON) normalization

Requirements:

- Deterministic: stable ordering of object keys.
- Noise reduction: optional suppression of GUID churn and other generated fields.
- Safe: never drop semantically meaningful fields by default without an explicit policy.

Proposed approach:

1. Parse JSON to `serde_json::Value`.
2. Canonicalize:
   - recursively sort object keys
   - leave arrays in original order by default
3. Apply targeted “array normalization” only for known arrays with stable identity keys (future phase):
   - e.g., arrays of objects that each contain `name` or `id` fields
4. Apply noise suppression policies:
   - GUID normalization: map GUID-like values to stable placeholders within a document (deterministic mapping)
   - drop known volatile paths if a profile enables it (explicit opt-in)

### TMDL normalization

MVP constraints:

- We need stable formatting without writing a full compiler immediately.

Proposed staged approach:

1. Lexical normalization:
   - normalize line endings
   - trim trailing whitespace
   - normalize indentation to a standard scheme (conservative)
2. Block normalization (next phase):
   - parse enough structure to identify blocks (tables, measures, relationships)
   - stable ordering of blocks where semantics allow it

### Normalization profiles

Expose a few profiles (simple and explainable):

- Strict: minimal normalization (safe, noisy)
- Balanced (default): key sorting + conservative GUID policy
- Aggressive: known volatile field suppression + deeper normalization (opt-in)

Each profile must emit a "Normalization Applied" summary string that the UI can show.

---

## Diff: From Document-Level to Entity-Level

### MVP diff (document-level)

For each path:

- classify change kind: Added/Removed/Modified/Unchanged
- for Modified:
  - compute a stable hash of normalized old/new
  - compute a patch or at least store both sides for display

### Entity-level evolution

Later milestone: extract entities for navigation and explain.

PBIR entity candidates:

- Report
- Page
- Visual
- Bookmark
- Theme

TMDL entity candidates:

- Model
- Table
- Column
- Measure
- Relationship

Entity extraction rules must be:

- deterministic
- tolerant to missing/unknown fields
- versioned (so stored diffs remain explainable)

---

## Robustness Plan (Hard Requirements)

- Never crash on malformed JSON/TMDL:
  - show clear errors in Summary/Details instead
- Handle partial projects:
  - missing files
  - unexpected directory layouts
- Deterministic normalization and hashing:
  - no randomized iteration order
  - stable placeholder mapping for GUID normalization
- Resource caps:
  - cap per-document displayed size
  - cap total normalized text stored per run (soft cap with graceful fallback)
- Clear error taxonomy in UI:
  - "Unsupported format"
  - "Parse error"
  - "Normalization error"
  - "Diff error"

---

## Testing + Fixtures (Protect the Rules)

### Fixtures to collect (must happen early)

- 1 small PBIP repo (few pages/visuals, small model)
- 1 medium PBIP repo (typical business report)
- 1 large-ish PBIP repo (many visuals/pages, larger model)

Each fixture set should include old/new variants that exercise:

- GUID churn
- ordering changes
- real semantic changes (property edits)
- add/remove entities

### Golden tests

- Normalization golden files:
  - input -> normalized output snapshot must match
- Diff golden files:
  - old/new -> expected change kind counts

### Robustness tests

- Fuzz JSON inputs for canonicalizer (bounded recursion, stack-safe)
- Malformed files must produce friendly errors

### UI snapshot coverage

- Add a deterministic UI scenario for PBIP:
  - loads fixture
  - runs compare
  - navigates to one changed document
  - captures Summary + Details screenshots

---

## Performance Plan

Iteration 2 introduces new IO and parsing workloads. Treat it as perf-risk.

- Add new perf targets:
  - scan + normalize time
  - diff time
  - peak memory estimate (rough is fine initially)
- Establish baselines once Phase 1 is implemented:
  - add to quick/gate suites if stable
- Run a full perf cycle when:
  - core normalization/diff primitives change materially
  - storage schema changes
  - large fixture behavior changes

---

## Implementation Milestones (Detailed Checklist)

### Milestone 0: Decisions + Interfaces (1-2 days)

- [x] Confirm Iteration 2 MVP UX: document-first viewer (changed files list + normalized details).
- [x] Lock the stable cross-domain contracts:
  - [x] `DiffDomain`
  - [x] `SelectionTarget`
  - [x] `NavigatorModel`
  - [x] `DetailsPayload`
- [x] Decide normalization profiles (Strict/Balanced/Aggressive) and document their semantics.
- [x] Create/confirm fixtures to use as “golden” inputs.

Acceptance:

- Written spec for the above contracts and profiles (this doc + any DTO stubs).

### Milestone 1: PBIP Scan + Normalize (3-6 days)

- [x] Implement PBIP directory scan:
  - [x] identify PBIR and TMDL files
  - [x] ignore rules (configurable)
  - [x] stable, relative paths
- [x] Implement PBIR normalization (canonical JSON):
  - [x] key sorting
  - [x] stable numeric/string rendering
  - [x] deterministic GUID placeholder mapping (profile-gated)
- [x] Implement TMDL MVP normalization:
  - [x] line ending/whitespace normalization
  - [x] stable indentation (conservative)
- [x] Add golden tests for normalization.

Acceptance:

- Given a fixture repo, produce deterministic normalized artifacts and store them.

### Milestone 2: Document Diff + Summary (4-8 days)

- [x] Implement document-level diff:
  - [x] Added/Removed/Modified/Unchanged classification
  - [x] normalized content hashing
  - [x] details payload generation (old/new text)
- [x] Implement summaries:
  - [x] counts by doc type + change kind
  - [x] list of changed docs for NavigatorModel
- [x] Backend storage:
  - [x] store per-document metadata and details payload access
  - [x] size caps + graceful fallback for huge docs
- [x] Add tests for diff classification and summary counts.

Acceptance:

- Desktop backend can produce a PBIP diff run with a summary and a list of changed documents.

### Milestone 3: Desktop UI Integration (4-8 days)

- [x] Add PBIP mode to the desktop UI:
  - [x] directory pickers
  - [x] show/hide domain-specific controls
- [x] Implement PBIP Navigator:
  - [x] table of changed documents
  - [x] selection -> updates Details/Explain
- [x] Implement PBIP Details view:
  - [x] render normalized old/new
  - [x] display normalization profile applied
- [x] Implement PBIP Explain (MVP):
  - [x] deterministic PR-language summary per document
  - [x] never-crash fallback
- [x] Add UI scenario + snapshot capture.

Acceptance:

- A developer can compare two PBIP folders and review changes in Summary/Details/Explain.

### Milestone 4: Entity-Aware Navigation (Optional but Recommended Next) (4-10 days)

- [x] PBIR entity extraction:
  - [x] pages + visuals + bookmarks + themes (staged)
- [x] TMDL entity extraction:
  - [x] tables + measures + relationships (staged)
- [x] Introduce a tree-style NavigatorModel (or grouped table) driven by entities.
- [x] Explain upgrades:
  - [x] “this page changed because visual X changed property Y”
- [x] Add fixtures and golden tests for entity extraction.

Acceptance:

- Users can navigate by Power BI concepts rather than file paths.

---

## Git UX Kit (Deliverables)

Ship a small set of assets that make adoption easy:

- Recommended `.gitattributes` templates (diff drivers, text normalization hints).
- Suggested `git difftool` integration for PBIP mode.
- CLI commands (or wrappers) to generate PR-friendly output from PBIP diffs.

---

## Risk Register (Top Issues and Mitigations)

- Risk: Over-normalization hides real changes.
  - Mitigation: profiles + explicit “what rules were applied” in UI; strict mode always available.
- Risk: Arrays in PBIR JSON reorder in semantically irrelevant ways.
  - Mitigation: keep array order by default; introduce targeted stable-sorts only for known arrays with stable identity keys.
- Risk: Huge projects create DB bloat / slow UI.
  - Mitigation: size caps; store hashes/excerpts; lazy-load details; DB aggregation for summaries.
- Risk: UI becomes a mess of `if domain == ...`.
  - Mitigation: enforce the shell vs adapter boundary and keep cross-domain contracts small.

---

## Definition of "Iteration 2 MVP Shippable"

- PBIP compare works end-to-end in desktop UI with:
  - clear Summary
  - usable Details
  - non-broken Explain (best-effort)
- Deterministic normalization + stable results on re-run.
- Fixtures + golden tests exist and run in CI/local.
- Perf baseline exists for scan/normalize/diff on at least one medium fixture.

