# Database Mode (Key-Based Diffing)

[Docs index](index.md)

Database mode aligns rows by *key columns* (like a primary key) instead of row position. This is useful for table-like sheets where rows reorder frequently.

## When it's the right tool

- Your sheet is a table (one logical row per record).
- One or more columns form a stable, unique identifier.
- Rows are commonly inserted, deleted, or reordered.

If you don't have stable keys, positional diffing is usually more appropriate.

## CLI usage

Enable database mode with `--database`, then choose keys via `--keys` or `--auto-keys`.

### Keys format (`--keys`)

Keys are comma-separated Excel column letters:

- `A` = column index `0`
- `B` = `1`
- `AA` = `26`

Example:

```bash
excel-diff diff --database --sheet Data --keys A,C old.xlsx new.xlsx
```

### Auto-detect keys (`--auto-keys`)

Auto-detect tries to pick columns that uniquely identify rows (powered by `suggest_key_columns`).

```bash
excel-diff diff --database --sheet Data --auto-keys old.xlsx new.xlsx
```

### Sheet selection rules (if `--sheet` is omitted)

1. If either workbook contains a sheet named `Data` (case-insensitive), that sheet is used.
2. Else, if both workbooks have exactly one sheet, that sheet is used.
3. Otherwise, the CLI errors and requires `--sheet`.

### Streaming output for huge tables

```bash
excel-diff diff --database --sheet Data --keys A,C --format jsonl old.xlsx new.xlsx > out.jsonl
```

## Library usage

`WorkbookPackage` exposes database mode APIs:

```rust
use excel_diff::{DiffConfig, WorkbookPackage};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let old = WorkbookPackage::open(File::open("old.xlsx")?)?;
    let new = WorkbookPackage::open(File::open("new.xlsx")?)?;

    let keys: Vec<u32> = vec![0, 2]; // A,C
    let report = old.diff_database_mode(&new, "Data", &keys, &DiffConfig::default())?;

    println!("complete={} ops={}", report.complete, report.ops.len());
    Ok(())
}
```

For very large diffs, use `diff_database_mode_streaming` with a `DiffSink` (e.g., `JsonLinesSink`).

## Troubleshooting

### Duplicate keys

If key columns aren't unique, the engine may warn and fall back to positional behavior. In the CLI, you may also see a hint suggesting alternate keys.

### Missing sheet

If the sheet name doesn't exist in one of the workbooks, you'll get a "sheet not found" error that lists available sheets.

### Incomplete results

If you see `complete=false` and warnings, it usually means a safety rail triggered (timeouts / memory budgets / limits). See [Configuration](config.md).

## Example output shape (text format)

Illustrative excerpt (exact output depends on the workbooks):

```text
Sheet "Data":
  Row 42: ADDED
  Cell B2: "old" -> "new"
---
Summary:
  Total changes: 123
  Status: complete
```
