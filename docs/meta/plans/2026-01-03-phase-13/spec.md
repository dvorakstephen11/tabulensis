## Phase 13 implementation plan: eliminate remaining high‑risk edges and finish ship‑grade integration

Phase 13 (as written in `13_phase_plan.md`) is explicitly about removing the last “sharp edges” that could undermine trust at ship time: (1) permission bindings / DPAPI correctness, (2) clarifying and executing on missing container formats (example: XLSB), and (3) a final ship sweep across workflows, docs, and host parity so the integrated product matches the strength of the core. 
This aligns with the project’s own completion analysis: the core is strong, but these remaining edges + integrated maturity need to catch up before calling it “done.” 

Below is a deeply expanded, codebase‑grounded plan to execute Phase 13 in a way that is realistic for the current architecture and tooling.

---

# 0) Phase 13 goals, scope boundaries, and success definition

### Goals

1. **Correctness + clarity for Permission Bindings (DPAPI)**
   Implement actual parsing/validation logic and reflect Excel’s real behavior when bindings are present, including clear warnings/tests. 
2. **Decide and execute on missing formats (XLSB as the explicit example)**
   Either support, or explicitly detect + reject with a first‑class error experience across CLI/web/desktop, and wire it into regression + perf/RC gates. 
3. **Ship sweep**
   Ensure releases, docs, and host parity are consistent and polished so “integrated maturity catches up with strong core.”

### Non‑goals (to keep Phase 13 bounded)

* Re‑architecting the diff engine or redoing earlier phase work.
* Large schema redesigns (unless required for correctness / stability).
* Implementing full XLSB cell/formula parsing **unless** the decision is “XLSB supported in this release” (and the plan below makes that a clearly separate branch with explicit cost).

### Definition of “done”

* Permission bindings are no longer opaque “raw bytes only” behavior; the tool matches the documented semantics and emits actionable diagnostics when it cannot validate.
* XLSB and any other intentionally unsupported formats are rejected with a *precise* error code/message (not “missing workbook.xml”) and are covered by automated regression gates.
* Release workflow + docs + host behaviors are consistent and verified end‑to‑end (CI + UI tests + perf gates + wasm budgets).

---

# 1) Workstream A — Permission Bindings / DPAPI: real validation + clear warnings + tests

## A1. Current codebase reality (baseline)

### Where permission bindings exist today

* The DataMashup framing parser already extracts the **permission_bindings** segment into `RawDataMashup.permission_bindings`.
* `build_data_mashup()` currently **does not interpret** those bindings; it simply copies them into `DataMashup.permission_bindings_raw: Vec<u8>`.
* Permissions are parsed from `raw.permissions` via `parse_permissions(&raw.permissions)` (defaults on malformed/missing). Bindings do not participate in determining `DataMashup.permissions`.
* Tests today only verify “bindings present/nonempty” and “empty bindings doesn’t crash.” 

This matches the phase plan statement that bindings are “currently retained as raw bytes.” 

## A2. Target behavior grounded in spec/reality

Microsoft’s QDEFF spec describes Permission Bindings as a cryptographic checksum over **Package Parts** and **Permissions**, encrypted using DPAPI (scoped to current user) with optional entropy `"DataExplorer Package Components"`. ([Microsoft Learn][1])
It also states that if the checksum does not match, **the Permissions stream must be discarded and replaced with defaults**. ([Microsoft Learn][1])

A widely referenced deep dive explains the intent: “Saved permissions should only apply if the current user is the user that saved them and Package Parts/Permissions haven’t been tampered with; otherwise fall back to defaults (FirewallEnabled=true, WorkbookGroupType=null).” ([Ben Gribaudo][2])

### Implication for our codebase

Right now, `build_data_mashup()` can return permissions that Excel itself would ignore. That’s a correctness hole (and exactly the kind of “high‑risk edge” Phase 13 is meant to remove).

So Phase 13 should make `DataMashup.permissions` represent **effective permissions** consistent with Excel’s rules, not just “whatever was in the XML.”

## A3. Design: add a PermissionBindings validation layer that is safe, cross‑platform, and testable

### A3.1 Introduce an internal validation model

Add a new internal module in core (suggested location):

* `core/src/permission_bindings.rs` (or `core/src/datamashup_permission_bindings.rs`)

Core functions (conceptual API; not code yet):

* `classify_permission_bindings(raw: &[u8]) -> PermissionBindingsKind`

  * `Missing` (len == 0)
  * `NullByteSentinel` (len == 1 && raw[0] == 0x00)  *(spec allows this “special case”)* ([Microsoft Learn][1])
  * `DpapiEncryptedBlob` (everything else)
* `validate_permission_bindings(raw: &RawDataMashup, decryptor: &dyn DpapiDecryptor) -> PermissionBindingsValidation`

  * `Verified` (decrypt success + hashes match)
  * `InvalidOrTampered` (decrypt success + hashes mismatch or plaintext malformed)
  * `Unverifiable` (decrypt not available / decrypt failure)
  * `Disabled` (null byte sentinel)
  * `Missing` (empty)

And a single “effective permissions” decision function:

* `effective_permissions(raw: &RawDataMashup, parsed_permissions: Permissions, validation: &PermissionBindingsValidation) -> (Permissions, PermissionBindingsDecision)`

Where `PermissionBindingsDecision` captures *why* permissions are what they are:

* `UsedPersistedPermissions` (bindings disabled/missing/verified)
* `DefaultedDueToBindingsUnverifiable`
* `DefaultedDueToBindingsMismatchOrTamper`

Why this matters in your codebase:

* You currently have no warning channel in `build_data_mashup()`; returning only `Permissions::default()` loses the “why.”
* Capturing a structured “decision” lets the diff/report layer produce clear warnings without inventing brittle heuristics.

### A3.2 Add SHA‑256 hashing in core with minimal dependency risk

Spec requires SHA‑256 of `raw.package_parts` and `raw.permissions`. ([Microsoft Learn][1])

**Codebase reality:** `core/Cargo.toml` currently does not depend on a SHA‑256 implementation. 
So Phase 13 needs to add one. The simplest, most portable choice is `sha2` (pure Rust, wasm‑friendly).

Plan:

* Add `sha2` to `core/Cargo.toml`.
* Keep hashing code isolated to the new module so its impact is easy to measure and, if necessary, optimize.

Perf guardrails:

* Only compute hashes when we are actually in the DPAPI path (i.e., when bindings are not empty and not the null‑byte sentinel), because hashing potentially large `PackageParts` bytes adds a linear scan cost.

### A3.3 DPAPI decrypt strategy that fits your multi‑host reality

Your project targets:

* CLI (native)
* Desktop (Tauri, native)
* Web (WASM)
  …and already has workflows for wasm budgets and multi‑platform builds.

DPAPI is Windows‑specific. ([Ben Gribaudo][2])

So the plan should explicitly separate:

* **Cross‑platform behavior** (always available)
* **Windows‑only verification** (optional enhancement)

#### Cross‑platform baseline (required)

* If `permission_bindings` is DPAPI‑blob (not sentinel) and we cannot decrypt:

  * Treat bindings as **unverifiable**, and per spec, **discard permissions and use defaults**. ([Microsoft Learn][1])
  * Record a decision reason so hosts can warn: “DPAPI bindings present; cannot validate on this platform; permissions defaulted to Excel fallback.”

This makes Linux/macOS/web behavior honest and consistent.

#### Windows verification (recommended, but gated)

Implement a small Windows‑only decryptor behind `cfg(windows)` and a feature flag (e.g., `feature = "dpapi"`), using a light dependency such as `windows-sys` to call `CryptUnprotectData`.

* Use optional entropy: `"DataExplorer Package Components"` (UTF‑8) per spec. ([Microsoft Learn][1])
* Decrypted plaintext format per spec: two SHA‑256 hashes preceded by 4‑byte little‑endian lengths. ([Microsoft Learn][1])
* Compare decrypted hashes to computed hashes of `raw.package_parts` and `raw.permissions`.

Why feature‑gated:

* Core compiles to wasm; you do not want Windows deps pulled into wasm builds.
* You may want a “pure” core for non‑Windows.

## A4. Integrate into `build_data_mashup()` without destabilizing everything

### A4.1 Update `build_data_mashup()` to compute **effective** permissions

Right now, the function is:

* parse package parts
* parse permissions
* parse metadata
* store raw bindings


Phase 13 should change it to:

1. `package_parts = parse_package_parts(&raw.package_parts)?`
2. `persisted_permissions = parse_permissions(&raw.permissions)` (unchanged parsing behavior)
3. `validation = validate_permission_bindings(raw, decryptor_for_platform)`
4. `effective_permissions = apply_validation(persisted_permissions, validation)`
5. `metadata = parse_metadata(&raw.metadata)?`
6. Return `DataMashup { permissions: effective_permissions, permission_bindings_raw: raw.permission_bindings.clone(), ... }`

This keeps the core API surface the same (still returns `DataMashup`), but fixes correctness.

### A4.2 Decide how to expose diagnostics without breaking too much

You need “clear warnings” per Phase 13. 
But `build_data_mashup()` has no warnings channel.

Two viable approaches, grounded in the existing design:

**Option 1 (minimal API change; warnings only in diff/report paths):**

* Keep `DataMashup` struct unchanged.
* In diff flows (WorkbookPackage::diff and PbixPackage::diff), inspect `raw.permission_bindings` behavior only indirectly is hard because you don’t have the “decision reason.”
* Therefore, you’d still need some signal stored in `DataMashup` (see option 2).

**Option 2 (recommended; small, explicit addition to DataMashup):**

* Add a new field to `DataMashup`:

  * `pub permission_bindings_status: PermissionBindingsStatus`
  * (and potentially `pub permissions_source: PermissionsSource`)

This is the cleanest way to support warnings across CLI/web/desktop consistently, because those hosts can read this field when rendering “info” and “diff” views. It also avoids the problematic alternative of repurposing `DiffReport.warnings` (which currently implies `complete=false`).

Tradeoff:

* Adding a field to a public struct is technically a breaking change. `DataMashupError` is already `#[non_exhaustive]`, but `DataMashup` is not.
  If you want to minimize downstream breakage, Phase 13 can also mark `DataMashup` as `#[non_exhaustive]` going forward (still a breaking change, but it prevents repeat breakage later).

Given Phase 13 is a ship sweep, I’d treat this as an intentional “API stabilization” move: make the diagnostic state explicit now, so you don’t have to invent it later.

## A5. Warnings: where they should appear and what they should say

### A5.1 Warning channel choice

* For the main `excel_diff` UX, warnings should surface in:

  * CLI output (stderr) and JSON payloads
  * Web UI warnings section
  * Desktop UI warnings

You already have:

* `DiffReport { warnings: Vec<String>, complete: bool }` where `add_warning()` flips `complete=false`.
* CLI exit code behavior depends on `complete` and ops count (warnings can make exit code non‑zero). 

Because permission bindings issues mean “permissions may not be trustworthy,” it is defensible for diffs to be marked incomplete when bindings are unverifiable/mismatched (otherwise CI could miss meaningful differences). So using `DiffReport.warnings` is acceptable **if** you are deliberate.

Plan:

* Add warnings only when:

  * bindings are a DPAPI blob and decrypt is unavailable/fails, OR
  * decrypt succeeds but hashes mismatch, OR
  * decrypted plaintext format is invalid.

Don’t warn for:

* empty bindings
* null‑byte sentinel (unless you want a debug‑level note)

### A5.2 Warning content (actionable)

Standardize on messages that include:

* what was detected (DPAPI blob)
* what the tool did (defaulted permissions)
* what the user can do (re-save / run on same Windows user / expect defaults)

Example wording (conceptually):

* `[EXDIFF_DM_009] Permission bindings are DPAPI-encrypted and could not be validated on this platform; permissions have been defaulted to FirewallEnabled=true and WorkbookGroupType=null to match Excel fallback behavior.`

Implementation detail:

* Add `EXDIFF_DM_009` to `core/src/error_codes.rs` (DM currently ends at 008). 

## A6. Test plan: deterministic, cross‑platform, and covers real behaviors

You already have strong unit tests around:

* Raw framing parse behavior
* permissions parse defaulting on malformed/missing xml 
* fixture‑based DataMashup presence/behavior

Phase 13 tests should add **binding‑aware** coverage:

### A6.1 Pure unit tests (no DPAPI, runs everywhere)

Create synthetic `RawDataMashup` values (you already do this in tests).

Add tests for:

1. **Null‑byte sentinel means “do not enforce checksum”**

   * `permission_bindings = vec![0x00]`
   * `permissions = <FirewallEnabled>false</...>`
   * Expect `DataMashup.permissions.firewall_enabled == false` and `permissions_source == Persisted`
2. **Non‑empty DPAPI blob on non‑Windows (or when decryptor = None)**

   * `permission_bindings = vec![0x01,0x02,...]`
   * Provide non-default permissions XML
   * Expect `DataMashup.permissions == Permissions::default()`
   * Expect status `Unverifiable` (or `DefaultedDueToBindingsUnverifiable`)
3. **Malformed decrypted plaintext (test via injected “fake decryptor”)**

   * If you implement decrypt behind trait, add a test decryptor returning plaintext with wrong length prefixes.
   * Expect default permissions + warning reason “InvalidOrTampered”

### A6.2 Windows‑only integration test (optional, but ideal)

If you implement a real DPAPI decryptor:

* On Windows CI, generate ciphertext at runtime with `CryptProtectData` using the same entropy string and known plaintext (two hashes).
* Then validate that your decrypt path returns `Verified` and preserves permissions.

This avoids non‑deterministic fixture blobs (DPAPI output is user/machine dependent).

### A6.3 Fixture generator impact

Your fixture generator currently rewrites package parts + permissions + metadata but **preserves bindings bytes** from the base file (`bindings = ...` and reused in `_assemble_sections`).
That is *very likely* why your fixtures remain deterministic.

Phase 13 must ensure this doesn’t accidentally create “real DPAPI blobs that now mismatch” (which would cause permissions to always default on non‑Windows). Plan:

* Add a generator‑side assertion/logging mode to dump the bindings section length and first bytes for `templates/base_query.xlsx`.
* If it is sentinel `0x00`, we’re safe.
* If it is a DPAPI blob, then existing permissions fixtures are logically inconsistent with spec and must be re-based to use sentinel bindings (see below).

If base file uses DPAPI blob:

* Update templates to have sentinel binding (length 1, byte 0x00) so generated fixtures behave consistently cross‑platform.
* Add a dedicated “dpapi_blob_present” negative fixture to test the defaulting/warning behavior deterministically.

### A6.4 Wire into robustness regression suite

Your robustness regression harness is YAML-driven and expects either ok or a specific error code.
For bindings work, add a fixture that triggers “bindings unverifiable” and ensure:

* parsing succeeds (result ok),
* but report includes warnings in self-diff invariants? (If you choose to warn on open/diff, be careful: the suite checks `self_diff_empty`. )

Recommendation:

* Keep permission-bindings warnings scoped to diff of two different files, not self-diff, unless the self-diff contract explicitly allows warnings. Otherwise you’ll disrupt `self_diff_empty` invariants.

## A7. Documentation updates

Ship-grade means you must document behavior that surprises users:

* Add a doc section (README or `docs/`) explaining:

  * What Permissions are (FirewallEnabled, WorkbookGroupType) and that they are guarded by Permission Bindings
  * What happens when bindings can’t be validated (defaults)
  * Why you might see different results on Windows vs wasm/web
* This should be referenced from CLI `--help` or “limitations” docs.

---

# 2) Workstream B — Missing container formats: decide + execute (XLSB example)

## B1. Current supported input formats in codebase (reality check)

Across hosts, supported extensions are explicitly enumerated today:

* Web UI `<input type="file" accept="...">` accepts: `xlsx,xlsm,xltx,xltm,pbix,pbit` (no xlsb). 
* CLI’s and UI-payload’s host-kind detection similarly whitelists: `.xlsx .xlsm .xltx .xltm .pbix .pbit`.
* `PackageError::WorkbookXmlMissing` / `MissingPart` are used when `xl/workbook.xml` isn’t present in an OPC container.

So today, if a user feeds an XLSB-like OPC zip to core APIs, they may see a confusing “missing workbook.xml” error rather than “XLSB unsupported.”

Phase 13 calls out XLSB as a missing format to explicitly decide on and handle.

## B2. Decision checkpoint (explicit, written outcome)

Before implementing, Phase 13 should produce a simple “Supported formats policy” decision doc (can live in `docs/` and summarized in README):

* **Supported now**: `.xlsx .xlsm .xltx .xltm .pbix .pbit`
* **Explicitly unsupported (with friendly error)**: `.xlsb` (and optionally `.xls`/others if you want)
* **Rationale**:

  * XLSB uses binary workbook parts (`xl/workbook.bin`, `xl/worksheets/sheet1.bin`) and requires a separate parser; implementing it is not a small “container wiring” task.

This is what “decide and execute” should mean in Phase 13: users don’t discover support status by trial/error.

## B3. Execute option (recommended): explicit XLSB detection + first-class unsupported error

This is the “high value / low risk” path for Phase 13.

### B3.1 Core: detect XLSB signature inside OPC container

Where to implement:

* In `core/src/excel_open_xml.rs`, inside `open_workbook_from_container` or immediately around where `xl/workbook.xml` is loaded.

Behavior:

* When `xl/workbook.xml` is missing:

  * check if `xl/workbook.bin` exists
  * if yes: return `PackageError::UnsupportedFormat { message: "XLSB detected (xl/workbook.bin present); convert to .xlsx/.xlsm" }`
  * else: keep existing `WorkbookXmlMissing` / `MissingPart` behavior.

This ensures:

* Existing missing-part tests stay valid.
* XLSB yields a specific, user-actionable error with the correct error code (`EXDIFF_PKG_009`).

### B3.2 Host parity: ensure all entry points surface the same error

Update:

* CLI host detection: you can keep `.xlsb` excluded (so file chooser doesn’t suggest it), but you must still handle “user passed xlsb path anyway.” 
* ui_payload host detection (web/desktop) similarly: it currently rejects unknown extensions; keep that behavior *or* add `.xlsb` and map to “Workbook” but let core emit unsupported. 

  * If you keep rejecting `.xlsb` at host detection, error is “unsupported file extension” before core sees bytes.
  * If you accept and let core reject, you get the richer “XLSB detected” error.

Recommendation for best UX:

* Accept `.xlsb` as “Workbook-like container” in host detection and let core return the specific unsupported-format error. That keeps errors consistent across “load by extension” and “load by sniffed content.”

Also update web UI:

* Either keep accept list as is (signals unsupported), but ensure drag/drop or programmatic loading surfaces the clear error.
* Or add xlsb to accept list only if you want to allow selection and show explicit unsupported message. 

## B4. Tests and regression wiring (RC/perf gates)

### B4.1 Add a deterministic XLSB-like fixture

You don’t need a real Excel-authored xlsb to test detection. Create a minimal OPC zip that contains:

* `[Content_Types].xml`
* `_rels/.rels`
* `xl/workbook.bin` (can be dummy bytes)
  …and intentionally does **not** contain `xl/workbook.xml`.

Then:

* Add it to fixtures (generated or checked-in).
* Add a test asserting it errors as `EXDIFF_PKG_009` (UnsupportedFormat), not `EXDIFF_PKG_003`.

### B4.2 Add to `robustness_regressions.yaml`

Add an entry:

* file: `xlsb_stub.xlsb` (or `.xlsx` if your harness keys off “type”; better to add a new `FixtureKind::Xlsb` if you want)
* expectation: error, `EXDIFF_PKG_009`

The robustness suite already checks error codes and deterministic behavior.

### B4.3 Perf/RC gates

Even for “unsupported” formats, Phase 13 should ensure:

* No perf regression from format sniffing (it should be O(1) existence checks in a zip index, not scanning huge parts).
* The change doesn’t break wasm budgets (it shouldn’t, unless you add heavy deps). wasm budgets workflow exists. 
* CI runs these checks consistently (see Workstream C).

## B5. Alternate execute option (only if you decide “XLSB supported now”)

If the decision is to actually support XLSB, Phase 13 needs a much larger plan:

* Implement a BIFF12 parser for workbook.bin and sheetX.bin
* Support shared strings / styles / formulas in a binary record format
* Update diff engine to work with both XML and BIN representations
* Ensure wasm budgets remain in bounds

Given Phase 13 is positioned as “ship sweep / eliminate remaining edges,” this is likely not the intended scope unless XLSB support is a hard requirement.

---

# 3) Workstream C — Final ship sweep: workflows, docs, and consistent behavior across hosts

This is where Phase 13 turns the strong core into a “product you can trust” across CLI/web/desktop.

## C1. Release workflow hardening

### C1.1 Reality: release workflow exists and is substantial

You already have a `release.yml` that:

* validates tag format and versions
* builds cross-platform artifacts
* generates checksums
* updates Homebrew/Scoop metadata
* attaches release body info

Phase 13 should ensure the new Phase-13 behaviors are reflected in release processes:

### C1.2 Add explicit “format support / known limitations” to release notes template

Your release workflow writes release body content (it references demo links etc).
Update that template to include:

* Supported formats list
* Explicit “XLSB unsupported” (or “supported”) statement
* Permission bindings caveat: “Permissions may be defaulted when DPAPI bindings cannot be validated.”

### C1.3 Ensure verify_release_versions includes *all shipped crates*

You have a version verification script used by release workflow.
Phase 13 should confirm it checks:

* `core`
* `cli`
* `desktop` (if shipped)
* `wasm`/web crates (if versioned/shipped)
* `ui_payload`

If any are excluded today, add them so releases can’t silently ship mismatched versions.

## C2. CI and host parity sweep

### C2.1 Reality: CI currently runs tests on Ubuntu only

Your CI workflow `ci.yml` is Ubuntu-only for tests. 
But Phase 13 introduces Windows-specific behavior (DPAPI optional) and needs parity across hosts. The plan should upgrade CI coverage accordingly.

### C2.2 Add OS matrix for core tests (targeted)

At minimum:

* `ubuntu-latest` (status quo)
* `windows-latest` (to exercise DPAPI feature if enabled, plus Windows path edge cases)
* `macos-latest` (filesystem/path + packaging parity)

Even if you don’t run *all* suites on all OSes, add a “smoke test” matrix that runs:

* `cargo test -p excel_diff` (core)
* `cargo test -p excel_diff_cli` (CLI)
* optionally `cargo test -p excel_diff_desktop` if it has tests

This is directly in service of “consistent behavior across hosts.”

### C2.3 Web/CLI parity: keep `web_ui_tests.yml` green

You already have a workflow that generates UI fixtures and runs web UI tests to ensure payload compatibility. 
Phase 13 changes (warnings, format detection errors) may affect those fixtures:

* If a fixture now triggers a warning (e.g., DPAPI binding defaults), update expected snapshots or fixture manifests accordingly.
* Ensure error payloads for unsupported formats are stable and human‑readable.

## C3. Documentation sweep (ship-grade)

### C3.1 Update format support docs in README + web UI

* The web demo currently advertises accepted formats via file input accept list. 
* The CLI likely documents input types; ensure it matches host detection logic.
* Add a single source-of-truth doc section:

  * Formats supported
  * Formats intentionally unsupported with recommended workaround (“Save as .xlsx”)
  * PBIX/PBIT nuance (DataMashup vs enhanced metadata), since `PackageError::NoDataMashupUseTabularModel` already encodes this guidance. 

### C3.2 Document warning semantics and exit codes

Because warnings can make `complete=false` and affect exit codes, it must be documented clearly for CI users.
Add a short doc:

* Exit 0: no ops + complete
* Exit 1: changes OR incomplete (warnings/limits/unsupported subcomponents)
* Exit 2: fatal error opening/parsing

This becomes more important if Phase 13 adds new warnings for DPAPI bindings.

## C4. “Final ship checklist” gate (do not rely on memory)

Create a single checklist doc (and optionally a CI job) that verifies:

1. **Permission bindings**

   * sentinel 0x00 path works
   * DPAPI-unverifiable path defaults + warns
   * (Windows) DPAPI-verified path preserves permissions (if implemented)
2. **XLSB**

   * rejected with EXDIFF_PKG_009 + clear message
   * web/desktop/cli all show consistent error
3. **Regression gates**

   * robustness regressions suite passes
   * perf thresholds pass
   * wasm budgets pass 
   * web UI tests pass 
4. **Release**

   * `release.yml` dry run produces artifacts + checksums
   * release notes include supported formats + limitations 

---

# 4) Deliverables produced by Phase 13

### Code deliverables

* `core`: permission bindings validation module + SHA‑256 dependency, integrated into `build_data_mashup()` (and optionally Windows DPAPI decrypt).
* `core`: XLSB detection and unsupported-format error in the Excel package open path.
* `cli/ui_payload/web/desktop`: consistent format handling and error surfacing.

### Test + gate deliverables

* New unit tests for permission-bindings decision logic.
* New fixture + regression entry for XLSB unsupported detection. 
* CI expansion for OS parity (at least smoke coverage). 

### Documentation + release deliverables

* Updated docs for supported formats, limitations, and warning/exit semantics.
* Release notes template updated to include Phase 13 edge behavior. 

---

# 5) Practical execution order (to minimize churn)

A realistic order that reduces rework:

1. **Permission bindings core logic first**

   * implement classification + “effective permissions” decision
   * add tests using synthetic RawDataMashup
   * integrate into build_data_mashup
   * only then decide whether to add Windows DPAPI decrypt

2. **XLSB explicit detection**

   * implement detection in core open path
   * add stub fixture + regression yaml entry
   * update hosts to surface same error

3. **Ship sweep**

   * update docs + release notes template
   * expand CI matrix where needed
   * run full gates (robustness + perf + wasm budgets + web UI tests)

This mirrors the intent that Phase 13 is about the last high-risk edges and final integration polish.

[1]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-qdeff/d0959ba8-ac8d-4bee-bb58-9a869d7b226a "[MS-QDEFF]: Permission Bindings | Microsoft Learn"
[2]: https://bengribaudo.com/blog/2020/04/22/5198/data-mashup-binary-stream "The Data Mashup Binary Stream: How Power Queries Are Stored | Ben Gribaudo"
