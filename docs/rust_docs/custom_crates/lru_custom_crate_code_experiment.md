Below is a concrete, end-to-end implementation plan for the **`custom-lru` candidate** (desktop backend), aimed at: (1) removing the `lru` dependency, (2) keeping behavior identical for your use cases, and (3) fitting your existing A/B/C + parity-testing workflow.

---

## 0) What we’re replacing and what “correct” means

### Where `lru` is used today

In `desktop/backend/src/diff_runner.rs`, `EngineState` owns two caches: a workbook cache (capacity 4) and a PBIX cache (capacity 2).  

These caches back:

* `open_workbook_cached(...)`
* `open_pbix_cached(...)`

They do:

* `get(&key)` → if hit, return a **clone** of the cached package
* else open from disk, parse, then `put(key, pkg.clone())` 

### Behavioral contract you must preserve

You only use a tiny subset of LRU behavior:

* **Insert** (`put`)
* **Lookup** (`get`) that also counts as a “use” (affects eviction order)
* **Eviction** when capacity is exceeded

The candidate document explicitly calls out: eviction correctness + “hit semantics identical to `lru` crate”.  

So for correctness:

1. **If `get(k)` returns Some**, that entry must become **most recently used** (MRU).
2. **If `put(k, v)` inserts a new key**, it must be MRU.
3. **If `put(k, v)` overwrites an existing key**, it must become MRU (and not temporarily count twice).
4. **If adding would exceed capacity**, evict the **least recently used** (LRU).

Given capacities 4 and 2, any O(N) implementation is fine and likely *simpler* than the general-purpose crate. 

---

## 1) High-level design choice

### Data structure

Use a **tiny MRU-ordered vector**:

* `Vec<Entry<K, V>>`, where index `0` is MRU and the last element is LRU.
* `get` = linear scan for key, then move that entry to index 0.
* `put` = remove existing if present, insert new at index 0, then truncate to capacity.

This is intentionally “dumb” but correct and fast for N=2/4:

* fewer moving parts (no HashMap, no linked list),
* fewer dependencies,
* and performance is dominated by tiny linear scans + cheap memmoves.

This matches the suggested direction (“fixed-size array or VecDeque-based LRU specialized for small N”). 

---

## 2) Feature gating strategy (A/B/C compatible)

Your experiment framework expects:

* **A: Baseline** (third-party crate enabled by default)
* **B: Custom** (no third-party crate in graph)
* **C: Parity tests** (both implementations enabled for side-by-side tests)

This aligns with your broader experiment method: make the third-party dep optional behind a baseline feature enabled by default, add a `custom-*` feature, and add parity tests.  

### Proposed features for `desktop_backend`

* `lru-crate` (baseline, enabled by default): pulls in `dep:lru`
* `custom-lru` (custom implementation): uses your tiny cache

---

## 3) Step-by-step implementation plan

### Step 3.1 — Make `lru` optional in `desktop/backend/Cargo.toml`

Today `desktop_backend` depends on `lru = "0.12"` unconditionally. 

Change it so `lru` is optional and enabled only via a baseline feature.

**Goal / acceptance:**

* Variant **B** must compile with **no `lru` in `cargo tree`**.

---

### Step 3.2 — Add a tiny cache module

Create a new module, e.g.:

* `desktop/backend/src/tiny_lru.rs`

Implement `TinyLruCache<K, V>` with:

* `pub fn new(cap: NonZeroUsize) -> Self`
* `pub fn get(&mut self, key: &K) -> Option<&V>`
* `pub fn put(&mut self, key: K, value: V)`

Keep it minimal and generic.

**Invariants to enforce internally:**

* `entries.len() <= cap`
* keys are unique (no duplicates)
* MRU at index 0

**Goal / acceptance:**

* Unit tests prove eviction + hit semantics.
* No dependencies added.

---

### Step 3.3 — Wire `diff_runner.rs` to use either implementation

Currently `diff_runner.rs` directly imports and uses `lru::LruCache`. 

Change it so:

* When `custom-lru` is enabled, caches use `TinyLruCache`
* Otherwise, caches use `lru::LruCache` (baseline default)

**Goal / acceptance:**

* `EngineState` code stays almost unchanged (still calls `new`, `get`, `put`).  
* No runtime branching; this is compile-time selection.

---

### Step 3.4 — Add unit tests for the custom cache

Add tests in `tiny_lru.rs` (or a separate test file) that cover:

1. **Eviction order**
   Capacity 2:

* put A
* put B
* put C
  → A must be evicted

2. **Hit updates recency**
   Capacity 2:

* put A
* put B
* get A   (A becomes MRU)
* put C
  → B must be evicted (because A was recently used)

3. **Overwrite updates recency**
   Capacity 2:

* put A=1
* put B=2
* put A=3 (overwrite, becomes MRU)
* put C=4
  → B must be evicted

4. **Repeated gets don’t break invariants**
   Capacity 4:

* put A,B,C,D
* get B, get B, get A, get D
* put E
  → eviction must match LRU order computed from accesses

**Goal / acceptance:**

* These tests pass in **custom-only** builds (no lru crate required).

---

### Step 3.5 — Add parity tests vs `lru` crate (variant C)

The candidate doc explicitly asks for side-by-side tests comparing both implementations. 

Add `#[cfg(all(feature = "custom-lru", feature = "lru-crate"))]` tests that:

* Run the same operation sequences against:

  * `lru::LruCache`
  * `TinyLruCache`
* Validate the same outcomes (which keys exist after sequences, and optionally the return values of `put` if you implement them).

You don’t need introspection APIs; just test via `get` behavior and eviction outcomes.

**Goal / acceptance:**

* When both are enabled, tests demonstrate identical semantics on representative sequences.

---

## 4) How to run A/B/C for this candidate

This follows your existing A/B/C workflow. 

### A — Baseline (default behavior)

Since `desktop_backend` currently has default features `["model-diff"]`, you’ll extend that to also include `lru-crate` by default.

Run:

* `cargo test -p desktop_backend`
* `cargo build -p desktop_backend`

### B — Custom (no `lru` crate in the graph)

Run with no default features and explicitly enable what you need, including `custom-lru`.

Example:

* `cargo test -p desktop_backend --no-default-features --features "model-diff custom-lru"`

(Include `perf-metrics` too if you want, but keep this candidate isolated.)

Then confirm:

* `cargo tree -p desktop_backend -i lru` shows nothing.

### C — Parity tests

Enable both `custom-lru` and `lru-crate` so parity tests compile.

Example:

* `cargo test -p desktop_backend --features "custom-lru"`

If `lru-crate` is default, this will compile both; your parity tests should be gated as described.

---

## 5) Measuring “is it worth it?” for `custom-lru`

Your candidate doc frames the expected benefits as mostly dependency/simplicity, with small performance wins possible. 

### Measure the right things

For LRU in this specific location, the likely wins are:

* Smaller dependency tree / fewer transitive deps
* Potentially smaller binary
* Potentially faster clean builds / incremental builds

Your experiment plan explicitly encourages tracking build time and binary size when “simpler and smaller” is part of the motivation.  

#### Dependency footprint

* Baseline: `cargo tree -p desktop_backend -i lru`
* Custom: same command should return nothing

#### Binary size

You already have scripts for size reporting in `scripts/` (e.g., `size_report.py`, `check_size_budgets.py`). 

A reasonable measurement:

* Build `desktop_wx` in release and compare stripped size.

**Important wiring note:**
`desktop_wx` currently depends on `desktop_backend = { path = "./backend" }` (default features on). 
If `desktop_backend` default includes `lru-crate`, then simply enabling `custom-lru` somewhere won’t remove `lru` from the final app unless you also control the dependency’s default-features behavior from `desktop_wx`.

So for **full app** size/build-time measurement, you’ll likely want an extra (optional) step:

* In `desktop/wx/Cargo.toml`, set `desktop_backend = { path = "./backend", default-features = false, features = [...] }`
* Then define desktop_wx features that map to backend features (baseline vs custom).

If you only care about `desktop_backend` crate’s dependency tree and compile time in isolation, you can skip this.

---

## 6) Rollout strategy and decision criteria

### Rollout

1. Land the feature-gated custom implementation + tests.
2. Keep default on `lru-crate` until you’ve measured and are satisfied.
3. If it’s a clear win, you can flip default to `custom-lru` later (or just delete `lru` entirely).

### Decision criteria (consistent with your experiment framework)

Your candidate doc’s promotion criteria are a good fit here: no regressions, no memory harm, and clear complexity reduction; for LRU specifically, the “clear complexity reduction” and “dependency footprint delta” may be the primary wins even if runtime is neutral. 

---

## 7) Optional “get more out of this area” follow-up (not required for the LRU swap)

This is not part of the `lru` candidate itself, but it’s worth noticing because it could dwarf the cache-structure win:

On cache hit, you do `pkg.clone()` where `WorkbookPackage` / `PbixPackage` are `#[derive(Clone)]` (deep clones). 
If cloning is heavy relative to parsing for your workloads, a bigger optimization could be:

* cache `Arc<WorkbookPackage>` / `Arc<PbixPackage>` and return `Arc` clones, or
* refactor call sites to borrow cached values.

I’d treat that as a separate candidate (“cache value representation”) after the dependency-removal experiment, because it touches more code paths and types.

---

# Appendix: Concrete code-level checklist (what files change)

1. **desktop/backend/Cargo.toml**

* make `lru` optional
* add features:

  * `lru-crate = ["dep:lru"]`
  * `custom-lru = []`
  * include `lru-crate` in default

2. **desktop/backend/src/lib.rs**

* add `mod tiny_lru;` (likely behind `cfg(feature = "custom-lru")`)

3. **desktop/backend/src/tiny_lru.rs**

* implement `TinyLruCache`
* add unit tests
* add parity tests gated on `custom-lru` + `lru-crate`

4. **desktop/backend/src/diff_runner.rs**

* replace direct `use lru::LruCache;` with cfg-based type selection
* update `EngineState` fields to use chosen cache type
* keep `open_workbook_cached/open_pbix_cached` logic the same

---

## What you’re replacing, exactly

Right now the desktop backend’s diff engine keeps two tiny in-memory caches inside `EngineState`:

* `workbook_cache: LruCache<CacheKey, WorkbookPackage>` with capacity **4**
* `pbix_cache: LruCache<CacheKey, PbixPackage>` with capacity **2** 

Those caches are only used in:

* `open_workbook_cached`: `get(&key)` then `put(key, pkg.clone())` 
* `open_pbix_cached`: `get(&key)` then `put(key, pkg.clone())` 

And they currently use the external `lru` crate in `desktop/backend/src/diff_runner.rs` (`use lru::LruCache;`) .

Your experiment doc already calls this out as a “tiny predictable cache” case and suggests a fixed-size implementation behind a `custom-lru` feature. 

## Behavioral spec you must match

To safely swap implementations, you only need to match the subset of semantics you rely on today, plus a couple of behaviors that are easy to test and prevent “surprising” future bugs.

From the `lru` crate docs for v0.12.x:

* `new(cap: NonZeroUsize)` creates a bounded cache. ([wide.gitlabpages.inria.fr][1])
* `get(&mut self, k)` returns `Option<&V>` and **moves the key to the head (most recently used)** on hit. ([wide.gitlabpages.inria.fr][1])
* `put(&mut self, k, v)` inserts; if key existed it replaces and returns the old value (`Option<V>`). If inserting a new key causes the cache to exceed capacity, the **least recently used entry is evicted** (silently). ([wide.gitlabpages.inria.fr][1])
* `iter()` yields entries in **most-recently-used order** (useful for parity tests). ([wide.gitlabpages.inria.fr][1])

Even if you don’t call `iter()` in production, implementing it in the custom cache makes “differential tests” straightforward.

## Design choice for a tiny LRU in your situation

Because your capacities are 4 and 2, the simplest fast-enough design is:

* store entries in a `Vec<(K, V)>` kept in **MRU → LRU** order
* `get`: linear scan to find the key; if found, rotate/move that entry to index 0
* `put`: linear scan; if found, remove it and insert new at front; else if full, `pop()` last; then insert at front

This has:

* **O(N)** operations, but N ≤ 4 so it’s effectively constant-time and avoids hashing overhead + linked-list node churn
* minimal code and no dependency

This matches your “keep it simple” goal and the experiment doc’s “fixed-size array or VecDeque” direction.

If you later decide N might grow (say 128+), you can keep the feature-flag escape hatch to revert to the crate.

## Step-by-step implementation plan

### Phase 1: Add feature gating and make `lru` optional

**Goal:** keep default behavior unchanged, but allow a build that removes `lru` from the dependency graph.

1. In `desktop/backend/Carg:contentReference[oaicite:10]{index=10}the `lru = "0.12"`dependency to`optional = true` (it’s currently non-optional)

   * add two features:

     * `lru-crate = ["dep:lru"]`
     * `custom-lru = []`
   * add `lru-crate` to `default` features so normal builds keep working without any command line flags.

2. Add a compile-time guard so there’s no accidental “no cache backend selected” build:

   * if **neither** `custom-lru` nor `lru-crate` is enabled → `compional) if you want to prevent ambiguity, you can also fail when **both** are enabled, but I’d *avoid* that because “both enabled” is useful for parity testing. A better pattern is:

     * allow both, but prefer custom in production code when `custom-lru` is set
     * compile parity tests only when both are enabled

**Build matrix to support:**

* baseline: defaults → uses `lru` crate
* custom backend only: `--no-default-features --features "model-diff custom-lru"`
* parity: defaults + `--features custom-lru` (since defaults include `lru-crate`)

### Phase 2: Implement the tiny cache in a self-contained module

Create a new file: `desktop/backend/src/tiny_lru.rs` (or `small_lru.rs`).

Minimum API surface that matches how `diff_runner.rs` uses it:

* `pub(crate) struct TinyLruCache<K, V> { ... }`
* `pub(crate) fn new(cap: NonZeroUsize) -> Self`
* `pub(crate) fn get(&mut self, key: &K) -> Option<&V>`
* `pub(crate) fn put(&mut self, key: K, value: V) -> Option<V>`

I strongly recommend also implementing these for testability (and because they are trivial):

* `pub(crate) fn len(&self) -> usize`
* `pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)>`

**Trait bounds:**

* For your implementation: `K: Eq` is sufficient for linear scanning.
* For “drop-in expectations”: you can match the crate’s headline bound and use `K: Eq + Hash`, even if you don’t hash internally. This makes it harder to accidentally use keys that would have been invalid for the crate. ([wide.gitlabpages.inria.fr][1])

### Phase 3: Wire it into `diff_runner.rs` with minimal diffs

In `desktop/backend/src/diff_runner.rs` you currently import `lru::LruCache` and instantiate it in `EngineState::new`.

Replace the import with conditional selection:

* when `custom-lru` is enabled:

  * `use crate::tiny_lru::TinyLruCache as LruCache;`
* otherwise:

  * `use lru::LruCache;`

This approach keeps all existing types/field declarations intact (`workbook_cache: LruCache<...>`) and avoids churn.

No other logic should change in `open_workbook_cached` and `open_pbix_cached`; they should still do `get` then `put`. P
You want two layers:

1. **behavior tests** for the tiny LRU itself (no dependency on the crate)
2. **parity tests** that compare tiny LRU vs `lru` crate when both are built

This mirrors the “swap behind a feature + verify equivalence” strategy you used on the base64 work.

#### Where to put the tests

* Put unit tests in `desktop/backend/src/tiny_lru.rs` under `#[cfg(test)]`:

  * calds if you choose
  * fast and isolated

* Put parity tests in the same module gated by:

  * `#[cfg(all(test, feature = "custom-lru", feature = "lru-crate"))]`

That ensures:

* parity runs in your “defaults + custom-lru” test configuration
* parity does **not** require the `lru` crate when you’re testi build (`--no-default-features --features custom-lru`)

### Phase 5: Measurement and decision criteria

Because this is a tiny cache, you’re unlikely to see runtime wins in end-to-end diff timing. The wins to look for are:

* reduced dependency tree
* reduced compile time / incremental compile time for the desktop backend
* (possibly) slightly smaller binaries

Suggested measurements:

1. **Dependency delta**

   * compare `cargo tree -p desktop_backend` baseline vs custom-only build
   * confirm `lru` disappears in custom-only

2. **Compile timing**

   * compare clean build time for backend:

     * baseline (defaults)
     * custom-only (`--no-default-features --features "model-diff custom-lru"`)
   * if you have nightly available, `cargo build -Z timings` gives structured numbers

3. **Binary size**

   * compare release binary sizes for the desktop app or backend artifact

If the dependency removal is meaningful and tests are strong, keep it. If compile-size wins are negligible and you want to minimize custom code surface, rolling back is reasonable—this candidate is “nice to have,” not a core hot path.

## Exact test cases

Below are concrete test cases you should implement. I’m listing them as **deterministic sequences** with precise expectations.

### A. Core behavior tests for `TinyLruCache`

1. `get_on_empty_returns_none`

* cap = 2
* `get("a")` → `None`
* `len()` → 0

2. `put_then_get_hits`

* cap = 2
* `put("a", 1)` → `None`
* `get("a")` → `Some(&1)`
* `len()` → 1

3. `put_existing_key_returns_old_value_and_updates`

* cap = 2
* `put("a", 1)` → `None`
* `put("a", 9)` → `Some(1)`
* `get("a")` → `Some(&9)`
* `len()` → 1

4. `evicts_lru_when_capacity_exceeded`

* cap = 2
* `put("a", 1)`
* `put("b", 2)`
* `put("c", 3)` (this must evict `"a"`)
* `get("a")` → `None`
* `get("b")` → `Some(&2)`
* `get("c")` → `Some(&3)`
* `len()` → 2

5. `get_promotes_to_mru_affecting_eviction`

* cap = 3
* `put("a", 1)`  → order `[a]`
* `put("b", 2)`  → order `[b, a]`
* `put("c", 3)`  → order `[c, b, a]`
* `get("a")`     → order becomes `[a, c, b]`
* `put("d", 4)`  → evict `"b"` (LRU), order `[d, a, c]`
* expectations:

  * `get("b")` → `None`
  * `get("a")` → `Some(&1)`
  * `get("c")` → `Some(&3)`
  * `get("d")` → `Some(&4)`

6. `put_counts_as_use_promotes_to_mru`

* cap = 2
* `put("a", 1)` → `[a]`
* `put("b", 2)` → `[b, a]`
* `put("a", 3)` → should promote `"a"` to MRU: `[a, b]`
* `put("c", 4)` → should evict `"b"` (LRU)
* expectations:

  * `get("b")` → `None`
  * `get("a")` → `Some(&3)`
  * `get("c")` → `Some(&4)`

7. `get_miss_does_not_change_order`

* cap = 2
* `put("a", 1)` → `[a]`
* `put("b", 2)` → `[b, a]`
* `get("zzz")` → `None`, order should remain `[b, a]`
* `put("c", 3)` → should evict `"a"` (LRU)
* expectations:

  * `get("a")` → `None`
  * `get("b")` → `Some(&2)`
  * `get("c")` → `Some(&3)`

8. `capacity_one_always_keeps_last_used`

* cap = 1
* `put("a", 1)`
* `get("a")` → `Some(&1)`
* `put("b", 2)` → `"a"` evicted
* expectations:

  * `get("a")` → `None`
  * `get("b")` → `Some(&2)`
  * `len()` → 1

9. `iter_is_mru_order`
   (Only if you implement `iter()`.)

* cap = 3
* `put("a", 1)`
* `put("b", 2)`
* `put("c", 3)`
* collect `iter().map(|(k, _)| k)` → `["c","b","a"]`
* then `get("a")`
* collect keys again → `["a","c","b"]`

### B. Parity tests against the `lru` crate

All parity tests are only compiled when both `custom-lru` and `lru-crate` are enabled.

10. `parity_matches_lru_crate_example_sequence`
    Use the exact sequence from the v0.12.x docs example. ([wide.gitlabpages.inria.fr][2])

* cap = 2
* operations:

  * `put("apple", 3)`
  * `put("banana", 2)`
  * `get("apple")` → 3
  * `get("banana")` → 2
  * `get("pear")` → None
  * `put("banana", 4)` → returns old value 2
  * `put("pear", 5)` → returns None, and evicts `"apple"`
  * `get("pear")` → 5
  * `get("banana")` → 4
  * `get("apple")` → None
* assertions:

  * tiny and crate results match at each step
  * final `iter()` order matches as well (MRU order). ([wide.gitlabpages.inria.fr][1])

11. `parity_random_operation_trace_small_keyspace`
    This catches subtle ordering/eviction mistakes.

* cap = 4
* keys = 0..8 (u8)
* values = u16
* deterministic PRNG (no `rand` dependency): implement a tiny xorshift in-test
* run 1,000 ops:

  * 70%: `get(key)`
  * 30%: `put(key, next_value)`
* After *each* operation:

  * compare `len()`
  * compare `iter()` sequences as `Vec<(K,V)>` (clone/copy into owned values)

    * lru crate `iter()` is MRU order ([wide.gitlabpages.inria.fr][1])
  * optionally also compare point lookups using `peek()` on the crate side, and a non-mutating `peek()` you add to tiny cache (if you implement it)

12. `parity_update_existing_key_does_not_evict_unnecessarily`

* cap = 2
* `put(1, 10)`
* `put(2, 20)`
* `put(2, 21)` (update)
* `put(3, 30)` (should evict 1, not 2)
* assert both caches have exactly keys {2,3} with values {21,30}, and `iter()` is `[3,2]`

### C. “Dependency-free build” compilation test

13. `custom_lru_builds_without_lru_dependency`
    This is not a Rust `#[test]`; it’s a CI/build step.

* Run: `cargo test -p desktop_backend --no-default-features --features "model-diff custom-lru"`
* Expectations:

  * compiles and runs tests
  * parity tests are skipped because `lru-crate` is not enabled

## One extra improvement worth considering (optional, but high leverage)

Right now cache hits still clone the entire `WorkbookPackage` / `PbixPackage` (`return Ok(pkg.clone())`).

If those clones are heavy, caching might be leaving performance on the table. A future follow-up (separate from “remove `lru` dependency”) is:

* store `Arc<WorkbookPackage>` / `Arc<PbixPackage>` in the cache
* return `Arc` clones on hits (cheap)
* adjust call sites accordingly

That’s a larger behavioral change than swapping the LRU container, so I’d keep it as a second experiment after the tiny LRU swap is stable.

---

# Iteration 1 (2026-02-02)

## Baseline (pre-change) — tests + perf-metrics (excel_diff)

**Environment**
- Timestamp (UTC): 2026-02-02T16:44:31Z
- OS: Linux 6.6.87.2-microsoft-standard-WSL2 (x86_64)
- CPU: 11th Gen Intel(R) Core(TM) i7-11800H @ 2.30GHz (8C/16T)
- Rust: rustc 1.87.0-nightly (f280acf4c 2025-02-19)
- Cargo: cargo 1.87.0-nightly (ce948f461 2025-02-14)
- CPU scaling governor: unavailable (WSL)

**Commands (baseline)**
- `cargo test -p excel_diff`
  - Result: PASS
  - Notes: warning `unused_mut` in `core/tests/pg4_diffop_tests.rs`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
  - Result: PASS
  - Notes: warnings `unused_imports` (perf_large_grid_tests) + `unused_mut` (pg4_diffop_tests)

**Raw PERF_METRIC lines (baseline)**
```
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1619 total_time_ms=1619 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=372 total_time_ms=372 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=7591 parse_time_ms=4088 diff_time_ms=3503 signature_build_time_ms=3502 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1053510254 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=48835 parse_time_ms=48464 diff_time_ms=371 signature_build_time_ms=306 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1324303946 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=110616 parse_time_ms=110473 diff_time_ms=143 signature_build_time_ms=134 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1363330202 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=129649 parse_time_ms=129262 diff_time_ms=387 signature_build_time_ms=361 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=869944620 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=373668 parse_time_ms=373593 diff_time_ms=75 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=675 parse_time_ms=0 diff_time_ms=675 signature_build_time_ms=542 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1161524565 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=2458 parse_time_ms=0 diff_time_ms=2458 signature_build_time_ms=2458 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1738396933 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=454 parse_time_ms=0 diff_time_ms=454 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766245486 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_completely_different total_time_ms=1035 parse_time_ms=0 diff_time_ms=1035 signature_build_time_ms=224 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=810 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766245633 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=950 parse_time_ms=0 diff_time_ms=950 signature_build_time_ms=777 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766245633 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=4326 parse_time_ms=0 diff_time_ms=4326 signature_build_time_ms=3861 move_detection_time_ms=128 alignment_time_ms=0 cell_diff_time_ms=322 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766245633 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

## Implementation summary (custom-lru)
- Made `lru` optional and added `lru-crate` (default) + `custom-lru` features in `desktop/backend/Cargo.toml`.
- Added `desktop/backend/src/tiny_lru.rs` with a small MRU-ordered vector cache, plus unit tests and parity tests.
- Added compile-time feature guard and cfg-based cache selection in `desktop/backend/src/diff_runner.rs`.
- Added `custom-lru` module wiring in `desktop/backend/src/lib.rs`.
- Added desktop_wx feature forwarding so `desktop_backend` features can be controlled via `desktop/wx/Cargo.toml`.

## Desktop backend A/B/C checks

**A — Baseline (lru crate, default features)**
- `cargo test -p desktop_backend` → PASS
- `cargo build -p desktop_backend` → PASS
- `cargo tree -p desktop_backend -i lru` → shows `lru v0.12.5` as a dependency

**B — Custom-only (no lru in graph)**
- `cargo test -p desktop_backend --no-default-features --features "model-diff custom-lru"` → PASS
- `cargo build -p desktop_backend --no-default-features --features "model-diff custom-lru"` → PASS
- `cargo tree -p desktop_backend -i lru --no-default-features -F "model-diff custom-lru"` → error: `package ID specification lru did not match any packages` (expected; confirms removal)

**C — Parity (custom + lru-crate)**
- `cargo test -p desktop_backend --features "custom-lru"` → PASS (parity tests executed)

## Post-change — tests + perf-metrics (excel_diff)

**Commands (post-change)**
- `cargo test -p excel_diff`
  - Result: PASS
  - Notes: warning `unused_mut` in `core/tests/pg4_diffop_tests.rs`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
  - Result: PASS
  - Notes: warnings `unused_imports` (perf_large_grid_tests) + `unused_mut` (pg4_diffop_tests)

**Raw PERF_METRIC lines (post-change)**
```
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1131 total_time_ms=1131 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=203 total_time_ms=203 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=5391 parse_time_ms=2942 diff_time_ms=2449 signature_build_time_ms=2448 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1050703843 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=31202 parse_time_ms=30996 diff_time_ms=206 signature_build_time_ms=163 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1234276374 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=72423 parse_time_ms=72327 diff_time_ms=96 signature_build_time_ms=88 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1363502828 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=87535 parse_time_ms=87275 diff_time_ms=260 signature_build_time_ms=241 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=870389305 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=326778 parse_time_ms=326705 diff_time_ms=73 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=614 parse_time_ms=0 diff_time_ms=614 signature_build_time_ms=504 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1143771157 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=2348 parse_time_ms=0 diff_time_ms=2348 signature_build_time_ms=2347 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1780678154 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=396 parse_time_ms=0 diff_time_ms=396 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537469 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_completely_different total_time_ms=988 parse_time_ms=0 diff_time_ms=988 signature_build_time_ms=267 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=720 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537469 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=912 parse_time_ms=0 diff_time_ms=912 signature_build_time_ms=741 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537469 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=3975 parse_time_ms=0 diff_time_ms=3975 signature_build_time_ms=3497 move_detection_time_ms=126 alignment_time_ms=0 cell_diff_time_ms=337 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537469 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

## Delta vs baseline (excel_diff perf-metrics)

Notes:
- Median/p95 not reported by the harness for these tests; values are single-run totals.
- Binary size delta: not measured in this iteration.

| Workload | total_time_ms (baseline → post) | Δ% | peak_memory_bytes (baseline → post) | Δ% |
|---|---:|---:|---:|---:|
| datamashup_decode | 1619 → 1131 | -30.14% | 43,288,732 → 43,288,732 | +0.00% |
| datamashup_text_extract | 372 → 203 | -45.43% | 69,779,305 → 69,779,305 | +0.00% |
| e2e_p4_sparse | 7,591 → 5,391 | -28.98% | 1,053,510,254 → 1,050,703,843 | -0.27% |
| e2e_p2_noise | 48,835 → 31,202 | -36.11% | 1,324,303,946 → 1,234,276,374 | -6.80% |
| e2e_p1_dense | 110,616 → 72,423 | -34.53% | 1,363,330,202 → 1,363,502,828 | +0.01% |
| e2e_p3_repetitive | 129,649 → 87,535 | -32.48% | 869,944,620 → 870,389,305 | +0.05% |
| e2e_p5_identical | 373,668 → 326,778 | -12.55% | 805,169,683 → 805,169,683 | +0.00% |
| perf_50k_adversarial_repetitive | 675 → 614 | -9.04% | 1,161,524,565 → 1,143,771,157 | -1.53% |
| perf_50k_99_percent_blank | 2,458 → 2,348 | -4.48% | 1,738,396,933 → 1,780,678,154 | +2.43% |
| perf_50k_identical | 454 → 396 | -12.78% | 1,766,245,486 → 1,766,537,469 | +0.02% |
| perf_50k_completely_different | 1,035 → 988 | -4.54% | 1,766,245,633 → 1,766,537,469 | +0.02% |
| perf_50k_dense_single_edit | 950 → 912 | -4.00% | 1,766,245,633 → 1,766,537,469 | +0.02% |
| perf_50k_alignment_block_move | 4,326 → 3,975 | -8.11% | 1,766,245,633 → 1,766,537,469 | +0.02% |

## Notes / follow-ups
- No functional regressions detected in desktop_backend or excel_diff test suites.
- Optional follow-up in the guidance (switching cached values to `Arc<...>` to avoid deep clones) was not implemented in this iteration because it is explicitly described as a separate candidate.

---

# Size/build measurements (desktop_wx) — baseline vs custom-lru (2026-02-02)

## Build times

**Baseline (default features → backend-lru-crate)**
- Clean: `cargo clean -p desktop_wx` (Removed 669 files, 1.0GiB total)
- Clean build: `/usr/bin/time -p cargo build -p desktop_wx --release --bin desktop_wx`
  - Result: PASS (warning `unused_mut` in `desktop/wx/src/main.rs`)
  - real 247.76s, user 2278.99s, sys 261.06s
- Incremental: `touch desktop/backend/src/lib.rs` then `/usr/bin/time -p cargo build -p desktop_wx --release --bin desktop_wx`
  - Result: PASS (warning `unused_mut` in `desktop/wx/src/main.rs`)
  - real 12.82s, user 67.81s, sys 1.85s

**Custom-lru (`--no-default-features --features "backend-custom-lru"`)**
- Clean: `cargo clean` (Removed 93,362 files, 35.9GiB total)
- Clean build: `/usr/bin/time -p cargo build -p desktop_wx --release --bin desktop_wx --no-default-features --features "backend-custom-lru"`
  - Result: PASS (warning `unused_mut` in `desktop/wx/src/main.rs`)
  - real 260.58s, user 2401.07s, sys 273.97s
- Incremental: `touch desktop/backend/src/lib.rs` then `/usr/bin/time -p cargo build -p desktop_wx --release --bin desktop_wx --no-default-features --features "backend-custom-lru"`
  - Result: PASS (warning `unused_mut` in `desktop/wx/src/main.rs`)
  - real 13.29s, user 71.33s, sys 1.88s

**Notes**
- The baseline “clean build” used `cargo clean -p desktop_wx`, while custom-lru used a full `cargo clean`, so clean-build timing comparisons may include differences in cache warmness.

## Binary size (stripped) — desktop_wx

**Baseline (default features)**
- `cp target/release/desktop_wx target/size_artifacts/desktop_wx_baseline`
- `strip target/size_artifacts/desktop_wx_baseline`
- `python3 scripts/size_report.py --label desktop_wx_baseline_stripped --path target/size_artifacts/desktop_wx_baseline --zip --out target/size_reports/desktop_wx_baseline.json`
  - raw_bytes: 23,652,144
  - zip_bytes: 9,013,850

**Custom-lru**
- `cp target/release/desktop_wx target/size_artifacts/desktop_wx_custom_lru`
- `strip target/size_artifacts/desktop_wx_custom_lru`
- `python3 scripts/size_report.py --label desktop_wx_custom_lru_stripped --path target/size_artifacts/desktop_wx_custom_lru --zip --out target/size_reports/desktop_wx_custom_lru.json`
  - raw_bytes: 23,630,224
  - zip_bytes: 9,001,163

**Delta (custom-lru vs baseline)**
- raw_bytes: -21,920 (-0.09%)
- zip_bytes: -12,687 (-0.14%)

---

# Iteration 2 (2026-02-02) — cache value representation (Arc)

## Baseline (pre-change) — tests + perf-metrics (excel_diff)

**Environment**
- Timestamp (UTC): 2026-02-02T17:38:37Z
- OS: Linux 6.6.87.2-microsoft-standard-WSL2 (x86_64)
- CPU: 11th Gen Intel(R) Core(TM) i7-11800H @ 2.30GHz (8C/16T)
- Rust: rustc 1.87.0-nightly (f280acf4c 2025-02-19)
- Cargo: cargo 1.87.0-nightly (ce948f461 2025-02-14)
- CPU scaling governor: unavailable (WSL)

**Commands (baseline)**
- `cargo test -p excel_diff`
  - Result: PASS
  - Notes: warning `unused_mut` in `core/tests/pg4_diffop_tests.rs`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
  - Result: PASS
  - Notes: warnings `unused_imports` (perf_large_grid_tests) + `unused_mut` (pg4_diffop_tests)

**Raw PERF_METRIC lines (baseline)**
```
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1181 total_time_ms=1181 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=201 total_time_ms=201 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=5168 parse_time_ms=2802 diff_time_ms=2366 signature_build_time_ms=2366 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1053552090 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=31545 parse_time_ms=31310 diff_time_ms=235 signature_build_time_ms=197 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1190355278 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=73068 parse_time_ms=72954 diff_time_ms=114 signature_build_time_ms=105 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1363759372 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=88573 parse_time_ms=88239 diff_time_ms=334 signature_build_time_ms=314 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=870427615 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=318952 parse_time_ms=318877 diff_time_ms=75 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=741 parse_time_ms=0 diff_time_ms=741 signature_build_time_ms=569 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1161524581 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=2562 parse_time_ms=0 diff_time_ms=2562 signature_build_time_ms=2561 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1739182441 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=422 parse_time_ms=0 diff_time_ms=422 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537362 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_completely_different total_time_ms=1154 parse_time_ms=0 diff_time_ms=1154 signature_build_time_ms=288 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=865 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537509 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=1285 parse_time_ms=0 diff_time_ms=1285 signature_build_time_ms=1067 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537509 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=4612 parse_time_ms=0 diff_time_ms=4612 signature_build_time_ms=4120 move_detection_time_ms=129 alignment_time_ms=0 cell_diff_time_ms=345 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537509 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

## Implementation summary (Arc cache values)
- Added `arc-cache` feature to `desktop/backend/Cargo.toml` and exposed it via `desktop/wx/Cargo.toml`.
- Switched cache value representation to `Arc<WorkbookPackage>` / `Arc<PbixPackage>` when `arc-cache` is enabled.
- Kept call sites unchanged by using `WorkbookHandle`/`PbixHandle` type aliases and `wrap_*` helpers.

**Verification (desktop_backend)**
- `cargo test -p desktop_backend --features "custom-lru arc-cache"` → PASS

## Post-change — tests + perf-metrics (excel_diff)

**Commands (post-change)**
- `cargo test -p excel_diff`
  - Result: PASS
  - Notes: warning `unused_mut` in `core/tests/pg4_diffop_tests.rs`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
  - Result: PASS
  - Notes: warnings `unused_imports` (perf_large_grid_tests) + `unused_mut` (pg4_diffop_tests)

**Raw PERF_METRIC lines (post-change)**
```
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1188 total_time_ms=1188 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=220 total_time_ms=220 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=5816 parse_time_ms=3234 diff_time_ms=2582 signature_build_time_ms=2582 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1059018180 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=34116 parse_time_ms=33874 diff_time_ms=242 signature_build_time_ms=199 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1324018538 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=78024 parse_time_ms=77905 diff_time_ms=119 signature_build_time_ms=110 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1423838234 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=91769 parse_time_ms=91509 diff_time_ms=260 signature_build_time_ms=237 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=869719639 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=347217 parse_time_ms=347137 diff_time_ms=80 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=670 parse_time_ms=0 diff_time_ms=670 signature_build_time_ms=566 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1161524581 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=2450 parse_time_ms=0 diff_time_ms=2450 signature_build_time_ms=2450 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1738100348 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=467 parse_time_ms=0 diff_time_ms=467 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537362 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=1149 parse_time_ms=0 diff_time_ms=1149 signature_build_time_ms=928 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537362 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_completely_different total_time_ms=1130 parse_time_ms=0 diff_time_ms=1130 signature_build_time_ms=252 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=878 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537362 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=4378 parse_time_ms=0 diff_time_ms=4378 signature_build_time_ms=3941 move_detection_time_ms=118 alignment_time_ms=0 cell_diff_time_ms=306 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1766537362 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

## Delta vs baseline (excel_diff perf-metrics)

| Workload | total_time_ms (baseline → post) | Δ% | peak_memory_bytes (baseline → post) | Δ% |
|---|---:|---:|---:|---:|
| datamashup_decode | 1,181 → 1,188 | +0.59% | 43,288,732 → 43,288,732 | +0.00% |
| datamashup_text_extract | 201 → 220 | +9.45% | 69,779,305 → 69,779,305 | +0.00% |
| e2e_p4_sparse | 5,168 → 5,816 | +12.54% | 1,053,552,090 → 1,059,018,180 | +0.52% |
| e2e_p2_noise | 31,545 → 34,116 | +8.15% | 1,190,355,278 → 1,324,018,538 | +11.23% |
| e2e_p1_dense | 73,068 → 78,024 | +6.78% | 1,363,759,372 → 1,423,838,234 | +4.41% |
| e2e_p3_repetitive | 88,573 → 91,769 | +3.61% | 870,427,615 → 869,719,639 | -0.08% |
| e2e_p5_identical | 318,952 → 347,217 | +8.86% | 805,169,683 → 805,169,683 | +0.00% |
| perf_50k_adversarial_repetitive | 741 → 670 | -9.58% | 1,161,524,581 → 1,161,524,581 | +0.00% |
| perf_50k_99_percent_blank | 2,562 → 2,450 | -4.37% | 1,739,182,441 → 1,738,100,348 | -0.06% |
| perf_50k_identical | 422 → 467 | +10.66% | 1,766,537,362 → 1,766,537,362 | +0.00% |
| perf_50k_completely_different | 1,154 → 1,130 | -2.08% | 1,766,537,509 → 1,766,537,362 | -0.00% |
| perf_50k_dense_single_edit | 1,285 → 1,149 | -10.58% | 1,766,537,509 → 1,766,537,362 | -0.00% |
| perf_50k_alignment_block_move | 4,612 → 4,378 | -5.07% | 1,766,537,509 → 1,766,537,362 | -0.00% |

## Notes / follow-ups
- Arc-based cache values increased end-to-end times on several e2e workloads in this run; memory deltas were mixed.
- The custom-lru dependency removal benefits are covered above; Arc cache representation should be evaluated independently with additional runs or larger sample sizes if pursued.

---

# Iteration 3 (2026-02-02) — cache stats + cache-hit benchmark + tiny_lru put refinement

## Baseline (pre-change)

Baseline for this iteration reuses the **Iteration 2 post-change** perf-metrics (same codebase state for excel_diff). No excel_diff code changes occurred between Iteration 2 post-change and this iteration’s desktop_backend-only edits.

## Implementation summary
- `TinyLruCache::put` now pops the LRU entry *before* inserting a new key when at capacity to avoid a temporary `len = cap + 1` growth.
- Added cache hit/miss counters in `EngineState` and a `DiffRunner::cache_stats()` accessor.
- Added ignored test `cache_hit_loop_smoke` to exercise repeated cache hits via `load_sheet_meta`, `load_cells_in_range`, and `load_sheet_payload`.

## Verification (desktop_backend)
- `cargo test -p desktop_backend --features "custom-lru"` → PASS (includes parity tests; `cache_hit_loop_smoke` ignored)

## Post-change — tests + perf-metrics (excel_diff)

**Commands (post-change)**
- `cargo test -p excel_diff`
  - Result: PASS
  - Notes: warning `unused_mut` in `core/tests/pg4_diffop_tests.rs`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
  - Result: PASS
  - Notes: warnings `unused_imports` (perf_large_grid_tests) + `unused_mut` (pg4_diffop_tests)

**Raw PERF_METRIC lines (post-change)**
```
PERF_METRIC datamashup_decode fixture=synthetic_datamashup_8mib iterations=6 payload_chars=11626319 decoded_bytes=8388608 parse_time_ms=1384 total_time_ms=1384 peak_memory_bytes=43288732
PERF_METRIC datamashup_text_extract iterations=4 payload_chars=11626319 parse_time_ms=309 total_time_ms=309 peak_memory_bytes=69779305
PERF_METRIC e2e_p4_sparse total_time_ms=7010 parse_time_ms=3917 diff_time_ms=3093 signature_build_time_ms=3093 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1050336634 grid_storage_bytes=3670016 string_pool_bytes=3844681 op_buffer_bytes=0 alignment_buffer_bytes=10133944 rows_processed=99994 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=533076 new_bytes=533089 total_input_bytes=1066165
PERF_METRIC e2e_p2_noise total_time_ms=46707 parse_time_ms=46378 diff_time_ms=329 signature_build_time_ms=266 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1324126501 grid_storage_bytes=48000960 string_pool_bytes=1786 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=12684057 new_bytes=12684051 total_input_bytes=25368108
PERF_METRIC e2e_p1_dense total_time_ms=110557 parse_time_ms=110402 diff_time_ms=155 signature_build_time_ms=146 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1423577853 grid_storage_bytes=48000960 string_pool_bytes=92214090 op_buffer_bytes=0 alignment_buffer_bytes=40426248 rows_processed=100002 cells_compared=140 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=3280973 new_bytes=3280995 total_input_bytes=6561968
PERF_METRIC e2e_p3_repetitive total_time_ms=128016 parse_time_ms=127680 diff_time_ms=336 signature_build_time_ms=306 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=869785483 grid_storage_bytes=120002400 string_pool_bytes=454894 op_buffer_bytes=0 alignment_buffer_bytes=88465368 rows_processed=100002 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=8065706 new_bytes=8065730 total_input_bytes=16131436
PERF_METRIC e2e_p5_identical total_time_ms=474883 parse_time_ms=474785 diff_time_ms=98 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=805169683 grid_storage_bytes=240004800 string_pool_bytes=479697619 op_buffer_bytes=0 alignment_buffer_bytes=168530568 rows_processed=100002 cells_compared=5000100 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 old_bytes=16406390 new_bytes=16406392 total_input_bytes=32812782
PERF_METRIC perf_50k_adversarial_repetitive total_time_ms=860 parse_time_ms=0 diff_time_ms=860 signature_build_time_ms=715 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1161524581 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=350 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <120s; target: <15s)
PERF_METRIC perf_50k_99_percent_blank total_time_ms=3552 parse_time_ms=0 diff_time_ms=3552 signature_build_time_ms=3551 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1739182441 grid_storage_bytes=3670016 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=10134544 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <2s)
PERF_METRIC perf_50k_identical total_time_ms=530 parse_time_ms=0 diff_time_ms=530 signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1764937362 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=0 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (target: <1s)
PERF_METRIC perf_50k_dense_single_edit total_time_ms=1417 parse_time_ms=0 diff_time_ms=1417 signature_build_time_ms=1161 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1764937509 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=700 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <30s; target: <5s)
PERF_METRIC perf_50k_completely_different total_time_ms=1457 parse_time_ms=0 diff_time_ms=1457 signature_build_time_ms=362 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=1094 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1764937509 grid_storage_bytes=240000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=168527200 rows_processed=100000 cells_compared=5000000 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 (enforced: <60s; target: <10s)
PERF_METRIC perf_50k_alignment_block_move total_time_ms=6079 parse_time_ms=0 diff_time_ms=6079 signature_build_time_ms=5527 move_detection_time_ms=143 alignment_time_ms=0 cell_diff_time_ms=392 op_emit_time_ms=0 report_serialize_time_ms=0 peak_memory_bytes=1764937509 grid_storage_bytes=120000000 string_pool_bytes=203 op_buffer_bytes=448 alignment_buffer_bytes=88463600 rows_processed=100000 cells_compared=0 anchors_found=0 moves_detected=1 hash_lookups_est=5000000 allocations_est=100100 (alignment/move coverage)
```

## Delta vs baseline (excel_diff perf-metrics)

| Workload | total_time_ms (baseline → post) | Δ% | peak_memory_bytes (baseline → post) | Δ% |
|---|---:|---:|---:|---:|
| datamashup_decode | 1,188 → 1,384 | +16.50% | 43,288,732 → 43,288,732 | +0.00% |
| datamashup_text_extract | 220 → 309 | +40.45% | 69,779,305 → 69,779,305 | +0.00% |
| e2e_p4_sparse | 5,816 → 7,010 | +20.53% | 1,059,018,180 → 1,050,336,634 | -0.82% |
| e2e_p2_noise | 34,116 → 46,707 | +36.91% | 1,324,018,538 → 1,324,126,501 | +0.01% |
| e2e_p1_dense | 78,024 → 110,557 | +41.70% | 1,423,838,234 → 1,423,577,853 | -0.02% |
| e2e_p3_repetitive | 91,769 → 128,016 | +39.50% | 869,719,639 → 869,785,483 | +0.01% |
| e2e_p5_identical | 347,217 → 474,883 | +36.77% | 805,169,683 → 805,169,683 | +0.00% |
| perf_50k_adversarial_repetitive | 670 → 860 | +28.36% | 1,161,524,581 → 1,161,524,581 | +0.00% |
| perf_50k_99_percent_blank | 2,450 → 3,552 | +44.98% | 1,738,100,348 → 1,739,182,441 | +0.06% |
| perf_50k_identical | 467 → 530 | +13.49% | 1,766,537,362 → 1,764,937,362 | -0.09% |
| perf_50k_dense_single_edit | 1,149 → 1,417 | +23.32% | 1,766,537,362 → 1,764,937,509 | -0.09% |
| perf_50k_completely_different | 1,130 → 1,457 | +28.94% | 1,766,537,362 → 1,764,937,509 | -0.09% |
| perf_50k_alignment_block_move | 4,378 → 6,079 | +38.85% | 1,766,537,362 → 1,764,937,509 | -0.09% |

## Notes / follow-ups
- The new cache-hit benchmark is available via `cargo test -p desktop_backend --features "custom-lru" -- --ignored --nocapture cache_hit_loop_smoke` to capture hit/miss deltas and wall time.
- The excel_diff perf swings here are not attributed to the LRU change (desktop_backend-only); treat as environmental noise unless corroborated by a backend-specific benchmark.

## Cache-hit benchmark (desktop_backend)

Command:
- `cargo test -p desktop_backend --features "custom-lru" -- --ignored --nocapture cache_hit_loop_smoke`

Output:
```
cache_hit_loop iterations=50 elapsed_ms=1521 workbook_hits_delta=300 workbook_misses_delta=0 pbix_hits_delta=0 pbix_misses_delta=0
```

Interpretation:
- 50 iterations yielded 300 workbook cache hits (6 hits per iteration) and 0 misses; PBIX cache was not used.
