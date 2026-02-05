# Desktop Performance + UX Improvement Plan

**Date:** 2026-02-05  
**Scope:** Desktop responsiveness, first-run diff latency, and perceived UX speed  
**Status:** Initial instrumentation complete; optimization plan prioritized

## 1. Goal

Make the desktop app feel dramatically faster by:
- Reducing first-run diff latency (especially workbook open/parse time).
- Improving perceived responsiveness during long operations.
- Preserving correctness and deterministic outputs.

## 2. What Was Measured

### 2.1 Existing e2e perf data (repo baseline artifacts)

From `benchmarks/latest_e2e.json`, parse dominates end-to-end runtime:
- Aggregate: `parse_time_ms = 37,413 / 37,526 total` (`99.7%`)
- `e2e_p5_identical`: `25,110 / 25,129` parse (`99.9%`)

Conclusion: the biggest wins are in workbook open/parse path, not diff core logic.

### 2.2 New targeted instrumentation (added)

Added opt-in open-path profiling (`EXCEL_DIFF_PROFILE_OPEN=1`) in:
- `core/src/excel_open_xml.rs`
- `core/src/package.rs`

This emits:
- `PERF_OPEN_WORKBOOK ...`
- `PERF_OPEN_PACKAGE ...`

Measured command:

```bash
EXCEL_DIFF_PROFILE_OPEN=1 cargo test --release --features perf-metrics --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1
```

### 2.3 Instrumentation findings (aggregated across 10 package opens)

| Stage | Time (ms) | Share |
| --- | ---: | ---: |
| `sheet_parse_ms` | 24,055 | 80.83% |
| `drawing_parse_ms` | 4,167 | 14.00% |
| `sheet_read_ms` | 1,474 | 4.95% |
| all other tracked stages | 0 | ~0% |

Per-case open totals (old+new workbook in each e2e):
- `e2e_p1_dense`: `3,526ms`
- `e2e_p2_noise`: `1,503ms`
- `e2e_p3_repetitive`: `4,445ms`
- `e2e_p4_sparse`: `116ms`
- `e2e_p5_identical`: `20,170ms`

### 2.4 Syscall sanity check

`strace -c` on `e2e_p5_identical` showed low syscall-heavy read/open share relative to total runtime. This supports the conclusion that user-space XML parse work dominates.

## 3. Root-Cause Summary

1. **Worksheet XML parsing is the primary bottleneck.**
2. **A second pass over worksheet XML for drawing refs is a meaningful secondary cost.**
3. **I/O exists, but it is not the dominant component in observed runs.**
4. **UX currently reflects backend latency too literally; perceived responsiveness can improve independently.**

## 4. Prioritized Improvement Roadmap

## P0 (highest impact, lowest delay): Parse path and immediate UX

### P0.1 Eliminate extra worksheet XML pass for drawing refs

Current behavior in `open_workbook_from_container`:
- Parse sheet XML to build grid.
- Parse same sheet XML again for drawing references (`parse_worksheet_drawing_rids`).

Plan:
- Collect drawing relationship ids during main sheet parse, or return them from parsing pipeline.
- Avoid re-reading/re-parsing same XML bytes for drawing lookup.

Targets:
- `core/src/excel_open_xml.rs`
- `core/src/grid_parser.rs`

Expected impact:
- Cut most of `drawing_parse_ms` (~14% of open path in sampled run).

### P0.2 UX stage clarity + progress quality

Plan:
- Expose parse sub-stages in desktop progress text (old open, new open, diff, snapshot).
- Keep status updates stable and monotonic to reduce perceived jitter.

Targets:
- `desktop/backend/src/diff_runner.rs`
- `desktop/wx/src/main.rs`

Expected impact:
- Better perceived speed and user trust during long parses.

### P0.3 Stop eagerly writing huge JSON blobs into text controls

Current behavior:
- Large summary/detail payloads are serialized and written to `TextCtrl` immediately.

Plan:
- Render concise summaries by default.
- Load full JSON/details lazily (on tab focus or explicit action).

Targets:
- `desktop/wx/src/main.rs`

Expected impact:
- Better UI responsiveness after compare completion, especially on large runs.

## P1 (high impact, moderate complexity): Parser and container efficiency

### P1.1 Streaming sheet parse from ZIP entry (avoid full sheet XML materialization)

Current behavior:
- `read_file_checked` reads full part into memory (`Vec<u8>`), then parser walks bytes.

Plan:
- Add stream-capable parse path from ZIP entry reader where feasible.
- Keep existing path as fallback for compatibility.

Targets:
- `core/src/container.rs`
- `core/src/excel_open_xml.rs`
- `core/src/grid_parser.rs`

Expected impact:
- Lower memory spikes and potentially better throughput for very large sheets.

### P1.2 Capacity-aware ZIP reads

Current behavior:
- `Vec::new()` + `read_to_end` in container path.

Plan:
- Reserve using known uncompressed entry size before reading.

Targets:
- `core/src/container.rs`

Expected impact:
- Smaller allocation churn; incremental but low-risk gain.

## P2 (high payoff, higher risk): Concurrency model for workbook opening

### P2.1 Parallel old/new package open

Plan:
- Evaluate safe parallelization design while preserving string-pool correctness and deterministic behavior.
- Prototype behind a feature flag.

Targets:
- `core/src/package.rs`
- `desktop/backend/src/diff_runner.rs`

Risks:
- Shared string pool semantics across packages.
- Increased complexity in deterministic output guarantees.

Expected impact:
- Potential near-2x wall-clock improvement on multi-core machines for parse-heavy scenarios.

## P3 (UX depth): Navigation and interaction improvements for large outputs

### P3.1 Large-mode UX optimization

Plan:
- Make large-mode default UI path summary-first with deferred heavy views.
- Keep searchable, paged operations/cells without full payload pressure.

Targets:
- `desktop/wx/src/main.rs`
- `ui_payload/src/lib.rs`
- `web/` (if WebView path is used)

## 5. Delivery Plan

### Phase A (1-2 days)
- Keep current instrumentation hooks.
- Add one-pass worksheet parse + drawing ref extraction implementation plan and tests.
- Implement `TextCtrl` lazy detail loading.

Exit criteria:
- No functional regressions.
- Verified stage timings still emitted with `EXCEL_DIFF_PROFILE_OPEN=1`.

### Phase B (2-4 days)
- Implement one-pass parsing change.
- Implement container preallocation optimization.
- Run quick/gate perf checks and compare e2e parse deltas.

Exit criteria:
- Meaningful reduction in `sheet_parse_ms + drawing_parse_ms` on `e2e_p3`/`e2e_p5`.

### Phase C (optional, R&D)
- Parallel open prototype behind feature flag.
- Determinism and correctness validation.

Exit criteria:
- Speedup demonstrated on large fixtures with no determinism failures.

## 6. Validation Strategy

Policy now documented as major vs minor perf validation:
- **Major perf-risk changes:** full pre/post perf cycle.
- **Routine changes:** quick suite; add gate suite for large-grid/streaming touch points.
- Escalate to full cycle when quick/gate results regress or are suspicious.

Recommended metrics to track each phase:
- `parse_time_ms`, `total_time_ms` from e2e suite.
- Open-path stage split from `PERF_OPEN_WORKBOOK`.
- UX responsiveness checks via scenario harness:
  - `desktop/ui_scenarios/`
  - visual checks from `docs/ui_visual_regression_plan.md`.

## 7. Risks and Mitigations

- **Risk:** Optimization changes accidentally alter diff semantics.
  - **Mitigation:** Keep existing tests and add focused regression tests around parser output equivalence.

- **Risk:** Overfitting to single-machine variance.
  - **Mitigation:** Compare multiple runs and trend medians for decisions.

- **Risk:** Large architecture changes (parallel parse) increase complexity.
  - **Mitigation:** Gate behind feature flag and keep default conservative path until proven.

## 8. Commands (repro + profiling)

```bash
# Targeted open/parse stage profiling
cd core
EXCEL_DIFF_PROFILE_OPEN=1 cargo test --release --features perf-metrics --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1

# Routine perf checks
python scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv
python scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests

# Full cycle for major perf-risk changes
python3 scripts/perf_cycle.py pre
python3 scripts/perf_cycle.py post --cycle <cycle_id>
```
