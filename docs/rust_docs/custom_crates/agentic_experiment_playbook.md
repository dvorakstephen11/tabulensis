# Agentic Experiment Playbook (Custom Crates)

This guide is for repeatable, high-signal custom-crate experiments in this repo.

## 1) Pick a Candidate and Define Scope

- Choose one candidate and one first slice (single crate + single hotspot).
- Record:
  - target crate/module,
  - baseline dependency,
  - custom replacement boundary,
  - expected win metric (latency, memory, size, or build time).

## 2) Build the A/B/C Matrix

- `A` baseline: third-party crate path (default behavior).
- `B` custom: feature-gated custom path.
- `C` parity (optional, test-only): run both implementations and compare outputs.

Rules:
- Do not mix multiple candidate rewrites in one iteration.
- Keep custom fallback logic test-only; avoid release-time dual-path complexity.

## 3) Run Matched Baselines

Before code edits:
- Run tests and perf for the touched crate/path.
- Run shared/perf-cycle suites only when they are relevant to the changed path.

Minimum baseline metadata to log:
- commit SHA,
- Rust toolchain/profile,
- command lines,
- fixtures/manifests,
- feature flags,
- run count and aggregation.

## 4) Keep Sampling Symmetric

- Use identical run settings pre/post (`--runs`, aggregation, fixtures, parallelism).
- Default to median aggregation.
- Use at least 5 runs when expected delta is small or noise is visible.
- Prefer alternating A/B runs (`A1, B1, A2, B2, ...`) to reduce thermal and background drift; record the run order.

## 5) Tie Metrics to the Changed Area

- If changing desktop backend cache logic, include desktop-backend cache-hit workloads.
- If changing core parser/container logic, include workbook-open and parse-heavy workloads.
- Avoid claiming wins from unrelated suite movement.

## 6) Required Experiment Log Format

For each iteration, append:
1. Baseline commands and raw metrics.
2. Code/feature change summary.
3. Post-change commands and raw metrics.
4. Delta table (absolute + percent).
5. Decision: ship / keep behind flag / drop.
6. Next action and risk notes.

## 7) Promotion Gate

Promote custom implementation only if all hold:
- no correctness regressions,
- repeatable perf or memory win on target workloads,
- acceptable complexity and maintenance cost,
- explicit evidence in the experiment doc.

## 8) Common Failure Modes

- Mismatched pre/post run counts.
- Comparing different fixture sets.
- Claiming based on one outlier run.
- Running only broad suites and missing candidate-specific workloads.
- Forgetting to update experiment docs/index after each iteration.
