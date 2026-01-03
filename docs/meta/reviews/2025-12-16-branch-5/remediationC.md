I reviewed your updated `codebase_context.md` against **Branch 5: API Unification** in `next_sprint_plan.md`. Overall, you’ve implemented *almost all* of Branch 5 cleanly (WorkbookPackage, consolidated errors, unified DiffOp stream including M ops, deprecations, etc.). The one **material correctness issue** I see is in **`WorkbookPackage::diff_streaming`**: it currently **finishes the sink before emitting M ops**, and it also **never finishes again after emitting M ops**. That can break streaming consumers and/or lose buffered output.

Below is a concrete patch plan with code.

---

## What looks complete for Branch 5

### 1) `WorkbookPackage` exists and parses workbook + DataMashup in one pass

Your `WorkbookPackage::open` opens a single OPC container and reads workbook + DataMashup from it, building `DataMashup` only if present. 
This satisfies the “single open parses entire file” intent from Branch 5.1. 

### 2) DiffOp has M-query variants and they’re part of the same op stream

`DiffOp` includes `QueryAdded/Removed/...` variants (and related enums). 
Your `WorkbookPackage::diff` appends M ops into the same `report.ops`. 
Branch 5.2/5.3 alignment looks good. 

### 3) Old APIs are deprecated (migration path present)

Your build/test logs show downstream tests compiling but emitting deprecation warnings for `open_workbook`/`diff_workbooks` and `ExcelOpenError`, which is consistent with “deprecated, not removed”.

### 4) Tests for package diff and M ops exist

You have tests using `WorkbookPackage::open()` and `report.m_ops()` to validate M diffs. 

---

## What is unfinished/incorrect

## Critical bug: `WorkbookPackage::diff_streaming` emits M ops after the sink is finished

### Why it’s wrong

Your current implementation:

1. Calls `crate::engine::try_diff_workbooks_streaming(..., sink)`
2. That engine function **calls `sink.finish()?` internally** at the end of the grid diff. 
3. After that returns, `WorkbookPackage::diff_streaming` emits M ops via `sink.emit(op)` anyway. 

So for any sink that treats `finish()` as terminal (close file, write trailer, etc.), you’ll get either:

* errors (“emit after finish”), or
* silently malformed output (M ops missing or appended after a closed stream), or
* buffering not flushed (because you also don’t call `finish()` again after M ops). 

This directly violates the intention of Branch 5.3 “unified streaming diff_packages API”. 

---

# Patch plan

## Patch 1: Add a “do-not-finish” sink adapter

Add a small adapter sink that **forwards emits** but **swallows finish()**. Put it in `core/src/sink.rs` (best reuse point).

### `core/src/sink.rs`

```rust
use crate::diff::{DiffError, DiffOp};

/// Trait for streaming diff operations to a consumer.
pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;

    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
    }
}

/// Adapter that forwards `emit` but turns `finish()` into a no-op.
/// Useful when composing streaming phases where an inner routine
/// calls `finish()` too early.
pub(crate) struct NoFinishSink<'a, S: DiffSink> {
    inner: &'a mut S,
}

impl<'a, S: DiffSink> NoFinishSink<'a, S> {
    pub(crate) fn new(inner: &'a mut S) -> Self {
        Self { inner }
    }
}

impl<'a, S: DiffSink> DiffSink for NoFinishSink<'a, S> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        self.inner.emit(op)
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        // swallow finish so callers can finish at the real end
        Ok(())
    }
}
```

No public API impact (it’s `pub(crate)`), and it avoids touching the engine API.

---

## Patch 2: Fix `WorkbookPackage::diff_streaming` to finish once, after ALL ops

Update `core/src/package.rs` to:

* run the grid streaming diff against a `NoFinishSink`, so the engine’s internal `finish()` doesn’t close the real sink
* emit M ops into the real sink
* call `sink.finish()` exactly once at the end
* if the grid diff returns `Err`, still best-effort `finish()` the sink to flush/close resources

### `core/src/package.rs`

```rust
use crate::config::DiffConfig;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary};
use crate::sink::{DiffSink, NoFinishSink};
use crate::workbook::Workbook;

#[derive(Debug, Clone)]
pub struct WorkbookPackage {
    pub workbook: Workbook,
    pub data_mashup: Option<DataMashup>,
}

impl WorkbookPackage {
    // open(...) unchanged

    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            // 1) Run grid diff, but suppress engine's internal sink.finish()
            let grid_result = {
                let mut no_finish = NoFinishSink::new(sink);
                crate::engine::try_diff_workbooks_streaming(
                    &self.workbook,
                    &other.workbook,
                    &mut session.strings,
                    config,
                    &mut no_finish,
                )
            };

            // 2) If grid diff errored, flush/finish best-effort and return the error
            let mut summary = match grid_result {
                Ok(summary) => summary,
                Err(e) => {
                    let _ = sink.finish();
                    return Err(e);
                }
            };

            // 3) Emit M ops (still using the same pool/session)
            let m_ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            for op in m_ops {
                sink.emit(op)?;
                summary.op_count = summary.op_count.saturating_add(1);
            }

            // 4) Finish exactly once, after all ops
            sink.finish()?;

            Ok(summary)
        })
    }
}
```

This is the minimal surgical fix that makes `diff_streaming` correct without needing to refactor the engine.

---

## Patch 3: Add a regression test that fails on “emit after finish”

Right now, you have good streaming tests for `try_diff_workbooks_streaming`, but nothing that exercises the **package streaming + M ops** integration.

Add a sink that **errors if emit happens after finish**, and ensure `WorkbookPackage::diff_streaming` succeeds and emits at least one M op.

### New file: `core/tests/package_streaming_tests.rs`

```rust
use excel_diff::{DiffConfig, DiffError, DiffOp, DiffSink, WorkbookPackage};
use excel_diff::workbook::{Workbook, Sheet, SheetKind, Grid};
use excel_diff::datamashup::{DataMashup, Permissions, Metadata};
use excel_diff::datamashup_package::{PackageParts, PackageXml, SectionDocument};

#[derive(Default)]
struct StrictSink {
    finished: bool,
    finish_calls: usize,
    ops: Vec<DiffOp>,
}

impl DiffSink for StrictSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        if self.finished {
            return Err(DiffError::SinkError {
                message: "emit called after finish".to_string(),
            });
        }
        self.ops.push(op);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.finish_calls += 1;
        self.finished = true;
        Ok(())
    }
}

fn make_dm(section_source: &str) -> DataMashup {
    DataMashup {
        version: 0,
        package_parts: PackageParts {
            package_xml: PackageXml { raw_xml: "<Package/>".to_string() },
            main_section: SectionDocument { source: section_source.to_string() },
            embedded_contents: Vec::new(),
        },
        permissions: Permissions::default(),
        metadata: Metadata { formulas: Vec::new() },
        permission_bindings_raw: Vec::new(),
    }
}

fn make_workbook(sheet_name: &str) -> Workbook {
    // IMPORTANT: WorkbookPackage::diff_streaming uses the crate's default session pool internally.
    // So ensure the sheet name StringId is allocated from that same pool.
    let sheet_id = excel_diff::with_default_session(|session| session.strings.intern(sheet_name));

    Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: Grid::new(0, 0),
        }],
    }
}

#[test]
fn package_diff_streaming_does_not_emit_after_finish_and_finishes_once() {
    let wb = make_workbook("Sheet1");

    // Force at least one M op (rename/add/remove).
    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let pkg_a = WorkbookPackage { workbook: wb.clone(), data_mashup: Some(dm_a) };
    let pkg_b = WorkbookPackage { workbook: wb, data_mashup: Some(dm_b) };

    let mut sink = StrictSink::default();
    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("diff_streaming should succeed");

    assert!(sink.finished, "sink should be finished at end");
    assert_eq!(sink.finish_calls, 1, "sink.finish() should be called exactly once");

    assert!(
        sink.ops.iter().any(|op| op.is_m_op()),
        "expected at least one M diff op in streaming output"
    );

    assert_eq!(
        summary.op_count, sink.ops.len(),
        "summary.op_count should match ops actually emitted"
    );
}
```

This test will **fail** on the current implementation because the engine finishes the sink before M ops are emitted. With the adapter fix, it will pass.

---

## Optional cleanup (not required for correctness)

Your test suite still uses deprecated APIs heavily (warnings in `cycle_summary.txt`).
This isn’t “unfinished,” but if you want a clean build, you can progressively migrate tests from:

* `open_workbook(...)` → `WorkbookPackage::open(file).workbook` (or better: use package end-to-end)
* `diff_workbooks(&wb_a, &wb_b, ...)` → `pkg_a.diff(&pkg_b, ...)`

---

## Summary

✅ Branch 5 is largely complete: unified package type, consolidated errors, unified DiffOp stream including M-query ops, deprecations.
❌ The remaining correctness gap is **streaming package diff**: `WorkbookPackage::diff_streaming` currently finishes too early (engine finishes the sink) and doesn’t finish after emitting M ops.
✅ The fix is straightforward: introduce a `NoFinishSink` adapter + finish once at the end, and add a regression test to lock it in.

If you apply the patches above, `WorkbookPackage::diff_streaming` will behave correctly for sinks that require strict finalization semantics.
