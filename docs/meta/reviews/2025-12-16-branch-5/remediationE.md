## Branch 5 coverage vs. your codebase

### What Branch 5 asks for

Branch 5’s plan is centered on (a) introducing a `WorkbookPackage` domain object with a single open path, (b) extending `DiffOp` to cover Power Query (“M”) changes, and (c) providing a unified package-level diff API (`diff` and `diff_streaming`), while deprecating the older entry points and adding helpers like `grid_ops()` / `m_ops()`.

### What you already have (implemented and looks correct)

#### 5.1 `WorkbookPackage` + single open path

* `WorkbookPackage` exists and contains `workbook` plus optional `data_mashup`. 
* `WorkbookPackage::open(...)` opens the OPC container, parses the workbook, and parses/builds the DataMashup (if present). 
* Error consolidation: you have a single `PackageError` in the Excel Open XML layer, and `WorkbookPackage::open` returns that.

Net: this meets the Branch 5 intent of “open one thing, get everything.” 

#### 5.2 M query ops added to `DiffOp` + `diff_m_queries` updated

* `DiffOp` includes M-query-related operations: `QueryAdded`, `QueryRemoved`, `QueryRenamed`, `QueryDefinitionChanged`, and `QueryMetadataChanged`, plus `QueryChangeKind` and `QueryMetadataField`.
* `m_diff` now emits `Vec<DiffOp>` (not a separate `MQueryDiff` type) and you’ve got a deprecated `diff_m_queries` shim pointing users to `WorkbookPackage::diff`.

Net: Branch 5’s “M changes are DiffOps too” is implemented. 

#### 5.3 Unified package diff APIs + deprecations + projection helpers

* `WorkbookPackage::diff(...)` runs the workbook diff and then appends M ops into the same `DiffReport`. It also updates `report.strings` after adding M ops (important because M diff interns additional strings). 
* `WorkbookPackage::diff_streaming(...)` runs the workbook diff in streaming mode and then emits M ops into the same sink.
* Older root APIs are deprecated/hidden with a migration path.
* You implemented `DiffReport::grid_ops()` and `DiffReport::m_ops()` using `DiffOp::is_m_op()`.

Net: the unified user-facing API described by Branch 5 exists and is wired correctly.

---

## One real correctness gap to fix (Branch 5 streaming)

### The issue

`WorkbookPackage::diff_streaming` guarantees `sink.finish()` is called in the **grid** error path (good), but it does **not** guarantee `sink.finish()` if the sink fails while emitting **M ops**, because the M loop uses `sink.emit(op)?;` and will early-return without finishing.

This is exactly the kind of “works most of the time, but breaks in production when output IO fails mid-stream” issue that will show up when streaming to files/sockets.

Also: your test named `package_diff_streaming_finishes_on_error` doesn’t actually assert that `finish()` was called, so it won’t catch this category of regression. 

---

## Implementation plan to complete Branch 5 fully

### 1) Fix `WorkbookPackage::diff_streaming` to finish on M-emit errors

#### Code to replace (current `diff_streaming` as shown in `core/src/package.rs`)

```rust
pub fn diff_streaming<S: DiffSink>(
    &self,
    other: &Self,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    crate::with_default_session(|session| {
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

        let mut summary = match grid_result {
            Ok(summary) => summary,
            Err(e) => {
                let _ = sink.finish();
                return Err(e);
            }
        };

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

        sink.finish()?

        Ok(summary)
    })
}
```

#### Replace with (finish is guaranteed on M-emit errors)

```rust
pub fn diff_streaming<S: DiffSink>(
    &self,
    other: &Self,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    crate::with_default_session(|session| {
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

        let mut summary = match grid_result {
            Ok(summary) => summary,
            Err(e) => {
                let _ = sink.finish();
                return Err(e);
            }
        };

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            &mut session.strings,
            config,
        );

        for op in m_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        sink.finish()?;
        Ok(summary)
    })
}
```

### 2) Strengthen tests so this stays correct

You already have a good “success path finish exactly once” test that also verifies at least one M op is streamed. 

What’s missing is:

* asserting finish in the existing error test, and
* a test that triggers an error specifically during **M emission**, not during the grid phase.

#### 2a) Update the existing error test to assert `finish_called`

##### Code to replace (current tail of `package_diff_streaming_finishes_on_error`)

```rust
let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);
assert!(result.is_err(), "diff_streaming should return error");
```

##### Replace with

```rust
let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);
assert!(result.is_err(), "diff_streaming should return error");
assert!(sink.finish_called, "sink.finish() should be called on error");
```

#### 2b) Add a new test: fail only when an M op is emitted

Add this test right after `package_diff_streaming_finishes_on_error` in `core/tests/package_streaming_tests.rs`. It uses your existing `make_workbook`/`make_dm` helpers and forces the sink to fail on the first M op by checking `op.is_m_op()`.

##### New code to insert

```rust
#[test]
fn package_diff_streaming_finishes_on_m_emit_error() {
    struct FailOnMOpSink {
        finish_called: bool,
        finish_calls: usize,
    }

    impl DiffSink for FailOnMOpSink {
        fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
            if op.is_m_op() {
                return Err(DiffError::SinkError {
                    message: "fail on m op".to_string(),
                });
            }
            Ok(())
        }

        fn finish(&mut self) -> Result<(), DiffError> {
            self.finish_calls += 1;
            self.finish_called = true;
            Ok(())
        }
    }

    let wb = make_workbook("Sheet1");

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let pkg_a = WorkbookPackage {
        workbook: wb.clone(),
        data_mashup: Some(dm_a),
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
    };

    let mut sink = FailOnMOpSink {
        finish_called: false,
        finish_calls: 0,
    };

    let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);

    assert!(result.is_err(), "expected sink error during M op emission");
    assert!(sink.finish_called, "sink.finish() should be called on M emit error");
    assert_eq!(sink.finish_calls, 1, "finish should be called exactly once");
}
```

This test will fail on your current implementation and pass after the `diff_streaming` fix above.
