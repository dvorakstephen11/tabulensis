Below is a concrete, “do-this-next” implementation plan for **Branch 7: Documentation & Polish**, based on your sprint plan’s Branch 7 deliverables/acceptance criteria  and the current repo layout + CLI/library surface area in `codebase_context.md` .

Because you’ve already implemented branches **1–4**, this plan assumes the CLI + database mode + object-graph diff ops exist and should now be *documented accurately* (database flags/behavior, output formats, warnings, error codes, etc.).

---

## 1) Establish a docs structure that matches the workspace reality

### Decision: where docs live

Branch 7 asks for:

* README quickstart
* CLI reference
* DiffConfig/config guide
* Git integration tutorial
* Database mode guide
* FAQ
* Architecture overview
* Migration guide
* Example programs 

Your repo already has a root `README.md` and a `.gitignore` entry anticipating a `docs/` area (`docs/meta/...`).

**Plan:**

* Keep **root `README.md`** as the primary user-facing entry point (CLI-first, with a library section).
* Create a **`docs/`** folder for the longer guides:

  * `docs/cli.md`
  * `docs/configuration.md`
  * `docs/git.md`
  * `docs/database_mode.md`
  * `docs/faq.md`
  * `docs/architecture.md`
  * `docs/migration.md`

This matches Branch 7 deliverables and keeps links stable. 

### Decision: where example programs go (workspace nuance)

Branch 7 lists example programs as `examples/basic_diff.rs`, etc. 

But your repo is a **workspace** with `core` (library) and `cli` (binary). 
Cargo examples need to live under a **package**, so the practical placement is:

* `core/examples/basic_diff.rs`
* `core/examples/streaming.rs`
* `core/examples/database_mode.rs`
* `core/examples/custom_config.rs`

Then users run them via: `cargo run -p excel_diff --example basic_diff -- <args...>`

This is the cleanest way to satisfy “all examples compile and run” in a workspace.

---

## 2) Root README.md: implement a CLI-first quick start that’s faithful to your flags/behavior

Branch 7 explicitly requires a README with a quick start and clear installation + usage instructions. 
Your CLI is the `excel-diff` binary (package `excel_diff_cli`).

### README content outline (write in this order)

#### 2.1. “What is this?”

Explain:

* It diffs Excel workbooks (Open XML `.xlsx` / `.xlsm`) into “diff operations”.
* It supports:

  * spreadsheet-style alignment + cell/row/col diffs
  * **database mode** (key-based row alignment)
  * Power Query (“M”) diffs (semantic/textual)
  * object graph diffs (named ranges, charts, VBA modules) since you completed Branch 4.

#### 2.2. Installation section (be honest with current state)

Because Branch 5 packaging may not be done yet, make the “from source” path first-class:

* `cargo build --release -p excel_diff_cli`
* run `target/release/excel-diff ...`

Optionally also include:

* `cargo install --path cli` (if that’s your preferred local install method)

(If you have a published crate or releases later, you can add those later; Branch 7 only requires clear instructions now.)

#### 2.3. Quick start: show the 4 most important commands

These should match implemented CLI flags:

1. Standard diff (text output):

* `excel-diff diff old.xlsx new.xlsx`

2. JSON output:

* `excel-diff diff --format json old.xlsx new.xlsx`

3. Streaming JSONL output (note the special-case behavior):

* `excel-diff diff --format jsonl old.xlsx new.xlsx`

  * explain that JSONL uses the streaming path (and cannot be combined with `--git-diff`).

4. Database mode (keys vs auto keys):

* `excel-diff diff --database --sheet Data --keys A old.xlsx new.xlsx`
* `excel-diff diff --database --sheet Data --auto-keys old.xlsx new.xlsx`

#### 2.4. Output formats and exit codes (document what your code actually does)

Your CLI returns:

* `0` if `report.ops` is empty and `report.complete` is true
* `1` otherwise
* `2` on any CLI/runtime error (argument conflicts, parsing failures, etc.)

Document that plainly in README.

#### 2.5. “Common flag constraints” callout

Include a small “gotchas” block reflecting current enforcement:

* `--fast` and `--precise` are mutually exclusive
* `--git-diff` cannot be used with `--format json` or `--format jsonl`
* `--sheet`, `--keys`, `--auto-keys` require `--database`
* `--keys` and `--auto-keys` are mutually exclusive

These constraints are extremely useful to users and prevent wasted time.

#### 2.6. Short “Git integration” teaser + link

Add a short snippet and link to `docs/git.md`. The deeper tutorial lives there.

#### 2.7. Short “Library usage” teaser + link

Because this is a workspace with a public library crate, include a minimal `WorkbookPackage` example and link to API docs / rustdoc. `WorkbookPackage` is the central entry point you already have in core.

---

## 3) Write the user docs files (docs/...) to satisfy Branch 7 deliverables

### 3.1 docs/cli.md (CLI reference)

This should be a faithful reference to `excel-diff`’s clap surface:

* Commands: `diff`, `info`
* `diff` flags:

  * `--format {text|json|jsonl}`
  * `--git-diff`
  * `--fast`, `--precise`
  * `-q/--quiet`, `-v/--verbose`
  * `--database`
  * `--sheet <NAME>`
  * `--keys <A,B,AA,...>`
  * `--auto-keys`
* `info` flags:

  * `--queries` (Power Query info)

Also include a section “Exit codes” and “Common errors” (e.g., invalid column in `--keys` causes exit code 2).

**Implementation detail:** generate the “authoritative” option list by copy-pasting `excel-diff --help` output and then reformat it; this reduces drift.

### 3.2 docs/configuration.md (DiffConfig options)

This doc needs to bridge:

* CLI presets (`DiffConfig::fastest()`, `DiffConfig::most_precise()`) used by `--fast/--precise`
* Library users using `DiffConfig::builder()` and fields like preflight thresholds, alignment budgets, limits, etc. (your config file shows many knobs).

**Structure the guide by mental model, not by field order:**

1. “What DiffConfig controls”
2. “Choosing a preset”
3. “Safety + limits” (max ops, alignment limits, timeouts)
4. “Move detection + alignment knobs”
5. “Semantic diff knobs” (formula/M semantics)
6. “LimitBehavior: what happens when we hit limits” (especially important; ties to `report.complete`)

Where possible, include **recommendations**, e.g.:

* if you want a stable CI gate, prefer `--format jsonl` + grep for `complete=false`
* if you want best human readability, prefer text + verbose

### 3.3 docs/git.md (Git integration tutorial)

This doc should explain *both*:

* “textconv” style (for `git diff` on binary files)
* “difftool” style (for interactive tool)

Your sprint plan explicitly wanted Git difftool integration.
Your CLI already supports:

* `excel-diff info` (used for text conversion)
* `excel-diff diff --git-diff` (unified diff style output)

**Recommended structure:**

1. Why Git needs help with `.xlsx`
2. Option A: `textconv` using `excel-diff info`

   * `.gitattributes` snippet (`*.xlsx diff=xlsx`, `*.xlsm diff=xlsx`)
   * `.gitconfig` snippet defining `[diff "xlsx"] textconv = excel-diff info` and `binary = true`
3. Option B: `difftool` using `excel-diff diff --git-diff`
4. Troubleshooting:

   * quoting paths on Windows
   * large repo performance notes
   * if `excel-diff` not on PATH

Also mention there is a `.gitattributes.example` file in the repo (per layout) and recommend copying/adapting it. 

### 3.4 docs/database_mode.md (Database mode guide + examples)

This should match actual behavior:

* Requirement: `--database` with either `--keys` or `--auto-keys` (and they’re mutually exclusive).
* Key parsing:

  * comma-separated letter tokens
  * rejects empty tokens, non-letters, duplicates
  * supports AA/AB style columns
* Sheet selection heuristic (important):

  * if `--sheet` provided, use it
  * else if a sheet named “Data” exists in either workbook, default to “Data”
  * else if both have exactly one sheet, use that
  * else error “Multiple sheets found; please specify --sheet” 
* Auto-key detection behavior:

  * uses `suggest_key_columns` and prints “Auto-detected key columns: ...”

Include:

* “How to choose key columns”
* “What happens with duplicate keys”

  * tie this to warning output and the CLI’s fallback hint logic (it prints a suggestion if it detects a warning mentioning duplicate keys and fallback).

### 3.5 docs/faq.md (pitfalls, limitations, error codes)

Branch 7 explicitly calls for FAQ including error codes. 
You have error code constants like `EXDIFF_DM_*` and `EXDIFF_DIFF_*`.

Include FAQs such as:

* “Why did my diff exit with code 2?”
* “What does `complete=false` mean?”
* “Why does database mode say duplicate keys / fallback?”
* “Why is `--git-diff` not allowed with JSON output?”
* “Do you support `.xls`?” (likely no; Open XML focus)
* “How are Power Query changes represented?” (high-level; link to M diff docs / output)

Also include a short table mapping error codes to meaning and “Suggestion:” text style (you’ve embedded suggestions in some errors already).

---

## 4) API documentation: make `cargo doc --open` genuinely useful

Branch 7 requires “complete rustdoc for all public types”, “code examples”, plus architecture overview and migration guide.

### 4.1 Do a public API audit first (fast but high leverage)

From `core/src/lib.rs`, the public surface includes (at least):

* `WorkbookPackage`, `PackageError`, `VbaModule`, `VbaModuleType`
* `DiffConfig` + builder/presets
* `DiffReport`, `DiffOp`, `DiffSummary`/streaming summary types
* Sink types (`JsonLinesSink`, `VecSink`, `CallbackSink`)
* Core IR types: `Workbook`, `Sheet`, `Grid`, `CellSnapshot`, etc.
* `StringId`, `StringPool`, `DiffSession`, `with_default_session`
* Output helpers in `output` module 
* `error_codes` constants 

**Task:** walk each public type and ensure:

* a 1–3 sentence doc comment exists
* “what it represents” is clear
* any invariants are stated
* important caveats (like “StringId must be resolved via a session/pool”) are stated

### 4.2 Fix the crate-level docs example so it compiles as a doctest

Your crate-level docs currently include an `ignore` example. 
Branch 7 wants code examples in doc comments. 

**Plan:**

* Convert the crate-level example to `no_run` and make it compile cleanly.
* Prefer `Result<(), Box<dyn std::error::Error>>` in the example rather than unwrap/expect.

Also add **two short focused examples** (still `no_run` is fine):

* “Streaming JSONL” (shows `diff_streaming` + `JsonLinesSink`)
* “Database mode diff” (shows `diff_database_mode` with keys)

Those will directly reflect your API.

### 4.3 Document “StringId + resolution” prominently

A lot of your types store `StringId`.
Users will otherwise hit a wall.

**Plan:**

* Add explicit docs to:

  * `StringId`
  * `StringPool`
  * `DiffSession`
  * `with_default_session`
* Include a tiny example showing: intern -> id -> resolve.

(Your tests already demonstrate the string pool behavior; translate that into docs.)

### 4.4 Make architecture + migration guides visible in rustdoc (not just as repo files)

Branch 7 requires:

* architecture overview document
* migration guide from old APIs

If you only put these in `docs/*.md`, `cargo doc --open` won’t naturally “include” them. To meet the acceptance criterion that rustdoc is useful , make them show up inside rustdoc.

**Plan:**

* Add `docs/architecture.md` and `docs/migration.md` as repo files.
* Add a small module in the library crate that includes them via `include_str!`, e.g. `core/src/docs.rs` with:

  * `pub mod architecture { #![doc = include_str!("../../docs/architecture.md")] }`
  * `pub mod migration { #![doc = include_str!("../../docs/migration.md")] }`

Then link to `excel_diff::docs::architecture` from the crate-level docs.

This makes `cargo doc --open` satisfy the “useful docs” criterion, even when browsing only rustdoc.

### 4.5 Migration guide content (what to write)

Your library has deprecated/hidden old entry points (based on the crate docs and the plan’s intention).

**Migration doc should include:**

* “Old way” → “New way” mapping:

  * open workbook → `WorkbookPackage::open(...)`
  * diff → `WorkbookPackage::diff(...)`
  * streaming → `WorkbookPackage::diff_streaming(...)`
  * database mode → `WorkbookPackage::diff_database_mode(...)` / streaming variant
* How to deal with `StringId` and why it exists
* How to interpret `DiffReport` vs streaming summary

---

## 5) Example programs: implement the four required examples and wire them into CI

Branch 7 requires four example programs: basic, streaming, database mode, custom config. 

### 5.1 core/examples/basic_diff.rs

Goal: show the simplest library usage:

* open two files
* `diff(...)`
* print:

  * whether complete
  * op count
  * (optionally) print first N ops

**Implementation notes:**

* accept two args: old/new path
* do not require external crates beyond std

### 5.2 core/examples/streaming.rs

Goal: show streaming JSONL:

* open
* create `JsonLinesSink` targeting stdout
* call `diff_streaming(...)`
* print summary to stderr (op_count + complete + warning count)

This should align with the CLI’s streaming implementation.

### 5.3 core/examples/database_mode.rs

Goal: demonstrate database mode API:

* args: old, new, sheet name, keys (or default to “Data” + “A”)
* call `diff_database_mode(...)`
* print summary

This matches Branch 2’s API spec and what your CLI uses.

### 5.4 core/examples/custom_config.rs

Goal: demonstrate `DiffConfig::builder()`:

* build config tweaking a couple of “safe” knobs (not too many)
* run diff and print summary

This aligns with your config builder usage style in tests.

### 5.5 Make examples “run in CI” without extra user files

You already generate fixtures in CI before running tests. 
So we can run the examples against known fixtures to prove “compile and run”.

**Plan:**

* Add a CI step after tests:

  * `cargo build --workspace --examples`
  * run each example once with fixtures that exist in the manifest, e.g.:

    * `equal_sheet_a.xlsx` + `equal_sheet_b.xlsx`
    * `db_equal_ordered_a.xlsx` + `db_equal_ordered_b.xlsx`

This directly satisfies Branch 7 acceptance: “All examples compile and run.” 

---

## 6) Add “no broken links” + docs build checks to CI

Branch 7 acceptance criteria includes:

* README clear installation/usage
* `cargo doc --open` produces useful documentation
* examples compile and run
* no broken links in docs 

Your CI currently:

* installs fixture generator
* generates fixtures
* `cargo test --workspace`
* `cargo clippy --workspace ...` 

### 6.1 Add a doc build step

Add a CI step:

* `cargo doc -p excel_diff --no-deps`
  Optionally also:
* `cargo test -p excel_diff --doc` (or just rely on `cargo test --workspace` if it already runs doctests in your setup)

This guards against broken rustdoc, missing include_str paths, and non-compiling doctests.

### 6.2 Add a simple Markdown link checker

You don’t need a heavy tool initially. A small script is enough:

* scan `README.md` and `docs/*.md`
* for relative links (`(docs/...)`, `(../...)`) confirm the target file exists
* ignore http(s) links for now (or optionally validate them later)

Put it at `scripts/check_doc_links.py` and run it in CI.

This directly enforces “no broken links.”

---

## 7) Final “Definition of Done” checklist mapped to Branch 7 acceptance criteria

Use this as your branch merge checklist.

### User docs

* [ ] Root `README.md` has:

  * [ ] installation instructions that work today
  * [ ] quick-start commands for diff / json / jsonl / database mode
  * [ ] documented exit codes
  * [ ] links to deeper docs
* [ ] `docs/cli.md` matches clap flags and constraints
* [ ] `docs/git.md` includes both textconv and difftool patterns and uses `excel-diff info` / `--git-diff`
* [ ] `docs/database_mode.md` documents sheet selection + keys parsing + auto-keys behavior exactly
* [ ] `docs/faq.md` includes error codes table and common pitfalls

### API docs

* [ ] `cargo doc --open` has:

  * [ ] meaningful crate-level overview
  * [ ] public types have doc comments
  * [ ] examples compile (prefer doctests `no_run` where needed)
  * [ ] architecture + migration visible (via included docs module)

### Examples

* [ ] 4 example programs exist and build under the library package 
* [ ] CI runs each example once against generated fixtures

### Link hygiene

* [ ] CI runs a docs link checker and it passes 

---

If you want, I can also give you a **proposed table-of-contents + concrete section headings** for each `docs/*.md` file (so you can implement them almost mechanically), but the plan above is already aligned to your current CLI behavior, database mode heuristics, and error-code system.
