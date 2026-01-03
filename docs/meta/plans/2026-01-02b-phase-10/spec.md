## Phase 10 implementation plan: Close the host‑parity gap while preserving core purity for WASM

Phase 10 in the roadmap is explicitly about (1) tightening feature gating so host‑only dependencies don’t leak into the `excel_diff` core, and (2) aligning CLI/web/WASM/desktop on config/presets, output schemas, streaming/large‑mode behavior, and perf/release gates. 

This plan is grounded in the current reality that:

* The CLI already has presets wiring (via `DiffConfig::fastest()`, `DiffConfig::most_precise()`, and default/balanced) and auto‑switches to JSONL at a fixed threshold. 
* Desktop already has “large mode” for workbooks, implemented via streaming to the SQLite op store, but **PBIX always runs in payload mode** today. 
* The web UI already normalizes “desktop outcome” vs “raw payload” results, but the worker path currently just returns a payload and ignores options.
* The WASM bindings always use a default config with `max_memory_mb = 256` and don’t accept a preset/config/options envelope from JS yet. 
* The core crate has a nontrivial feature surface (default features include `std-fs` and `vba`, and there is an explicit wasm guard for `parallel`). 
* “Host parity” is called out as a remaining uncertainty in the completion analysis: several features are present but not uniformly wired in every host.

---

# 10.0 Goals, non‑goals, and the contract we will enforce

### Goals

1. **Core purity for WASM remains true and becomes easier to prove.**
   The `excel_diff` crate must build for `wasm32-unknown-unknown` with a minimal, intentional feature set and without host‑only dependencies. This is already partially enforced in CI (via a wasm “smoke” build), and Phase 10 should strengthen it, not weaken it.

2. **A single, shared “diff options contract” is accepted by all hosts.**
   The same inputs + same config/preset should produce the same logical diff semantics (with explicit, documented differences only where the environment forces them—e.g., large mode storage on desktop). This is the “host parity” core of the phase.

3. **Output artifacts become interoperable.**
   A diff produced by one host can be opened/rendered by the others (within the constraints of “large mode”). This is already partially true: the web renderer can accept a raw `DiffReport` or a `{report, sheets, alignments}` payload. 

4. **Streaming/large‑mode behavior is coherent across platforms.**
   CLI JSONL streaming, desktop large‑mode storage, and browser WASM behavior should follow the same policy and expose the same “mode” decisions in the output envelope.

5. **Perf + RC gates prevent drift.**
   CI should catch parity regressions: feature leakage, schema breaks, large‑mode regressions, and incompatible config behavior.

### Non‑goals (explicitly out of scope for Phase 10)

* Rewriting the diff engine algorithms.
* Building a full “desktop store equivalent” for the browser (IndexedDB op store, etc.). We can lay hooks for it, but not require it to “ship” in Phase 10.
* Replacing the current web UI architecture; Phase 10 should use the existing `diffClient` abstraction and normalization path.

---

# 10.1 Establish a parity matrix and invariants (spec first, then code)

**Deliverable:** `docs/host_parity.md` (or similar), plus a “capability matrix” test harness in CI.

### 10.1.1 Write the parity matrix (what “parity” means in this repo)

Create a document that enumerates, for each host, the current and target state for:

* **Input kinds:** XLSX/XLSM/etc and PBIX/PBIT. (These are already recognized in `ui_payload::host_kind_from_name/path`.) 
* **Config surface:** presets, limits/hardening, and any host-specific defaults.
* **Output surface:**

  * CLI: `text`, `json` (DiffReport), `jsonl` streaming today. 
  * Web/WASM: returns a payload JSON string from `diff_files_with_sheets_json`. 
  * Desktop: returns `DiffOutcome { diffId, mode, payload?, summary? }`. 
* **Large-mode policy** (thresholds, when it triggers, and what artifact is returned). Desktop uses a fixed `AUTO_STREAM_CELL_THRESHOLD = 1_000_000` for workbooks. 
* **Streaming policy:** JSONL streaming (CLI), store streaming (desktop), none in browser today.
* **Versioning & schema:** DiffReport schema version, UI payload shape, desktop outcome shape.
* **Feature gates that materially change output:** `vba`, `model-diff`, `parallel`. 

The matrix is not just documentation—Phase 10 will use it as the “source of truth” for the CI gates in 10.6.

### 10.1.2 Define hard invariants

Add a short “invariants” section that the code must obey:

* Core must compile for wasm with the intended minimal features.
* A preset name means the same thing in every host.
* Output “mode” has the same meaning in every host:

  * `payload`: full report + optional previews/alignments.
  * `large`: summary-first artifact; details are streamed (CLI JSONL) or stored (desktop store) or deliberately unavailable (web, unless explicitly streamed for download).

---

# 10.2 Core purity: feature-gating audit and enforceability upgrades

Phase 10 calls out “audit feature gating so host-only dependencies don’t leak into core; keep optional domains cleanly removable.” 

This workstream has two parts: (A) auditing the actual feature boundaries, and (B) adding build/test gates so regressions are caught immediately.

## 10.2.1 Codify the “intended” feature sets (build profiles)

Today we already have distinct profiles in practice:

* **CLI** depends on `excel_diff` with `model-diff` and `parallel`, and (because it doesn’t disable default features) it also pulls `std-fs` and `vba` via core defaults.
* **WASM + ui_payload + desktop** depend on `excel_diff` with `default-features = false`, enabling `excel-open-xml` and `model-diff`.

Make those feature sets explicit in one place (doc + CI), and name them:

* `engine-host` = core default features + `model-diff` (+ `parallel` where desired)
* `engine-wasm` = `--no-default-features --features "excel-open-xml,model-diff"`

## 10.2.2 Audit for leakage (mechanical steps)

Run (locally and in CI) a feature/dependency inspection:

* `cargo tree -p excel_diff -e features` under:

  * default features
  * `--no-default-features --features excel-open-xml,model-diff`
  * `--no-default-features --features excel-open-xml` (to ensure model-diff is removable)
  * `--no-default-features --features model-diff` (to validate domain isolation, if that combo is intended)

Then act on findings:

* Any use of `std::fs`, OS paths, threads, or rayon must be behind:

  * `feature = "std-fs"` for filesystem
  * `feature = "parallel"` for rayon (already guarded; keep it that way) 
  * `cfg(not(target_arch = "wasm32"))` where the concept is truly non-wasm (e.g., some perf/memory metrics are already gated this way). 

## 10.2.3 Decide and implement parity stance on “VBA” support

This is a real parity gap today:

* Web view model can render VBA module ops (`VbaModuleAdded/Removed/Changed`). 
* But desktop and WASM builds currently disable core default features, which means **they likely do not include `vba` support unless explicitly re-enabled**.

**Phase 10 plan (pragmatic):**

1. **Desktop:** enable `excel_diff/vba` in the desktop engine build *if it builds cleanly and the size/cost is acceptable*. This brings desktop closer to CLI without touching WASM purity. (Desktop is not wasm, so this is low-risk to core purity.)
2. **WASM:** keep `vba` disabled by default unless it is proven wasm-compatible and within the wasm size budget. The wasm CI already enforces a size budget on the produced `.wasm` artifact. 
3. **Transparency:** regardless of whether we enable it, add an explicit “capabilities/features” report in host APIs (CLI `--version --verbose`, WASM `get_capabilities()`, desktop “about”) so users know whether VBA domain diffs are included.

This keeps “optional domains cleanly removable” true while making the parity story explicit rather than accidental.

## 10.2.4 Add enforceable CI gates for purity (no more “accidental parity”)

Add a CI job (or extend the existing wasm workflow) that must pass on every PR:

* `cargo check -p excel_diff --no-default-features --features "excel-open-xml,model-diff"` on the host toolchain (fast, catches accidental `std-fs` reliance)
* `cargo build --target wasm32-unknown-unknown --no-default-features -p excel_diff --bin wasm_smoke` (already exists; keep as hard gate) 
* `cargo check -p ui_payload` and `cargo check -p excel_diff_wasm --target wasm32-unknown-unknown` to ensure the “engine-wasm” set remains valid.

---

# 10.3 Presets/config semantics: one contract, wired everywhere

Phase 10 explicitly requires aligning “CLI/web/wasm/desktop on presets/config semantics”. 

Today:

* CLI builds config from flags and uses presets internally. 
* Desktop uses `DiffConfig::default()` unconditionally (no preset selection), and doesn’t accept a config/preset override from the UI.
* Web passes `options` into `diffClient.diff(...)`, but the worker ignores them, and wasm doesn’t accept them.

## 10.3.1 Define a shared options envelope (JS + Rust)

Create a single “options envelope” schema used across:

* web worker client
* native (tauri) client
* wasm binding functions
* desktop tauri command payloads
* CLI flags mapping

**Proposed shape (conceptual):**

* `preset`: `"fastest" | "balanced" | "most_precise"`
* `limits` (optional): `{ maxMemoryMb, timeoutMs, maxOps, onLimitExceeded }`
* `trusted` (desktop-only input handling; already exists as `trusted`). 
* Keep view-only settings like `ignoreBlankToBlank` in a separate UI options object (web uses it in rendering; it is not engine config today).

This envelope becomes the “truth” that flows through `diffClient` in both web and desktop.

## 10.3.2 Wire options through the web worker path (currently dropped)

Right now, worker receives `msg.options` but calls `diff_files_with_sheets_json(...)` without using it. 

Plan:

1. Add a new wasm export that accepts an options JSON/object and returns an outcome (see 10.4):

   * `diff_files_outcome_json(oldBytes, newBytes, oldName, newName, optionsJson)`
     (Keep the existing `diff_files_with_sheets_json` export for backward compatibility and internal tests.)
2. Update `web/diff_worker.js` to:

   * parse/use `msg.options`
   * call the new wasm export
   * return the normalized outcome shape (mode/payload/summary)

This change closes one of the most concrete parity gaps in the repo today.

## 10.3.3 Wire options through the desktop path

Desktop’s tauri side currently defines a small `DiffOptions` with `ignore_blank_to_blank` and `trusted`. 
But the engine config is built as default in `diff_runner`. 

Plan:

1. Extend the tauri `DiffOptions` to include:

   * `preset`
   * `limits` (max memory, timeout, max ops, onLimitExceeded)
   * optional `configJson` for advanced overrides (if you want that UI later)
2. In `desktop/src-tauri/src/diff_runner.rs`, build the effective config:

   * base = preset (balanced by default)
   * apply limits overrides into `DiffConfig.hardening`
3. Persist the **effective config JSON** into the store (`config_json` already exists in `diff_runs`). 
4. Emit the effective config (or at least preset name) back in the outcome metadata so UI can show “what was used”.

## 10.3.4 Update CLI semantics to match the same contract

CLI currently uses `--fast`/`--precise` and hardening flags. 

Plan:

* Add `--preset {fastest|balanced|most-precise}` (or `most_precise` to match config naming).
* Keep `--fast` and `--precise` as compatibility shims that map to preset.
* Ensure the default preset is “balanced” everywhere (CLI default currently corresponds to `DiffConfig::default()`). 
* Ensure any host-specific defaults (like wasm’s `max_memory_mb = 256`) are expressed as **host defaults**, not “preset semantics”. 

**Key principle:** “preset” changes algorithmic behavior; “limits” change safety/perf constraints. Don’t conflate them.

---

# 10.4 Output schema compatibility: make artifacts portable

Phase 10 requires “output schema compatibility”. 
The repo already has multiple related schemas:

* `DiffReport` (core JSON): versioned, carries `ops` and `strings`. 
* UI payload: `ui_payload::DiffWithSheets { report, sheets, alignments }` (used by desktop payload mode and by wasm web path).
* Desktop “envelope”: `DiffOutcome { diff_id, mode, payload?, summary? }`. 
* CLI streaming: JSONL contract with header+ops (and desktop store is conceptually equivalent: strings stored + ops rows).

## 10.4.1 Standardize on an “Outcome envelope” across hosts

Web already expects two cases and normalizes them:

* if `result.mode` exists, treat as outcome
* else treat as payload mode and wrap it 

Phase 10 should make **every host return the same envelope**:

* Web worker returns outcome
* Desktop returns outcome (already)
* WASM bindings can return outcome JSON
* CLI optionally can output outcome JSON (new format), enabling “portable artifact” flow.

### Proposed shared types location

To avoid polluting core with UI/host specifics while still sharing types:

* Put `DiffOutcome` (portable version) into `ui_payload` as a new module, e.g. `ui_payload::outcome`.

  * It’s already depended on by both wasm and desktop.
  * It does not require `tauri` or `rusqlite`.

Desktop can keep its internal `diff_id` semantics, but the JSON it returns to the UI should match the shared schema.

## 10.4.2 Add a CLI output format that produces UI‑loadable JSON

Today CLI JSON output is a `DiffReport`, which the web renderer can render (with “missing snapshots” status). 
To truly close the schema gap, CLI should be able to output the same payload used by the UI:

* Add CLI format: `--format=payload`
  Output `ui_payload::DiffWithSheets` JSON for workbook diffs (including snapshots and alignments), and a reasonable payload for PBIX (likely just `report` with empty sheets/alignments).

* Add CLI format: `--format=outcome`
  Output a `DiffOutcome` envelope:

  * `mode: "payload"` + payload for small diffs
  * `mode: "large"` + summary (and maybe “download jsonl separately”) for huge diffs

This makes “CLI -> open in viewer” a first-class workflow and guarantees the schema stays compatible with web/desktop.

## 10.4.3 Versioning strategy

Avoid surprise breaks:

* Keep `DiffReport.version` as the canonical schema version for report semantics. 
* Treat UI payload/outcome as “derived schemas” that embed a `report` and thus inherit the report version.
* If you introduce new top-level fields (like `capabilities`), keep them additive and optional.
* Add a small “compatibility test suite” that loads older sample JSON artifacts (fixtures) into `buildWorkbookViewModel` and `renderReportHtml`.

---

# 10.5 Streaming and large‑mode alignment

This is the hardest “real codebase” part because the hosts have fundamentally different storage constraints.

### Current reality

* CLI has a threshold-based switch to JSONL output to avoid huge JSON reports in-memory. 
* Desktop has large mode for workbooks, streaming ops to SQLite via `OpStoreSink`, and returns a summary object that the web UI can render as “Large Mode Summary.”
* Desktop **does not** do large mode for PBIX yet (it always builds a full report and inserts ops). 
* Web/WASM always returns full payload; no large mode. 

## 10.5.1 Unify the large‑mode decision policy (one constant, one explanation)

Desktop and CLI both use `AUTO_STREAM_CELL_THRESHOLD = 1_000_000` but it’s duplicated.

Plan:

* Move this constant into a “pure” place shared by all hosts:

  * Best option: `excel_diff::limits` or `excel_diff::policy` module in core, since it’s just a number and doesn’t carry host deps.
* Provide a single helper:

  * `should_use_large_mode(estimated_cell_volume: u64, config: &DiffConfig) -> bool`
* Use it in:

  * CLI auto-switch (`Json` -> `Jsonl`)
  * Desktop mode selection
  * WASM mode selection (new)

## 10.5.2 Desktop: add PBIX large‑mode parity

Desktop currently always runs PBIX in payload mode and stores ops after the fact. 

Plan:

1. Implement PBIX streaming path:

   * Use `PbixPackage::diff_streaming_*` (or whatever streaming API core provides) with an `OpStoreSink`, just like workbook large mode. (Desktop already imports `PbixPackage` and `DiffSink` and uses store sinks.) 
2. Decide the trigger:

   * If PBIX diffs are generally small, you can still stream always for PBIX to simplify semantics (streaming isn’t expensive and keeps memory bounded).
   * Or define a PBIX-specific heuristic (e.g., based on embedded DataMashup size), but keep it simple for Phase 10.
3. Return `mode: "large"` + summary for PBIX runs when streaming is used.
4. Ensure the UI behaves:

   * In desktop large mode, the UI can load sheet payloads on demand; PBIX has no sheets, so it should remain a summary-only experience (and the UI already supports summary-only).

## 10.5.3 Web/WASM: implement “large mode” as a first-class result

Web already has UI for large-mode summary (it renders `summary.counts` and `summary.sheets`).
What’s missing is an engine path that produces that summary without producing a massive payload.

Plan:

1. Add a new wasm export that returns a `DiffOutcome` envelope (JSON string) and accepts options:

   * If small: `mode="payload"`, compute `DiffWithSheets` exactly as today (`diff_files_with_sheets_json`), plus optional summary.
   * If large: `mode="large"`, **do not** build sheet snapshots/alignments; instead compute a summary via streaming.

2. Implement summary computation in Rust (WASM-safe):

   * Create a lightweight `CountingSink` that implements `DiffSink` and only:

     * increments total op count
     * classifies ops into added/removed/modified/moved counts
     * aggregates per-sheet counts and per-sheet op counts
   * This mirrors what desktop’s store sink does while writing ops (it already computes counts and sheet stats as it stores).

3. Emit a `summary` object shaped like what the web large-mode UI expects:

   * `opCount`
   * `warnings` (if incomplete)
   * `counts { added, removed, modified, moved }`
   * `sheets[] { sheetName, opCount, counts }` 

4. Ensure metadata doesn’t break web:

   * In web large mode, the current code builds meta from summary using `oldPath/newPath`.
   * For browser large mode, either:

     * populate `oldPath/newPath` with file names, or
     * adjust `buildMetaFromSummary` to fall back to file names when paths are missing (recommended, because it also makes the desktop/web code more robust).

5. Optional (but highly valuable parity win): expose “download JSONL” for browser large mode

   * Since the browser can’t use the desktop SQLite store, the parity analogue to “large mode details on demand” is “large mode details downloadable as a JSONL stream.”
   * Implement a second wasm export that streams JSONL chunks via a JS callback (or returns chunks iteratively through worker messages).
   * This should follow the same JSONL contract that CLI uses (header + ops). Desktop store is already conceptually aligned (strings + ops).

## 10.5.4 Make “mode” visible and consistent

After this work:

* Desktop always returns an outcome with a `mode` field (already true). 
* Web worker should return an outcome with `mode`, not a raw payload. (JS already supports both, but parity means it should always be outcome.) 
* CLI should optionally emit outcome envelopes for portability.

---

# 10.6 Perf + RC gates: stop parity regressions at the source

Phase 10 explicitly calls out aligning “perf/RC gates”. 
The repo already has strong wasm gates (size budget + wasm smoke build). 
Phase 10 should extend that discipline to cross-host parity.

## 10.6.1 Build matrix gates (fast, cheap, effective)

Add a CI job that runs on every PR:

* `cargo check -p excel_diff` (default features)
* `cargo check -p excel_diff --no-default-features --features "excel-open-xml,model-diff"`
* `cargo check -p ui_payload`
* `cargo check -p excel_diff_wasm --target wasm32-unknown-unknown`
* `cargo check -p excel_diff_desktop` (at minimum; full Tauri build can be optional if the environment is heavy)

This catches core-purity regressions and host wiring regressions early.

## 10.6.2 Schema compatibility gates (web renderer as the canary)

Add a “compat test” that proves the viewer can open artifacts produced by other hosts:

* Generate a CLI output (new `--format=payload` and/or `--format=outcome`) from a small fixture workbook.
* Run Node tests that call `buildWorkbookViewModel(...)` and `renderReportHtml(...)` on that JSON. (There are already JS test files and a renderer path in the web directory.) 
  This test is an extremely practical “schema gate”: if you break payload schema, the web tests fail.

## 10.6.3 Large-mode gate

Add a regression test that ensures the large-mode trigger still works:

* Construct/fixture a workbook whose estimated cell volume is > 1,000,000
* Confirm:

  * CLI chooses JSONL (or emits `mode=large` if using outcome output)
  * Desktop chooses large mode for workbooks
  * WASM returns `mode=large` and does not attempt snapshots/alignments

This ensures the worst-case behavior remains safe.

## 10.6.4 WASM size/memory budgets remain hard gates

Keep the existing wasm workflow size check and memory bench gates. Any parity work that increases wasm size should be justified, measured, and accounted for here. 

---

# 10.7 Ordered execution plan (what to build first so you don’t fight the code)

This is the recommended sequencing to keep integration smooth and reduce backtracking:

### Step 1 — Document the contract and add the CI “feature matrix” checks

Do this before changing behavior; it prevents accidental regressions while refactoring.

### Step 2 — Introduce the shared “options envelope” in JS and pass it everywhere

* Web: UI → diffClient → worker message (already passes options; keep) 
* Desktop: UI → native_diff_client → tauri invoke options (already exists; extend)
* CLI: flags → envelope mapping (internal)

No engine changes yet; just plumbing.

### Step 3 — Add wasm export that accepts options and returns an outcome envelope

This closes the “worker ignores options” parity gap quickly and lets you standardize the JS side around outcomes.

### Step 4 — Update web worker to always return outcomes

This simplifies the UI layer and makes the desktop and web paths symmetric. 

### Step 5 — Desktop: apply preset/limits to engine config and persist effective config

Now desktop semantics match CLI and web; this also makes debugging stored diffs far easier because the store already records `config_json`.

### Step 6 — Implement WASM large-mode + CountingSink summary

This is the biggest correctness/perf win for browser safety, and it aligns behavior with the existing web “large summary” UI.

### Step 7 — Desktop PBIX streaming parity

Make PBIX behave like workbook diffs in terms of memory safety and mode selection. 

### Step 8 — CLI payload/outcome output formats (optional but high leverage)

This creates true schema portability across hosts. It also provides a golden artifact for CI schema gates and makes web/desktop renderer testing much easier.

### Step 9 — Add schema + large-mode CI gates, then tighten the docs

Once everything is wired, lock it in with CI and update the parity matrix to reflect the new guarantees. 

---

# 10.8 Definition of Done (concrete, testable)

Phase 10 is “done” when all of the following are true:

### Core purity / features

* `excel_diff` builds for wasm with minimal features and without default features (existing smoke build + new host checks).
* Feature leakage is prevented by CI (build matrix) and documented in `docs/host_parity.md`.

### Presets/config parity

* Web worker path actually applies preset/limits options (no longer ignored).
* Desktop applies preset/limits options to the engine config (no longer always default).
* CLI supports `--preset` and maps old flags cleanly; defaults are consistent with “balanced” semantics. 

### Output schema parity

* Web + desktop both consume a consistent outcome envelope shape (web no longer has a “special” worker payload shape in practice).
* CLI can emit UI-loadable JSON (payload and/or outcome), and web tests can render it without code changes.

### Streaming / large mode parity

* Desktop large mode exists for both workbook and PBIX (or PBIX is explicitly documented as always-streamed). 
* WASM returns `mode="large"` for large workbooks and produces summary without snapshots/alignments.
* CLI/desktop/WASM share the same large-mode threshold policy (or the divergence is explicitly documented).

### Perf/RC gates

* CI includes: feature matrix checks, wasm smoke + size budget, schema compatibility checks using the web renderer, and at least a build check of the desktop crate.

---

If you’d like, I can also turn this Phase 10 plan into a checklist-style “issue tree” (epics → tasks → sub-tasks) using the actual file/module names as task titles, but the plan above is already structured so each section maps directly to concrete code touch points in this repo (`web/diff_worker.js`, `wasm/src/lib.rs`, `desktop/src-tauri/src/diff_runner.rs`, core feature gates, and CI workflows).
