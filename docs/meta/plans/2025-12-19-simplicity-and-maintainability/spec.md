# Maintainability + Elegant Simplicity Implementation Plan

This plan targets two concrete outcomes:

* **Maintainability posture**: a future maintainer can navigate the diff pipeline, change one subsystem, and not fear hidden coupling.
* **Elegant simplicity**: the diff pipeline reads like a linear narrative (fast-path -> moves/masks -> alignment -> cell diff), with the complex decisions isolated into small, named functions.

The highest-leverage work is concentrated in `core/src/engine.rs` (orchestration + algorithm selection) and a few “stringly/duplicated” seams like report construction and signature serde. 

---

## Implementation ordering (do this in small, safe PR-sized steps)

1. **Remove dead/duplicated code and centralize report-building**
   (Low risk, high clarity, prevents repeated rework.) 
2. **Refactor `EmitCtx` creation + limit-exceeded fallback**
   (Eliminates repeated boilerplate and makes limit behavior easy to audit.) 
3. **Make “mask diff” path return `()` instead of always-true `bool`**
   (Tiny but meaningful simplification: removes fake branching.) 
4. **Linearize `diff_grids_core` into a readable pipeline**
   (No algorithm changes, just structure. The code should look like the spec’s phases.) 
5. **Decompose `try_diff_with_amr` into named decision helpers**
   (Same logic, far smaller cognitive load. Enables unit tests for the decision points.) 
6. **(Optional but recommended) Split `engine.rs` into submodules**
   (File size reduction; makes navigation and change isolation much better.) 

At each step, rely on the existing test philosophy (behavioral invariants, vertical slices) to confirm no regressions. 

---

## Step 1: Signature serde cleanup (remove duplication + dead code)

### Why

In `workbook.rs`, `RowSignature` and `ColSignature` have:

* a cfg-gated helper module (`signature_serde`) and
* full manual `Serialize`/`Deserialize` impls that duplicate the same logic. 

This is accidental complexity: harder to modify, more surface area for bugs, and the cfg gate is misleading since the types are actually serialized in normal output.

### What to change

Replace the manual impls + cfg module with:

* `#[derive(Serialize, Deserialize)]` on both structs
* a single always-on `signature_hex` field serde adapter

### Code change (replace this block in `core/src/workbook.rs`) 

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RowSignature {
    pub hash: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColSignature {
    pub hash: u128,
}

#[cfg(any(test, feature = "dev-apis"))]
mod signature_serde {
    use serde::de::Error as DeError;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize_u128<S>(val: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:032x}", val).serialize(serializer)
    }

    pub fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u128::from_str_radix(&s, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))
    }
}

impl serde::Serialize for RowSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("RowSignature", 1)?;
        s.serialize_field("hash", &format!("{:032x}", self.hash))?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for RowSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        #[derive(serde::Deserialize)]
        struct Helper {
            hash: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        let hash = u128::from_str_radix(&helper.hash, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))?;
        Ok(RowSignature { hash })
    }
}

impl serde::Serialize for ColSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ColSignature", 1)?;
        s.serialize_field("hash", &format!("{:032x}", self.hash))?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for ColSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        #[derive(serde::Deserialize)]
        struct Helper {
            hash: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        let hash = u128::from_str_radix(&helper.hash, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))?;
        Ok(ColSignature { hash })
    }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RowSignature {
    #[serde(with = "signature_hex")]
    pub hash: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ColSignature {
    #[serde(with = "signature_hex")]
    pub hash: u128,
}

mod signature_hex {
    use serde::de::Error as DeError;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(val: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{:032x}", val);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u128::from_str_radix(&s, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))
    }
}
```

### Tests

* Existing snapshot/serde tests should continue to pass without changes.
* Add one focused serde roundtrip test for `RowSignature` and `ColSignature` if there isn’t already one.

---

## Step 2: Centralize `DiffReport` construction (remove repeated boilerplate)

### Why

Report construction is repeated in:

* `engine.rs::try_diff_workbooks` 
* `engine.rs::diff_grids_database_mode` 
* `output/json.rs::build_report_from_sink` 

This is classic “copy-paste glue” that becomes a maintenance hazard whenever you add a field (like metrics, schema changes, warnings semantics).

### What to change

Add a single helper constructor on `DiffReport`:

* `DiffReport::from_ops_and_summary(ops, summary, strings)`

Then update all call sites.

### Code change (replace `impl DiffReport` block in `core/src/diff.rs`) 

```rust
impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
            ops,
            complete: true,
            warnings: Vec::new(),
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn with_partial_result(ops: Vec<DiffOp>, warning: String) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
            ops,
            complete: false,
            warnings: vec![warning],
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
        self.complete = false;
    }

    pub fn resolve(&self, id: StringId) -> Option<&str> {
        self.strings.get(id.0 as usize).map(|s| s.as_str())
    }

    pub fn grid_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| !op.is_m_op())
    }

    pub fn m_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| op.is_m_op())
    }
}
```

```rust
impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
            ops,
            complete: true,
            warnings: Vec::new(),
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn from_ops_and_summary(ops: Vec<DiffOp>, summary: DiffSummary, strings: Vec<String>) -> DiffReport {
        let mut report = DiffReport::new(ops);
        report.complete = summary.complete;
        report.warnings = summary.warnings;
        #[cfg(feature = "perf-metrics")]
        {
            report.metrics = summary.metrics;
        }
        report.strings = strings;
        report
    }

    pub fn with_partial_result(ops: Vec<DiffOp>, warning: String) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
            ops,
            complete: false,
            warnings: vec![warning],
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
        self.complete = false;
    }

    pub fn resolve(&self, id: StringId) -> Option<&str> {
        self.strings.get(id.0 as usize).map(|s| s.as_str())
    }

    pub fn grid_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| !op.is_m_op())
    }

    pub fn m_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| op.is_m_op())
    }
}
```

### Update call sites

#### A) `core/src/engine.rs::try_diff_workbooks` 

```rust
pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    let summary = try_diff_workbooks_streaming(old, new, pool, config, &mut sink)?;
    let ops = sink.into_ops();
    let mut report = DiffReport::new(ops);
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    #[cfg(feature = "perf-metrics")]
    {
        report.metrics = summary.metrics;
    }
    report.strings = pool.strings().to_vec();
    Ok(report)
}
```

```rust
pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    let summary = try_diff_workbooks_streaming(old, new, pool, config, &mut sink)?;
    let strings = pool.strings().to_vec();
    Ok(DiffReport::from_ops_and_summary(sink.into_ops(), summary, strings))
}
```

#### B) `core/src/engine.rs::diff_grids_database_mode` 

```rust
pub fn diff_grids_database_mode(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut sink = VecSink::new();
    let mut op_count = 0usize;
    let summary = diff_grids_database_mode_streaming(
        old,
        new,
        key_columns,
        pool,
        config,
        &mut sink,
        &mut op_count,
    )
    .unwrap_or_else(|e| panic!("{}", e));
    let mut report = DiffReport::new(sink.into_ops());
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    report.strings = pool.strings().to_vec();
    report
}
```

```rust
pub fn diff_grids_database_mode(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut sink = VecSink::new();
    let mut op_count = 0usize;
    let summary = diff_grids_database_mode_streaming(
        old,
        new,
        key_columns,
        pool,
        config,
        &mut sink,
        &mut op_count,
    )
    .unwrap_or_else(|e| panic!("{}", e));
    let strings = pool.strings().to_vec();
    DiffReport::from_ops_and_summary(sink.into_ops(), summary, strings)
}
```

#### C) `core/src/output/json.rs::build_report_from_sink` 

```rust
fn build_report_from_sink(sink: VecSink, summary: DiffSummary, session: DiffSession) -> DiffReport {
    let mut report = DiffReport::new(sink.into_ops());
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    #[cfg(feature = "perf-metrics")]
    {
        report.metrics = summary.metrics;
    }
    report.strings = session.strings.into_strings();
    report
}
```

```rust
fn build_report_from_sink(sink: VecSink, summary: DiffSummary, session: DiffSession) -> DiffReport {
    DiffReport::from_ops_and_summary(sink.into_ops(), summary, session.strings.into_strings())
}
```

### Tests

* No new tests required; existing output tests should remain valid.
* If you have JSON snapshot tests, this should be schema-identical.

---

## Step 3: Make `EmitCtx` creation uniform and simplify limit-exceeded fallback

### Why

`try_diff_grids` currently:

* constructs `EmitCtx` twice via struct literal
* calls `positional_diff` directly (then partially updates metrics) instead of using the existing `run_positional_diff_with_metrics` helper 

This increases the “surface area” of the limit-exceeded behavior and makes it harder to ensure all paths treat metrics consistently.

### What to change

* Add `EmitCtx::new(...)`
* Refactor `try_diff_grids` limit handling:

  * compute warning once
  * on fallback/partial: run positional via `run_positional_diff_with_metrics`
  * only start `Phase::MoveDetection` after limit checks pass (more truthful metrics and simpler control flow)

### Code changes

#### A) Replace `EmitCtx` definition in `core/src/engine.rs` 

```rust
struct EmitCtx<'a, S: DiffSink> {
    sheet_id: &'a SheetId,
    pool: &'a StringPool,
    config: &'a DiffConfig,
    cache: &'a mut FormulaParseCache,
    sink: &'a mut S,
    op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}
```

```rust
struct EmitCtx<'a, S: DiffSink> {
    sheet_id: &'a SheetId,
    pool: &'a StringPool,
    config: &'a DiffConfig,
    cache: &'a mut FormulaParseCache,
    sink: &'a mut S,
    op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    fn new(
        sheet_id: &'a SheetId,
        pool: &'a StringPool,
        config: &'a DiffConfig,
        cache: &'a mut FormulaParseCache,
        sink: &'a mut S,
        op_count: &'a mut usize,
    ) -> Self {
        Self {
            sheet_id,
            pool,
            config,
            cache,
            sink,
            op_count,
        }
    }

    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}
```

#### B) Replace the `EmitCtx` literal in `SheetGridDiffer::new` 

```rust
Self {
    emit_ctx: EmitCtx {
        sheet_id,
        pool,
        config,
        cache,
        sink,
        op_count,
    },
    old,
    new,
    old_view,
    new_view,
    old_mask,
    new_mask,
    #[cfg(feature = "perf-metrics")]
    metrics,
}
```

```rust
Self {
    emit_ctx: EmitCtx::new(sheet_id, pool, config, cache, sink, op_count),
    old,
    new,
    old_view,
    new_view,
    old_mask,
    new_mask,
    #[cfg(feature = "perf-metrics")]
    metrics,
}
```

#### C) Replace `try_diff_grids` in `core/src/engine.rs` 

```rust
fn try_diff_grids<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == 0 && new.nrows == 0 {
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.rows_processed = m
            .rows_processed
            .saturating_add(old.nrows as u64)
            .saturating_add(new.nrows as u64);
        m.start_phase(Phase::MoveDetection);
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols;
    if exceeds_limits {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(*sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );
        match config.on_limit_exceeded {
            LimitBehavior::FallbackToPositional => {
                let mut emit_ctx = EmitCtx {
                    sheet_id,
                    pool,
                    config,
                    cache: &mut ctx.formula_cache,
                    sink,
                    op_count,
                };
                positional_diff(&mut emit_ctx, old, new)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnPartialResult => {
                ctx.warnings.push(warning);
                let mut emit_ctx = EmitCtx {
                    sheet_id,
                    pool,
                    config,
                    cache: &mut ctx.formula_cache,
                    sink,
                    op_count,
                };
                positional_diff(&mut emit_ctx, old, new)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id.clone(),
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.max_align_rows,
                    max_cols: config.max_align_cols,
                });
            }
        }
        return Ok(());
    }

    diff_grids_core(
        sheet_id,
        old,
        new,
        config,
        pool,
        sink,
        op_count,
        ctx,
        #[cfg(feature = "perf-metrics")]
        metrics,
    )?;
    Ok(())
}
```

```rust
fn try_diff_grids<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == 0 && new.nrows == 0 {
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.rows_processed = m
            .rows_processed
            .saturating_add(old.nrows as u64)
            .saturating_add(new.nrows as u64);
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols;

    if exceeds_limits {
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(*sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );

        match config.on_limit_exceeded {
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id.clone(),
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.max_align_rows,
                    max_cols: config.max_align_cols,
                });
            }
            behavior => {
                if matches!(behavior, LimitBehavior::ReturnPartialResult) {
                    ctx.warnings.push(warning);
                }

                let mut emit_ctx = EmitCtx::new(
                    sheet_id,
                    pool,
                    config,
                    &mut ctx.formula_cache,
                    sink,
                    op_count,
                );

                #[cfg(feature = "perf-metrics")]
                run_positional_diff_with_metrics(&mut emit_ctx, old, new, metrics.as_deref_mut())?;
                #[cfg(not(feature = "perf-metrics"))]
                run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;

                return Ok(());
            }
        }
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::MoveDetection);
    }

    diff_grids_core(
        sheet_id,
        old,
        new,
        config,
        pool,
        sink,
        op_count,
        ctx,
        #[cfg(feature = "perf-metrics")]
        metrics,
    )?;

    Ok(())
}
```

### Tests

* Existing limit behavior tests should remain correct (you already have limit tests in `core/tests/limit_behavior_tests.rs`). 
* Add one unit test asserting that `FallbackToPositional` does not add a warning but still emits diffs.

---

## Step 4: Remove “fake boolean” from mask diff path

### Why

`SheetGridDiffer::diff_with_masks` returns `Result<bool, DiffError>` but always returns `Ok(true)`. In `diff_grids_core`, the return value is used as if it might be false, but it never is. This is accidental complexity: it suggests a control-flow branch that doesn’t exist. 

### What to change

* Change `diff_with_masks` to return `Result<(), DiffError>`
* Simplify `diff_grids_core` to unconditionally return after mask diff is run

### Code changes

#### A) Replace `diff_with_masks` signature and body 

```rust
fn diff_with_masks(&mut self) -> Result<bool, DiffError> {
    if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
        if diff_aligned_with_masks(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_mask,
            &self.new_mask,
        )? {
            return Ok(true);
        }
        positional_diff_with_masks(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_mask,
            &self.new_mask,
        )?;
    } else {
        positional_diff_masked_equal_size(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_mask,
            &self.new_mask,
        )?;
    }
    Ok(true)
}
```

```rust
fn diff_with_masks(&mut self) -> Result<(), DiffError> {
    if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
        if diff_aligned_with_masks(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_mask,
            &self.new_mask,
        )? {
            return Ok(());
        }
        positional_diff_with_masks(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_mask,
            &self.new_mask,
        )?;
        return Ok(());
    }

    positional_diff_masked_equal_size(
        &mut self.emit_ctx,
        self.old,
        self.new,
        &self.old_mask,
        &self.new_mask,
    )?;

    Ok(())
}
```

#### B) Replace the mask block in `diff_grids_core` 

```rust
if differ.has_mask_exclusions() {
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }
    let result = differ.diff_with_masks()?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.end_phase(Phase::CellDiff);
    }
    if result {
        return Ok(());
    }
}
```

```rust
if differ.has_mask_exclusions() {
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }
    differ.diff_with_masks()?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.end_phase(Phase::CellDiff);
    }
    return Ok(());
}
```

### Tests

* Existing move/mask tests should be unaffected (this is behavior-preserving). 

---

## Step 5: Linearize `diff_grids_core` into a “phase pipeline”

### Why

Even after the above, `diff_grids_core` still reads as a mixed bag:

* fast-path equality
* move detection + masks
* alignment phase selection (AMR, row changes, single-column change, fallback positional) 

Elegant simplicity here means a reader can glance at the function and see the pipeline.

### What to change

Do a purely-structural refactor: keep algorithms identical, but make the selection logic “flat” via methods or small helper functions.

Target shape:

```text
fast_path_equal?
  -> return

init state
detect moves
if masks active:
  diff remaining
  return

try AMR
try row alignment
try single-column alignment
fallback positional
```

### Implementation details

1. Add these methods to `SheetGridDiffer` (all private):

   * `fn try_amr(&mut self) -> Result<bool, DiffError>`
   * `fn try_row_alignment(&mut self) -> Result<bool, DiffError>`
   * `fn try_single_column_alignment(&mut self) -> Result<bool, DiffError>`
   * `fn positional(&mut self) -> Result<(), DiffError>`

2. `diff_grids_core` becomes an orchestration function that only calls these, with minimal metrics starts/ends.

3. Keep every existing helper (`emit_row_aligned_diffs`, `emit_column_aligned_diffs`, `run_positional_diff_with_metrics`) as-is; you are not changing the algorithms, only re-homing calls.

### Tests

* No new tests required for correctness if you keep refactor mechanical.
* Add one new unit test for `fast_path_equal` behavior if you want extra confidence.

---

## Step 6: Decompose `try_diff_with_amr` into named decision helpers

### Why

`try_diff_with_amr` currently carries:

* AMR alignment + post-processing
* move injection vs move stripping
* multiple early-return heuristics
* metrics bookkeeping and phase termination 

It is correct-looking, but it is not readable. Maintainability improves dramatically once the decision points have names and can be tested independently.

### What to change

Refactor into a small “driver” that calls pure-ish decision helpers. The helpers should be:

* tiny
* named after the policy they represent
* ideally side-effect free (or limited to mutating alignment)

Proposed helper extraction (all in `engine.rs` initially, no file moves required):

1. `fn amr_build_alignment(old_view, new_view, config) -> Option<(RowAlignment, Vec<RowSignature>, Vec<RowSignature>)>`
2. `fn amr_apply_move_policy(old, new, alignment, sigs_old, sigs_new, config)`
3. `fn amr_should_fallback_for_structural_changes(old, new, alignment, config) -> bool`
4. `fn amr_should_fallback_for_row_edits_with_inserts_deletes(old, new, alignment, config) -> bool`
5. `fn amr_try_single_column_alignment(old_view, new_view, old, new, config) -> Option<ColumnAlignment>`
6. `fn amr_should_fallback_for_multiset_equal_reorder(old, new, alignment, config) -> bool`

Then the driver becomes:

* build alignment
* apply policy
* check fallbacks in a readable sequence
* emit either column-aligned diffs or row-aligned diffs

### How to implement (mechanical, behavior-preserving)

* Cut/paste each contiguous block from `try_diff_with_amr` into one helper function.
* Keep the logic identical at first.
* Only after it compiles and tests pass should you simplify further (like consolidating repeated metric end calls).

### Tests (high value)

Add a small table-driven unit test module for the helpers. You do not need full grid fixtures; you can construct tiny `Grid` objects (2-10 rows) and assert which decision triggers.
This directly matches the testing philosophy: “tests as documentation for invariants and decisions.” 

---

## Step 7 (Optional, but strongly recommended): Split `engine.rs` into submodules

### Why

`engine.rs` currently mixes:

* workbook-level orchestration
* sheet key identity concerns
* grid diff orchestration
* move detection + masking
* alignment heuristics 

Even with refactors, the file stays conceptually crowded. Splitting improves navigation and change isolation.

### Target layout

Create:

* `core/src/engine/mod.rs` (public facade)
* `core/src/engine/workbook_diff.rs` (sheet enumeration, stable ordering, calls into grid diff)
* `core/src/engine/grid_diff.rs` (diff_grids_core, try_diff_grids, limit policy)
* `core/src/engine/move_mask.rs` (SheetGridDiffer move detection + mask diff helpers)
* `core/src/engine/amr.rs` (try_diff_with_amr + extracted decision helpers)

### Implementation details

* Keep public functions and signatures in `engine/mod.rs` to preserve the API surface.
* Move code with minimal changes and fix imports.
* Do not change algorithm logic during the move; only relocate.

### Tests

* No new tests needed if moves are mechanical.
* Run `cargo test` and ensure there is no behavior change.

---

## Cross-cutting guideline: Make the pipeline match the spec’s mental model

The spec frames the system as a pipeline of layers (domain -> diff, and within diff: preprocessing/move detection/alignment/cell diff). When the code mirrors that narrative, you get both maintainability and elegance “for free.” 

Concretely:

* Prefer “phase-named functions” over deep nesting.
* Make early returns obvious and intentional.
* Keep “policy decisions” (heuristics) in small functions with names that explain them.

---

## Definition of Done (for this refactor series)

* All existing tests pass (especially the grid/move/alignment suites).
* No schema changes to `DiffReport` JSON output (unless you explicitly choose to version it).
* `engine.rs` (or the `engine/` folder) reads like a pipeline, not a maze.
* The hardest function (`try_diff_with_amr`) has at least 5-7 named helper functions, each individually testable.

If you want, I can also provide a concrete “helper-by-helper” extraction of `try_diff_with_amr` (exact code blocks for each extracted function, plus the final simplified driver) as the next step.
