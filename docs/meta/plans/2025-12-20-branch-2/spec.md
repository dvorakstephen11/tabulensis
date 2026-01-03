Below is a concrete, file-by-file implementation plan to complete **Branch 2 (Database Mode API Integration)** end-to-end: core API + streaming + CLI flags + auto-key detection + tests using the D1–D4 fixtures. 

---

## 0) Mental model to keep everything consistent

“Database mode” in this repo means: treat a sheet like a table, **align rows by key columns**, and then diff non-key cells for matched keys. Reordering rows with the same keys should not produce diffs (that’s the whole point). This already exists at the **grid level** via `diff_grids_database_mode` and is validated by the existing D1 tests. 

Branch 2 is about:

* surfacing that capability at the **WorkbookPackage** level (sheet lookup, sheet-scoped report, streaming variant) 
* wiring it into the CLI with `--database`, `--sheet`, `--keys`, and `--auto-keys` 
* making sure CLI tests cover D1–D4.

---

## 1) Core: WorkbookPackage database-mode diff API

### 1.1 Expose a sheet-scoped database-mode streaming engine function

You already have a streaming implementation for database mode in `core/src/engine/grid_diff.rs`, but it’s currently file-private. Importantly, it already takes a `sheet_id` argument, so it can produce ops for the real sheet name (not just `"<database>"`). 

**Goal:** Make that streaming function usable from `core/src/package.rs`.

**Edits:**

* `core/src/engine/grid_diff.rs`

  * Change the visibility of the database-mode streaming function from `fn ...` to `pub(crate) fn ...` (or create a `pub(crate)` wrapper with a `try_` name that calls it).
  * While you’re in there, ensure the function still supports the current behavior:

    * try keyed alignment
    * on duplicate keys, fall back to positional `try_diff_grids` (this behavior is relied on by existing tests). 
* `core/src/engine/mod.rs`

  * Re-export the now-`pub(crate)` function so `package.rs` can call it via `crate::engine::...` rather than reaching into a private submodule. (This matches how workbook-level streaming is exposed via `try_diff_workbooks_streaming` today.) 

**Why this is necessary:** the CLI’s git/text output groups ops by `sheet` name; if database mode ops are emitted under a placeholder sheet id, UX and tests become confusing. The streaming function already accepts the correct `sheet_id`, so use it.

---

### 1.2 Add `WorkbookPackage::diff_database_mode` (non-streaming)

Branch 2 requires adding a workbook-level method that:

* finds the requested sheet by name (case-insensitive)
* diffs those two grids in database mode
* returns a unified report. 

**Edits:**

* `core/src/package.rs` (where `diff`, `diff_with_pool`, `diff_streaming`, and `diff_streaming_with_pool` live today) 

**Implementation sketch (behavior, not code):**

1. Use `with_default_session(|session| ...)` like other methods so you can access the shared `StringPool`. 
2. Resolve and match sheet names case-insensitively in both workbooks:

   * Iterate `self.workbook.sheets` and `other.workbook.sheets`.
   * For each `Sheet`, get `pool.resolve(sheet.name)` and compare `.to_lowercase()` to `sheet_name.to_lowercase()`.
3. If either sheet is missing: **return an error** (Branch 2 requires an error on sheet not found). 
4. Use the old sheet’s `name` `StringId` as `sheet_id` (so diff ops refer to the sheet consistently, even if casing differs).
5. Run a database-mode diff on `old_sheet.grid` vs `new_sheet.grid` using the **sheet-scoped** streaming engine function you exposed in 1.1, but via a `VecSink` to collect ops into a `DiffReport` (same pattern as `diff_grids_database_mode` uses internally). 
6. Include Power Query diffs (`m_ops`) in the returned report the same way `diff` does (unified package report). 

**Signature choice (practical):**

* Branch 2’s spec shows returning `DiffReport`, but also requires “Return error if sheet not found”. 
  The cleanest way to satisfy both is:

  * `diff_database_mode(...) -> Result<DiffReport, DiffError>` (or a small new error type)
    This doesn’t break any existing API because the method is new.

---

### 1.3 Add `WorkbookPackage::diff_database_mode_streaming`

This is explicitly required. 

**Edits:**

* `core/src/package.rs` again, mirroring how `diff_streaming_with_pool` works today. 

**Key detail to preserve:** `diff_streaming_with_pool` computes M diffs **before** calling the workbook streaming diff, because the streaming sink writes the string pool on `begin()`. You must do the same, or your JSONL header won’t include strings introduced by M diffs. 

**Implementation sequence (match existing streaming pattern):**

1. `m_ops = diff_m_ops_for_packages(...)` first (to intern all needed strings into the pool). 
2. Wrap the sink in `NoFinishSink` while running the sheet database-mode streaming diff (so you can append `m_ops` and call `finish()` once). This is the exact pattern already used in `diff_streaming_with_pool`. 
3. Emit `m_ops` through the sink, increment op count, then `sink.finish()`.

Return `DiffSummary`.

---

### 1.4 Add a “sheet not found” error variant

Right now `DiffError` only covers limit/sink failures. 
Branch 2 requires “Return error if sheet not found”. 

**Edits:**

* `core/src/diff.rs`: add a new `DiffError` variant, e.g.:

  * `SheetNotFound { requested: String, available: Vec<String> }`
* Build `available` using the string pool at the callsite (package has access to pool).
* Use a display message that is CLI-friendly (“Sheet ‘X’ not found. Available: …”).

Because `DiffError` is `#[non_exhaustive]`, adding a variant is manageable, but you’ll need to update any exhaustive matches in-repo (if any).

---

## 2) Core: Auto-key detection helper

Branch 2 stretch includes `suggest_key_columns(grid, pool) -> Vec<u32>` and a CLI flag that uses it.

### 2.1 Where to implement it

**Best placement:**

* Implement the function in the core crate (public), because CLI is a separate crate.
* Add it near other table/database logic (either:

  * a new small module (e.g. `core/src/database_keys.rs`), or
  * inside `database_alignment.rs` with a public wrapper re-exported from `lib.rs`).

### 2.2 Heuristic that matches your fixtures

Your `db_keyed` generator uses a sheet named **“Data”** with headers like `ID, Name, Amount, Category`.
So a simple heuristic should correctly return column `A` (index 0).

**Suggested heuristic (matches sprint spec):**

1. Prefer column 0 if:

   * values in column 0 are unique (use the same key extraction semantics as database mode: value+formula matters, blank/None is still a key component)
   * and the header cell (row 0) contains one of: `id`, `key`, `sku` (case-insensitive)
2. Otherwise, scan columns for:

   * header contains `id|key|sku` and column values are unique
3. Otherwise, scan for first unique column.

Return a vector of indices (even if it’s a single column).

---

## 3) CLI: Implement database mode flags and wiring

Today the CLI explicitly bails if `key_columns` is set and tells you to use Branch 2 flags in the future.
Branch 2 is that future.

### 3.1 Add new CLI flags in `cli/src/main.rs`

Branch 2 requires: `--database`, `--sheet`, `--keys`, plus stretch `--auto-keys`. 

**Edits:**

* `cli/src/main.rs` `Commands::Diff` fields:

  * `database: bool`
  * `sheet: Option<String>`
  * `keys: Option<String>`
  * `auto_keys: bool`
* Remove or repurpose `key_columns`. Options:

  * Remove it entirely (cleaner).
  * Or keep as a hidden alias for `--keys` temporarily, but update behavior so it works.

Also update the `commands::diff::run(...)` call signature to pass the new args. 

### 3.2 Implement parsing for `--keys` (column letters -> indices)

Requirement: `A=0, B=1, AA=26`. 

**Recommended implementation detail:**

* Parse `--keys` as comma-separated tokens.
* For each token:

  * trim whitespace
  * validate it is letters only `[A-Za-z]+`
  * convert to 0-based column index

You have a couple good options to avoid reinventing:

* Use the existing public `address_to_index` helper by appending `1` (e.g. `"AA" -> "AA1"`), then take the column index. That function already understands Excel-style column lettering. 
  (Still validate “letters only” so input like `A1` doesn’t silently become `A11`.)
* Or promote the internal column-letter helper in `core/src/addressing.rs` (there’s already a `col_letters_to_u32` helper) and expose a clean “letters only” API. 

**Helpful error messages (required):**

* Empty tokens (`--keys=A,,B`)
* Non-letter tokens (`--keys=1`)
* Duplicate columns (`--keys=A,A`)
* Optionally: out-of-range columns for the selected sheet (after opening workbook and knowing `grid.ncols`)

All should become ExitCode 2 via anyhow errors.

### 3.3 Implement sheet selection (`--sheet`) and defaults

Branch spec includes `--sheet=Data` examples. 

I’d implement:

* If `--sheet` is provided: use it.
* If not provided:

  * default to `"Data"` (works naturally with db fixtures), otherwise:
  * if both workbooks have exactly one worksheet, use that sheet name
  * else error: “Multiple sheets found; specify --sheet”.

To implement “exactly one sheet” selection, you’ll need to resolve sheet names from `StringId`. The CLI can do this using `excel_diff::with_default_session` + `session.strings.resolve(...)` because packages and sheet names are interned through the default session.

### 3.4 Wire database mode into `cli/src/commands/diff.rs`

Right now the CLI always runs `old_pkg.diff(&new_pkg, &config)` or streaming `diff_streaming` depending on output format. 

Update `run()` flow:

1. Parse config + open workbooks (unchanged).
2. If `database == false`:

   * keep existing spreadsheet-mode behavior
   * if user passed `--keys/--sheet/--auto-keys`, error early with a message (avoid silent no-ops).
3. If `database == true`:

   * determine `sheet_name` and `key_columns`:

     * if `--keys` present: parse it
     * else if `--auto-keys`: call `suggest_key_columns` on the chosen sheet grid
     * else: error: must pass `--keys` or `--auto-keys`
4. For JSONL streaming:

   * call `old_pkg.diff_database_mode_streaming(&new_pkg, sheet_name, &key_columns, &config, &mut JsonLinesSink::new(writer))`
5. For text/json/git-diff:

   * call `old_pkg.diff_database_mode(&new_pkg, sheet_name, &key_columns, &config)` to get a `DiffReport`
   * render using existing output modules

That directly fulfills the CLI integration requirements.

### 3.5 “Print suggested keys when database mode fails due to duplicates”

Stretch requirement.

You have two viable ways to implement this cleanly:

**Option A (recommended): add a recognizable warning when database mode falls back**

* When the engine falls back due to duplicate keys, emit a warning string like:

  * `"database-mode: duplicate keys for requested columns; falling back to spreadsheet mode"`
* The CLI already prints warnings to stderr via `print_warnings_to_stderr`. 
* If `--auto-keys` is set, the CLI can detect that warning and then:

  * call `suggest_key_columns(...)`
  * print a second message: “Suggested keys: A” (or `A,C`).
  * You can format column letters using `index_to_address(0, col)` and stripping the trailing `1`.

**Option B: detect duplicates pre-run in CLI**

* Add a core-exposed “validate keys are unique” helper (or expose the alignment error), and if it fails:

  * print suggestions
  * return error exit 2 (instead of running fallback)
    This is a stronger UX, but it’s more API surface than Branch 2 strictly asks for.

Given your existing tests rely on fallback behavior at grid level, Option A is the lowest-risk path. 

---

## 4) Tests: make D1–D4 pass at the CLI level

Branch 2 explicitly requires CLI integration tests using the D1–D4 fixtures.
Those fixture outputs are already part of your fixture generation manifests.

### 4.1 Update/remove the obsolete CLI test

There is currently an integration test that asserts `--key-columns` errors out (“not implemented”). After Branch 2, that test should be deleted or rewritten.

### 4.2 Add CLI integration tests for D1–D4

In `cli/tests/integration_tests.rs`, add tests that execute the binary similarly to existing tests (you already have patterns for running the CLI and checking exit codes and JSON validity). 

**Suggested test matrix (covers acceptance criteria):**

1. **D1 reorder should be clean (exit 0)**

   * command: `excel-diff diff --database --sheet=Data --keys=A db_equal_ordered_a.xlsx db_equal_ordered_b.xlsx`
   * assert exit code 0
   * optional: also run `--format=json` and assert `ops.len() == 0`

2. **D2 row added should be detected (exit 1, contains RowAdded)**

   * `... db_equal_ordered_a.xlsx db_row_added_b.xlsx`
   * run `--format=json`
   * parse JSON, assert at least one `RowAdded`

3. **D3 row updated should be detected (exit 1, contains CellEdited)**

   * `... db_equal_ordered_a.xlsx db_row_update_b.xlsx`
   * run `--format=json`
   * parse JSON, assert at least one `CellEdited`

4. **D4 reorder + change should only show the change**

   * `... db_equal_ordered_a.xlsx db_reorder_and_change_b.xlsx`
   * run `--format=json`
   * parse JSON, assert:

     * contains at least one `CellEdited`
     * (and importantly) the diff is small; depending on fixture determinism, you can assert `ops.len()` equals 1 or “< 10”.

5. **Multi-column key parsing sanity**

   * Use D1 but pass `--keys=A,C` (non-contiguous) to cover multi-token parsing behavior. 
   * Expect exit 0.

6. **Invalid column errors**

   * `--keys=1` should exit 2 and mention invalid columns.

7. **Sheet not found errors**

   * `--sheet=NoSuchSheet` should exit 2 and include the “available sheets” list.

### 4.3 Add/adjust core tests (optional but valuable)

Even though Branch 2’s acceptance calls out CLI, I’d add at least one core test validating `WorkbookPackage::diff_database_mode` works and uses the real sheet id (so text/git output won’t show `"<database>"`).

You already have core tests proving database-mode semantics at grid-level and fixtures access.
A new test file like `core/tests/package_database_mode_tests.rs` can:

* open `db_equal_ordered_a.xlsx` and `db_equal_ordered_b.xlsx`
* call the new `WorkbookPackage` method
* assert `report.ops.is_empty()`.

---

## 5) Finish checklist mapped to Branch 2 deliverables

### 2.1 WorkbookPackage API 

* [ ] Add `WorkbookPackage::diff_database_mode(...)` (sheet lookup + call database mode + unify report with M diffs)
* [ ] Case-insensitive sheet matching
* [ ] Error when sheet missing (new `DiffError` variant)
* [ ] Add `diff_database_mode_streaming(...)` mirroring `diff_streaming_with_pool` ordering rules 

### 2.2 CLI integration 

* [ ] Add flags: `--database`, `--sheet`, `--keys`
* [ ] Parse keys as column letters -> indices, with helpful errors
* [ ] Route to package database-mode APIs for both streaming and non-streaming paths

### 2.3 Auto keys stretch

* [ ] Implement `suggest_key_columns(grid, pool)`
* [ ] Add `--auto-keys`
* [ ] When duplicate keys cause fallback, print suggested keys (via warning detection + suggestion formatting)

### Acceptance criteria

* [ ] `excel-diff diff --database --keys=A ...` works
* [ ] D1 reorder produces no diff
* [ ] D3/D4 changes in non-key columns show correctly
* [ ] Multi-key parsing works
* [ ] CLI integration tests pass using `db_*` fixtures

---

If you want, I can follow up with a **file-by-file sequencing plan** (exact order to implement so everything compiles at each step), but the above should already be enough to implement Branch 2 without design gaps.
