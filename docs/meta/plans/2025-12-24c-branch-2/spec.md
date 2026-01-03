## Section 2 implementation plan: prune fluff, consolidate fuzz, drop redundant vendor, gate legacy API

Section #2 in `next_sprint_outline.md` is explicitly the “Unnecessary fluff to remove (high-confidence)” set of changes (A–D).

### Definition of done for this section

* `tmp/` is removed from the repo and ignored going forward.
* Only one fuzz harness remains (canonical: `core/fuzz`), and CI fuzz is simpler (no duplicated logic).
* No redundant vendored Python trees remain; fixture generation uses pip-managed dependencies (preferably pinned).
* Legacy entrypoints the outline calls out are not available by default; they require an explicit legacy feature flag.

### Suggested PR structure (single PR, 4 commits)

1. Delete + ignore `tmp/`
2. Fuzz harness consolidation + simplify `fuzz.yml`
3. Python vendor cleanup + pin fixture deps in CI
4. Gate legacy APIs behind `legacy-api` feature + compile checks

---

## A) Checked-in temporary artifacts: delete tmp/ and ignore it

The snapshot shows `tmp/_release_test`, `tmp/openpyxl_sdist`, and `tmp/xlsxwriter_sdist` (plus older xlsxwriter copies) checked in.

### Steps

1. Confirm nothing relies on `tmp/`

* Run `rg -n "tmp/"` and `rg -n "tmp\\\\"` at repo root.
* Expect to find only references in docs/context, not in build scripts or runtime code.

2. Remove `tmp/` from version control

* `git rm -r tmp`
* Commit: `chore: remove checked-in tmp artifacts`

3. Add `tmp/` to `.gitignore`

* Current `.gitignore` does not ignore `tmp/`.

### Code change

Replace this in `.gitignore`:

```gitignore
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
fixtures/generated/*.csv
fixtures/generated/*.xlsm


# Docs
docs/meta/completion_estimates/

# WASM build output
web/wasm/
```

With:

```gitignore
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Local scratch
tmp/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
fixtures/generated/*.csv
fixtures/generated/*.xlsm


# Docs
docs/meta/completion_estimates/

# WASM build output
web/wasm/
```

### Validation

* `python -m pip install -e fixtures` then `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force`
* `cargo test --workspace`
* `git status` should show no `tmp/` tracked/untracked surprises

---

## B) Redundant fuzz harnesses: keep core/fuzz, delete the other, simplify CI

The outline claims there are *two* fuzz harnesses and says to keep `core/fuzz`.
In the current repo snapshot, `core/fuzz` clearly exists and contains multiple fuzz targets.
The current fuzz workflow already runs the targets called out in the outline.

### Steps

1. Verify whether a second harness actually exists

* In the provided directory tree, I only see `core/fuzz` (no top-level `fuzz/`).
* Still confirm in the repo with `ls -la fuzz` / `find . -maxdepth 2 -type d -name fuzz`.

2. If a top-level `fuzz/` exists, remove it

* `git rm -r fuzz`
* Ensure no CI steps reference it (current fuzz.yml uses `core/fuzz`). 

3. Simplify `.github/workflows/fuzz.yml`

* Collapse repeated steps into a single loop (same behavior, less YAML).

### Code change

Replace this in `.github/workflows/fuzz.yml`:

```yaml
name: Fuzzing

on:
  schedule:
    - cron: "0 4 * * 0"
  workflow_dispatch: {}

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust (nightly)
        uses: dtolnay/rust-action@stable
        with:
          toolchain: nightly

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Fuzz core datamashup framing
        run: cargo fuzz run fuzz_datamashup_parse -max_total_time=60
        working-directory: core/fuzz

      - name: Fuzz core grid diff
        run: cargo fuzz run fuzz_diff_grids -max_total_time=60
        working-directory: core/fuzz

      - name: Fuzz M parser + AST diff
        run: cargo fuzz run fuzz_m_section_and_ast -max_total_time=60
        working-directory: core/fuzz

      - name: Upload fuzz artifacts
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: fuzz-artifacts
          path: |
            core/fuzz/artifacts
          if-no-files-found: ignore
```

With:

```yaml
name: Fuzzing

on:
  schedule:
    - cron: "0 4 * * 0"
  workflow_dispatch: {}

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust (nightly)
        uses: dtolnay/rust-action@stable
        with:
          toolchain: nightly

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz --locked

      - name: Fuzz core targets
        working-directory: core/fuzz
        run: |
          set -e
          for target in fuzz_datamashup_parse fuzz_diff_grids fuzz_m_section_and_ast; do
            echo "Running $target"
            cargo fuzz run "$target" -max_total_time=60
          done

      - name: Upload fuzz artifacts
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: fuzz-artifacts
          path: |
            core/fuzz/artifacts
          if-no-files-found: ignore
```

### Validation

* Local sanity: `cd core/fuzz && cargo fuzz run fuzz_m_section_and_ast -max_total_time=10`
* CI: confirm logs show all three “Running …” lines

---

## C) Vendored Python sdists / expanded vendor trees: drop redundancy, prefer pinned pip deps

The outline calls out redundant vendored XlsxWriter sources (“expanded tree + tarball”) and says to keep either pinned pip deps or a single vendored artifact.

From the snapshot:

* There is definitely vendored XlsxWriter material under `tmp/` (handled by A).
* I do **not** see `fixtures/vendor/...` listed under `fixtures/` in the provided directory tree (so this may already be gone in your current snapshot); still verify in the actual repo.
* `fixtures/pyproject.toml` currently uses `>=` dependencies (not pinned). 
* CI currently installs fixtures via `pip install -e fixtures`. 
* The repo tree includes `fixtures/requirements.txt`, which is the natural place to pin. 

### Steps

1. Remove redundant vendored trees (if present)

* If `fixtures/vendor/XlsxWriter-3.2.0/` exists: `git rm -r fixtures/vendor`
* If there’s both a tarball and expanded tree anywhere you decide to keep vendored artifacts: keep only one.

2. Prefer pinned pip deps (recommended)

* Ensure `fixtures/requirements.txt` contains pinned versions for fixture generation (including XlsxWriter if needed), so output stays deterministic across time.
* Update CI to install dependencies from `requirements.txt`, then install the fixtures package with `--no-deps`.

### Code change (CI pinning)

Replace this in `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Install Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: Install fixture generator
        run: python -m pip install -e fixtures

      - name: Generate test fixtures
        run: generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force

      - name: Run tests
        run: cargo test --workspace

      - name: Build examples
        run: cargo build --workspace --examples

      - name: Run clippy (deny unwrap/expect)
        run: cargo clippy --workspace -- -D clippy::unwrap_used -D clippy::expect_used
```

With:

```yaml
name: CI

on:
  push:
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Install Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: Install fixture generator deps
        run: python -m pip install -r fixtures/requirements.txt

      - name: Install fixture generator
        run: python -m pip install -e fixtures --no-deps

      - name: Generate test fixtures
        run: generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force

      - name: Run tests
        run: cargo test --workspace

      - name: Build examples
        run: cargo build --workspace --examples

      - name: Run clippy (deny unwrap/expect)
        run: cargo clippy --workspace -- -D clippy::unwrap_used -D clippy::expect_used
```

### Validation

* CI/local:

  * `python -m pip install -r fixtures/requirements.txt`
  * `python -m pip install -e fixtures --no-deps`
  * `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force`

---

## D) Legacy API surface: hide behind explicit legacy feature flag

The outline says a legacy diff entrypoint is “panic-based” and asks to make the legacy path non-public by default, behind an explicit legacy feature flag, and ensure internal callers use the safe APIs.

In `core/src/lib.rs`, there are deprecated, `#[doc(hidden)]` public wrappers (`diff_workbooks`, `try_diff_workbooks`, `open_workbook`) that are currently compiled unconditionally.
Separately, the engine already has safe/structured behavior (e.g., best-effort report + warnings rather than panicking). 
`core/Cargo.toml` currently has no “legacy” feature flag. 

### Steps

1. Add `legacy-api` feature in `core/Cargo.toml`

* Do **not** enable it by default.

2. Gate the deprecated wrappers in `core/src/lib.rs`

* Keep `with_default_session` available (it’s used widely in workspace code/tests).
* Gate only the deprecated wrappers.

3. Confirm no workspace crates depend on the legacy wrappers

* `wasm` and `cli` already use `WorkbookPackage`/`PbixPackage`, not the deprecated wrappers.

4. Add build checks

* `cargo build -p excel_diff` (default features, without legacy)
* `cargo build -p excel_diff --features legacy-api` (opt-in path still works)

### Code changes

#### 1) `core/Cargo.toml`

Replace this `[features]` block in `core/Cargo.toml`:

```toml
[features]
default = ["excel-open-xml"]
excel-open-xml = []
std-fs = ["excel-open-xml"]
model-diff = []
perf-metrics = []
dev-apis = []
```

With:

```toml
[features]
default = ["excel-open-xml"]
excel-open-xml = []
std-fs = ["excel-open-xml"]
model-diff = []
perf-metrics = []
dev-apis = []
legacy-api = []
```

#### 2) `core/src/lib.rs`

Replace this block in `core/src/lib.rs`:

```rust
#[doc(hidden)]
pub fn with_default_session<T>(f: impl FnOnce(&mut DiffSession) -> T) -> T {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        f(&mut session)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::try_diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[deprecated(note = "use WorkbookPackage::open")]
#[allow(deprecated)]
#[doc(hidden)]
pub fn open_workbook(path: impl AsRef<std::path::Path>) -> Result<Workbook, ExcelOpenError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        excel_open_xml::open_workbook(path, &mut session.strings)
    })
}
```

With:

```rust
#[doc(hidden)]
pub fn with_default_session<T>(f: impl FnOnce(&mut DiffSession) -> T) -> T {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        f(&mut session)
    })
}

#[cfg(feature = "legacy-api")]
#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[cfg(feature = "legacy-api")]
#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::try_diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[cfg(all(feature = "legacy-api", feature = "excel-open-xml", feature = "std-fs"))]
#[deprecated(note = "use WorkbookPackage::open")]
#[allow(deprecated)]
#[doc(hidden)]
pub fn open_workbook(path: impl AsRef<std::path::Path>) -> Result<Workbook, ExcelOpenError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        excel_open_xml::open_workbook(path, &mut session.strings)
    })
}
```

