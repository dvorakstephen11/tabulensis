## What Branch 1 needs to deliver

Branch 1 is about making the existing library usable from the command line, and making it integrate cleanly with Git workflows (difftool + textconv). The sprint plan explicitly calls for:

* A new `cli/` crate producing an `excel-diff` binary, with `diff` + `info` subcommands and flags like `--format text|json|jsonl`, `--fast`, `--precise`, and a placeholder `--key-columns`. 
* Git integration docs + a `--git-diff` output mode. 
* Human-readable text output structured by sheet plus a Power Query section, along the lines of the provided example. 
* Exit codes: `0 identical`, `1 different`, `2 error`. 

## Key facts from the current codebase that drive the design

1. **Diff ops are rich but reference a string table via IDs**
   `DiffOp` variants (sheet/row/col adds/removes, block moves, cell edits, and Power Query changes) carry `StringId` / `SheetId` values. Those IDs must be resolved through a string table. `DiffReport` already includes `strings: Vec<String>` and a helper `resolve(StringId) -> Option<&str>`.

2. **Streaming is already first-class**
   The library exposes a `DiffSink` trait and a built-in `JsonLinesSink` that writes a header containing the string table, then one JSON op per line.

3. **WorkbookPackage is the intended entry point for CLI**
   `WorkbookPackage::open(reader)` opens an `.xlsx` from any `Read+Seek`, and `WorkbookPackage::diff_streaming(...)` streams ops and appends M/query ops at the end.

4. **Workspace currently only contains `core`**
   Root `Cargo.toml` has `[workspace] members = ["core"]`, so Branch 1 must extend the workspace to include `cli`. 

These constraints imply:

* For **text** and **single JSON** output, it’s easiest to produce a full `DiffReport` so you can resolve string IDs deterministically.
* For **JSONL**, you should use streaming (`JsonLinesSink`) so very large diffs don’t require building an in-memory `Vec<DiffOp>`.

---

## Detailed implementation plan

### Phase A — Create the CLI crate and wire workspace

**A1) Add `cli/` as a workspace member**

* Edit root `Cargo.toml`:

  * Change `members = ["core"]` → `members = ["core", "cli"]`. 

**A2) Create `cli/Cargo.toml`**

* Package name: recommend `excel_diff_cli` (to avoid clashing with the library crate name `excel_diff`).
* Binary name: `excel-diff` (via `[[bin]] name = "excel-diff"`), matching the sprint plan. 
* Dependencies:

  * `clap` with `derive` (required by the plan). 
  * `excel_diff = { path = "../core" }`
  * `serde_json` (for `--format=json` in CLI; even though core uses it internally, CLI will need it too for direct report writing)
  * Optional but very useful:

    * `anyhow` (error context; even before Branch 3’s deeper refactors)
    * `owo-colors` or `anstyle` (optional TTY color; Branch 1 marks color as optional) 

**A3) Create the directory structure exactly as specified**
The plan calls for:

* `cli/src/main.rs`
* `cli/src/commands/{mod.rs,diff.rs,info.rs}`
* `cli/src/output/{mod.rs,text.rs,json.rs}` 

Keep this structure; it will also make Branch 2/3 follow-ons less painful.

---

### Phase B — Clap interface and command dispatch

**B1) Implement `Cli` + `Commands`**

* Follow the plan’s clap skeleton:

  * `excel-diff diff <old> <new> [--format text|json|jsonl] [--key-columns ...] [--fast|--precise]`
  * `excel-diff info <path> [--queries]` 

Implementation details that matter for correctness:

* Make `--fast` and `--precise` mutually exclusive (if both set, exit code `2` with a clear message).
* Default `--format` to `text` if missing.
* Add `--git-diff` (bool) as requested under Git Integration deliverables. 

  * Treat it as either:

    * a separate mode that overrides `--format` (recommended), or
    * another `--format` value like `git` (less direct to users).
  * If both `--git-diff` and `--format=json|jsonl` are provided, treat as usage error (exit 2).

**B2) Exit code policy**
Implement a shared helper used by both `diff` and `info`:

* `Ok + no changes`: exit `0`
* `Ok + changes`: exit `1`
* `Any error (parse, IO, invalid args)`: exit `2` 

Nuance worth baking in now:

* Only return `0` if:

  * op_count is `0`, **and**
  * the diff is `complete == true`
    If `complete == false`, you can’t be sure the files are identical; treat as “different” (exit `1`) and print warnings to stderr.

This aligns with `DiffReport.complete` / `DiffSummary.complete` semantics.

---

### Phase C — Implement the `diff` subcommand

#### C1) Open both workbooks

* Use `std::fs::File::open(path)` and pass the `File` to `WorkbookPackage::open(file)`. 
* Handle errors with human-friendly messages:

  * file not found / permissions
  * not a ZIP / not an OPC package (`ContainerError` paths)
  * malformed workbook parts (`PackageError`)
* Always print errors to **stderr** so stdout remains clean JSON when requested.

#### C2) Build `DiffConfig` from flags

Map presets to the existing API:

* `--fast` → `DiffConfig::fastest()`
* `--precise` → `DiffConfig::most_precise()`

Those methods are already part of the public config surface. 

(If your codebase uses builder methods instead, keep the same semantic mapping; the plan’s intent is “fast preset” vs “most precise preset”.) 

#### C3) Decide how to handle `--key-columns` in Branch 1

The sprint plan requires the flag to exist in Branch 1, but Branch 2 is where workbook-level database mode becomes real.

Recommended Branch 1 behavior (to avoid silently lying):

* Parse/accept the flag, but if it is set:

  * Print: “Database mode is not implemented in CLI yet; use Branch 2 flags (`--database --sheet --keys`) once available.”
  * Exit `2`

This satisfies the “flag exists” requirement without producing incorrect diffs.

(If you *do* want a minimal “single-sheet only” implementation now, do it explicitly as “experimental” and still plan to replace it in Branch 2.)

#### C4) Output modes

##### Mode: `--format=text` (default)

* Compute a full `DiffReport` (so you can resolve string IDs consistently via `report.resolve(id)`).
* Pass it to the text formatter described in Phase D.
* Exit code: based on `report.ops.is_empty()` + `report.complete`.

`DiffReport` already carries everything needed.

##### Mode: `--format=json`

* Write the `DiffReport` to stdout using `serde_json::to_writer`.
* Do not intermix logs; warnings go to stderr.

`DiffReport` is `Serialize`. 

##### Mode: `--format=jsonl`

* Use streaming to avoid building a full report:

  * Create `JsonLinesSink<std::io::StdoutLock>`
  * Call `pkg_a.diff_streaming(&pkg_b, &config, &mut sink)`
  * Get back a `DiffSummary` with `op_count`, `complete`, `warnings`
* Print warnings to stderr (not as extra JSON lines).
* Exit code:

  * `0` only if `summary.op_count == 0` and `summary.complete == true`
  * else `1` if ok but changed/incomplete
  * else `2` on error

This uses the library’s existing streaming model correctly.

##### Mode: `--git-diff`

* Produce “unified diff-style” text (Phase E).
* This mode is intended for human consumption in Git workflows, not for machine parsing. 

---

### Phase D — Implement the human-readable text formatter

The sprint plan gives a concrete shape for output: a header line, per-sheet grouping, and a Power Query section. 

#### D1) Formatting goals and invariants

* **Stable ordering**: same inputs produce identical output (important for Git diffs).
* **By-sheet grouping**: build a `BTreeMap<String, Vec<&DiffOp>>` keyed by resolved sheet name.
* **Don’t require access to the live `StringPool`**: use `DiffReport.resolve()` and/or `report.strings` (works for JSON too).

#### D2) Per-op formatting rules (core mapping)

Implement a `render_op(report: &DiffReport, op: &DiffOp, verbosity: Verbosity) -> Vec<String>`.

Suggested output mapping:

* `SheetAdded/SheetRemoved`
  `Sheet "<name>": ADDED/REMOVED`

* `RowAdded/RowRemoved`
  `Row <idx+1>: ADDED/REMOVED`
  (Excel users expect 1-based row numbers. The internal ops are 0-based indices.)

* `ColumnAdded/ColumnRemoved`
  `Column <A,B,C,...>: ADDED/REMOVED`
  (Convert index → letters using existing addressing helpers or your own.)

* `BlockMovedRows`
  `Block moved: rows <src+1>-<src+count> → rows <dst+1>-<dst+count>`

* `BlockMovedColumns`
  Similar, using column letters.

* `BlockMovedRect`
  Use A1 ranges: `B3:D5 → G10:I12` (build from `src_start_row/col` and counts).

* `CellEdited`
  `Cell <A1>: <old> → <new>`
  Where `<old>/<new>` is:

  * number rendered with sensible formatting (don’t print `42.0000000001` unless necessary)
  * strings quoted, with newlines escaped
  * blanks as empty
  * errors as their string value

There’s already similar “render cell value” logic in the JSON output support code; mirror that behavior so text/json agree on how values look. 

* Power Query ops (`op.is_m_op() == true`)
  Group under `Power Query:` and format based on:

  * added/removed/renamed
  * `QueryDefinitionChanged` includes `change_kind` (semantic vs formatting)
  * metadata changes include the field name and old/new values
    The op variants are explicitly defined.

#### D3) Summary statistics

At end, print:

* total op count
* breakdown counts by category (sheets / rows / cols / blocks / cells / queries)
* completeness + warnings (warnings printed as bullet list)

This is explicitly called out as a deliverable.

#### D4) Verbosity levels

Implement at least:

* `normal` (default): prints one-line ops like in the example
* `quiet`: only summary + whether changed
* `verbose`: includes additional details when available (e.g., show formulas when a formula changed, include block hashes/signatures)

This is explicitly in the deliverables list for text output. 

#### D5) Optional TTY color

If stdout is a terminal:

* green for ADDED
* red for REMOVED
* yellow for CHANGED / MOVED
  Keep it off when piping to files.

The plan marks this as optional. 

---

### Phase E — Implement `--git-diff` unified-diff-style output

The purpose of `--git-diff` is readability inside Git tooling. The sprint plan asks for “unified diff-style”. 

A practical, low-risk approach:

1. Print a git-like header:

   * `diff --git a/<old> b/<new>`
   * `--- a/<old>`
   * `+++ b/<new>`

2. For each sheet section:

   * `@@ Sheet "<name>" @@`

3. For each op, emit “patch-like” lines:

   * Additions: `+ Row 5: ADDED`
   * Removals: `- Row 12: REMOVED`
   * Changes: emit a `-` line for old and `+` line for new, e.g.:

     * `- Cell C7: 100`
     * `+ Cell C7: 150`

This isn’t intended to be applied as a patch; it’s intended to “look like” a unified diff so it feels native in Git.

(Keep real machine-readable output as `--format=json` / `--format=jsonl`.)

---

### Phase F — Implement the `info` subcommand

The plan says: “info subcommand showing workbook structure” and optionally Power Query information. 

#### F1) Workbook structure output

Open the workbook with `WorkbookPackage::open`, then print:

* `Workbook: <filename>`
* `Sheets: <n>`
* For each sheet:

  * `- "<name>" (<kind>) <nrows>x<ncols>, <cell_count> cells`

This is useful for git `textconv` because it’s stable, compact, and makes structural changes show up.

#### F2) `--queries`

If `--queries` is set:

* If `data_mashup` is absent: print `Power Query: none`
* Else:

  * call `build_queries(&data_mashup)`
  * list queries deterministically (sort by full name recommended)
  * show key metadata bits (group, load flags, etc.)

The library exports query-building helpers and metadata types for this. 

---

### Phase G — Git integration documentation + examples

Branch 1 requires that Git integration is documented and tested. 

#### G1) README changes

Add a “Git integration” section with:

* The `.gitconfig` snippet from the plan (difftool + diff driver via textconv). 
* A `.gitattributes` snippet example, e.g.:

  * `*.xlsx diff=xlsx` 
* A short explanation of terminology:

  * **difftool**: runs an external program on two file paths
  * **textconv**: converts a binary file into text so git can diff it line-by-line

#### G2) “Test with actual Git repositories”

Add a small manual test checklist to the repo (or as a README subsection):

* create a scratch repo
* commit `old.xlsx`
* modify → commit `new.xlsx`
* verify `git diff` shows info output diff via textconv
* verify `git difftool` launches `excel-diff diff` output

---

### Phase H — CLI-level tests (strongly recommended in Branch 1)

Even though the plan doesn’t explicitly demand automated tests, it demands “documented and tested” Git integration and clear help output. 

Add `cli/tests/` integration tests using:

* `assert_cmd` to run `excel-diff`
* Use existing generated fixtures in `fixtures/generated` (core already has helpers pointing there). 

Test matrix:

1. `excel-diff diff a.xlsx a.xlsx` → exit `0`
2. known-different fixture pair → exit `1`
3. `--format=json` output parses as JSON
4. `--format=jsonl` first line parses as header `{ kind: "Header", ... }` (the sink guarantees this shape) 
5. `info` prints sheet list, and `--queries` prints query list on mashup fixtures

---

## Deliverables-to-work mapping checklist

This maps 1:1 to the sprint plan deliverables so you can track completion:

* Create `cli/Cargo.toml` + clap dependency → Phase A2 + B1 
* `diff` text output → Phase C4(text) + Phase D
* `diff` JSON output → Phase C4(json) 
* `diff` JSONL output → Phase C4(jsonl) 
* `info` workbook structure → Phase F1 
* `--key-columns` flag exists → Phase C3 
* `--fast`, `--precise` → Phase C2 
* Exit codes 0/1/2 → Phase B2 
* Git docs + `.gitattributes` example → Phase G 
* `--git-diff` mode → Phase E 
* Clear `--help` output → Phase B1 (clap struct docstrings + value enums) 

---

## Notes that make Branch 2 easier later

If you make just two small structural choices now, Branch 2 becomes mostly additive:

1. In `commands/diff.rs`, isolate “diff strategy selection”:

   * `SpreadsheetMode` (current default)
   * `DatabaseMode { sheet, keys }` (future)
2. In `output/text.rs`, keep formatting driven purely by `DiffReport` / `DiffOp`, not by internal workbook types.

That way, when Branch 2 adds `WorkbookPackage::diff_database_mode(...)`, you swap the diff producer, not the formatter.
