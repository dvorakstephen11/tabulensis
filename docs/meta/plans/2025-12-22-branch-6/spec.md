Below is a concrete, codebase-aware implementation plan for **Branch 6: Performance Hardening** (memory budgeting, progress reporting, timeout support) based on what’s in `next_sprint_plan.md` and the current architecture described in `codebase_context.md`.

---

## Branch 6 goals and required behaviors

Branch 6 adds three “safety rails” to keep large diffs reliable:

1. **Memory budgeting** via `DiffConfig.max_memory_mb: Option<u32>`
   If the limit is approached: **(a)** return a partial result, **(b)** add a warning, **(c)** fall back to positional diff. 

2. **Progress reporting** via a `ProgressCallback` and a new entrypoint `diff_workbooks_with_progress(...)`. 

3. **Timeout support** via `DiffConfig.timeout_seconds: Option<u32>`
   Must check elapsed time in hot loops and return a partial result on timeout, plus a `--timeout` CLI flag. 

Acceptance criteria explicitly targets:

* **50K×100** grid diff in **< 10s**
* Memory stays **< 1GB** for **100K row** files
* Progress callback fires at “reasonable intervals”
* Timeout yields a graceful **partial** result 

---

## Cross-cutting design: add a single “hardening controller” used everywhere

If we implement memory checks, timeout checks, and progress calls independently in multiple modules, it’ll get scattered and inconsistent. The codebase already has clear phase boundaries in the engine (`workbook_diff` orchestrates sheets; `grid_diff` orchestrates sheet strategies; `grid_primitives` contains hot loops).

**Plan:** introduce one internal struct (name suggestions):

* `core/src/engine/hardening.rs` → `pub(super) struct HardeningController { ... }`

It should own:

* `start: Instant`
* `timeout: Option<Duration>` (computed from `DiffConfig.timeout_seconds`)
* `max_memory_bytes: Option<u64>` (computed from `DiffConfig.max_memory_mb`)
* `aborted: bool` (timeout triggered)
* `warned_timeout: bool` / `warned_memory: bool` (so we warn once)
* `progress: Option<&dyn ProgressCallback>` (trait object)
* minimal throttling state for progress + timeout checks (counters and/or last-update timestamp)

And expose methods like:

* `fn should_abort(&self) -> bool`
* `fn check_timeout(&mut self, warnings: &mut Vec<String>) -> bool`
* `fn memory_guard_or_warn(&mut self, estimated_extra_bytes: u64, warnings: &mut Vec<String>, context: &str) -> bool`

  * returns `true` if we should downgrade to positional (memory constraint)
* `fn progress(&mut self, phase: &'static str, percent: f32)` with throttling

This centralizes policy (“when to warn”, “how to throttle”, “what the warning says”) and keeps the hot loops cheap (a couple of integer checks). It also gives one place to tune overhead to meet the <10s requirement.

---

## 6.1 Memory budgeting

### 6.1.1 Add config field + plumbing

**Edit:** `core/src/config.rs` (`DiffConfig`)

* Add:

  * `pub max_memory_mb: Option<u32>,`
* Default should be `None` (no cap).
* Update presets:

  * `fastest()`, `most_precise()` should leave it as default unless you want presets to set a cap (I’d keep presets unchanged and let callers choose).

**Why this is low risk:** `DiffConfig` already grows over time and is `Default`-driven; adding optional fields is compatible with existing callers. 

### 6.1.2 Decide what “memory tracking” means in this codebase

The sprint plan allows “allocator stats or estimation”. 
Given the existing repo emphasizes portability and fast CI, start with **estimation-based budgeting** (no platform-specific allocator hooks required).

**What to estimate (first iteration):**

* Large temporary allocations that the “smart diff” path introduces vs positional diff:

  * `GridView` creation (row/col metadata + per-row vectors) 
  * alignment/move-detection auxiliary vectors (matched rows list, etc.)
* Optional (nice-to-have): growth of in-memory op buffering when using `VecSink` (non-streaming API). (This is the only way memory can explode even in positional diff.)

**Concrete estimation function(s):**

* `estimate_gridview_bytes(grid: &Grid) -> u64`

  * Use `grid.nrows`, `grid.ncols`, and `grid.cell_count()` (exists and used in CLI info output).
  * Multiply by `size_of::<RowView>()`, `size_of::<(u32, &Cell)>()`, etc.
* `estimate_advanced_sheet_diff_peak(old, new) -> u64`

  * `estimate_gridview_bytes(old) + estimate_gridview_bytes(new)`
  * plus a small constant overhead for alignment vectors based on max rows/cols

This doesn’t need to be perfect; it needs to be conservative enough to avoid pathological blowups and to enable deterministic tests (set max_memory_mb tiny and force fallback). 

### 6.1.3 Enforce memory budget at the *right* points

Do **not** check memory in every cell loop; memory doesn’t change meaningfully per cell unless you’re buffering ops in memory, and checks would add overhead.

Instead, enforce memory budget at the points where the “smart path” allocates the big stuff:

**Primary enforcement points:**

1. **Before building `GridView` / `SheetGridDiffer`** inside `core/src/engine/grid_diff.rs`
   That module currently orchestrates the strategy and is the right place to decide “smart vs positional”.

2. **Before attempting AMR / row alignment** in `diff_grids_core`
   (If you already built views, this is less about memory and more about time, but it’s still a good guardrail.)

**Behavior when memory cap would be exceeded (per sprint plan):**

* Add a warning (include sheet name if available)
* Mark result partial (in this codebase, “partial” is effectively “warnings non-empty”, because `complete = warnings.is_empty()` in workbook diff streaming summary construction).
* Fall back to `run_positional_diff_with_metrics(...)` for that sheet, then continue.

**Important nuance:** current “limit exceeded” behavior is controlled by `on_limit_exceeded` and may produce *complete* results with fallback (when `FallbackToPositional`). But Branch 6 explicitly requires a warning + partial when memory triggers. So memory-based fallback should be its own rule (always warn/partial), not dependent on `on_limit_exceeded`.

### 6.1.4 Optional but strongly recommended: remove duplicated `GridView` builds

Right now, `grid_diff` builds `GridView` for preflight, and then `SheetGridDiffer::new` builds `GridView` again if it proceeds to the full strategy path. That’s wasted time and memory in the exact “large file” scenarios Branch 6 cares about.

**Refactor plan:**

* Change `SheetGridDiffer::new(...)` to accept precomputed `old_view` and `new_view`, or add `SheetGridDiffer::from_views(...)`.
* In `diff_grids_core`, after preflight says “don’t short-circuit”, move the already-built views into the differ.
* This both improves the “<10s for 50K×100” target and makes memory budgeting more predictable.

---

## 6.2 Progress reporting

### 6.2.1 Public API: add `ProgressCallback` + new diff entrypoint

Add a new trait in core:

* **New file:** `core/src/progress.rs` (or `core/src/progress/mod.rs`)

  * Define the trait exactly as the plan specifies: 

    * `pub trait ProgressCallback: Send { fn on_progress(&self, phase: &str, percent: f32); }`
  * Provide a `NoProgress` implementation for internal defaults.

**New function:** `diff_workbooks_with_progress<P: ProgressCallback>(...) -> DiffReport` per spec. 

Where to wire it:

* `core/src/engine/workbook_diff.rs` already has `diff_workbooks(...)` and `try_diff_workbooks_streaming(...)`.
  Add parallel entrypoints:

  * `try_diff_workbooks_streaming_with_progress(old, new, pool, config, sink, progress: &dyn ProgressCallback)`
  * and wrappers that create `VecSink` and return `DiffReport`.

**Exports:**

* Re-export the trait and the new function from `core/src/lib.rs` (and/or `advanced` module), consistent with how other engine APIs are exposed.

### 6.2.2 “Major phases” mapping and where to emit progress

The plan calls out phases: parse, move detection, alignment, cell diff. 

The engine already has clear phase boundaries in `grid_diff.rs` (`MoveDetection`, `Alignment`, `CellDiff` under perf-metrics).
Even without `perf-metrics`, we can emit progress in those same places.

**Implementation detail:**

* Use the HardeningController as the single place to:

  * normalize/clamp percent
  * throttle updates (e.g., only report if percent advanced by >= 0.01, or at most N times per second)

**Where to call progress:**

1. **Workbook-level “parse” phase** (`workbook_diff.rs`)

   * Before building `old_sheets` / `new_sheets` maps: `parse = 0.0`
   * After producing the sorted `all_keys`: `parse = 1.0` 

2. **Sheet-level “move_detection” + “alignment”** (`grid_diff.rs`)

   * Emit at start/end:

     * `move_detection = 0.0` → `1.0` around `differ.detect_moves()` (or “skipped” path still sets to 1.0 quickly)
     * `alignment = 0.0` → `1.0` around the AMR/row/col alignment attempts.

3. **Hot loop “cell_diff”** (`grid_primitives.rs`)

   * Positional diff is the long-running case (50K×100).
   * Update progress every N rows (e.g., every 128 or 256 rows) rather than per row to avoid overhead.
   * Percent = `rows_processed / overlap_rows` (clamped). 

If you do the “work-weighted across sheets” approach, this is where you accumulate per-sheet progress into a single overall percentage. If you want simplicity, report percent per sheet; CLI can label the sheet name itself.

### 6.2.3 CLI: `--progress` flag + progress bar display

Branch 6 explicitly requires a CLI flag and terminal progress bar. 

**CLI parsing changes:**

* `cli/src/main.rs` add:

  * `--progress`
  * `--max-memory <MB>`
  * `--timeout <seconds>`

**Command runner changes:**

* `cli/src/commands/diff.rs`:

  * Extend `run(...)` signature to accept these new args
  * After `build_config(fast, precise)`, set:

    * `config.max_memory_mb = max_memory`
    * `config.timeout_seconds = timeout`

**Showing the bar without corrupting output:**

* Send progress UI to **stderr**, keep stdout clean for text/json/jsonl output.
* For JSONL streaming (`JsonLinesSink`), progress must *not* interleave with stdout (so definitely stderr).

**Implementation approach:**

* Add a small progress implementation in CLI:

  * either pull in `indicatif` (simple + robust), or
  * implement a minimal “single-line updating bar” using `\r` on stderr when stderr is a TTY.
* Implement `ProgressCallback` in CLI (e.g., `CliProgress`) that updates the bar.

**Finishing behavior:**

* Ensure the bar is finalized/cleared when diff completes (including partial/timeout paths).

---

## 6.3 Timeout support

### 6.3.1 Add config field

As specified: `DiffConfig.timeout_seconds: Option<u32>`. 
Add to `core/src/config.rs` with default `None`. 

### 6.3.2 Where to check elapsed time

The plan says “hot loops”, and the hottest loops in this engine are in `grid_primitives` (positional diff over overlap rows/cols) and parts of database mode that iterate row×col.

**Primary check sites:**

1. `run_positional_diff_with_metrics` (positional cell diff)
2. masked positional diff helpers (if they can run long)
3. database-mode row/col loops in `try_diff_grids_database_mode_streaming` (it has a `for (row_a,row_b)` then `for col` loop)

**Throttle the checks:**

* Do not call `Instant::now()` every cell.
* Example pattern:

  * increment a counter per row
  * every 128 rows, call `check_timeout(...)`
* The HardeningController can hold `next_timeout_check_counter` and update it.

### 6.3.3 What happens when timeout triggers

Branch 6 requires: “Return partial result on timeout”. 

Given how the engine currently constructs “complete” (`complete = ctx.warnings.is_empty()`) and returns warnings in `DiffSummary`, the cleanest implementation is:

* On timeout:

  * push exactly one warning like:

    * `timeout after X seconds; diff aborted early; results may be incomplete`
  * set a global `aborted` flag in HardeningController
  * unwind by returning `Ok(())` from inner functions (not `Err`) so already-emitted ops are preserved
  * `workbook_diff` breaks out of the sheet loop when `aborted == true` and then returns `DiffSummary { complete: false, warnings, op_count, ... }`

This avoids a major pitfall: if you return an error mid-stream, you can end up with ops already emitted to the sink but a summary that says `op_count = 0` (the current error wrapper in `diff_workbooks_streaming` hardcodes `op_count: 0` on Err). So timeout should not be modeled as `DiffError` unless you also redesign error handling to preserve partial output. 

### 6.3.4 CLI `--timeout`

Add `--timeout <seconds>` and wire it into `DiffConfig.timeout_seconds`.

The CLI exit code logic already treats “incomplete” as non-zero even if no ops were produced (`exit_code_from_report` checks both ops and `complete`). That’s good: a timeout that yields incomplete results should exit `1` rather than `0`.

---

## Testing plan (core + CLI) aligned to existing patterns

### 1) Core unit tests: new `hardening_tests.rs`

Follow the style of existing limit behavior tests that validate warnings + `complete=false` when limits are exceeded.

Add tests:

**A. Memory budget forces positional fallback + warning**

* Build two single-sheet workbooks with a modest diff.
* Set `config.max_memory_mb = Some(0)` (or very small).
* Expect:

  * `report.complete == false`
  * `report.warnings` contains “memory” + “positional”
  * Some ops still present (e.g., `CellEdited`)

**B. Timeout yields partial + warning**

* Set `config.timeout_seconds = Some(0)` (immediate timeout) so it’s deterministic.
* Expect:

  * `complete == false`
  * warning contains “timeout”
  * ops may be empty or partial, but must not panic/hang.

**C. Progress callback fires**

* Create a test callback collecting `(phase, percent)` into a Vec.
* Run `diff_workbooks_with_progress`.
* Assert:

  * saw at least `cell_diff` updates
  * percent is within `[0.0, 1.0]`
  * number of callbacks is “reasonable” (i.e., not per-cell; you can assert `< 10_000` for a moderately sized grid). 

### 2) CLI integration tests

Add test cases to `cli/tests/...`:

* `excel-diff diff --max-memory 0 ...`:

  * exit code should be `1` (incomplete implies non-zero)
  * stderr should contain `Warning:` and mention memory
* `excel-diff diff --timeout 0 ...`:

  * exit code `1`
  * stderr warning mentions timeout
* `excel-diff diff --format jsonl --progress ...`:

  * stdout should still be valid JSONL header/op lines (no progress noise)
  * stderr should show progress output (or at least not be empty if you implement it that way).

---

## Performance validation plan (so Branch 6 doesn’t regress the 50K×100 target)

The repo already has perf-focused fixtures and a manifest that includes 50K×100 grids for stress tests. 

### Ensure new checks don’t break the <10s goal

* Timeout checks must be throttled (per N rows, not per cell).
* Progress callbacks must be throttled similarly.
* Memory checks should happen at strategy boundaries (before big allocations), not in inner loops.

### Validate memory target for 100K row files

* Add a new “perf_large” fixture scenario in the generator manifest (or generate locally) with ~100K rows and moderate columns. 
* Run:

  * normal diff (no memory cap)
  * and a constrained run with `--max-memory` to ensure graceful fallback without OOM

If you want CI enforcement, update the perf harness to also report peak RSS or estimated peak allocations. The existing scripts explicitly mention memory tracking as a future enhancement, so Branch 6 is the right moment to extend that.

---

## Work breakdown by file/module (high-signal checklist)

### Core crate

* `core/src/config.rs`

  * add `max_memory_mb`, `timeout_seconds`
  * update defaults/presets/builder/serde tests
* `core/src/progress.rs` (new)

  * define `ProgressCallback`, `NoProgress` 
* `core/src/engine/workbook_diff.rs`

  * add `*_with_progress` entrypoints
  * instantiate HardeningController, emit parse-phase progress
  * break out on timeout-abort
* `core/src/engine/grid_diff.rs`

  * add memory-based downgrade to positional
  * add phase progress hooks around move/alignment/cell diff
  * (recommended) refactor to reuse preflight GridViews
* `core/src/engine/grid_primitives.rs`

  * add timeout checks + cell-diff progress updates in row loops 
* `core/src/package.rs`

  * add `diff_with_progress` / `diff_streaming_with_progress` wrappers so CLI can use progress on package diffs too
* `core/tests/*`

  * add hardening tests as above

### CLI crate

* `cli/src/main.rs`

  * add flags: `--max-memory`, `--timeout`, `--progress`
* `cli/src/commands/diff.rs`

  * thread new flags through
  * set config fields
  * if `--progress`: use `diff_*_with_progress` path and display a progress bar on stderr
* `cli/Cargo.toml`

  * add progress bar dependency (if using `indicatif`) 
* `cli/tests/*`

  * add flag coverage tests 

---

## One key policy decision to lock in early

**Should timeout stop the whole workbook diff, or just downgrade strategies and continue?**

Given the sprint plan language (“Return partial result on timeout”) and streaming constraints, I’d implement **timeout as a global abort**:

* stop processing additional sheets
* return summary/report with warning + `complete=false`

That’s the simplest behavior to explain, and the easiest to make consistent across sinks and APIs.

---

If you want, I can also propose the exact warning strings (so tests match them) and the throttling constants (rows-per-progress-tick, rows-per-timeout-check) tuned for the existing 50K×100 perf fixtures.
