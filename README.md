# Excel Diff

Excel Diff compares Excel workbooks (`.xlsx` / `.xlsm` / `.xltx` / `.xltm`) and Power BI packages (`.pbix` / `.pbit`) and emits a structured diff: cell edits, sheet structure, named ranges, charts/VBA modules (shallow), and Power Query (M) changes.

Use it via:
- CLI: `excel-diff`
- Rust library: `excel_diff` crate (`WorkbookPackage`)

## Installation

### From GitHub Releases

Download a prebuilt binary from https://github.com/dvora/excel_diff/releases (Windows `.exe` / `.zip`, macOS universal `.tar.gz`).

### From Source (Rust)

Requires [Rust](https://rustup.rs/) 1.85+:

```bash
cargo install --locked --path cli
```

### Web Demo

Try it in your browser at https://dvora.github.io/excel_diff (files are processed locally in your browser via WebAssembly).

---

### Windows (details)

**Option 1: Download from GitHub Releases**

1. Download the latest Windows asset from [Releases](https://github.com/dvora/excel_diff/releases):
   - `excel-diff-vX.Y.Z-windows-x86_64.exe` (standalone), or
   - `excel-diff-vX.Y.Z-windows-x86_64.zip` (portable folder)
2. Add it (or the extracted folder) to your PATH

**Option 2: Scoop**

```powershell
# Download `excel-diff.json` from the GitHub Release assets, then:
scoop install .\excel-diff.json

# Or, if you publish a Scoop bucket:
# scoop bucket add excel-diff https://github.com/dvora/scoop-excel-diff
# scoop install excel-diff
```

### macOS

**Option 1: Homebrew (formula from Release assets)**

```bash
# Download `excel-diff.rb` from the GitHub Release assets, then:
brew install --formula ./excel-diff.rb

# Or, if you publish a Homebrew tap:
# brew tap dvora/excel-diff
# brew install excel-diff
```

**Option 2: Download from GitHub Releases**

```bash
# Download the universal binary (works on both Intel and Apple Silicon)
VERSION=vX.Y.Z
curl -LO https://github.com/dvora/excel_diff/releases/download/$VERSION/excel-diff-$VERSION-macos-universal.tar.gz
tar -xzf excel-diff-$VERSION-macos-universal.tar.gz
sudo mv excel-diff /usr/local/bin/

# Or for user-only install:
mv excel-diff ~/.local/bin/
```

> **Note:** On first run, macOS may block the binary. Right-click and select "Open" or run:
> ```bash
> xattr -d com.apple.quarantine /usr/local/bin/excel-diff
> ```

---

## Quick Start

### CLI Usage

Compare two workbooks:

```bash
excel-diff diff old.xlsx new.xlsx
```

Copy/paste starters:

```bash
excel-diff diff old.xlsx new.xlsx                       # Human-readable (default)
excel-diff diff old.xlsx new.xlsx --format json > out.json  # Full JSON report
excel-diff diff old.xlsx new.xlsx --format jsonl > out.jsonl  # Streaming JSONL (better for large diffs)
excel-diff diff old.xlsx new.xlsx --git-diff            # Unified diff style (for Git tools)
```

Diff presets:

```bash
excel-diff diff old.xlsx new.xlsx --fast     # Faster, less precise move detection
excel-diff diff old.xlsx new.xlsx --precise  # More accurate, slower
```

Notes:
- `--git-diff` cannot be combined with `--format json|jsonl`.
- `--fast` and `--precise` are mutually exclusive.

Show workbook information:

```bash
excel-diff info workbook.xlsx            # Show sheets
excel-diff info workbook.xlsx --queries  # Include Power Query info
```

### Exit Codes

- `0`: Files are identical
- `1`: Files differ (or results are incomplete)
- `2`: Error (file not found, parse error, invalid arguments)

## Supported Formats

- Workbooks: `.xlsx`, `.xlsm`, `.xltx`, `.xltm`
- Power BI: `.pbix`, `.pbit`
- `.xlsb` is detected but not supported; Excel Diff returns `EXDIFF_PKG_009` with a "convert to .xlsx/.xlsm" hint.

## Library Usage (Rust)

```rust
use excel_diff::{DiffConfig, WorkbookPackage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let old = WorkbookPackage::open(std::fs::File::open("old.xlsx")?)?;
    let new = WorkbookPackage::open(std::fs::File::open("new.xlsx")?)?;

    let report = old.diff(&new, &DiffConfig::default());

    for op in &report.ops {
        println!("{:?}", op);
    }

    Ok(())
}
```

For large workbooks, prefer streaming output (`diff_streaming`) and consider setting `DiffConfig.hardening.max_memory_mb` / `DiffConfig.hardening.timeout_seconds`.

## Limits and Knobs

- PBIX/PBIT support is limited to legacy DataMashup extraction. Tabular-only PBIX files return
  `NoDataMashupUseTabularModel` (`EXDIFF_PKG_010`).
- DataMashup permissions are guarded by permission bindings. If DPAPI bindings cannot be validated,
  Excel Diff defaults permissions and emits warning `EXDIFF_DM_009` (the diff may be marked incomplete).
- Semantic M diff is enabled by default. The CLI `--fast` preset disables it; use default or
  `--precise` to keep semantic detail.
- Resource ceilings:
  - `--max-memory` (`DiffConfig.hardening.max_memory_mb`) can trigger a positional fallback with warnings.
  - `--timeout` (`DiffConfig.hardening.timeout_seconds`) aborts with a partial report and warnings.
  - Alignment limits (`alignment.max_align_rows` / `alignment.max_align_cols`) are enforced via `hardening.on_limit_exceeded`.

## Documentation

- Start here: [docs/index.md](docs/index.md)
- CLI: [docs/cli.md](docs/cli.md)
- Configuration: [docs/config.md](docs/config.md)
- Git integration: [docs/git.md](docs/git.md)
- Database mode: [docs/database_mode.md](docs/database_mode.md)
- FAQ: [docs/faq.md](docs/faq.md)
- Architecture: [docs/architecture.md](docs/architecture.md)
- Migration guide: [docs/migration.md](docs/migration.md)
- Release readiness: [docs/release_readiness.md](docs/release_readiness.md)

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

