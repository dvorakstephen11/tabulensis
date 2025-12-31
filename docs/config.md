# Configuration Guide (`DiffConfig`)

[Docs index](index.md)

`DiffConfig` controls thresholds, toggles, and safety rails for the diff engine.

## How diffing works (high level)

1. Parse the `.xlsx` / `.xlsm` container into a workbook IR (`Workbook` -> `Sheet` -> `Grid`).
2. Diff sheets:
   - alignment + move detection (when enabled and within limits)
   - cell edits, row/column adds/removes, and block moves
3. Add object diffs (named ranges, charts, VBA modules).
4. Add Power Query (M) diffs when a DataMashup section is present.

## Presets

- `DiffConfig::default()`: balanced default behavior.
- `DiffConfig::fastest()`: disables or reduces expensive strategies (mapped to CLI `--fast`).
- `DiffConfig::most_precise()`: favors correctness/precision over speed (mapped to CLI `--precise`).

## Safety + robustness

These fields are intended to keep large diffs from exhausting resources:

- `max_memory_mb: Option<u32>`: soft cap on estimated memory usage for advanced strategies.
  - When exceeded, the engine may fall back to a cheaper positional strategy for the affected sheet and mark the overall result as incomplete with a warning.
- `timeout_seconds: Option<u32>`: abort the diff after a wall-clock timeout.
  - When exceeded, the engine stops early, preserves already-produced ops, and marks the result as incomplete with a warning.
- WASM bindings set a default `max_memory_mb` (256 MB) to reduce OOM risk in browser runtimes.

## Key options you actually tune

- Limit behavior:
  - `on_limit_exceeded: LimitBehavior` controls what happens when algorithm limits are hit.
- Move detection:
  - `enable_fuzzy_moves`, `max_move_iterations`, `max_move_detection_rows`, `max_move_detection_cols`
- Output context / diagnostics:
  - `max_context_rows`
  - `include_unchanged_cells` (diagnostic; emits `CellEdited` even when values are unchanged)
- Semantic diff toggles:
  - `enable_m_semantic_diff`
  - `enable_formula_semantic_diff`

## When to use database mode

If your rows have stable primary keys and often reorder, database mode can produce much more readable diffs than positional alignment.

See [Database mode](database_mode.md) for usage and examples.

## Recipes

### Large file, want a guaranteed finish

CLI (streaming JSONL + safety rails):

```bash
excel-diff diff old.xlsx new.xlsx --format jsonl --max-memory 512 --timeout 10 > out.jsonl
```

Rust (streaming + budget):

```rust
use excel_diff::{DiffConfig, JsonLinesSink, WorkbookPackage};
use std::fs::File;
use std::io::{self, BufWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let old = WorkbookPackage::open(File::open("old.xlsx")?)?;
    let new = WorkbookPackage::open(File::open("new.xlsx")?)?;

    let mut cfg = DiffConfig::default();
    cfg.max_memory_mb = Some(512);
    cfg.timeout_seconds = Some(10);

    let stdout = io::stdout();
    let mut sink = JsonLinesSink::new(BufWriter::new(stdout.lock()));
    let summary = old.diff_streaming(&new, &cfg, &mut sink)?;
    eprintln!("complete={} ops={}", summary.complete, summary.op_count);
    Ok(())
}
```

### I care about correctness above all

- Start from `DiffConfig::most_precise()` (or CLI `--precise`).
- If you hit alignment limits, increase `max_align_rows` / `max_align_cols` (or change `on_limit_exceeded`) and rerun.
