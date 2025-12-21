# CLI Reference

[Docs index](index.md)

The CLI binary is `excel-diff`.

For the canonical, always-up-to-date option list, run:

```bash
excel-diff --help
excel-diff diff --help
excel-diff info --help
```

## `excel-diff diff <OLD> <NEW>`

Compare two workbooks and emit a diff.

### Output selection

- `--format <text|json|jsonl>`: output format (default: `text`)
  - `text`: human-readable summary (good for terminals)
  - `json`: full `DiffReport` serialized as JSON
  - `jsonl`: streaming JSON lines (header line, then one op per line)
- `--git-diff`: unified-diff style output for Git tools
  - Constraint: cannot be combined with `--format json` or `--format jsonl`

### Presets / verbosity

- `--fast`: fastest preset (less precise move detection)
- `--precise`: most-precise preset (slower, more accurate)
  - Constraint: `--fast` and `--precise` are mutually exclusive
- `--quiet`: summary-only text output
- `--verbose`: include more detail in text output

### Database mode

- `--database`: enable key-based row alignment (table diff)
- `--sheet <NAME>`: sheet name (required if multiple sheets and no sheet named "Data" exists)
- `--keys <COLS>`: comma-separated column letters (e.g., `A,C,AA`)
- `--auto-keys`: auto-detect key columns

Validation rules:

- `--sheet`, `--keys`, and `--auto-keys` require `--database`
- `--database` requires exactly one of `--keys` or `--auto-keys`
- `--keys` and `--auto-keys` cannot be used together

### Hardening (large file safety)

- `--progress`: show a progress indicator on stderr
- `--max-memory <MB>`: set a soft memory budget (may trigger fallback + `complete=false`)
- `--timeout <SECONDS>`: abort the diff after this many seconds (partial result + `complete=false`)

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
