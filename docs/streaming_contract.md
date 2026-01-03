## Streaming Output Contract (Determinism + Lifecycle)

This document defines the non-negotiable contract for streaming output in excel_diff. The
contract applies to all streaming entry points (engine, WorkbookPackage, PBIX/PBIT) and all
DiffSink implementations (Vec, callback, JSONL, desktop DB store).

This contract is an API promise: consumers may rely on it for stable UI rendering, caching,
pagination, and reproducible tests.

---

## 1. Determinism

Determinism means: same inputs + same config = byte-for-byte identical meaning.

Concretely, that includes:

- The exact sequence of DiffOp items (order is part of the contract).
- The DiffSummary fields (complete, warnings, op_count).
- StringPool ordering and StringId values in streaming mode.
- Identical results across different rayon thread counts when "parallel" is enabled.

Determinism is required across:

- Report mode (DiffReport with ops vec),
- Streaming mode (DiffSink),
- Parallel mode (rayon thread-count variance).

---

## 2. Sink Lifecycle

The lifecycle is strict and enforced mechanically:

1. begin(pool) once, before any emit.
2. emit(op) zero or more times.
3. finish() once, after the last emit.

Rules:

- begin is called exactly once before any emit.
- finish is called exactly once after begin succeeds.
- If begin succeeded, finish must be attempted exactly once even if an error occurs later
  (best-effort; do not mask the original error).
- No emit after finish (enforced by tests).

Nested streaming (NoFinishSink) is allowed: inner streaming calls may use NoFinishSink to
prevent finishing the real sink, while still obeying begin/emit/finish on the wrapper.

---

## 3. String Table Rules for Streaming (JSONL)

JsonLinesSink writes a one-time header containing the string table:

- JsonLinesSink::begin() writes a "Header" line with pool.strings().
- Ops are emitted as JSON lines after the header.

Therefore:

- All strings that will be referenced by any emitted DiffOp must already exist in the pool
  by the time begin(pool) is called.
- After begin, the pool must be treated as frozen with respect to adding new strings.
  Lookups are fine; inserting new strings is not.

This is required to keep JSONL streams self-consistent.

---

## 4. Partial Output + Warnings Semantics

complete == true means: the diff ran without early aborts or fallbacks that might omit ops.

complete == false means: output may be partial or degraded. Consumers must consult warnings
for the reason (timeout, op cap, memory fallback, etc.). Warnings are ordered deterministically
by detection order.

---

## 5. Streaming Entry Point Map + Checklist

### Engine streaming entry points

- try_diff_workbooks_streaming(...) / try_diff_workbooks_streaming_with_progress(...)
  - begin(pool) once, emit ops via EmitCtx, finish once.
  - Must finish even when errors occur after begin.

- try_diff_grids_streaming(...) / try_diff_grids_streaming_with_progress(...)
  - begin once, emit ops, finish once.
  - Must finish even when errors occur after begin.

- try_diff_sheets_streaming(...) / try_diff_sheets_streaming_with_progress(...)
  - begin once, emit ops, finish once.
  - Must finish even when errors occur after begin.

- try_diff_grids_database_mode_streaming(...)
  - begin once, emit ops, finish once.
  - Must finish on timeout, duplicate-key fallback, and error paths.

### Package-level streaming entry points

- WorkbookPackage::diff_streaming_with_pool(...)
  - Precompute object ops + M ops (interning required strings).
  - Run workbook engine streaming via NoFinishSink.
  - Emit object ops, then M ops, then finish once.

- WorkbookPackage::diff_database_mode_streaming_with_pool(...)
  - Same pattern for database-mode grids.

### PBIX/PBIT streaming entry point

- PbixPackage::diff_streaming_with_pool(...)
  - Precompute M ops and model ops before begin (string table must be complete).
  - begin(pool), emit ops, finish once.

### Checklist for new streaming APIs

- begin called once before any emit.
- finish called once after begin succeeds (best-effort on errors).
- No emit after finish.
- Pool is fully interned before begin when JSONL output is possible.
- Ordering rules are documented and tested.

---

## 6. Stable Emission Ordering (per host)

These rules document and protect existing ordering. They are not new behavior.

### Workbook engine ordering

- Sheet-level ops are ordered by (name_lower, sheet_kind_order).
- Within a sheet, cell/row/col ops use the engine's stable ordering.

### WorkbookPackage ordering

1. Grid ops (streamed from the engine)
2. Object graph ops (named ranges, charts, VBA)
3. Power Query ops

### PBIX/PBIT ordering

1. Power Query ops
2. Model ops (when enabled)

