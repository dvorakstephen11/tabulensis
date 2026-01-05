# CLI Reference

[Docs index](index.md)

The CLI binary is `excel-diff`.

For the canonical, always-up-to-date option list, run:

```bash
excel-diff --help
excel-diff diff --help
excel-diff info --help
```

## Supported formats

- Workbooks: `.xlsx`, `.xlsm`, `.xltx`, `.xltm`
- Power BI: `.pbix`, `.pbit`
- `.xlsb` is detected but not supported yet; Excel Diff returns `EXDIFF_PKG_009` with a convert hint.

## `excel-diff diff <OLD> <NEW>`

Compare two workbooks and emit a diff.

### Output selection

- `--format <text|json|jsonl|payload|outcome>`: output format (default: `text`)
  - `text`: human-readable summary (good for terminals)
  - `json`: full `DiffReport` serialized as JSON
  - `jsonl`: streaming JSON lines (header line, then one op per line)
  - `payload`: UI-loadable `DiffWithSheets` JSON (report + snapshots + alignments)
  - `outcome`: shared outcome envelope (`mode`, optional `payload`, optional `summary`)
- `--git-diff`: unified-diff style output for Git tools
  - Constraint: cannot be combined with `--format json`, `--format jsonl`, `--format payload`, or `--format outcome`

### Presets / verbosity

- `--preset <fastest|balanced|most-precise>`: select a named preset (default: `balanced`)
- `--fast`: fastest preset (less precise move detection)
- `--precise`: most-precise preset (slower, more accurate)
  - Constraint: `--fast`, `--precise`, and `--preset` are mutually exclusive
- `--quiet`: summary-only text output
- `--verbose`: include more detail in text output

### Database mode

- `--database`: enable key-based row alignment (table diff)
- `--sheet <NAME>`: sheet name (required if multiple sheets and no sheet named "Data" exists)
- `--keys <COLS>`: comma-separated column letters (e.g., `A,C,AA`)
- `--auto-keys`: auto-detect key columns (may infer composite keys; if no reliable key exists, the CLI warns and falls back to spreadsheet mode)

Validation rules:

- `--sheet`, `--keys`, and `--auto-keys` require `--database`
- `--database` requires exactly one of `--keys` or `--auto-keys`
- `--keys` and `--auto-keys` cannot be used together

### Hardening (large file safety)

- `--progress`: show a progress indicator on stderr
- `--max-memory <MB>`: set a soft memory budget (may trigger fallback + `complete=false`)
- `--timeout <SECONDS>`: abort the diff after this many seconds (partial result + `complete=false`)

### Warnings

- Permission bindings (DPAPI) that cannot be validated cause permissions to default and emit
  warning `EXDIFF_DM_009`. This sets `complete=false` and triggers exit code `1` even if no ops.

### Exit codes

- `0`: no differences and the result is complete
- `1`: differences found, or the result is incomplete (warnings emitted)
- `2`: error (invalid arguments, parse failure, or I/O/output failure)

## `excel-diff info <FILE>`

Print a stable text representation of a single workbook:

- workbook filename
- list of sheets (name, kind, dimensions, non-empty cell count)
- optional Power Query summary with `--queries`

This output is suitable for Git `textconv` (see [Git integration](git.md)).
