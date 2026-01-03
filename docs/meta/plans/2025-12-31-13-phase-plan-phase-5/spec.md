## Phase 5 Implementation Plan: Nested `DiffConfig` Sub-configs (presets + serde compatibility)

Phase 5 in the 13-phase plan is specifically: refactor `DiffConfig` into nested sub-configs while preserving presets (`fastest`, `balanced`, `most_precise`) and maintaining serde compatibility (including aliases + round-trip behavior), with validation remaining centralized. 

This directly addresses an identified maintainability risk: the current `DiffConfig` is “large and multi-domain,” and the density of knobs increases cognitive load even though presets exist.

Below is a codebase-grounded implementation plan that is explicit about the concrete files, fields, and call sites involved.

---

## 1) Goals and non-goals

### Goals (must-haves)

1. **Nested shape in Rust**: `DiffConfig` becomes a composition of domain sub-structs (e.g., `AlignmentConfig`, `MoveConfig`, `PreflightConfig`, `HardeningConfig`, `SemanticConfig`).
2. **Presets remain first-class**: `DiffConfig::fastest()`, `balanced()`, `most_precise()` continue to exist and behave exactly as they do today.
3. **Serde compatibility preserved**:

   * Existing flat JSON keys still deserialize into the new config.
   * Existing aliases still work (`rare_frequency_threshold`, `low_info_cell_threshold`, `recursive_threshold`).
   * Default config still round-trips through serde (`to_string` then `from_str` equals the original).
4. **Centralized validation** remains in a single place (`DiffConfig::validate()`), and builder validation continues to rely on it.
5. **Behaviorally identical** engine defaults and presets: no algorithmic changes intended.

### Non-goals (explicitly out of scope for Phase 5)

* Changing default thresholds or preset values.
* Changing the external serialized schema to a nested JSON shape (optional follow-up, see “Optional extensions”).
* Introducing new config knobs.

---

## 2) Current reality in the codebase (baseline you must preserve)

### Where `DiffConfig` lives and what it looks like today

* `core/src/config.rs` defines:

  * `LimitBehavior`, `SemanticNoisePolicy`
  * `DiffConfig` as a flat struct containing all thresholds and knobs (move detection, alignment, preflight, hardening, semantic policy, output heuristics).
  * Defaults and presets (`fastest`, `balanced`, `most_precise`).
  * A centralized `validate()` doing range checks and non-zero checks using stable field-name strings in errors.
  * `DiffConfigBuilder` that mutates an internal `DiffConfig` and calls `validate()` in `build()`. 

### Presets and how they’re used “for real”

* CLI chooses presets via `--fast` and `--precise` and otherwise uses default; then it applies hardening overrides (`max_memory_mb`, `timeout_seconds`, `max_ops`).
* WASM sets a default memory cap by mutating `cfg.max_memory_mb`.
* Many tests and internal call sites use `DiffConfig::default()` and sometimes construct `DiffConfig { field: X, ..Default::default() }`.

### Hardening is currently wired to top-level fields

`HardeningController::new(config)` reads `timeout_seconds`, `max_memory_mb`, and `max_ops` directly from `DiffConfig`.

### Alignment/move logic reads config fields directly all over the place

For example, AMR gap alignment reads `context_anchor_k1/k2`, `max_lcs_gap_size`, etc. 
Move extraction reads `move_extraction_max_slice_len`, etc. 
Grid-view classification reads `rare_threshold` and `low_info_threshold`. 

This means the refactor is necessarily a *mechanical call-site update* across modules; it’s not isolated to config.rs.

---

## 3) Target design (what “nested sub-configs” means in this codebase)

### Key design choice: keep serialized form flat using `serde(flatten)`

To preserve backwards compatibility for config JSON (and avoid breaking any stored config blobs), the new nested sub-configs should be **nested in Rust** but **flattened in serde**:

* Rust API becomes `config.alignment.max_align_rows`, etc.
* JSON remains `{ "max_align_rows": 500000, "max_move_iterations": 20, ... }` (no new top-level `"alignment": { ... }` key unless you explicitly add optional support later).

This approach gives the maintainability and “shape” benefits for contributors (and rustdoc) without forcing a schema migration. It’s also the lowest-risk way to satisfy “keep aliases/serde round-trip behavior.”

### Proposed struct layout (concrete and field-complete)

Below is the exact grouping that matches how the code uses these knobs today.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffConfig {
    #[serde(flatten)]
    pub alignment: AlignmentConfig,
    #[serde(flatten)]
    pub moves: MoveConfig,
    #[serde(flatten)]
    pub preflight: PreflightConfig,
    #[serde(flatten)]
    pub hardening: HardeningConfig,
    #[serde(flatten)]
    pub semantic: SemanticConfig,
}
```

#### AlignmentConfig (alignment + anchor/gap strategy)

Move here (existing field names preserved exactly):

* `max_align_rows`, `max_align_cols`
* `max_block_gap`, `max_hash_repeat`
* `rare_threshold` (keep `serde(alias = "rare_frequency_threshold")`)
* `low_info_threshold` (keep `serde(alias = "low_info_cell_threshold")`)
* `recursive_align_threshold` (keep `serde(alias = "recursive_threshold")`)
* `small_gap_threshold`, `max_recursion_depth`
* `max_lcs_gap_size`, `lcs_dp_work_limit`
* `context_anchor_k1`, `context_anchor_k2`

These are read heavily by alignment/gap logic and metadata/anchor discovery.

#### MoveConfig (masked move detection + extraction caps + fuzziness)

Move here:

* `max_move_iterations`
* `enable_fuzzy_moves`
* `fuzzy_similarity_threshold`, `max_fuzzy_block_rows`
* `min_block_size_for_move`
* `move_extraction_max_slice_len`, `move_extraction_max_candidates_per_sig`
* `max_move_detection_rows`, `max_move_detection_cols`

These are directly tied to move detection/extraction gating and fuzziness.

#### PreflightConfig (bailouts and near-identical shortcuts)

Move here:

* `preflight_min_rows`
* `preflight_in_order_mismatch_max`
* `preflight_in_order_match_ratio_min`
* `bailout_similarity_threshold`
* `max_context_rows`

These are all described as preflight controls in the docs/comments and used in the “skip expensive work when obviously unnecessary” path.

#### HardeningConfig (resource caps and behavior on limits)

Move here:

* `on_limit_exceeded`
* `max_memory_mb`
* `timeout_seconds`
* `max_ops`

These feed `HardeningController` and limit-handling behavior in the engine.

#### SemanticConfig (semantic diff toggles + output policies)

Move here:

* `enable_m_semantic_diff`
* `enable_formula_semantic_diff`
* `semantic_noise_policy`
* `include_unchanged_cells`
* `dense_row_replace_ratio`, `dense_row_replace_min_cols`, `dense_rect_replace_min_rows`

These are the “semantic policy + output emission” knobs.

---

## 4) Step-by-step implementation plan

### Step 0: Inventory and “blast radius” checklist (fast, but essential)

Before editing:

* Enumerate every `DiffConfig` field access in the repo (use ripgrep for `config.` accesses and for the field names themselves).
* Categorize each hit by the target sub-config group (alignment/moves/preflight/hardening/semantic).
* Identify **struct update syntax** uses in tests and examples (e.g., `DiffConfig { max_move_iterations: 2, ..Default::default() }`), because these will need reshaping. 

Deliverable: a short internal checklist (even a scratch note) of modules/files to touch, so you don’t discover stragglers after refactor.

---

### Step 1: Introduce the sub-config structs in `core/src/config.rs`

In `core/src/config.rs`:

1. Define the new sub-structs with:

   * `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`
   * `#[serde(default)]`
   * `impl Default` matching today’s values for the fields you moved.

2. Move the existing field doc comments from `DiffConfig` onto the appropriate sub-struct fields (this is the biggest “cognitive load win” in practice).

3. Keep alias attributes on the specific moved fields:

   * `rare_threshold`: `#[serde(alias = "rare_frequency_threshold")]`
   * `low_info_threshold`: `#[serde(alias = "low_info_cell_threshold")]`
   * `recursive_align_threshold`: `#[serde(alias = "recursive_threshold")]`

Key rule: **do not rename the fields**. Field names are your stable serde contract right now.

---

### Step 2: Refactor `DiffConfig` to compose sub-configs (with `serde(flatten)`)

Still in `core/src/config.rs`:

1. Replace the flat `pub struct DiffConfig { ... }` with the composed version (shown above).

2. Add `#[serde(flatten)]` to each sub-config field to keep JSON flat.

3. Implement `Default for DiffConfig` as:

   * `alignment: AlignmentConfig::default()`, etc.
   * (Optional but useful) also implement `balanced()` as `Self::default()` to keep semantics.

This directly preserves the role of `DiffConfig` as “central knobs,” but now with a sane shape.

---

### Step 3: Re-implement presets as “mutate defaults” (avoid brittle struct update)

The existing preset constructors are currently expressed as partial struct literals with `..Default::default()`.

Once the config becomes nested, trying to keep the same pattern becomes noisy and fragile. Prefer:

* Start from `let mut cfg = Self::default();`
* Apply only the preset-specific deltas in the appropriate sub-config
* Return `cfg`

This also makes it harder to accidentally forget a new field in the future.

Preserve the preset deltas exactly as today:

* `fastest` modifies:

  * `moves.max_move_iterations = 5`
  * `alignment.max_block_gap = 1_000`
  * `alignment.small_gap_threshold = 20`
  * `alignment.recursive_align_threshold = 80`
  * `moves.max_move_detection_rows = 80`
  * `moves.enable_fuzzy_moves = false`
  * `semantic.enable_m_semantic_diff = false`

* `most_precise` modifies:

  * `moves.max_move_iterations = 30`
  * `alignment.max_block_gap = 20_000`
  * `moves.fuzzy_similarity_threshold = 0.95`
  * `alignment.small_gap_threshold = 80`
  * `alignment.recursive_align_threshold = 400`
  * `semantic.enable_formula_semantic_diff = true`
  * plus explicitly sets the move extraction/LCS caps and move detection row/col bounds (keep exactly as-is today).

---

### Step 4: Keep validation centralized, but update field paths (and keep error field strings stable)

Update `DiffConfig::validate()` to reference the nested fields while keeping:

* the *same* error variants
* and the *same* `field: "..."` string values used by `NonPositiveLimit` errors.

Concretely, update checks like:

* `self.moves.fuzzy_similarity_threshold` range check
* `ensure_non_zero_u32(self.alignment.max_align_rows, "max_align_rows")`
* `ensure_non_zero_u32(self.moves.move_extraction_max_slice_len, "move_extraction_max_slice_len")`
* `ensure_non_zero_u32(self.preflight.max_context_rows, "max_context_rows")`
* `if self.alignment.lcs_dp_work_limit == 0 { field: "lcs_dp_work_limit" }`

Even though fields moved, the “field name” in error payload should remain the legacy flat name for compatibility and for tests that match on it.

---

### Step 5: Update `DiffConfigBuilder` to target the new nested layout (preserve the builder API)

The builder is used in tests and is part of the crate’s ergonomics.

Plan:

1. Keep `DiffConfigBuilder` and all its methods with the same names/signatures.
2. Internally rewrite setters to update nested fields:

   * `self.inner.preflight.preflight_min_rows = value;`
   * `self.inner.hardening.max_memory_mb = value;`
   * `self.inner.semantic.semantic_noise_policy = value;`
3. Keep `.build()` calling `self.inner.validate()` and returning the inner config.

This preserves “presets are primary UX surface” while not breaking advanced callers/tests that use the builder.

Optional (nice-to-have, but don’t block Phase 5):

* Add “grouped” entry points like `.alignment(|a| ...)`, `.moves(|m| ...)` later. Not required to satisfy the phase plan.

---

### Step 6: Mechanical call-site migration in `core`

This is the bulk of the work. The goal is: **no behavior change, only new access paths**.

#### 6.1 Alignment-heavy modules

Update any module that currently reads alignment-ish fields:

Examples from current code:

* AMR/gap strategy reads `context_anchor_k1/k2`. 
* Rect block move detection and grid-view filtering read low-info thresholds.

Concrete action:

* Replace `config.max_block_gap` with `config.alignment.max_block_gap`
* Replace `config.rare_threshold` with `config.alignment.rare_threshold`
* Replace `config.max_lcs_gap_size` with `config.alignment.max_lcs_gap_size`
* Replace `config.lcs_dp_work_limit` with `config.alignment.lcs_dp_work_limit`
* Replace `config.context_anchor_k1` with `config.alignment.context_anchor_k1`, etc.

#### 6.2 Move detection/extraction modules

Update fields:

* `config.max_move_iterations` -> `config.moves.max_move_iterations`
* `config.enable_fuzzy_moves` -> `config.moves.enable_fuzzy_moves`
* `config.fuzzy_similarity_threshold` -> `config.moves.fuzzy_similarity_threshold`
* `config.max_move_detection_rows/cols` -> `config.moves.max_move_detection_rows/cols`
* `config.move_extraction_max_slice_len` -> `config.moves.move_extraction_max_slice_len`, etc.

#### 6.3 Preflight paths

Update:

* `preflight_*` fields -> `config.preflight.*`
* `bailout_similarity_threshold` -> `config.preflight.bailout_similarity_threshold`
* `max_context_rows` -> `config.preflight.max_context_rows`

#### 6.4 Semantic/output emission paths

Update:

* `enable_m_semantic_diff` -> `config.semantic.enable_m_semantic_diff`
* `enable_formula_semantic_diff` -> `config.semantic.enable_formula_semantic_diff`
* `semantic_noise_policy` -> `config.semantic.semantic_noise_policy`
* `include_unchanged_cells` -> `config.semantic.include_unchanged_cells`
* dense replacement knobs -> `config.semantic.*`

#### 6.5 Hardening controller and limit behaviors

Update:

* `HardeningController::new` to read:

  * `config.hardening.timeout_seconds`
  * `config.hardening.max_memory_mb`
  * `config.hardening.max_ops`

Update any place that reads:

* `config.on_limit_exceeded` -> `config.hardening.on_limit_exceeded`

---

### Step 7: Update other workspace crates that touch config directly

#### 7.1 CLI (`cli/src/commands/diff.rs`)

Where CLI currently does:

* `let mut config = build_config(fast, precise);`
* `config.max_memory_mb = max_memory;`
* `config.timeout_seconds = timeout;`
* `config.max_ops = max_ops;`

Update to:

* `config.hardening.max_memory_mb = max_memory;`
* `config.hardening.timeout_seconds = timeout;`
* `config.hardening.max_ops = max_ops;`

Optional (fits “presets primary UX surface,” but still small):

* Add `--preset {fastest|balanced|most-precise}` and keep `--fast/--precise` as backwards-compatible aliases that map to presets internally.

  * This is a CLI-only improvement; it does not require config schema changes.

#### 7.2 WASM (`wasm/src/lib.rs`)

Where WASM currently sets:

* `cfg.max_memory_mb = Some(WASM_DEFAULT_MAX_MEMORY_MB);`

Update to:

* `cfg.hardening.max_memory_mb = Some(WASM_DEFAULT_MAX_MEMORY_MB);`

This is important because WASM memory budgets are a core constraint and this is a real path used by the web worker.

#### 7.3 Desktop / other consumers

Even if desktop doesn’t mutate config much today, it imports `DiffConfig` and may start doing so; you want the workspace to compile cleanly after the refactor. 

---

### Step 8: Update tests (config tests + any tests that construct configs)

#### 8.1 Update `core/src/config.rs` unit tests

`defaults_match_limit_spec` and others must be updated to read nested values, but must assert the **same actual numbers and behaviors** as before.

Examples:

* `assert_eq!(cfg.max_align_rows, 500_000);` becomes `cfg.alignment.max_align_rows`
* `assert!(matches!(cfg.on_limit_exceeded, ...))` becomes `cfg.hardening.on_limit_exceeded`
* `assert!(cfg.enable_m_semantic_diff)` becomes `cfg.semantic.enable_m_semantic_diff`
* and so on.

Also update builder tests similarly.

#### 8.2 Fix struct-update syntax usages across integration tests

Any test that does something like:

```rust
let limited_config = DiffConfig {
    max_move_iterations: 2,
    ..DiffConfig::default()
};
```

will no longer compile. 

Replace these with one of:

* A builder usage (often simplest and keeps validation): `DiffConfig::builder().max_move_iterations(2).build().unwrap()`
* Or a nested struct update:

  * Start from `let mut cfg = DiffConfig::default(); cfg.moves.max_move_iterations = 2;`

Prefer the latter in tests where you deliberately want to bypass validation only if you really mean it (but most tests should keep validation intact).

#### 8.3 Add explicit serde compatibility tests (highly recommended)

The current suite checks “roundtrip preserves defaults.” Keep that. 

Add two additional tests to make “serde compatibility” non-accidental:

1. **Flat JSON shape remains flat**:

   * Serialize `DiffConfig::default()` to `serde_json::Value`
   * Assert it contains `"max_align_rows"` at top-level
   * Assert it does *not* contain `"alignment"`/`"moves"` keys

2. **Legacy alias keys still work** (keep existing alias test, but ensure it deserializes into the correct nested group):

   * JSON with `"rare_frequency_threshold": ...` lands in `cfg.alignment.rare_threshold`
   * JSON with `"recursive_threshold": ...` lands in `cfg.alignment.recursive_align_threshold`

This directly enforces the phase requirement “keep aliases/serde round-trip behavior.”

---

### Step 9: Documentation + ergonomics cleanup (small but high leverage)

This is where the refactor pays off for future contributors.

1. In `core/src/config.rs`, group the sub-configs in the file in the same order as the pipeline:

   * Preflight (early exits)
   * Alignment
   * Move detection/extraction
   * Semantic/output policy
   * Hardening

Or whichever ordering mirrors your engine mental model (design evaluation emphasizes modular pipeline clarity). 

2. Update crate docs/examples that reference direct fields:

   * If any README/doc snippet shows setting `cfg.max_memory_mb`, update to `cfg.hardening.max_memory_mb`.

---

## 5) Compatibility and migration notes (what will break, what won’t)

### What won’t break (by design)

* Config JSON files (flat keys) continue to parse.
* Serde aliases continue to work.
* Presets keep their names and semantics.
* Builder API keeps method names.
* Engine behavior should be identical under the same defaults/presets.

### What will break (and must be fixed in-repo)

* Any direct field access like `config.max_move_iterations` becomes `config.moves.max_move_iterations`, etc.
* Any struct update literals that set old fields directly must be rewritten.

Because this is a workspace with CLI + WASM + desktop, you’ll catch most breakages by compiling the full workspace and running tests, but the plan above calls out the known hotspots in advance.

---

## 6) Risk analysis and mitigations

### Risk: `serde(flatten)` field name collisions

Mitigation:

* Keep field names unique across sub-configs (they already are).
* Add a compile-time sanity check via `cargo test` (serde will fail to derive if collisions occur).

### Risk: silently breaking serialized shape (nested objects appear)

Mitigation:

* Add the “serialized JSON has no `alignment` key” test described above.

### Risk: preset deltas accidentally drift

Mitigation:

* Keep existing preset tests (and add more if needed) that assert preset-specific changes like `fastest` disabling `enable_m_semantic_diff`, `most_precise` setting `fuzzy_similarity_threshold = 0.95`, etc.

### Risk: error messages/fields change (downstream might match strings)

Mitigation:

* Keep validation centralized and keep `field: "..."` strings identical to the legacy field names, even though they now live in sub-configs.

---

## 7) Definition of done (Phase 5 exit criteria)

You can consider Phase 5 complete when:

1. `DiffConfig` is composed of named sub-configs (alignment/moves/preflight/hardening/semantic) in Rust.
2. All workspace crates compile (`core`, `cli`, `wasm`, `ui_payload`, `desktop`).
3. All tests pass, including:

   * default roundtrip serde test
   * alias compatibility test
   * builder validation tests
   * any integration tests that used struct-update syntax are migrated.
4. CLI and WASM paths still correctly set memory/time/op limits via the new `hardening` sub-config.

---

## Optional extensions (safe follow-ups, not required for Phase 5)

If you want to further align with “presets as primary UX surface” without touching serde schema:

* Add a `DiffPreset` enum in `core/src/config.rs` and implement:

  * `DiffConfig::from_preset(DiffPreset)`
  * `DiffPreset::to_config()`
* Update CLI to accept `--preset` while keeping `--fast/--precise` as aliases.
* Add a `DiffConfig::with_hardening(...)` helper or a small `HardeningConfigBuilder` to reduce verbosity in callers that only override resource caps.

These are ergonomic wins but not necessary to satisfy the phase plan’s core requirement.
