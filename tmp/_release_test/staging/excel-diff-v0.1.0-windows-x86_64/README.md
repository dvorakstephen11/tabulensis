# Excel Diff

A fast, cross-platform tool for comparing Excel workbooks. Works with `.xlsx` and `.xlsm` files.

## Installation

### Windows

**Option 1: Download from GitHub Releases**

1. Download the latest `excel-diff-vX.Y.Z-windows-x86_64.zip` from [Releases](https://github.com/dvora/excel_diff/releases)
2. Extract the ZIP file
3. Add the extracted folder to your PATH, or move `excel-diff.exe` to a folder in your PATH

**Option 2: Scoop**

```powershell
scoop bucket add excel-diff https://github.com/dvora/scoop-excel-diff
scoop install excel-diff
```

### macOS

**Option 1: Homebrew (recommended)**

```bash
brew tap dvora/excel-diff
brew install excel-diff
```

**Option 2: Download from GitHub Releases**

```bash
# Download the universal binary (works on both Intel and Apple Silicon)
curl -LO https://github.com/dvora/excel_diff/releases/latest/download/excel-diff-vX.Y.Z-macos-universal.tar.gz
tar -xzf excel-diff-vX.Y.Z-macos-universal.tar.gz
sudo mv excel-diff /usr/local/bin/

# Or for user-only install:
mv excel-diff ~/.local/bin/
```

> **Note:** On first run, macOS may block the binary. Right-click and select "Open" or run:
> ```bash
> xattr -d com.apple.quarantine /usr/local/bin/excel-diff
> ```

### Build from Source

Requires [Rust](https://rustup.rs/) 1.85+:

```bash
cargo install --path cli
# Or:
cargo build --release -p excel_diff_cli
```

### Web Demo

Try Excel Diff directly in your browser at **[dvora.github.io/excel_diff](https://dvora.github.io/excel_diff)**

No installation required. Files are processed entirely in your browser using WebAssembly.

---

## Directory Structure

- **`core/`**: The Rust library for comparing Excel workbooks.
- **`cli/`**: Command-line interface for the diff engine.
- **`wasm/`**: WebAssembly bindings for the web demo.
- **`web/`**: Web demo frontend.
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

