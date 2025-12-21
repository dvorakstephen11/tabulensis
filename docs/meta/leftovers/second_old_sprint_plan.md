# Excel Diff Engine: 7-Branch Sprint Plan to 90% MVP Completion

## Executive Summary

This plan identifies the remaining work needed to bring the Excel Diff engine from its current ~70% completion state to at least 90% MVP completion. The codebase has strong foundations in grid diffing, M parsing, formula parsing, and API design. The remaining work focuses on:

1. **Database Mode at workbook level** (critical for competitor parity)
2. **CLI tool with Git integration** (required for developer adoption)
3. **Robustness and error handling** (production readiness)
4. **Cross-platform packaging** (Mac/Win/Web)
5. **VBA and named range diffing** (object graph completeness)
6. **Documentation and polish** (developer experience)
7. **Performance hardening** (large file reliability)

---

## Current State Assessment

### Completed Components (from old_next_sprint_plan.md branches)

| Component | Status | Notes |
|-----------|--------|-------|
| **Grid Diff Engine** | ✅ Complete | AMR alignment, move detection, multi-gap handling |
| **DiffConfig** | ✅ Complete | Full builder, presets, serde serialization |
| **StringPool/Interning** | ✅ Complete | Memory-efficient string handling |
| **Streaming Output** | ✅ Complete | DiffSink trait, VecSink, JsonLinesSink |
| **WorkbookPackage API** | ✅ Complete | Unified entry point for parsing and diffing |
| **M Parser** | ✅ Complete | let/in, records, lists, function calls, canonicalization |
| **Formula Parser** | ✅ Complete | Full AST, canonicalization, shift detection |
| **Database Mode (grid level)** | ✅ Complete | `diff_grids_database_mode` implemented |
| **Performance Infrastructure** | ✅ Complete | Metrics, benchmarks, CI integration |
| **WASM Compilation** | ✅ Partial | Core compiles, smoke test exists |

### Missing for 90% MVP

| Component | Priority | Difficulty | Notes |
|-----------|----------|------------|-------|
| **CLI Tool** | Critical | Medium | No binary distribution exists |
| **Git Integration** | Critical | Medium | difftool/mergetool configuration |
| **Database Mode (workbook level)** | Critical | Low | API exists but not exposed via WorkbookPackage |
| **Error Handling** | High | Medium | Many unwraps, panics in edge cases |
| **VBA Module Diffing** | High | Medium | Currently ignored |
| **Named Range Diffing** | High | Low | Currently ignored |
| **Chart Object Diffing** | Medium | Medium | Currently ignored |
| **Three-way Merge** | Medium | High | Not implemented |
| **Mac/Win Packaging** | High | Medium | No installers/packages |
| **Web Demo** | Medium | Medium | WASM exists but no frontend |

---

## Branch Dependency Graph

```
                    ┌─────────────────────────┐
                    │  Branch 1: CLI Tool &   │
                    │  Git Integration        │
                    └───────────┬─────────────┘
                                │
           ┌────────────────────┼────────────────────┐
           │                    │                    │
           ▼                    ▼                    ▼
┌─────────────────────┐ ┌─────────────────┐ ┌─────────────────────┐
│ Branch 2: Database  │ │ Branch 3: Error │ │ Branch 7: Web Demo  │
│ Mode API            │ │ Hardening       │ │ (WASM Frontend)     │
└──────────┬──────────┘ └────────┬────────┘ └─────────────────────┘
           │                     │
           │                     │
           │                     ▼
           │            ┌─────────────────────┐
           └───────────►│ Branch 4: Object    │
                        │ Graph Completion    │
                        └──────────┬──────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                                         │
              ▼                                         ▼
┌─────────────────────┐                    ┌─────────────────────┐
│ Branch 5: Packaging │                    │ Branch 6: Perf &    │
│ & Distribution      │                    │ Robustness          │
└─────────────────────┘                    └─────────────────────┘
```

**Critical Path:** Branch 1 → Branch 2 → Branch 4 → Branch 5

**Parallel Work:** Branches 3, 6, 7 can proceed independently after Branch 1.

---

## Branch 1: CLI Tool & Git Integration

**Goal:** Create a usable command-line tool that can be integrated with Git as a difftool.

**Depends on:** Nothing (uses existing library)

**MVP Importance:** Critical — without a CLI, the library cannot be used.

### 1.1 Create CLI Binary Crate

**Technical Specification:**

Create `cli/` directory with a new crate:

```
cli/
  Cargo.toml
  src/
    main.rs
    commands/
      mod.rs
      diff.rs
      info.rs
    output/
      mod.rs
      text.rs
      json.rs
```

```rust
// cli/src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "excel-diff")]
#[command(about = "Compare Excel workbooks semantically")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Diff {
        #[arg(help = "Old workbook path")]
        old: PathBuf,
        #[arg(help = "New workbook path")]
        new: PathBuf,
        #[arg(long, help = "Output format: text, json, jsonl")]
        format: Option<String>,
        #[arg(long, help = "Use database mode with specified key columns")]
        key_columns: Option<String>,
        #[arg(long, help = "Use fastest preset")]
        fast: bool,
        #[arg(long, help = "Use most precise preset")]
        precise: bool,
    },
    Info {
        #[arg(help = "Workbook path")]
        path: PathBuf,
        #[arg(long, help = "Show Power Query information")]
        queries: bool,
    },
}
```

**Deliverables:**
- [ ] Create `cli/Cargo.toml` with clap dependency
- [ ] Implement `diff` subcommand with text output
- [ ] Implement `diff` subcommand with JSON output
- [ ] Implement `diff` subcommand with JSON Lines output
- [ ] Implement `info` subcommand showing workbook structure
- [ ] Add `--key-columns` flag for database mode
- [ ] Add config preset flags (`--fast`, `--precise`)
- [ ] Add exit codes (0 = identical, 1 = different, 2 = error)
- [ ] Write CLI usage documentation

### 1.2 Git Integration

**Technical Specification:**

The CLI must work as a Git difftool:

```bash
# .gitconfig
[difftool "excel-diff"]
    cmd = excel-diff diff \"$LOCAL\" \"$REMOTE\"

[diff "xlsx"]
    textconv = excel-diff info
    binary = true
```

**Deliverables:**
- [ ] Document Git configuration in README
- [ ] Add `--git-diff` output mode (unified diff-style)
- [ ] Test with actual Git repositories
- [ ] Add `.gitattributes` example for xlsx files

### 1.3 Human-Readable Text Output

**Technical Specification:**

```
Comparing: old.xlsx → new.xlsx

Sheet "Data":
  Row 5: ADDED
  Row 12: REMOVED
  Cell C7: 100 → 150
  Cell D7: "old" → "new"
  Block moved: rows 10-15 → rows 20-25

Power Query:
  Query "SalesData": DEFINITION CHANGED (semantic)
  Query "Transform": FORMATTING ONLY
  Query "NewQuery": ADDED
```

**Deliverables:**
- [ ] Implement text formatter for DiffOps
- [ ] Color-coded output (optional, detect TTY)
- [ ] Summary statistics at end
- [ ] Configurable verbosity levels

### Acceptance Criteria for Branch 1

- [ ] `excel-diff diff old.xlsx new.xlsx` produces readable output
- [ ] `excel-diff diff --format=json` produces valid JSON
- [ ] Exit code 0 when files identical, 1 when different
- [ ] Git difftool integration documented and tested
- [ ] `--help` output is clear and complete

---

## Branch 2: Database Mode API Integration

**Goal:** Expose database mode through WorkbookPackage API and CLI.

**Depends on:** Branch 1 (CLI infrastructure)

**MVP Importance:** Critical — this is a key differentiator from competitors.

### 2.1 Add Database Mode to WorkbookPackage

**Technical Specification:**

```rust
impl WorkbookPackage {
    pub fn diff_database_mode(
        &self,
        other: &Self,
        sheet_name: &str,
        key_columns: &[u32],
        config: &DiffConfig,
    ) -> DiffReport {
        // Find sheets by name
        // Call diff_grids_database_mode
        // Return unified report
    }
}
```

**Deliverables:**
- [ ] Add `diff_database_mode` method to WorkbookPackage
- [ ] Add sheet name matching (case-insensitive)
- [ ] Return error if sheet not found
- [ ] Add streaming variant `diff_database_mode_streaming`

### 2.2 Integrate with CLI

**Technical Specification:**

```bash
excel-diff diff --database --sheet=Data --keys=A old.xlsx new.xlsx
excel-diff diff --database --sheet=Data --keys=A,B,C old.xlsx new.xlsx
```

**Deliverables:**
- [ ] Add `--database` flag to diff command
- [ ] Add `--sheet` argument for sheet selection
- [ ] Add `--keys` argument parsing (comma-separated column letters)
- [ ] Convert column letters to indices (A=0, B=1, AA=26, etc.)
- [ ] Add helpful error messages for invalid columns

### 2.3 Auto-Key Detection (Stretch Goal)

**Technical Specification:**

Heuristically detect likely key columns:
- First column with unique values
- Column named "ID", "Key", "SKU", etc.

```rust
pub fn suggest_key_columns(grid: &Grid, pool: &StringPool) -> Vec<u32> {
    // Check column 0 for uniqueness
    // Check header row for common key names
}
```

**Deliverables:**
- [ ] Implement `suggest_key_columns` function
- [ ] Add `--auto-keys` flag to CLI
- [ ] Print suggested keys when database mode fails due to duplicates

### Acceptance Criteria for Branch 2

- [ ] `excel-diff diff --database --keys=A` works correctly
- [ ] Reordered rows with same keys produce no diff
- [ ] Changed values in non-key columns reported correctly
- [ ] Multi-column keys work (`--keys=A,C,E`)
- [ ] Integration tests with D1-D4 fixtures pass

---

## Branch 3: Error Handling & Robustness

**Goal:** Replace panics with proper error handling, improve resilience to malformed files.

**Depends on:** Nothing (can proceed in parallel)

**MVP Importance:** High — production code must not panic.

### 3.1 Audit and Fix Panics

**Technical Specification:**

Search for `.unwrap()`, `.expect()`, `panic!()`, and `unreachable!()` in non-test code:

```bash
rg '\.unwrap\(\)|\.expect\(|panic!\(|unreachable!\(' core/src --type rust
```

**Deliverables:**
- [ ] Audit all unwraps in `core/src`
- [ ] Replace with `?` operator where possible
- [ ] Add context to errors using `thiserror` or `anyhow`
- [ ] Document remaining intentional panics (logic errors)

### 3.2 Graceful Handling of Malformed Files

**Technical Specification:**

The parser should not crash on:
- Truncated ZIP files
- Missing required XML parts
- Invalid XML
- Unexpected DataMashup formats

```rust
pub enum PackageError {
    NotAZip(std::io::Error),
    MissingPart { path: String },
    InvalidXml { part: String, error: String },
    UnsupportedFormat { message: String },
    // ...
}
```

**Deliverables:**
- [ ] Create comprehensive `PackageError` enum
- [ ] Add tests with corrupt fixture files
- [ ] Ensure all XML parsing has try/catch
- [ ] Add validation for ZIP entry sizes (prevent zip bombs)

### 3.3 Improve Error Messages

**Technical Specification:**

Errors should include:
- File path (when available)
- XML path within the package
- Line/column for parse errors
- Actionable suggestions

**Deliverables:**
- [ ] Add context to all error types
- [ ] Implement `Display` with helpful messages
- [ ] Add error code constants for programmatic handling
- [ ] Document error codes in user documentation

### Acceptance Criteria for Branch 3

- [ ] No panics when processing corrupt fixtures
- [ ] All errors include actionable context
- [ ] `clippy::unwrap_used` lint passes (or exceptions documented)
- [ ] Fuzzing with `cargo-fuzz` finds no crashes

---

## Branch 4: Object Graph Completion

**Goal:** Add diffing for VBA modules, named ranges, and chart objects.

**Depends on:** Branch 2 (database mode), Branch 3 (error handling)

**MVP Importance:** High — competitors diff these objects.

### 4.1 VBA Module Diffing

**Technical Specification:**

VBA modules are stored in `xl/vbaProject.bin` (OLE compound document).

```rust
pub struct VbaModule {
    pub name: StringId,
    pub module_type: VbaModuleType,
    pub code: String,
}

pub enum VbaModuleType {
    Standard,
    Class,
    Form,
    Document,
}

// New DiffOps
pub enum DiffOp {
    // ... existing ops ...
    VbaModuleAdded { name: StringId },
    VbaModuleRemoved { name: StringId },
    VbaModuleChanged { name: StringId },
}
```

**Deliverables:**
- [ ] Add `oletools` or implement OLE parsing
- [ ] Extract VBA module source code
- [ ] Implement VBA module comparison (text diff)
- [ ] Add `VbaModule*` DiffOp variants
- [ ] Update WorkbookPackage to include VBA modules
- [ ] Add tests with `.xlsm` fixtures

### 4.2 Named Range Diffing

**Technical Specification:**

Named ranges are defined in `xl/workbook.xml`:

```xml
<definedNames>
  <definedName name="SalesData">Sheet1!$A$1:$Z$100</definedName>
</definedNames>
```

```rust
pub struct NamedRange {
    pub name: StringId,
    pub refers_to: StringId,
    pub scope: Option<StringId>, // Sheet-scoped or workbook-scoped
}

pub enum DiffOp {
    // ... existing ops ...
    NamedRangeAdded { name: StringId },
    NamedRangeRemoved { name: StringId },
    NamedRangeChanged { name: StringId, old_ref: StringId, new_ref: StringId },
}
```

**Deliverables:**
- [ ] Parse named ranges from workbook.xml
- [ ] Add NamedRange to Workbook struct
- [ ] Implement named range comparison
- [ ] Add `NamedRange*` DiffOp variants
- [ ] Add tests with named range fixtures

### 4.3 Chart Object Diffing (Basic)

**Technical Specification:**

Charts are complex objects. For MVP, detect add/remove/change at chart level:

```rust
pub struct ChartInfo {
    pub name: StringId,
    pub chart_type: StringId,
    pub data_range: Option<StringId>,
}

pub enum DiffOp {
    // ... existing ops ...
    ChartAdded { sheet: StringId, name: StringId },
    ChartRemoved { sheet: StringId, name: StringId },
    ChartChanged { sheet: StringId, name: StringId },
}
```

**Deliverables:**
- [ ] Parse chart metadata from `xl/charts/*.xml`
- [ ] Detect chart add/remove at sheet level
- [ ] Compute hash of chart XML for change detection
- [ ] Add `Chart*` DiffOp variants
- [ ] Document limitations (no deep chart diffing)

### Acceptance Criteria for Branch 4

- [ ] VBA module changes detected and reported
- [ ] Named range add/remove/change detected
- [ ] Chart add/remove/change detected
- [ ] No regressions in existing grid/M diff tests
- [ ] New DiffOps serialize correctly to JSON

---

## Branch 5: Packaging & Distribution

**Goal:** Create distributable packages for Windows, macOS, and web.

**Depends on:** Branch 4 (feature completeness)

**MVP Importance:** High — users need to install it.

### 5.1 Windows Packaging

**Technical Specification:**

Options:
1. **MSI installer** (via `cargo-wix`)
2. **Portable ZIP** (single .exe)
3. **Scoop/Chocolatey** package

```yaml
# .github/workflows/release.yml
jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release -p excel_diff_cli
      - uses: actions/upload-artifact@v4
        with:
          name: excel-diff-windows
          path: target/release/excel-diff.exe
```

**Deliverables:**
- [ ] Set up GitHub Actions release workflow
- [ ] Build Windows x64 binary
- [ ] Create portable ZIP package
- [ ] (Optional) Create MSI installer
- [ ] Add to Scoop bucket

### 5.2 macOS Packaging

**Technical Specification:**

Options:
1. **Homebrew tap**
2. **DMG with CLI**
3. **Universal binary** (x64 + ARM64)

**Deliverables:**
- [ ] Build macOS x64 binary
- [ ] Build macOS ARM64 binary (Apple Silicon)
- [ ] Create universal binary
- [ ] Create Homebrew formula
- [ ] Test on macOS Sequoia

### 5.3 Web Demo (WASM)

**Technical Specification:**

Simple web page that:
1. Accepts two file uploads
2. Runs diff in browser via WASM
3. Displays results

```
web/
  index.html
  main.js
  wasm/
    excel_diff_wasm.js
    excel_diff_wasm_bg.wasm
```

**Deliverables:**
- [ ] Create `wasm/` crate with wasm-bindgen exports
- [ ] Build with `wasm-pack`
- [ ] Create minimal HTML/JS frontend
- [ ] Deploy to GitHub Pages
- [ ] Test with various file sizes

### Acceptance Criteria for Branch 5

- [ ] Windows .exe downloadable from GitHub Releases
- [ ] macOS binary downloadable and runs on M1+
- [ ] Web demo functional at https://[user].github.io/excel_diff
- [ ] Installation instructions in README for all platforms
- [ ] CI builds all platforms on release tags

---

## Branch 6: Performance Hardening

**Goal:** Ensure reliable performance on large files, add safeguards.

**Depends on:** Nothing (can proceed in parallel)

**MVP Importance:** Medium — must not regress, should handle edge cases.

### 6.1 Memory Budgeting

**Technical Specification:**

Add memory limits to prevent OOM:

```rust
pub struct DiffConfig {
    // ... existing fields ...
    pub max_memory_mb: Option<u32>,
}
```

When limit approached:
1. Emit partial result
2. Add warning to DiffSummary
3. Fall back to positional diff

**Deliverables:**
- [ ] Add memory tracking (via allocator stats or estimation)
- [ ] Implement memory limit check in hot paths
- [ ] Add `--max-memory` CLI flag
- [ ] Test with fixtures that exceed limits

### 6.2 Progress Reporting

**Technical Specification:**

For long-running operations, emit progress:

```rust
pub trait ProgressCallback: Send {
    fn on_progress(&self, phase: &str, percent: f32);
}

pub fn diff_workbooks_with_progress<P: ProgressCallback>(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
    progress: &P,
) -> DiffReport
```

**Deliverables:**
- [ ] Define ProgressCallback trait
- [ ] Add progress hooks to major phases
- [ ] Add `--progress` flag to CLI
- [ ] Show progress bar in terminal

### 6.3 Timeout Support

**Technical Specification:**

Allow callers to set a timeout:

```rust
pub struct DiffConfig {
    // ... existing fields ...
    pub timeout_seconds: Option<u32>,
}
```

**Deliverables:**
- [ ] Add timeout field to DiffConfig
- [ ] Check elapsed time in hot loops
- [ ] Return partial result on timeout
- [ ] Add `--timeout` CLI flag

### Acceptance Criteria for Branch 6

- [ ] 50K×100 grid completes in <10s
- [ ] Memory stays under 1GB for 100K row files
- [ ] Progress callback fires at reasonable intervals
- [ ] Timeout triggers graceful partial result

---

## Branch 7: Documentation & Polish

**Goal:** Comprehensive documentation, examples, and developer experience.

**Depends on:** Branches 1-5 for accurate documentation

**MVP Importance:** Medium — affects adoption but not functionality.

### 7.1 User Documentation

**Deliverables:**
- [ ] README with quick start guide
- [ ] CLI reference documentation
- [ ] Configuration guide (DiffConfig options)
- [ ] Git integration tutorial
- [ ] Database mode guide with examples
- [ ] FAQ section

### 7.2 API Documentation

**Deliverables:**
- [ ] Complete rustdoc for all public types
- [ ] Code examples in doc comments
- [ ] Architecture overview document
- [ ] Migration guide from old APIs

### 7.3 Example Programs

**Deliverables:**
- [ ] `examples/basic_diff.rs` - Simple file comparison
- [ ] `examples/streaming.rs` - Large file streaming
- [ ] `examples/database_mode.rs` - Key-based diffing
- [ ] `examples/custom_config.rs` - Configuration options

### Acceptance Criteria for Branch 7

- [ ] README has clear installation and usage instructions
- [ ] `cargo doc --open` produces useful documentation
- [ ] All examples compile and run
- [ ] No broken links in documentation

---

## Execution Timeline

### Week 1-2: Foundation
- **Branch 1: CLI Tool** (primary focus)
- **Branch 3: Error Handling** (parallel)

### Week 3-4: Core Features
- **Branch 2: Database Mode API** (depends on Branch 1)
- **Branch 6: Performance** (parallel)

### Week 5-6: Completeness
- **Branch 4: Object Graph** (depends on Branch 2, 3)
- **Branch 7: Documentation** (start in parallel)

### Week 7: Distribution
- **Branch 5: Packaging** (depends on Branch 4)
- **Branch 7: Documentation** (complete)

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| VBA parsing complexity | High | Use existing `oletools` crate or defer to Phase 2 |
| WASM bundle size | Medium | Feature-gate heavy components, measure early |
| macOS code signing | Medium | Use ad-hoc signing initially, add notarization later |
| Cross-platform testing | Medium | Use GitHub Actions matrix builds |
| Documentation debt | Low | Write docs alongside code, not after |

---

## Definition of Done (All Branches)

- [ ] All new code has unit tests
- [ ] All public APIs documented with rustdoc
- [ ] No new clippy warnings
- [ ] CI passes on all platforms
- [ ] Integration tests cover happy path and error cases
- [ ] README updated if user-facing changes

---

## Post-90% MVP Roadmap

Features deferred beyond 90% MVP:

1. **Three-way merge** - Complex, needs design
2. **DAX/Data Model parsing** - Large effort (H5, difficulty 14)
3. **Step-level M diff** - Nice to have, not critical
4. **GUI application** - Significant investment
5. **Cloud/SaaS version** - Business decision
6. **Concurrent diffing** - Performance optimization
7. **Custom output templates** - User request driven

---

## Appendix: Completion Estimation

| Branch | Estimated Effort | MVP Contribution |
|--------|-----------------|------------------|
| Branch 1: CLI | 1-2 weeks | +8% |
| Branch 2: Database Mode | 3-4 days | +3% |
| Branch 3: Error Handling | 1 week | +4% |
| Branch 4: Object Graph | 1-2 weeks | +6% |
| Branch 5: Packaging | 1 week | +4% |
| Branch 6: Performance | 3-4 days | +2% |
| Branch 7: Documentation | 3-4 days | +3% |

**Total: ~6-8 weeks for +30% completion (70% → 100%)**

Current state: ~70% complete
After this sprint: ~100% of MVP scope

Note: "100% MVP" means all core features needed for a usable product. Advanced features (DAX, three-way merge, GUI) are Phase 2.

