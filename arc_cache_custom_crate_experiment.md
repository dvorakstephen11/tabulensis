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
