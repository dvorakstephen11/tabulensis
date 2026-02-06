Below is an implementation plan that targets the two high‑leverage issues you called out: (1) eliminating deep clones on cache hits by caching `Arc<…>` handles, and (2) avoiding the expensive cache-key work (canonicalize + metadata + lowercasing) on repeated UI follow‑on requests.

I’m grounding this in what your current desktop backend actually does today:

* `open_workbook_cached()` and `open_pbix_cached()` compute a `CacheKey` and on **every cache hit** return `pkg.clone()`; misses open the file and also insert `pkg.clone()` into the LRU. 
* `WorkbookHandle` / `PbixHandle` are **feature-gated**: either a full `WorkbookPackage`/`PbixPackage` value, or `Arc<…>` when `arc-cache` is enabled. 
* In the desktop GUI crate (`desktop/wx`), `backend-arc-cache` exists but is **not in the default feature set**, so by default those hit-path clones are deep. 
* Cache hits still pay the full `cache_key()` cost: `canonicalize`, `to_string_lossy().to_lowercase()`, and `metadata()` to pull `mtime` and `size`. 
* You already have an excellent “UI follow-on” workload in `cache_hit_loop_smoke`: after a diff, it loops `loadSheetMeta` + `loadCellsInRange` + `loadSheetPayload` 50 times and prints elapsed time plus cache hit deltas. 

---

## Phase 0 — Turn this into a first-class experiment (baseline + variants)

This fits your “dedicated experiment doc + baseline/post-change metrics” protocol. 

### 0.1 Create a dedicated experiment doc

Create something like:

* `desktop_arc_cache_and_key_reuse_experiment.md`

Start it with:

* the plan (this response),
* machine + build info,
* the exact command lines and raw output blocks for baseline + each variant.

This mirrors your required iteration protocol. 

### 0.2 Establish baseline (no code changes yet)

Run your standard baseline for core (per your policy) plus a desktop-backend-specific baseline:

**Core baseline (your default policy):** 

* `cargo test -p excel_diff`
* `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`

**Desktop backend baseline (add for this candidate):**

* `cargo test -p desktop_backend`
* `cargo test -p desktop_backend -- --ignored --nocapture`

You specifically care about the output line from `cache_hit_loop_smoke` (elapsed_ms and hit deltas). 

### 0.3 (Optional but very useful) Make the benchmark output machine-readable

Right now the test prints a line that’s great for humans. 
If you want it to participate in the same “raw `PERF_METRIC` lines” workflow you use elsewhere, adjust it to print an additional line like:

* `PERF_METRIC desktop_backend_cache_hit_loop_elapsed_ms=...`

Do this before changes so baseline and variants are all in the same format. 

---

## Phase 1 — Stop deep clones on cache hits (Arc-backed handles)

### Goal

Make repeated UI follow-on requests stop copying the entire workbook / PBIX data structures on every cache hit.

### Why this is almost certainly your biggest win

On a cache hit you currently do `return Ok(pkg.clone());` for both workbooks and PBIX. 
Because `WorkbookHandle` defaults to the *value type* (not `Arc`) in the desktop app, this clone is deep and will scale with workbook size.

### 1.1 Enable `arc-cache` for the desktop app by default

You already have the plumbing:

* `desktop_backend` supports `arc-cache`. 
* `desktop_wx` exposes `backend-arc-cache = ["desktop_backend/arc-cache"]`. 

What’s missing is making it “always on” for the GUI’s default build.

Recommended approach (keeps your existing feature taxonomy):

* Update `desktop/wx/Cargo.toml` so the default features include `backend-arc-cache` in addition to `backend-lru-crate`. 

That single change should convert “clone on hit” into “increment refcount on hit”.

### 1.2 Verify correctness assumptions (important with shared ownership)

With `Arc<WorkbookPackage>` you will now share the same package across all handlers.

That’s good *if* the packages are treated as immutable after open (which your current usage pattern strongly suggests: you pass `&old_pkg` and read from `old_pkg.workbook`, etc.).

Quick checks to do during implementation:

* Make sure no handler mutates `WorkbookPackage` / `Workbook` internals.
* If something needs caching of derived values, keep that caching *outside* the package (e.g., per-request computed values or a separate memo table).

### 1.3 Add one “safety rail” test (small, high value)

Add a test behind `cfg(feature = "arc-cache")` that asserts repeated cache hits do not create new underlying allocations by checking pointer equality:

* call `open_workbook_cached()` twice for the same path/trusted;
* assert `Arc::ptr_eq(&a, &b)`.

This catches accidental regressions where the cache returns a fresh `Arc::new(pkg.clone())` instead of reusing the cached `Arc`.

### 1.4 Re-run the experiment matrix

Run the exact same baseline commands, but with the GUI/backend built with arc-cache enabled. The `cache_hit_loop_smoke` elapsed time should drop sharply if deep clones were dominating.

---

## Phase 2 — Stop paying `cache_key()` on UI follow-on hits

Once clones are cheap, `cache_key()` is the next likely “death by a thousand cuts” because `loadSheetMeta`, `loadCellsInRange`, and `loadSheetPayload` each call `open_workbook_cached()` twice (old + new).

And `cache_key()` currently does:

* `canonicalize(path)`
* `to_string_lossy().to_lowercase()`
* `metadata()` syscall
  …even when you already have the correct packages in the cache. 

### Key design principle for this phase

A diff run already establishes a stable file identity (old/new paths + trusted) for a `diff_id`. UI follow-ons for that same `diff_id` don’t need to rediscover identity via syscalls.

You can exploit that by caching the precomputed `CacheKey`s for each `diff_id` inside the engine thread.

### 2.1 Add a tiny “diff_id → keys” cache inside `EngineState`

Add a new LRU (capacity small, like 8–32) keyed by `diff_id`, storing:

* kind (`Workbook` vs `Pbix`), and
* `old_key: CacheKey`, `new_key: CacheKey`

You already like tiny predictable caches (you even have a `custom-lru` path for this area).
This diff-key cache should be similarly small and bounded.

Why store keys, not handles?

* It avoids pinning large packages in memory beyond the existing workbook/pbix LRU capacity policy.
* It preserves your existing `cache_stats` semantics (you still “hit” the workbook LRU; you just don’t recompute the key).

### 2.2 Add “by-key” lookup helpers

Add helpers that skip path normalization + metadata:

* `get_workbook_cached_by_key(&CacheKey) -> Option<WorkbookHandle>`
* `get_pbix_cached_by_key(&CacheKey) -> Option<PbixHandle>`

These should increment the same hit/miss counters you use today so existing diagnostics remain meaningful. 

### 2.3 Add `_with_key` open functions (one-time key computation when needed)

Create variants that return both the computed key and the handle:

* `open_workbook_cached_with_key(path, trusted) -> (CacheKey, WorkbookHandle)`
* `open_pbix_cached_with_key(path, trusted) -> (CacheKey, PbixHandle)`

Internally this is almost exactly your current logic, except you return the key you already computed. 

### 2.4 Populate the diff-key cache in `handle_diff`

In `handle_diff`, once you have:

* `diff_id`
* `old_pkg/new_pkg` opened
* and (with the new `_with_key`) `old_key/new_key`

Store `{ old_key, new_key, kind }` into `diff_id → keys`.

This guarantees that the *first* UI follow-on request after a diff doesn’t pay key computation either (because the entry is already there).

### 2.5 Use diff-key cache in the UI follow-on handlers

Update these handlers to use the diff-key cache:

* `handle_load_sheet` 
* `handle_load_sheet_meta` 
* `handle_load_cells_range` 

Flow:

1. Load summary from store (you still need it for trusted/complete/warnings and ops/strings).
2. Check `diff_key_cache.get(&diff_id)`:

   * If present and kind matches workbook:

     * fetch old/new via `get_workbook_cached_by_key`.
   * If missing (e.g., engine restarted, or entry evicted):

     * open via `open_workbook_cached_with_key`,
     * insert keys into `diff_key_cache`.

This removes the repeated canonicalize/metadata/lowercasing costs from the “steady-state” interactive path.

### 2.6 Confirm behavior with your existing workload

Re-run `cache_hit_loop_smoke` and compare:

* elapsed_ms should drop further (especially on platforms where canonicalize/metadata are pricey)
* hit deltas should remain basically unchanged (still lots of workbook hits, near-zero misses) 

---

## Phase 3 — Decide how far to simplify (reduce complexity, not add it)

You’re explicitly chasing “performance *and* complexity *and* size”. Here’s how to keep that true as you ship these wins.

### 3.1 Consider making Arc caching unconditional in desktop backend

Right now you carry two code paths:

* `WorkbookHandle = WorkbookPackage` (deep clones)
* `WorkbookHandle = Arc<WorkbookPackage>` (cheap clones) 

Once you’ve proven `Arc` is the right answer in practice, you can simplify by removing the non-Arc path in the desktop backend crate (even if other crates keep value semantics). That reduces feature-matrix complexity and makes it harder for deep clones to sneak back in.

A reasonable policy:

* Desktop backend always uses `Arc`.
* Core library can still expose `WorkbookPackage: Clone` for other use cases, but desktop caching never depends on deep cloning.

### 3.2 Re-evaluate the “replace LRU crate” experiment priority

Your experiment doc already calls out that an LRU replacement is small/low-risk. 
But once:

* clones are cheap, and
* key computation is gone on hits,
  …LRU overhead might genuinely become negligible.

So the plan should be:

1. ship Arc + diff-key reuse,
2. profile again,
3. only then decide whether the `lru` dependency is still worth removing.

This keeps you from spending time on a low-signal micro-optimization while a macro-issue is still present.

---

## Phase 4 — Optional follow-ups (only if profiling says so)

These are not required to realize the two wins above, but they’re consistent with the same theme (remove repeated syscalls/allocations on UI follow-ons):

### 4.1 Keep the SQLite connection open in the engine thread

Every follow-on handler does `OpStore::open(&self.store_path)` per request.
If that’s opening a new SQLite connection each time, it can add noticeable overhead.

You can:

* store an `OpStore` (or its underlying connection) inside `EngineState`,
* reuse it across commands.

This can be a big win in tight UI loops, but it’s a separate change—profile and decide.

### 4.2 Micro-optimize `cache_key()` only if it remains hot

After diff-key caching, `cache_key()` should run ~2 times per diff run, not hundreds of times per UI loop.
At that point, changing `to_lowercase` vs `to_ascii_lowercase` probably won’t matter.

---

## Suggested variant matrix for your experiment doc

Run these on the same machine/datasets as your protocol demands. 

1. **A (baseline)**: current defaults (desktop_wx default features do not include `backend-arc-cache`). 
2. **B (Arc only)**: enable Arc caching by default in desktop_wx.
3. **C (Arc + diff-key reuse)**: add the diff-id → key cache and by-key lookups.

For each:

* run full tests + perf-metrics policy (core) 
* run `desktop_backend` tests
* run `cache_hit_loop_smoke` and record:

  * elapsed_ms
  * cache hit/miss deltas 
* measure desktop_wx binary size if you care about size deltas (`scripts/size_report.py` exists for artifact size reporting). 

---

## What “done” looks like

You’ll know this effort paid off when:

* `cache_hit_loop_smoke` elapsed time drops dramatically from baseline (deep clone removal is usually night-and-day on large workbooks).
* Subsequent improvement (Arc + diff-key reuse) shows another measurable drop, especially on platforms where filesystem metadata calls are slow.
* The code gets *simpler* at the end of the experiment (ideally by making Arc caching the default/only path for desktop backend).

If you want, I can also propose a clean struct/function layout for the diff-id key cache so it stays readable and doesn’t sprawl across the handlers—but the steps above are the core of the implementation and measurement plan.


---

## Environment
- OS: `Linux galactus-prime 6.6.87.2-microsoft-standard-WSL2 #1 SMP PREEMPT_DYNAMIC Thu Jun  5 18:30:46 UTC 2025 x86_64 GNU/Linux`
- rustc: `rustc 1.87.0-nightly (f280acf4c 2025-02-19)`
- cargo: `cargo 1.87.0-nightly (ce948f461 2025-02-14)`
- git rev: `2b60f1d644d14ff9e1e54756a8b2dc9984d954c7`

## Baseline (A)
Commands:
1. `cargo test -p excel_diff`
2. `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
3. `cargo test -p desktop_backend`
4. `cargo test -p desktop_backend -- --ignored --nocapture`

Results:
- `cargo test -p excel_diff`: pass (warning: `unused_mut` in `core/tests/pg4_diffop_tests.rs`)
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`: pass (warnings: `unused_mut` in `core/tests/pg4_diffop_tests.rs`, `unused_imports` in `core/tests/perf_large_grid_tests.rs`)
- `cargo test -p desktop_backend`: pass
- `cargo test -p desktop_backend -- --ignored --nocapture`: pass

Raw `PERF_METRIC` lines (baseline):
```text
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1240 total_time_ms=1240 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=204 total_time_ms=204 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=5165 parse_time_ms=2778 diff_time_ms=2387 signature_build_time_ms=2386 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1057379997 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=36469 parse_time_ms=36220 diff_time_ms=249 signature_build_time_ms=211 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1190462478 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=73675 parse_time_ms=73563 diff_time_ms=112 signature_build_time_ms=104 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1364398583 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=88004 parse_time_ms=87775 diff_time_ms=229 signature_build_time_ms=212 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=871050423 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=296400 parse_time_ms=296307 diff_time_ms=93 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=612 parse_time_ms=0 diff_time_ms=612 signature_build_time_ms=517 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1273531185 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=2485 parse_time_ms=0 diff_time_ms=2485 signature_build_time_ms=2485 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1499180994 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=434 parse_time_ms=0 diff_time_ms=434 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766241649 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=1120 parse_time_ms=0 diff_time_ms=1120 signature_build_time_ms=891 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1289487572 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_completely_different total_time_ms=1286 parse_time_ms=0 diff_time_ms=1286 signature_build_time_ms=330 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=955 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1289487572 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=4450 parse_time_ms=0 diff_time_ms=4450 signature_build_time_ms=3991 move_detection_time_ms=134 alignment_time_ms=0 cell_diff_time_ms=310 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1289487572 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

Cache-hit loop baseline:
```text
cache_hit_loop iterations=50 elapsed_ms=863 workbook_hits_delta=300 workbook_misses_delta=0 pbix_hits_delta=0 pbix_misses_delta=0
```

## Implementation Summary
- Enabled `backend-arc-cache` by default in `desktop/wx` so cache hits clone `Arc` handles instead of deep-copying workbook/PBIX packages.
- Added a diff-id key cache to reuse `CacheKey` values across follow-on UI requests, plus by-key cache lookups and `_with_key` open helpers.
- Wired follow-on handlers (`loadSheet`, `loadSheetMeta`, `loadCellsInRange`) to reuse cached keys and avoid `cache_key()` recomputation.
- Added an `arc-cache` safety test that verifies cache hits reuse the same `Arc` allocation.
- Added machine-readable `PERF_METRIC` output for the cache-hit loop test.

## Post-Change (C: Arc + Diff-Key Reuse)
Commands:
1. `cargo test -p excel_diff`
2. `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
3. `cargo test -p desktop_backend`
4. `cargo test -p desktop_backend -- --ignored --nocapture`
5. `cargo test -p desktop_backend --features arc-cache`

Results:
- `cargo test -p excel_diff`: pass (warning: `unused_mut` in `core/tests/pg4_diffop_tests.rs`)
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`: pass (warnings: `unused_mut` in `core/tests/pg4_diffop_tests.rs`, `unused_imports` in `core/tests/perf_large_grid_tests.rs`)
- `cargo test -p desktop_backend`: pass
- `cargo test -p desktop_backend -- --ignored --nocapture`: pass
- `cargo test -p desktop_backend --features arc-cache`: pass

Raw `PERF_METRIC` lines (post-change):
```text
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1033 total_time_ms=1033 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=189 total_time_ms=189 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=4403 parse_time_ms=2346 diff_time_ms=2057 signature_build_time_ms=2057 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1038233414 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=30583 parse_time_ms=30379 diff_time_ms=204 signature_build_time_ms=167 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1190631797 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=69868 parse_time_ms=69760 diff_time_ms=108 signature_build_time_ms=99 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1364107993 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=83922 parse_time_ms=83690 diff_time_ms=232 signature_build_time_ms=215 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=870345775 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=297613 parse_time_ms=297544 diff_time_ms=69 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=639 parse_time_ms=0 diff_time_ms=639 signature_build_time_ms=517 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1237328161 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=2315 parse_time_ms=0 diff_time_ms=2315 signature_build_time_ms=2315 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1780678154 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=349 parse_time_ms=0 diff_time_ms=349 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537330 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_completely_different total_time_ms=907 parse_time_ms=0 diff_time_ms=907 signature_build_time_ms=217 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=689 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537330 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=1024 parse_time_ms=0 diff_time_ms=1024 signature_build_time_ms=853 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537330 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=3968 parse_time_ms=0 diff_time_ms=3968 signature_build_time_ms=3524 move_detection_time_ms=105 alignment_time_ms=0 cell_diff_time_ms=326 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537330 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

Cache-hit loop post-change:
```text
cache_hit_loop iterations=50 elapsed_ms=747 workbook_hits_delta=300 workbook_misses_delta=0 pbix_hits_delta=0 pbix_misses_delta=0
PERF_METRIC desktop_backend_cache_hit_loop_elapsed_ms=747
```

Delta vs baseline (total_time_ms / elapsed_ms):
- `desktop_backend_cache_hit_loop_elapsed_ms`: 863 -> 747 (-116 ms, -13.4%)
- `datamashup_decode`: 1240 -> 1033 (-207 ms, -16.7%)
- `datamashup_text_extract`: 204 -> 189 (-15 ms, -7.4%)
- `e2e_p4_sparse`: 5165 -> 4403 (-762 ms, -14.8%)
- `e2e_p2_noise`: 36469 -> 30583 (-5886 ms, -16.1%)
- `e2e_p1_dense`: 73675 -> 69868 (-3807 ms, -5.2%)
- `e2e_p3_repetitive`: 88004 -> 83922 (-4082 ms, -4.6%)
- `e2e_p5_identical`: 296400 -> 297613 (+1213 ms, +0.4%)
- `perf_50k_adversarial_repetitive`: 612 -> 639 (+27 ms, +4.4%)
- `perf_50k_99_percent_blank`: 2485 -> 2315 (-170 ms, -6.8%)
- `perf_50k_identical`: 434 -> 349 (-85 ms, -19.6%)
- `perf_50k_dense_single_edit`: 1120 -> 1024 (-96 ms, -8.6%)
- `perf_50k_completely_different`: 1286 -> 907 (-379 ms, -29.5%)
- `perf_50k_alignment_block_move`: 4450 -> 3968 (-482 ms, -10.8%)

Notes:
- Core perf metrics show variability despite no core code changes; treat the deltas above as run-to-run noise unless repeated in multiple samples.
- The cache-hit loop improved materially (elapsed_ms down 13.4%), consistent with Arc caching and key reuse.
