## What Branch 3 is asking for

Branch 3 (“Configuration Infrastructure”) has two big outcomes:

1. A single, central `DiffConfig` that owns all tunable knobs (limits, thresholds, feature toggles), with defaults and presets, plus a builder and serde support for persistence. 
2. Every relevant diff entrypoint and algorithm path must accept `&DiffConfig` (no internal “grab default config” shortcuts), and hardcoded threshold constants in the diff algorithms must be removed in favor of config fields. Tests must be updated and expanded to prove config changes behavior.

## Codebase reality check (important for the agent)

The codebase already contains a `core/src/config.rs` with `DiffConfig`, defaults, and presets (`fastest`, `balanced`, `most_precise`), and much of the pipeline is already “with_config”-style. 

However, Branch 3 is not complete in the codebase because:

* Several public-ish helpers still have “no-config wrappers” that call `DiffConfig::default()` (e.g., `detect_exact_rect_block_move`, `detect_exact_row_block_move`, `detect_exact_column_block_move`, `align_row_changes`, etc.).
* Some exported entrypoints still don’t accept config (notably `diff_grids_database_mode`). 
* There are still hardcoded algorithm limits/constants (e.g., `MAX_LCS_GAP_SIZE`, `LCS_DP_WORK_LIMIT`, `MAX_SLICE_LEN`, `MAX_CANDIDATES_PER_SIG`, and a hardcoded `<= 256` move-detection column gate).
* Branch 3 requires a builder pattern + serde; these aren’t present yet in `DiffConfig`.
* `diff_m_queries` doesn’t accept config today. 

The plan below is written so an agent can finish Branch 3 by extending what’s already there, while using the codebase as the source of truth for naming/behavior.

---

# Implementation plan for Branch 3

## Phase 0 — Guardrails and audit checklist

### 0.1 Establish baselines

* Run full test suite and benches before changes:

  * `cargo test --workspace`
  * `cargo test --workspace --all-features`
  * `cargo bench` (if benches are used as regression signals)

### 0.2 Create two “audit queries” the agent will repeatedly run during the work

1. Find any default-config shortcuts that violate “all diff functions accept `&DiffConfig`”:

* `rg "DiffConfig::default\(\)" core/src`
* `rg "with_config\(.*DiffConfig::default" core/src`

2. Find algorithm hardcoded thresholds to migrate into `DiffConfig`:

* `rg "const (MAX|.*_LIMIT|.*THRESHOLD|.*CAP|.*CANDIDATES)" core/src`
* `rg "(<=|>=|<|>)\s*256\b" core/src` (and similar common magic numbers)

The goal is to end with *only* “definition” constants (hash mixing constants, etc.) outside config, and *no* behavioral knobs embedded in algorithm modules. Branch 3 explicitly requires this. 

---

## Phase 1 — Finish `DiffConfig`: serde + builder + missing knobs

### 1.1 Add serde support to `DiffConfig` and `LimitBehavior`

Branch 3 requires config persistence via serde.

**Changes: `core/src/config.rs`**

* Add derives:

  * `#[derive(Debug, Clone, Serialize, Deserialize)]` on `DiffConfig`
  * `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]` on `LimitBehavior`
* Add `#[serde(default)]` on `DiffConfig` to allow partial JSON to fill from defaults.
* Add `#[serde(rename_all = "snake_case")]` to `LimitBehavior` so JSON is stable.

**Compatibility aliasing (recommended)**
The sprint plan’s field names differ slightly from the codebase’s current ones (example: “rare_frequency_threshold” vs `rare_threshold`). Since the codebase is the source of truth, keep current field names but add `#[serde(alias = "...")]` for any spec or older names you want to accept. This avoids churn while meeting “persistence” goals.

Concrete aliases worth adding based on the Branch 3 spec text:

* `rare_threshold` should accept alias `rare_frequency_threshold`. 
* `low_info_threshold` should accept alias `low_info_cell_threshold`. 
* If the sprint text uses `recursive_threshold` anywhere, alias it to `recursive_align_threshold`.

### 1.2 Add missing “Branch 3 spec” fields (even if some are only wired later)

Branch 3’s spec includes feature toggles and output controls that aren’t in the current struct. 
Add them now so the struct matches the branch intent and so serde persistence is future-proof:

Add fields to `DiffConfig` (types shown as recommendation; match project style):

* `pub enable_fuzzy_moves: bool`
* `pub enable_m_semantic_diff: bool`
* `pub enable_formula_semantic_diff: bool`
* `pub include_unchanged_cells: bool`
* `pub max_context_rows: u32`
* `pub min_block_size_for_move: u32`

**Defaults to preserve current behavior**

* Set defaults so existing behavior is unchanged unless explicitly configured.

  * `enable_fuzzy_moves: true` (current engine uses fuzzy move detection in masked mode) 
  * `enable_m_semantic_diff: true` (current `diff_m_queries` uses semantic gating behavior) 
  * `enable_formula_semantic_diff: false` (formula AST diff isn’t implemented yet; keep off)
  * `include_unchanged_cells: false` (engine doesn’t emit unchanged ops today)
  * `max_context_rows: 3` (matches spec default; harmless until used)
  * `min_block_size_for_move`: choose a default that does not break existing move expectations; if in doubt, start at `1` (per current move behaviors and tests that treat single-column or small moves as meaningful). The spec suggests `3`, but codebase behavior appears to rely on smaller moves being detectable.

### 1.3 Add config fields to eliminate existing hardcoded algorithm constants

These are already hardcoded in algorithm modules and must move into `DiffConfig` to satisfy Branch 3 acceptance. 

Add fields (with defaults matching current constants):

* `pub max_lcs_gap_size: u32`

  * Default `1500` (replaces `MAX_LCS_GAP_SIZE`).
* `pub lcs_dp_work_limit: u32` (or `usize`)

  * Default `20_000` (replaces `LCS_DP_WORK_LIMIT`). 
* `pub move_extraction_max_slice_len: u32` (or `usize`)

  * Default `10_000` (replaces `MAX_SLICE_LEN`). 
* `pub move_extraction_max_candidates_per_sig: u32` (or `usize`)

  * Default `16` (replaces `MAX_CANDIDATES_PER_SIG`). 
* `pub context_anchor_k1: u32` and `pub context_anchor_k2: u32`

  * Defaults `4` and `8` (replaces hardcoded `discover_context_anchors(..., 4)` / `(..., 8)`). 
* `pub max_move_detection_cols: u32`

  * Default `256` (replaces hardcoded `<= 256` in move detection gate). 

### 1.4 Update presets to include the new fields

`DiffConfig::fastest()`, `balanced()`, `most_precise()` already exist and must be extended to set new fields explicitly or inherit via `..Default::default()`. 

Suggested preset intent (keep consistent with current preset philosophy in code):

* `fastest()`:

  * `enable_fuzzy_moves = false`
  * `enable_m_semantic_diff = false` (avoid parsing M)
  * possibly lower `max_move_iterations` further (already reduced) 
  * keep safety limits smaller where appropriate (e.g., `max_lcs_gap_size`, `move_extraction_max_slice_len`)
* `balanced()`:

  * Leave feature toggles on, keep defaults
* `most_precise()`:

  * `enable_fuzzy_moves = true`
  * `enable_m_semantic_diff = true`
  * increase limits (bigger `max_lcs_gap_size`, maybe higher candidate caps) where accuracy benefits

### 1.5 Implement the builder pattern

Branch 3 explicitly requires a builder.

**Recommended shape**

* `pub struct DiffConfigBuilder { inner: DiffConfig }`
* `impl DiffConfig { pub fn builder() -> DiffConfigBuilder }`
* Builder methods:

  * One setter per field (or per logical group)
  * `pub fn build(self) -> Result<DiffConfig, ConfigError>` with validation

**Validation rules (minimal, but aligned with configuration spec principles)**
The unified spec explicitly calls out validation and clear errors. 
Validate at least:

* `0.0 <= fuzzy_similarity_threshold <= 1.0` and finite
* `max_align_rows > 0`, `max_align_cols > 0`
* `max_lcs_gap_size >= 1`
* any “limit” fields that must be nonzero for algorithm correctness (if any)

Implementation detail: introduce `ConfigError` in `config.rs` (simple enum/string) or reuse existing error patterns.

### 1.6 Add unit tests for serde + builder

Add tests in `core/src/config.rs` or a `core/tests/config_tests.rs`:

* `serde_json` roundtrip:

  * serialize `DiffConfig::default()`, deserialize, assert key fields match
* alias deserialize:

  * JSON containing `rare_frequency_threshold` populates `rare_threshold`
* builder validation:

  * setting `fuzzy_similarity_threshold = 2.0` returns error
* presets sanity:

  * `fastest()` differs from `most_precise()` in expected directions

---

## Phase 2 — Make config mandatory in all diff APIs (remove default wrappers)

Branch 3’s signature updates are explicit. 
In this codebase, that translates to: remove the pattern “public function without config calls `_with_config(..., &DiffConfig::default())`”, and instead make the “canonical” function require `&DiffConfig`.

### 2.1 Engine entrypoints

**Target file: `core/src/engine.rs`** 

Do the following transformations:

1. Make `diff_workbooks` require config:

* Today you have:

  * `diff_workbooks(old, new)` which internally uses default config
  * `diff_workbooks_with_config(old, new, config)`
* End-state:

  * `pub fn diff_workbooks(old, new, config: &DiffConfig) -> DiffReport`
  * `pub fn try_diff_workbooks(old, new, config: &DiffConfig) -> Result<DiffReport, DiffError>`
* Optional: keep a convenience wrapper named something explicit like `diff_workbooks_default(old, new)` if desired, but Branch 3 acceptance is easiest if **no primary diff entrypoint lacks config**. 

2. Make the internal grid diff helper require config:

* Today `diff_grids(...)` exists and calls `try_diff_grids_with_config(..., &DiffConfig::default())`. 
* Change it to `diff_grids(old, new, config: &DiffConfig, ...)` and propagate.

3. Update `diff_grids_database_mode` to accept config:

* Currently signature does not take config. 
* Update to:

  * `pub fn diff_grids_database_mode(old, new, key_columns, config: &DiffConfig) -> DiffReport`
* Ensure fallback from database mode to positional mode also passes the *same config* (no default construction).

4. Fix move-detection gating constant:

* Replace `old.ncols.max(new.ncols) <= 256` with `<= config.max_move_detection_cols`. 
* Also wire `config.enable_fuzzy_moves` into the move detection loop:

  * keep exact moves always eligible
  * only attempt fuzzy row-block move if enabled (it’s currently unconditional as the last step). 

### 2.2 Algorithm modules: remove “no-config wrappers” and rename `_with_config` functions

You have multiple modules with this pattern:

* `row_alignment.rs`: `detect_exact_row_block_move` calls `_with_config(..., &DiffConfig::default())`, same for fuzzy + `align_row_changes`.
* `column_alignment.rs`: `detect_exact_column_block_move` wrapper. 
* `rect_block_move.rs`: `detect_exact_rect_block_move` wrapper. 

**End-state policy**

* The “real” function name should require config.
* If you keep wrappers at all, they should be:

  * either `#[cfg(test)]` only, or
  * renamed to `*_default()` to avoid violating “all diff functions accept config”.

**Concrete steps (repeat per module)**

* Rename:

  * `detect_exact_rect_block_move_with_config` → `detect_exact_rect_block_move`
  * `detect_exact_row_block_move_with_config` → `detect_exact_row_block_move`
  * `detect_fuzzy_row_block_move_with_config` → `detect_fuzzy_row_block_move`
  * `detect_exact_column_block_move_with_config` → `detect_exact_column_block_move`
  * `align_row_changes_with_config` → `align_row_changes`
* Delete (or `cfg(test)`-gate) the old wrapper functions that call `DiffConfig::default()`.

This directly satisfies the “all diff functions accept `&DiffConfig`” requirement. 

### 2.3 Update `diff_m_queries` signature and wire the new toggle

**Target file: `core/src/m_diff.rs`** 

* Change:

  * `pub fn diff_m_queries(old, new) -> Vec<QueryDiff>`
    to
  * `pub fn diff_m_queries(old, new, config: &DiffConfig) -> Vec<QueryDiff>`

* Use `config.enable_m_semantic_diff` to control the “semantic gate”.

  * When enabled (default), preserve current behavior: formatting-only changes (semantically equal) do not emit diffs. 
  * When disabled, treat any textual definition change as a `DefinitionChanged` (even if semantically equal), avoiding parse/canonicalization cost.

This gives you a clean “performance vs noise” knob that Branch 3 wants. 

### 2.4 Public re-exports and docs

**Target file: `core/src/lib.rs`** 

* Update `pub use` exports to match the new function names/signatures.
* Update doc examples so they show passing `&DiffConfig::default()` or a preset.

**Target file: `core/src/output/json.rs`**

* If `output::json::diff_workbooks(...)` calls engine `diff_workbooks` without config, update it to accept config (or explicitly use `DiffConfig::default()` *only if* this function is not considered part of “diff APIs” in Branch 3; safest is to accept config and thread through). 

---

## Phase 3 — Remove hardcoded constants by threading new config fields into alignment

This is where Branch 3 intersects performance work already done in Branch 1/2: the current code has hard caps embedded in alignment helpers.

### 3.1 Replace `MAX_LCS_GAP_SIZE` with `config.max_lcs_gap_size`

**Target file: `core/src/alignment/gap_strategy.rs`** 

* Remove `pub const MAX_LCS_GAP_SIZE: u32 = 1500;`
* In `select_gap_strategy`, replace:

  * `min(MAX_LCS_GAP_SIZE)` and `> MAX_LCS_GAP_SIZE` comparisons
    with
  * `min(config.max_lcs_gap_size)` and `> config.max_lcs_gap_size`

### 3.2 Make `align_small_gap` accept config; replace `LCS_DP_WORK_LIMIT`

**Target: `core/src/alignment/assembly.rs` (where `align_small_gap` appears)** 
You currently call `align_small_gap(old_slice, new_slice)` from `fill_gap`. 

Steps:

* Change `align_small_gap(old_slice, new_slice)` → `align_small_gap(old_slice, new_slice, config)`
* Update function signature accordingly.
* Replace `LCS_DP_WORK_LIMIT` with `config.lcs_dp_work_limit`. 
* Replace any remaining checks against `MAX_LCS_GAP_SIZE` with `config.max_lcs_gap_size`.

### 3.3 Move extraction constants into config

**Target file: `core/src/alignment/move_extraction.rs`** 

* Replace `MAX_SLICE_LEN` and `MAX_CANDIDATES_PER_SIG` with:

  * `config.move_extraction_max_slice_len`
  * `config.move_extraction_max_candidates_per_sig`
* Update functions that rely on these:

  * `find_block_move(...)` should accept `config: &DiffConfig` instead of using constants.
* Then update call sites (notably in `fill_gap` in `assembly.rs`) to pass config. 

### 3.4 Remove hardcoded `discover_context_anchors(..., 4/8)` values

**Target file: `core/src/alignment/assembly.rs`** 

* Replace:

  * `discover_context_anchors(old_slice, new_slice, 4)`
  * `discover_context_anchors(old_slice, new_slice, 8)`
* With:

  * `discover_context_anchors(old_slice, new_slice, config.context_anchor_k1 as usize)`
  * `discover_context_anchors(old_slice, new_slice, config.context_anchor_k2 as usize)`

### 3.5 Replace `find_block_move(..., 1)` with `config.min_block_size_for_move`

In `GapStrategy::MoveCandidate`, if there’s a nonzero offset and no moves were found from matched pairs, the code currently tries `find_block_move(old_slice, new_slice, 1)`. 
Replace that `1` with `config.min_block_size_for_move`.

This makes the “emit tiny move blocks?” behavior configurable without re-editing code later.

---

## Phase 4 — Test migration + new tests proving config matters

Branch 3 requires updating tests and adding tests demonstrating config tradeoffs. 

### 4.1 Mechanical test updates after signature changes

You will need a broad update because tests currently call things like:

* `diff_workbooks(&wb_a, &wb_b)` (no config) 
* `detect_exact_rect_block_move(&old, &new)` (no config) 
* `detect_exact_column_block_move(old, new)` (no config wrapper) 
* `align_row_changes(old, new)` (no config wrapper) 

After the refactor:

* Replace all these with the config-required versions, usually passing `&DiffConfig::default()`.

Do this in two passes:

1. Fix compile errors by adding `&DiffConfig::default()` everywhere.
2. Then selectively switch tests that are about performance/precision to use `DiffConfig::fastest()` / `balanced()` / `most_precise()`.

### 4.2 Add “config toggles behavior” tests (minimum set)

1. **M semantic diff toggle**

* Add a test where M query text changes only in formatting (whitespace/newlines).
* With `enable_m_semantic_diff = true`, expect **no** `DefinitionChanged`.
* With `enable_m_semantic_diff = false`, expect a `DefinitionChanged`. 

2. **Fuzzy move detection toggle**

* Construct a grid case where only fuzzy row-block move detection would succeed (not exact).
* With `enable_fuzzy_moves = true`, expect a row move op.
* With `enable_fuzzy_moves = false`, expect it falls back to cell edits / positional changes.

This ties directly to the engine’s masked move detection loop that currently attempts fuzzy row moves as a final step. 

3. **`max_move_detection_cols` gate**

* Build a grid with `ncols = 300` and an obvious rect/row move.
* With `max_move_detection_cols = 256` (default), move detection should be disabled by the gate; no move ops should appear.
* With `max_move_detection_cols = 512`, move detection should activate; move ops should appear. 

4. **Gap strategy cap is config-driven**

* Unit-test `select_gap_strategy` or `fill_gap` logic:

  * Set `config.max_lcs_gap_size` small (e.g., 10)
  * Use slices of size 11 and assert it picks the hash fallback path.
    This directly replaces the previous `MAX_LCS_GAP_SIZE` constant behavior.

### 4.3 Add serde roundtrip tests for persistence

* `serde_json::to_string(&DiffConfig::most_precise())` should succeed.
* Deserialize that JSON back and confirm key fields match.
* Include at least one test that deserializes using an alias field name (e.g., `rare_frequency_threshold`) and ensures the struct populates correctly.

---

## Phase 5 — Final audits to satisfy Branch 3 acceptance

Branch 3 acceptance criteria checklist: 

### 5.1 “All diff functions accept `&DiffConfig`”

* Confirm there are no remaining “wrapper diff functions” without config in production code:

  * `rg "pub .*fn .*\\(" core/src | rg -v "config: &DiffConfig" | rg "diff_|align_|detect_"` (tweak as needed)
* Ensure any remaining no-config functions are:

  * renamed (`*_default`) or
  * test-only.

### 5.2 “No hardcoded algorithm constants remain”

* Re-run the constant scans from Phase 0.
* Confirm the specific known offenders are gone from algorithm modules:

  * `MAX_LCS_GAP_SIZE`
  * `LCS_DP_WORK_LIMIT` 
  * `MAX_SLICE_LEN`, `MAX_CANDIDATES_PER_SIG` 
  * move-detection `<= 256` gate 
  * context anchor `4`/`8` literals 

### 5.3 Presets + serialization

* Ensure presets compile after adding new fields and have stable behavior.
* Confirm serde derives compile in both default and `--all-features`.

### 5.4 Run formatting and lint

* `cargo fmt`
* `cargo clippy --workspace --all-targets --all-features -D warnings` (or project standard)

---

# Deliverable mapping (so the agent can track completion)

* **3.1 DiffConfig Definition**

  * `DiffConfig` exists; extend with missing spec fields + fields replacing remaining constants; add serde derives; add builder + validation; update presets and default.

* **3.2 Thread config through pipeline**

  * Update signatures of `diff_workbooks`, `diff_grids_database_mode`, `diff_m_queries`, and eliminate all default-config wrappers in row/col/rect move detection and alignment helpers.

* **Hardcoded constants removed**

  * Replace: `MAX_LCS_GAP_SIZE`, `LCS_DP_WORK_LIMIT`, move-extraction caps, anchor k sizes, `<=256` gate.

* **Tests updated + new behavior tests**

  * Update existing tests to pass config everywhere; add targeted toggle tests and serde/builder tests.

---

If you want the coding agent to be extra disciplined, tell them to keep a running “Branch 3 checklist” PR description containing:

* A list of every changed public signature
* A list of every migrated constant (old location → new `DiffConfig` field)
* The exact new tests added to prove config affects outcomes
