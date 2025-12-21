Below is a concrete, repo-aware implementation plan for **Branch 7: Documentation & Polish** as defined in `next_sprint_plan.md` , tailored to the actual workspace structure and APIs described in `codebase_context.md` (workspace crates, exports, CLI flags, DiffConfig fields, etc.) .

---

## What Branch 7 must deliver

Branch 7 explicitly asks for three buckets of work :

1. **7.1 User Documentation**

   * README quick start
   * CLI reference
   * Configuration guide (DiffConfig)
   * Git integration tutorial
   * Database mode guide with examples
   * FAQ

2. **7.2 API Documentation**

   * Complete rustdoc for all public types
   * Doc-comment examples
   * Architecture overview document
   * Migration guide from old APIs

3. **7.3 Example Programs**

   * `basic_diff.rs`
   * `streaming.rs`
   * `database_mode.rs`
   * `custom_config.rs`

Acceptance criteria: the README is genuinely “getting started”-quality, `cargo doc --open` is useful, examples compile/run, and docs don’t have broken links .

---

## Repo reality check (important for where things live)

### Workspace layout affects “examples/…”

The root `Cargo.toml` is **workspace-only** (`members = ["core", "cli", "wasm"]`) with no root package . That means a top-level `examples/basic_diff.rs` (at repo root) will **not** be runnable via Cargo in the normal way.

**Plan adaptation:** place the examples under `core/examples/` (so they build as examples of the `excel_diff` crate). This still satisfies the intent of Branch 7’s “example programs compile/run” requirement .

### What should documentation reflect?

The codebase already has:

* A full CLI (`excel-diff`) with `diff` and `info` commands, output formats (json/jsonl/text/git-diff), and hardening flags like `--max-memory`, `--timeout`, and `--progress` .
* A public library surface exporting `WorkbookPackage`, `DiffConfig`, `DiffReport`, streaming sinks, DataMashup parsing, formula parsing, etc.
* `DiffConfig` is fairly rich and has a builder plus “preset” constructors and hardening fields (`max_memory_mb`, `timeout_seconds`) .
* The crate-level Quick Start exists but is marked `ignore` right now, so it doesn’t even compile-check in docs .

So Branch 7 is mostly: **organize, explain, and put examples right next to the public APIs they use**.

---

## Implementation plan

### Phase 0: Doc inventory + “what’s public?” checklist (1 pass before writing)

Goal: make sure we don’t miss rustdoc on exported items.

1. **Generate an export inventory from `core/src/lib.rs`**

   * Everything in the `pub use …` block is “public surface area” that Branch 7 promises to document .
   * Make a short internal checklist grouped by theme:

     * High-level: `WorkbookPackage`, `DiffConfig`, `DiffReport`, `DiffOp`, `DiffError`
     * Streaming: `DiffSink`, `JsonLinesSink`, `VecSink`, `CallbackSink`
     * Progress/hardening: `ProgressCallback`, `NoProgress`
     * DataMashup / M: `DataMashup`, `Query`, `QueryMetadata`, M parse/canonicalize functions
     * Workbook IR: `Workbook`, `Sheet`, `Grid`, `CellValue`, etc.
     * Utilities: addressing helpers, `suggest_key_columns`
2. **Open `cargo doc` locally and identify “blank pages”**

   * Run: `cargo doc -p excel_diff --no-deps` and click around.
   * Specifically check: `WorkbookPackage` and `JsonLinesSink` (both are exported but currently have no docs in the snippet) .
3. **Decide doc placement strategy**

   * Rustdoc should cover API usage and guarantees.
   * Longer narrative docs go into `/docs/*.md` and are linked from README.

This inventory prevents the classic “docs are great but the main entry types are undocumented” failure mode.

---

## 7.1 User documentation

### 7.1.1 README quick start (repo-root `README.md`)

Even if a README exists today, Branch 7 wants a “quick start guide” level README .

**Changes/structure to implement:**

1. **Top section: what it does**

   * One paragraph: compares two `.xlsx` workbooks and emits a structured diff (cells, sheet structure, charts/named ranges/VBA, Power Query M changes).
   * Mention the two main entry points:

     * CLI (`excel-diff`)
     * Library (`excel_diff` crate / `WorkbookPackage`)

2. **Install section**

   * Keep it simple and consistent with the repo’s release workflows (Windows binary + brew formula + web demo exist per workflows) .
   * Minimal but practical options:

     * From source: `cargo install -p excel_diff_cli --locked`
     * From releases: (link to GitHub releases)
     * Web demo: (link to GitHub Pages)

3. **CLI quick start section**

   * 3 “copy/paste” commands:

     * Text summary diff
     * JSON output
     * Git-diff output mode
   * Make sure it matches real CLI behavior, including:

     * `--git-diff` cannot be combined with `--format json|jsonl` .
     * `--fast` and `--precise` are mutually exclusive .

4. **Library quick start section**

   * Minimal snippet using `WorkbookPackage::open` (takes a reader) and `diff` .
   * Mention that for large workbooks you should use streaming (`diff_streaming`) and/or set `max_memory_mb` / `timeout_seconds` in config .

5. **Docs index**

   * Link to `/docs/cli.md`, `/docs/config.md`, `/docs/git.md`, `/docs/database_mode.md`, `/docs/faq.md`, `/docs/architecture.md`, `/docs/migration.md`.

**Success criteria for this README:**

* A new user can install and run a diff in <5 minutes.
* Every link is relative (except the web demo / releases), so internal links are robust.

---

### 7.1.2 Add `/docs/` with an explicit navigation page

Create:

* `docs/index.md` (or `docs/README.md`)

  * Short overview + links to each guide
  * A consistent “audience” note:

    * “If you want Git integration, start here…”
    * “If you want to embed this in Rust code, start here…”

This reduces “where do I look?” churn.

---

### 7.1.3 CLI reference documentation (`docs/cli.md`)

This should mirror what clap defines today (not what the sprint plan originally imagined).

From `cli/src/commands/diff.rs`, the real `diff` command supports:

* Output format options including `json`, `jsonl`, `text`, plus `--git-diff` mode .
* Preset flags: `--fast`, `--precise` (mutually exclusive) .
* Verbosity: `--quiet`, `--verbose` .
* Database mode flags: `--database`, `--sheet`, `--keys`, `--auto-keys` (with validation rules) .
* Hardening: `--progress`, `--max-memory`, `--timeout` .

**Implementation tasks:**

1. Document each command:

   * `excel-diff diff <OLD> <NEW>`
   * `excel-diff info <FILE>`
2. For `diff`, document:

   * Output formats and when to use each (text vs json vs jsonl vs git-diff).
   * Behavior of `--git-diff` constraints (no JSON formats) .
   * Presets: what “fast” changes and what “precise” changes (tie to DiffConfig presets in config guide).
   * Hardening flags and what they do to output (warnings + `complete=false` semantics).
3. For `info`, document:

   * It prints a stable textual representation suitable for git `textconv` (based on existing git integration tests) .
4. Add “Exit codes / failures”

   * Brief mapping: argument validation errors vs parsing errors vs diff errors.

**Optional (but strongly recommended):**

* Add a tiny script or hidden CLI subcommand to print markdown help from clap to avoid drift.

  * If you do this, document the regeneration step in `docs/cli.md` (“run X to regenerate”).

---

### 7.1.4 Configuration guide (`docs/config.md`)

This guide should be written around the actual `DiffConfig` API and builder.

Key facts to capture:

* `DiffConfig` exists with defaults, presets (`fastest`, `most_precise`), and a builder with validation .
* It contains hardening knobs:

  * `max_memory_mb: Option<u32>`
  * `timeout_seconds: Option<u32>` .
* It contains “limit behavior” (fallback-to-positional vs abort) .
* It contains knobs for move detection and semantic M/formula diffing (enable flags are visible in defaults test) .

**Concrete doc structure:**

1. “How diffing works at a high level”

   * In plain terms: alignment, move detection, cell edit reporting, plus object diffs and M diffs.
2. “Presets”

   * `DiffConfig::fastest()` vs `DiffConfig::most_precise()`
   * Map these to CLI flags `--fast` / `--precise` .
3. “Safety + robustness”

   * Timeouts and memory budgets: what happens when exceeded (partial results + warnings + fallback) .
4. “Key options you actually tune”

   * Move detection enablement + thresholds
   * Include unchanged cells + context rows (if present)
   * Semantic diff toggles for M and formulas (as surfaced in defaults) .
5. “When to use database mode”

   * Short teaser linking to database mode guide.

Include at least 2 mini-recipes:

* “Large file, want guaranteed finish”: set timeout + memory limit.
* “I care about correctness above all”: use most_precise preset and increase align limits.

---

### 7.1.5 Git integration tutorial (`docs/git.md`)

The codebase clearly supports:

* A `--git-diff` output mode intended for `git diff`-style consumers (and has specific constraints) .
* A textual `info` command used successfully as `diff.xlsx.textconv` in tests .

**Doc should include two setups:**

1. **Textconv (always works; shows a stable “view” of a single workbook)**

   * `.gitattributes`: map `*.xlsx diff=xlsx`
   * `.gitconfig`: `diff.xlsx.textconv = excel-diff info` (and mark binary)

2. **True workbook-vs-workbook diff (uses excel-diff diff)**

   * A `difftool` approach (recommended), because `git diff` driver integration can be trickier across platforms.
   * Show how to invoke `excel-diff diff --git-diff "$LOCAL" "$REMOTE"`.

**Edge cases to document:**

* `--git-diff` can’t emit json/jsonl .
* Binary `.xlsx` handling and why textconv is useful.
* Performance tip: for big workbooks, set global max-memory/timeout via git alias wrappers.

---

### 7.1.6 Database mode guide (`docs/database_mode.md`)

Database mode exists both in CLI and library:

* CLI has `--database`, `--sheet`, `--keys` and `--auto-keys` with validation and default sheet selection heuristics (prefers “Data”, else single-sheet, else requires explicit) .
* Library has `WorkbookPackage::diff_database_mode(...)` and streaming variants .
* There’s also `suggest_key_columns` exported for auto-key suggestions .

**Doc structure:**

1. What database mode is (key-based alignment)
2. When it’s the right tool (tables with stable primary keys; rows reorder frequently)
3. CLI usage

   * `--database` requirement
   * `--keys` format (e.g., `A,C` column letters) and how it is interpreted (column indices) .
   * `--auto-keys` behavior and how it picks keys (ties to `suggest_key_columns`) .
   * Sheet selection rules (Data, single-sheet, else require `--sheet`) .
4. Library usage

   * `pkg.diff_database_mode(other, sheet_name, key_columns, config)`
   * Streaming variant for huge tables
5. Troubleshooting section

   * Duplicate keys
   * Missing sheet
   * Why positional fallback might occur if limits exceeded (link to config guide)

Include at least one “realistic” example scenario and show expected output shape (text summary + some representative ops).

---

### 7.1.7 FAQ (`docs/faq.md`)

This is where you de-risk adoption with quick answers.

Include entries that align with the actual behavior:

* “Why does it say results incomplete?” -> `DiffReport.complete == false` + warnings when timeouts/memory or other hardening triggers happen .
* “How do I diff huge files?” -> streaming + memory/timeouts + jsonl output
* “Why are there no ‘deep chart diffs’?” -> chart ops are add/remove/change (hash-based) and are intentionally shallow (Branch 4 notes limitation) .
* “How do I interpret error codes?” -> point to `error_codes` module and the code prefixing in errors .
* “Can I use this in the browser?” -> wasm/web demo exists; point to web folder and wasm crate.

---

## 7.2 API documentation (rustdoc + deeper docs)

### 7.2.1 Fix crate-level docs first (`core/src/lib.rs`)

Right now the Quick Start is an `ignore` block . That means:

* It doesn’t compile-check.
* It won’t be copied into docs.rs in a useful, trustworthy way.

**Tasks:**

1. Change Quick Start to a compiling snippet:

   * Use `no_run` and wrap with a `fn main() -> Result<(), Box<dyn std::error::Error>>`.
   * Use `WorkbookPackage::open(File::open(...)? )` (since `open` takes a reader) .
2. Add a second example: streaming to `JsonLinesSink` (because it’s a core differentiator and already a public type) .
3. Add a third example (short) showing database mode call.

This one change alone goes a long way toward meeting “cargo doc --open produces useful docs” .

---

### 7.2.2 Document `WorkbookPackage` and “the right entry points”

`WorkbookPackage` is clearly intended as the main high-level API (it wraps workbook, M queries, and VBA modules) , but it currently lacks doc comments in the snippet.

**Doc content to add (at type-level + key methods):**

1. **Type-level (`///`) docs**

   * What it contains: parsed workbook IR + optional DataMashup + optional VBA modules .
   * What it’s for: diffing two packages and producing ops + resolved strings.
2. **`open`**

   * Reader-based API; feature-gated on `excel-open-xml` .
3. **`diff` vs `diff_streaming`**

   * `diff` returns `DiffReport` (ops + strings)
   * `diff_streaming` emits ops into a sink and returns `DiffSummary` (memory-friendly) .
4. **Object diffs + M diffs are included**

   * Explain (briefly) that diff includes named ranges, charts, VBA modules, and M query ops in addition to cell/grid ops .
5. **Database mode methods**

   * Explain key columns, sheet selection responsibility, and expected failure modes.

Also add 2–3 short doc examples directly on the relevant methods (`diff_streaming`, `diff_database_mode`).

---

### 7.2.3 Document the streaming surface area: `DiffSink` and `JsonLinesSink`

`DiffSink` is a key trait for scaling, and `JsonLinesSink` is the canonical sink type exported publicly .

**Tasks:**

1. Add rustdoc to `JsonLinesSink`:

   * Explain file format: header line includes schema version and strings, then ops per line.
   * Mention that it uses the string pool header and ops refer to `StringId`s.
2. Add “how to implement a sink” guidance on `DiffSink`:

   * Call order: begin -> emit* -> finish.
   * How errors are handled.
   * Mention the library tries hard to call `finish()` once even on error (as enforced by tests) .

---

### 7.2.4 Document `DiffReport`, `DiffSummary`, and “incomplete results”

This is crucial for user trust.

You already have semantics in engine code:

* When errors happen in non-try APIs, they produce `complete=false` and a warning string in report/summary .
* Hardening controller adds warnings when timeout/memory triggers .

**Tasks:**

1. Ensure `DiffReport` docs explicitly explain:

   * `complete` meaning
   * `warnings` meaning
   * schema version + `strings` vector is required to resolve ids
2. Ensure `DiffSummary` docs explain:

   * `op_count`
   * `complete` / `warnings`
   * metrics field when feature enabled
3. Add a short “how to resolve ids” snippet (e.g., `report.resolve(StringId)` if exists, or use `report.strings[id.0 as usize]` pattern used in tests) .

---

### 7.2.5 Architecture overview document (`docs/architecture.md`)

This is a narrative doc, not rustdoc.

**Recommended content (high level, with a diagram):**

1. Inputs

   * `.xlsx` is a ZIP/OPC container; parsed with limits for safety (container limits exist) .
2. Parsing pipeline

   * Open container -> parse workbook parts -> build workbook IR (`Workbook`, `Sheet`, `Grid`) .
   * Parse DataMashup (Power Query) into `DataMashup`, `Query` structures when present .
3. Diff pipeline

   * Grid diff (alignment + move detection + cell edits)
   * Object diffs (named ranges/charts/VBA)
   * M diffs (query add/remove/rename/semantic vs formatting)
4. Output pipeline

   * In-memory report vs streaming sinks
   * JSON vs JSONL

Keep it understandable, but concrete enough that contributors can navigate the modules (`diff`, `engine`, `grid_parser`, `m_diff`, `object_diff`, etc.) .

---

### 7.2.6 Migration guide (`docs/migration.md`)

`core/src/lib.rs` contains deprecated legacy entry points (`diff_workbooks`, `try_diff_workbooks`, `open_workbook`) . Branch 7 wants a migration guide.

**Guide sections:**

1. “Old API -> New API” mapping table (written as prose + code blocks)

   * `open_workbook(path) -> WorkbookPackage::open(File::open(path)?)`
   * `diff_workbooks(old, new, cfg) -> pkg_a.diff(&pkg_b, cfg)`
   * `try_diff_workbooks(...) -> pkg_a.diff_streaming(...)` for more controlled error handling
2. String pool/session notes

   * Explain that `WorkbookPackage` uses a default session internally (thread-local) .
   * Point power users to `advanced` module for manual session/pool management .
3. Streaming recommendations
4. Database mode changes
5. Feature flags notes (what defaults include)

---

## 7.3 Example programs

### Placement decision

Put them in `core/examples/` because root is not a crate .

### Example 1: `core/examples/basic_diff.rs`

Purpose: minimal file comparison.

**Behavior:**

* Accept two paths from args.
* Open with `WorkbookPackage::open(File::open(...))`
* Run `diff` with default config.
* Print:

  * `complete`, warnings count
  * number of ops
  * optionally first N ops in debug

APIs involved: `WorkbookPackage`, `DiffConfig`, `DiffReport` .

---

### Example 2: `core/examples/streaming.rs`

Purpose: show large file handling.

**Behavior:**

* Accept two paths.
* Use `JsonLinesSink<std::io::StdoutLock>` and `diff_streaming` .
* Print summary to stderr: op_count, complete, warnings.

Why this is valuable: it demonstrates the “scale path” that avoids holding ops in memory.

---

### Example 3: `core/examples/database_mode.rs`

Purpose: key-based diffing.

**Behavior:**

* Args: `old_path new_path sheet_name keys`

  * keys example: `A,C` or `0,2` (pick one format and be explicit)
* Convert keys to `Vec<u32>`
* Call `diff_database_mode`
* Print summary + a few representative diffs

Tie-in: mention `suggest_key_columns` in output if keys aren’t provided (optional) .

---

### Example 4: `core/examples/custom_config.rs`

Purpose: show config tuning.

Use `DiffConfigBuilder` or preset + overrides:

* Example “fast”: `DiffConfig::fastest()` (or builder equivalent)
* Example “budgeted”: set `max_memory_mb` + `timeout_seconds` .
* Run a diff and print whether it completed.

---

### “Examples compile and run” enforcement

Because the existing CI runs `cargo test --workspace` , it may not guarantee examples are built.

**Plan change:** update CI to compile examples explicitly:

* Add a step:

  * `cargo build --workspace --examples`
  * or `cargo test --workspace --examples` (either is fine; build is faster)

This directly supports Branch 7 acceptance criteria .

---

## Broken links prevention (docs QA)

Branch 7 requires “no broken links” . Do this pragmatically:

1. Prefer **relative links** between `README.md` and `/docs/*.md`.
2. Add a small “Docs sanity” checklist to PR template or CONTRIBUTING notes (optional).
3. Optional but useful: add a lightweight link checker in CI (e.g., `lychee`) configured to:

   * Always check internal links
   * Only warn (not fail) on flaky external links, or restrict to internal only

Even without a tool, relative links + `cargo doc` verification catches most issues.

---

## Final verification checklist (Definition of Done)

When Branch 7 work is complete, you should be able to run:

```bash
cargo doc --workspace --no-deps
```

and see:

* Crate docs show a compiling Quick Start (not ignored) 
* `WorkbookPackage`, `DiffConfig`, `DiffReport`, `DiffSink`, `JsonLinesSink` have meaningful docs

Also:

```bash
cargo build --workspace --examples
```

and:

```bash
cargo run -p excel_diff --example basic_diff -- old.xlsx new.xlsx
cargo run -p excel_diff --example streaming -- old.xlsx new.xlsx > out.jsonl
cargo run -p excel_diff --example database_mode -- old.xlsx new.xlsx Data A,C
cargo run -p excel_diff --example custom_config -- old.xlsx new.xlsx
```

And the README links to every guide in `/docs/` and those pages link back appropriately.

---

If you want, I can also turn this plan into a concrete “Branch 7 task list” organized as a sequence of small PRs (README/docs first, then rustdoc sweep, then examples + CI hook), but the steps above already map 1:1 to the Branch 7 deliverables and the codebase’s actual public surface and CLI behavior.
