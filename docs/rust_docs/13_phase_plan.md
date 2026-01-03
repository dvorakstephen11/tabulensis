1. **Add true end‑to‑end benchmarks: open + parse + diff on real artifacts**

   * Build a benchmark path that measures `WorkbookPackage::open` + diff (not just IR→diff), and records parse time explicitly.
   * Version the results alongside the existing benchmark JSON so performance claims are “resource‑truthful” over time.

2. **Make full‑scale perf/RC tests “real gates,” not ignored signals**

   * Promote at least one representative 50k/large‑file scenario into always‑run CI (and move the rest to scheduled/nightly if needed), with enforced thresholds.
   * Ensure the perf threshold tooling covers parse+diff and produces a clear pass/fail signal for regressions.

3. **Treat peak memory as a first‑class metric; add budget regression tests (incl. WASM)**

   * Turn “peak_memory_bytes under X for scenario Y” into an automated invariant; add regression tests for low‑similarity peak behavior.
   * Implement low‑similarity optimizations (early exits / avoid heavy metadata when bailout threshold is hit; reuse arenas/buffers).
   * Add explicit WASM budgets and a WASM‑targeted perf/memory test path.

4. **Lock down determinism + streaming sink lifecycle as non‑negotiable invariants**

   * Expand determinism tests to cover more entry points (including streaming + parallel paths) and enforce stable emission ordering.
   * Document + test sink lifecycle rules (“begin/emit/finish ordering; finish exactly once; no emit after finish; partial outputs + warnings semantics”).

5. **Refactor `DiffConfig` into nested sub‑configs while preserving presets + serde compatibility**

   * Introduce `AlignmentConfig` / `MoveConfig` / `PreflightConfig` / `HardeningConfig` / `SemanticConfig` (or equivalent) and keep aliases/serde round‑trip behavior.
   * Make presets (`fastest`, `balanced`, `most_precise`) the primary UX surface and keep validation centralized.

6. **Add direct “leaf diff” APIs; stop cloning workbooks to diff sheets/grids; re‑scope `Diffable`**

   * Add explicit `diff_grids(...)` / `diff_sheets(...)` routes (and streaming variants where useful) that don’t require temporary workbook construction.
   * Narrow or remove the current clone‑heavy `Diffable for Sheet/Grid` wrappers and make workbook orchestration an explicit workbook concern.

7. **Mechanically enforce architecture boundaries + clean up error taxonomy + strengthen parse→IR boundaries**

   * Enforce layering (subcrates or stricter `pub(crate)` discipline) to prevent “upward dependency” creep as more domains land.
   * Split “open/parse” errors from “diff/runtime” errors (reduce reliance on `PackageError::Diff`, which currently exists).
   * Make parse outputs “pure IR” for artifacts (charts, defined names, etc.) so parsing doesn’t embed diff assumptions.

8. **Harden fixtures + add the maintainer docs that lower ongoing design risk**

   * Make fixtures reproducible or validated by manifest/checksum in CI (no more missing‑file failures).
   * Add “entry‑point maps” and a short conceptual narrative doc (parse → IR → diff → output with key types) in `core/src/lib.rs` or `/docs`.

9. **Build a weird‑file robustness loop: corpus growth → fuzz findings → regression fixtures**

* Expand the corpus with “nasty enterprise files” for Excel/PBIX/DataMashup; treat crashes/misparses as top‑priority regressions.
* Formalize triage: every fuzz/corpus failure becomes a minimized fixture + test.

10. **Close the host‑parity gap while preserving “core purity” for WASM**

* Audit feature gating so host‑only dependencies don’t leak into core; keep optional domains cleanly removable.
* Align CLI/web/wasm/desktop on: presets/config semantics, output schema compatibility, streaming behavior, and perf/RC gates.

11. **Upgrade sheet identity in IR for rename‑robustness and future roadmap readiness**

* Preserve workbook‑internal sheet IDs when available; use them (with deterministic fallback) to improve rename detection and reduce name‑only ambiguity.
* Add targeted tests for duplicates/renames to ensure no determinism regressions.

12. **Deepen post‑MVP model diff: move from measure‑only to real tabular + DAX semantics**

* Expand model IR + diff beyond measure‑level changes (tables/columns/types/relationships; semantic comparison of expressions).
* Integrate PBIX “no DataMashup → use DataModelSchema” paths into testable, benchmarked end‑to‑end flows.

13. **Eliminate remaining high‑risk edges: DPAPI/permission bindings + any missing formats + ship‑grade release polish**

* Implement real parsing/validation for permission bindings / DPAPI (currently retained as raw bytes), with clear warnings + tests.
* Decide and execute on any intended missing container formats (e.g., XLSB) and wire them through the same RC/perf gates. 
* Final “ship” sweep: release workflows + docs + consistent behavior across hosts so the integrated maturity catches up with the strong core.
