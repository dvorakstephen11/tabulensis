# Excel Diff Monorepo

This repository consolidates the plan, implementation, and test fixtures for the Excel Diff engine.

## Directory Structure

- **`core/`**: The Rust library for comparing Excel workbooks.
- **`cli/`**: Command-line interface for the diff engine.
- **`fixtures/`**: Python tools to generate Excel file fixtures for testing.
- **`docs/`**: Project documentation, plans, and meta-programming logs.

## Quick Start

### Building

```bash
cargo build --release
```

The `excel-diff` binary will be available at `target/release/excel-diff`.

### CLI Usage

Compare two workbooks:

```bash
excel-diff diff old.xlsx new.xlsx
```

Output formats:

```bash
excel-diff diff old.xlsx new.xlsx --format text   # Human-readable (default)
excel-diff diff old.xlsx new.xlsx --format json   # Full JSON report
excel-diff diff old.xlsx new.xlsx --format jsonl  # Streaming JSON lines
excel-diff diff old.xlsx new.xlsx --git-diff      # Unified diff style
```

Diff presets:

```bash
excel-diff diff old.xlsx new.xlsx --fast     # Faster, less precise move detection
excel-diff diff old.xlsx new.xlsx --precise  # More accurate, slower
```

Show workbook information:

```bash
excel-diff info workbook.xlsx            # Show sheets
excel-diff info workbook.xlsx --queries  # Include Power Query info
```

### Exit Codes

- `0`: Files are identical
- `1`: Files differ
- `2`: Error (file not found, parse error, invalid arguments)

## Git Integration

The CLI integrates with Git in two ways:

### Git Difftool

Use `excel-diff` as an external diff viewer for Excel files:

```gitconfig
# ~/.gitconfig or .git/config
[diff]
    tool = excel-diff

[difftool "excel-diff"]
    cmd = excel-diff diff \"$LOCAL\" \"$REMOTE\"
```

Then run:

```bash
git difftool --tool=excel-diff -- path/to/file.xlsx
```

### Git Textconv (for `git diff`)

Convert Excel files to text for line-by-line diff with `git diff`:

```gitconfig
# ~/.gitconfig or .git/config
[diff "xlsx"]
    textconv = excel-diff info
    cachetextconv = true
    binary = true
```

Add to `.gitattributes`:

```gitattributes
*.xlsx diff=xlsx
*.xlsm diff=xlsx
```

Now `git diff` will show structural changes in Excel files:

```bash
git diff HEAD~1 -- data.xlsx
```

### Git Diff Driver with Full Diff

For detailed diff output instead of just structural info:

```gitconfig
[diff "xlsx-full"]
    textconv = "excel-diff diff --git-diff /dev/null"
    cachetextconv = true
    binary = true
```

## Library Usage (Rust)

```rust
use excel_diff::{WorkbookPackage, DiffConfig};

let old = WorkbookPackage::open(std::fs::File::open("old.xlsx")?)?;
let new = WorkbookPackage::open(std::fs::File::open("new.xlsx")?)?;

let report = old.diff(&new, &DiffConfig::default());

for op in &report.ops {
    println!("{:?}", op);
}
```

## Testing

### Core Tests (Rust)

```bash
cargo test
```

### Fixture Generation (Python)

```bash
cd fixtures
uv pip install -r requirements.txt
python src/generate.py
```

## Documentation

See `docs/` for detailed architectural plans and design documents.

