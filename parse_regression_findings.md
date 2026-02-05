# Parse Regression Findings

Cycle: `2026-02-03_194917`

## Summary
- E2E regressions are almost entirely in `parse_time_ms`, with `diff_time_ms` and all diff sub-phases essentially flat.
- All input sizes and memory-related metrics are identical pre/post, which suggests the parse slowdown is not driven by different inputs or allocation behavior.
- Pre and post use the same git commit (`60b1eaaa099d`) and branch, so the delta is unlikely to be caused by a parser code change in this cycle.

## Evidence
- For every E2E test, `total_time_ms` increased by the same amount as `parse_time_ms`.
- `diff_time_ms` moved only slightly (+0 to +5 ms) and sub-phase timings (`signature_build_time_ms`, `alignment_time_ms`, etc.) did not change in a way that explains the total regression.
- Input sizes are stable: `old_bytes`, `new_bytes`, and `total_input_bytes` are unchanged across all tests.
- Memory metrics are stable: `peak_memory_bytes`, `grid_storage_bytes`, `string_pool_bytes`, and `alignment_buffer_bytes` are unchanged across all tests.

Concrete deltas (pre -> post):
- `e2e_p1_dense`: total +1004 ms, parse +1004 ms, diff +0 ms.
- `e2e_p2_noise`: total +124 ms, parse +122 ms, diff +2 ms.
- `e2e_p3_repetitive`: total +373 ms, parse +368 ms, diff +5 ms.
- `e2e_p4_sparse`: total +12 ms, parse +7 ms, diff +5 ms.
- `e2e_p5_identical`: total +3864 ms, parse +3863 ms, diff +1 ms.

## Interpretation
- The regression appears to be a pure parse-time slowdown unrelated to diff logic (all diff metrics are flat), and unrelated to input size or memory usage (all such metrics are unchanged).
- Given the identical commit and unchanged inputs, the most plausible cause is environmental variance (CPU frequency scaling, background load, disk cache state, or transient OS-level contention).

## Suggested Follow-ups (no code changes)
- Re-run the E2E perf suite twice to check for stability and cache effects, keeping the same build and environment.
- If available, pin CPU governor to performance or run with reduced background load to see if parse time normalizes.
- Compare parse times after a clean reboot vs. a warmed cache run to confirm I/O or caching influence.
