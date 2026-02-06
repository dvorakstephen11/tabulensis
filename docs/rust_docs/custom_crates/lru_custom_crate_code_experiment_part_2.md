Here’s a detailed implementation plan for the **LRU candidate** (the “tiny fixed-size cache for desktop backend”), plus the **exact test cases** you should have (including the ones you already implemented), and what I’d do next to decide whether to keep it.

---

## What the LRU candidate actually is in your codebase

In your codebase, the LRU cache is used only in the **desktop backend engine thread** to cache parsed packages:

* `workbook_cache: LruCache<CacheKey, WorkbookPackage>` with capacity **4**
* `pbix_cache: LruCache<CacheKey, PbixPackage>` with capacity **2** 

Those caches are hit inside:

* `open_workbook_cached()` and `open_pbix_cached()` which do:

  * compute a `CacheKey` (normalized path + metadata + trusted flag),
  * `cache.get(&key)` and if hit return `pkg.clone()`,
  * otherwise open + parse the file, then `cache.put(key, pkg.clone())`. 

The original motivation for rewriting `lru` was: *the cache sizes are tiny and predictable; a general-purpose LRU may be unnecessary overhead and dependency footprint.* 

---

## Implementation plan for the LRU candidate

### Phase 0 — Confirm you’re optimizing the right thing

Before doing (or promoting) a rewrite, answer two questions:

1. **Is the cache hot?**
   Instrument (even temporarily) cache hit/miss rates in `open_workbook_cached/open_pbix_cached`. If it’s mostly misses, the cache implementation won’t matter.

2. **Is the LRU operations cost even visible?**
   In your current usage, each “cache lookup” is dominated by:

   * `cache_key()` calling `canonicalize()` + `metadata()` and allocating/lowercasing a path string, and
   * cloning `WorkbookPackage` / `PbixPackage` (these are `#[derive(Clone)]` and contain large vectors/grids; clones can be deep). 
     That means: even if your tiny LRU were 10× faster than the crate, the end-to-end effect might still be ~0.

This matters because your Iteration 1 perf deltas came from running `excel_diff` perf-metrics, which (by dependency direction) shouldn’t be affected by changes isolated to `desktop_backend`. So treat those big improvements as *measurement noise / environmental variation* until proven otherwise.

What you should do instead is measure **desktop backend workloads** that repeatedly hit these caches (see “Measurement plan” below).

---

### Phase 1 — Create clean A/B/C build variants

You already did the right structural work:

* Make `lru` an **optional** dependency and gate it behind a baseline feature like `lru-crate` (enabled by default). 
* Add a `custom-lru` feature that removes `lru` from the graph. 
* Add feature forwarding in `desktop_wx` so the GUI can select the backend implementation cleanly. 

This is important because otherwise you can’t measure dependency footprint changes “for real”.

**The ideal steady-state matrix:**

* **A: Baseline** (default): `lru-crate` enabled.
* **B: Custom**: `custom-lru` enabled, `lru` removed from graph.
* **C: Parity** (tests only): both enabled so you can compare behavior.

(You already validated A/B/C in your iteration notes; structurally that’s perfect.)

---

### Phase 2 — Implement the tiny cache with an API-compatible subset

#### Goal

Replace only what you use from `lru::LruCache` in `diff_runner`:

* `LruCache::new(NonZeroUsize)`
* `get(&mut self, &K) -> Option<&V>` (and “hit promotes to MRU” semantics)
* `put(&mut self, K, V) -> Option<V>` (and insert/update promotes to MRU)

Your current `TinyLruCache` is exactly this: MRU-ordered `Vec<(K, V)>` with linear scan. 

#### Data structure choice

For capacity 2 and 4, linear scan is totally fine, and you can keep the implementation small and dependency-free.

What matters more than asymptotic complexity is “no surprises”:

* no hidden allocations in steady state,
* no semantic drift.

#### One improvement I would make to your `put`

Right now, the implementation does `insert(0, ...)` and then `truncate()`. That temporarily grows the `Vec` to `cap + 1`, which *can* trigger a one-time reallocation the first time the cache overflows.

It’s minor (and likely happens once), but you can make the implementation “fixed-size” in a stricter sense by popping before inserting when full and inserting a new key. I’d do this purely because it’s simple and removes even a theoretical allocation.

(If you want, I can give you the exact replacement code for just the `put()` method following your “no diffs” format.)

---

### Phase 3 — Wire it into `diff_runner` with `cfg` aliasing

You already used the cleanest approach:

* `use crate::tiny_lru::TinyLruCache as LruCache;` when `custom-lru`
* `use lru::LruCache;` when `lru-crate` 

This keeps the call sites identical and avoids runtime branching.

Also good: the compile-time guard that requires at least one feature. 

---

### Phase 4 — Testing strategy

You already have the right layers:

1. **Unit tests** on the tiny cache itself (deterministic, fast).
2. **Parity tests** (compile-only when both features enabled) that run the same operation traces against:

   * `TinyLruCache`
   * `lru::LruCache`
     and compare results + iteration order snapshots. 

That is exactly the right way to do this. No runtime fallback shipped, no branching in production paths.

---

### Phase 5 — Measurement plan (the part you still need)

Since the `lru` replacement is in **desktop_backend**, you need a measurement that actually exercises **desktop backend cache hits**.

I’d do three things:

#### 1) Confirm dependency and binary impact

This is the main likely win for this candidate.

* Dependency graph:

  * Baseline: `cargo tree -p desktop_backend -i lru`
  * Custom: `cargo tree -p desktop_backend -i lru --no-default-features --features "model-diff custom-lru"`

* Binary size:

  * Build release GUI or CLI entrypoints that include the backend (likely `desktop_wx`).
  * Compare stripped sizes.

* Build time:

  * Clean build and incremental build comparisons.

#### 2) A targeted desktop-backend “cache-hit” benchmark

Add a **single ignored test** (or an `xtask`) that:

* runs a diff once (to populate the store / baseline),
* then repeatedly calls:

  * `load_sheet_meta`, `load_cells_range`, `load_sheet` in a loop (these call `open_workbook_cached`),
* record total wall time and maybe count cache hits/misses.

This will measure the thing the cache affects (UI follow-on operations), rather than `excel_diff` core perf tests.

#### 3) Run A/B on the same commit

Because you now have feature flags, you can compare A and B without code changes, which massively reduces “apples-to-oranges”.

---

## Exact test cases to implement

You already have almost all of these in `desktop/backend/src/tiny_lru.rs`. I’m listing them explicitly with inputs and expected outcomes (and I’d keep them exactly as-is).

### Unit tests (tiny cache only)

1. **`get_on_empty_returns_none`**

   * Setup: `cap=2`, empty
   * Action: `get("a")`
   * Expect: `None`, `len == 0` 

2. **`put_then_get_hits`**

   * Setup: `cap=2`
   * Action: `put("a",1)` then `get("a")`
   * Expect: `put` returns `None`, `get` returns `Some(&1)`, `len == 1` 

3. **`put_existing_key_returns_old_value_and_updates`**

   * Setup: `cap=2`
   * Action: `put("a",1)`, then `put("a",9)`
   * Expect: second `put` returns `Some(1)`, `get("a") == Some(&9)`, `len == 1` 

4. **`evicts_lru_when_capacity_exceeded`**

   * Setup: `cap=2`
   * Action: `put("a",1)`, `put("b",2)`, `put("c",3)`
   * Expect: `"a"` is evicted; `get("a")==None`, `get("b")==Some(&2)`, `get("c")==Some(&3)`, `len==2` 

5. **`get_promotes_to_mru_affecting_eviction`**

   * Setup: `cap=3`
   * Action: `put a,b,c`; `get a` (promote); `put d`
   * Expect: `"b"` is evicted (because after `get a`, the LRU is `b`) 

6. **`put_counts_as_use_promotes_to_mru`**

   * Setup: `cap=2`
   * Action: `put a=1`, `put b=2`, `put a=3`, `put c=4`
   * Expect: `"b"` evicted; `a==3`, `c==4` 

7. **`get_miss_does_not_change_order`**

   * Setup: `cap=2`
   * Action: `put a`, `put b`, `get zzz` (miss), `put c`
   * Expect: eviction behaves as if miss didn’t alter MRU/LRU ordering; `"a"` should be evicted, `b` and `c` remain. 

8. **`repeated_gets_do_not_break_invariants`**

   * Setup: `cap=4`, `put a,b,c,d`
   * Action: repeated gets promoting various keys; then `put e`
   * Expect: correct eviction (here `"c"` gets evicted in the scripted sequence), and remaining keys return correct values. 

9. **`capacity_one_always_keeps_last_used`**

   * Setup: `cap=1`
   * Action: `put a`, `get a`, `put b`
   * Expect: `"a"` evicted; `b` remains; `len==1` 

10. **`iter_is_mru_order`** (test-only iterator)

* Setup: `cap=3`, `put a,b,c`
* Expect: iter yields `[c,b,a]` (MRU first)
* Then `get a`
* Expect: iter yields `[a,c,b]` 

### Parity tests (only when both `custom-lru` and `lru-crate` enabled)

1. **`parity_matches_lru_crate_example_sequence`**

   * Run a known sequence of `put/get` operations
   * Assert: each `get/put` result matches the crate and final iteration snapshots match. 

2. **`parity_random_operation_trace_small_keyspace`**

   * Deterministic pseudo-RNG generates 1000 ops:

     * 70% gets, 30% puts
     * small keyspace (0..8), cap=4
   * After each op:

     * compare `get/put` return values,
     * compare `len`,
     * compare iter snapshot ordering. 

3. **`parity_update_existing_key_does_not_evict_unnecessarily`**

   * Setup: cap=2
   * Sequence: put(1), put(2), put(2 updated), put(3)
   * Expect: no “extra eviction” due to updating an existing key; final snapshot equals `[(3,30),(2,21)]` (MRU first) and matches crate. 

### Non-LRU but important “does the backend still work” coverage

Your `desktop/backend/tests/integration_smoke.rs` provides end-to-end smoke validation (diff, export, search index). That’s valuable coverage to keep when toggling features. 

---

## Additional suggestions based on your current implementation

### 1) Don’t trust the current `excel_diff` perf deltas as evidence for the LRU change

Your LRU candidate lives in `desktop_backend` (`diff_runner`), but your perf metrics were collected by running `cargo test -p excel_diff ...`. Those numbers can move dramatically due to OS page cache, CPU frequency/turbo behavior, background load, etc., and they’re not measuring the changed code path anyway.

If you re-run anything, re-run **A/B on the same commit** by toggling features and measuring a **desktop backend workload** that hits `open_workbook_cached/open_pbix_cached`.
