# Implementation Plan: Pro Context Payload Generator + CI Signing/Notarization

Run date: 2026-02-09

Scope: implement two “coding agent” action items:

1. **Pro context payload generator** (per `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md:260` and `docs/meta/prompts/pro_context_payload.md`).
2. **Wire signing + notarization into the release pipeline** (per `docs/release_signing.md` + `.github/workflows/release.yml`).

This plan is written to be executed by a coding agent with access to the repo.

## Constraints / Guardrails

- Do not commit secrets (certs, key material, tokens, API keys). Only add placeholders and document required secret names/paths.
- Keep changes targeted (avoid repo-wide churn).
- `docs/index.md` is the canonical docs entrypoint; if new operating docs/checklists are added, link them there.
- Prefer deterministic outputs (stable ordering, bounded size, explicit truncation markers).

## Decision Gates (Must Resolve Before “Final”)

### DR-0007: Context Payload Output Location

Status: pending in `docs/meta/results/decision_register.md`.

The generator should support writing anywhere via `--out`, but we still need a default.

Options (from checklist):
- Local-only default: `tmp/pro_context_payload.md`
- Committed default: `docs/meta/results/pro_context_payloads/YYYY-MM-DD_HHMMSS_pro_context_payload.md`

Recommended approach for implementation:
- Implement generator with **explicit `--out`** and a **safe local-only default** (write under `tmp/`).
- Once `DR-0007` is chosen, update the default and reconcile docs/checklist.

## Part A: Pro Context Payload Generator

### Goal

Produce one deterministic, bounded markdown file that can be pasted into a large-context model for planning/review, matching the MUST requirements in `docs/meta/prompts/pro_context_payload.md`.

### Non-goals

- Do not attempt to automatically read vendor dashboards yet (that’s `vendor_snapshot` workstream).
- Do not include private corpora or binary artifacts.
- Do not “summarize” the included SOPs; the point is verbatim context.

### Success Criteria (Acceptance)

- A single command produces a payload file that:
  - Contains the required sections (in the spec’s order).
  - Includes `AGENTS.md`, `docs/index.md`, and `meta_methodology.md` verbatim.
  - Concatenates operating docs in the same order as `docs/index.md`’s “Operating docs” list.
  - Includes last N daily logs (bounded).
  - Includes a deterministic repo manifest and tree summary.
  - Enforces `--max-chars` with explicit per-section truncation markers.
  - Reports total chars + token estimate (`chars/4`) and per-section counts.
- Running the command twice without repo changes yields the same output byte-for-byte (except timestamps).

### Implementation Strategy

Prefer a new script under `scripts/` rather than adding complexity to the large `docs/meta/prompts/generate_review_context.py`.

Create:
- `scripts/generate_pro_context_payload.py`

Rationale:
- Small, dependency-free script is easier to audit and less likely to regress unrelated “review context” tooling.
- We can reuse patterns from `scripts/generate_daily_plan_context.py` (bounded writer, stable tree) without importing it.

### CLI Interface (Proposed)

```text
python3 scripts/generate_pro_context_payload.py \
  --out tmp/pro_context_payload.md \
  --max-chars 800000 \
  --daily-logs 3 \
  --ops-logs 0 \
  --vendor-snapshot
```

Flags:
- `--out <path>`: output file location (required for now, or default to `tmp/pro_context_payload.md`).
- `--copy`: copy output file *contents* to clipboard (best-effort; same clipboard logic as other scripts).
- `--max-chars <int>`: hard budget (default 800000). When exhausted, truncate current section and stop.
- `--daily-logs N`: include last N daily logs (default 3).
- `--ops-logs N`: include last N ops logs (default 0; optional).
- `--vendor-snapshot`: include vendor snapshot placeholder section (even if empty placeholders).
- `--include-path <path>` / `--exclude-path <path>`: optional extra files to include or paths to skip.

### Output Format

Follow `docs/meta/prompts/pro_context_payload.md` exactly:
- `# Tabulensis Pro Context Payload`
- `## Generation metadata` (timestamp, repo root, git branch/commit, generator version, chars + token estimate)
- Then required sections in the specified order.

When embedding files:
- Use `### File: <repo-relative-path>`
- Use **four-backtick fenced blocks** so source markdown containing triple backticks remains safe.

Language tag mapping:
- `.md` -> `markdown`
- `.rs` -> `rust`
- `.py` -> `python`
- `.toml` -> `toml`
- `.yaml` / `.yml` -> `yaml`
- `.json` -> `json`
- `.csv` -> `csv`
- default -> `text`

### Deterministic SOP Concatenation Order

Implementation approach:
1. Parse `docs/index.md` and extract the bullet list under “Operating docs (SOPs, runbooks, checklists)”.
2. Resolve each link to a repo-relative path.
3. Concatenate files in that exact order.
4. Skip duplicates already included earlier (example: `meta_methodology.md`).
5. If a listed file is missing, include a stub “missing file” section to preserve ordering.

Do not “hardcode” the list in the script; the spec’s list is documentation, not the canonical ordering source.

### Repo Manifest + Tree Summary

Manifest:
- Prefer `git ls-files` (deterministic).
- Exclude:
  - `target/`, `tmp/`, `vendor/`, `node_modules/`, `**/__pycache__/`
  - `docs/meta/audio/*.wav`, `docs/meta/audio/*.mp3`
  - `corpus_private/` and other private dirs

Tree summary:
- Produce a stable, bounded tree (like `scripts/generate_daily_plan_context.py`).
- Minimum: show `core/`, `cli/`, `desktop/`, `scripts/`, `docs/`.

### Truncation + Per-Section Accounting

Implement a small “section builder” abstraction:
- `start_section(name)`: records starting char count.
- `write(text)`: bounded writes.
- `end_section()`: records size, truncated flag.

At the end, emit:
- Total chars + token estimate.
- A per-section table (section name, chars, truncated?).

Truncation marker:
- Append a single line: `... truncated (max-chars reached) ...`

### Vendor Snapshot Placeholder

When `--vendor-snapshot` is set:
- Append `## Vendor snapshot (manual placeholders)`
- Include headings:
  - Stripe
  - Cloudflare
  - Resend
  - Fastmail
  - Actions suggested
- Include explicit redaction warnings.

### Docs + Checklist Reconciliation

After implementation:
- Update `docs/meta/prompts/README.md` with:
  - example command lines for generating the payload
  - where outputs live (once `DR-0007` is chosen)
- Update `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`:
  - Check off the generator-related items once the generator exists and is validated:
    - `:264` generator exists
    - `:270` SOP concatenation determinism (already in spec; verify, then check)
    - `:271` `--max-chars` implemented
    - `:272` `--vendor-snapshot` implemented

### Validation Checklist (Developer)

- Run:
  - `python3 scripts/generate_pro_context_payload.py --out tmp/pro_context_payload.md --max-chars 200000 --daily-logs 1`
  - Re-run and diff (expect identical except timestamps).
- Confirm payload contains:
  - `AGENTS.md` verbatim
  - `docs/index.md` verbatim including checklist auto-index block
  - Operating docs in the same order as `docs/index.md`
- Confirm truncation markers appear when `--max-chars` is small.

## Part B: Wire CI Signing + Notarization Into Releases

### Goal

On real releases (tag pushes `v*`), produce release artifacts that are:
- Windows: Authenticode-signed.
- macOS: Developer ID signed + notarized (and stapled if applicable).

### Current State (Verified)

- Pipeline: `.github/workflows/release.yml`
- macOS universal job performs **ad-hoc signing**: `codesign -s - excel-diff`.
- No notarization step exists.
- No Windows signing step exists.
- Artifacts shipped are CLI-only (no desktop app bundles).

### Non-goals

- Do not change the release artifact product scope (still CLI-only for now).
- Do not introduce auto-updater changes.

### Success Criteria (Acceptance)

- On tag push:
  - Windows artifacts pass `signtool verify`.
  - macOS artifacts pass:
    - `codesign --verify --strict --verbose=2`
    - `spctl -a -vvv` on the distributed artifact
    - Notarization job reports success (“Accepted”) for the submitted artifact.
- On `workflow_dispatch` with `dry_run=true`:
  - Build still works without signing secrets.
  - Signing/notarization steps are skipped cleanly.

### Required Secrets (Proposed Naming)

macOS:
- `MACOS_CERT_P12_B64`: base64 of Developer ID Application cert `.p12`
- `MACOS_CERT_PASSWORD`: password for the `.p12`
- `MACOS_SIGNING_IDENTITY`: codesign identity string
- `MACOS_NOTARY_KEY_B64`: base64 of App Store Connect API key `.p8`
- `MACOS_NOTARY_KEY_ID`: key id
- `MACOS_NOTARY_ISSUER_ID`: issuer id

Windows:
- Azure Artifact Signing / Trusted Signing (OIDC; no `.pfx` in CI):
  - `AZURE_TENANT_ID`
  - `AZURE_SUBSCRIPTION_ID`
  - `AZURE_CLIENT_ID`
  - `AZURE_ARTIFACT_SIGNING_ENDPOINT` (example: `https://eus.codesigning.azure.net/`)
  - `AZURE_ARTIFACT_SIGNING_ACCOUNT_NAME`
  - `AZURE_ARTIFACT_SIGNING_CERT_PROFILE_NAME`

Update `docs/release_signing.md` after wiring to match the real secret names.

### Packaging Format Decision (macOS)

Notarytool commonly supports `.zip` / `.pkg` / `.dmg`. The current macOS artifacts are `.tar.gz`.

To notarize reliably, choose one:
- Option A (recommended): **switch macOS artifacts to `.zip`** (contains `tabulensis` + README).
  - Update manifest generation in `.github/workflows/release.yml` accordingly (Homebrew formula should point at `.zip`).
- Option B: produce both `.zip` (notarized) and `.tar.gz` (legacy), but only advertise the `.zip`.

The simplest approach is A (single artifact format that matches notarization workflow).

### Implementation Steps (Windows)

In `.github/workflows/release.yml` `build-windows` job:
1. Ensure the job has `permissions: id-token: write` (required for GitHub OIDC).
2. Authenticate to Azure using OIDC (`azure/login` with `client-id`/`tenant-id`/`subscription-id`).
3. Sign `target/release-cli/tabulensis.exe` using Azure Artifact Signing (Trusted Signing).
   - Use the official GitHub Action (example: `azure/artifact-signing-action@v1`) with:
     - `endpoint` (example: `https://eus.codesigning.azure.net/`)
     - `signing-account-name`
     - `certificate-profile-name`
     - a file/folder selector targeting the built EXE
4. Verify signature:
   - `signtool verify /pa /v <exe>`
5. Only after signing:
   - copy/rename into the standalone `.exe` and staging dir
   - compute SHA256 hashes

Guard:
- Wrap signing steps with `if: startsWith(github.ref, 'refs/tags/v') && github.event.inputs.dry_run != 'true'`
- Use a GitHub Environment with required reviewers to gate release signing (decision: `docs/meta/results/decision_register.md` `DR-0020`).

### Implementation Steps (macOS)

In `.github/workflows/release.yml` macOS jobs (or only in the universal job):
1. Import signing cert into a temporary keychain:
   - decode `MACOS_CERT_P12_B64` to file
   - `security create-keychain`, `security import`, `security set-keychain-settings`
2. Codesign the final universal binary:
   - `codesign --force --options runtime --timestamp --sign "$MACOS_SIGNING_IDENTITY" tabulensis`
3. Package a `.zip` for notarization (recommended):
   - include `tabulensis`, `README.md`
4. Notarize:
   - decode `MACOS_NOTARY_KEY_B64` to `AuthKey.p8`
   - `xcrun notarytool submit <zip> --key <p8> --key-id ... --issuer ... --wait`
5. Staple:
   - `xcrun stapler staple <zip>` (or staple the binary then re-zip if stapler does not support the zip format used)
6. Verify:
   - `codesign --verify --strict --verbose=2 tabulensis`
   - `spctl -a -vvv <zip>`
7. Only after stapling:
   - compute SHA256 hashes for the distributed artifact(s)

Guard:
- Same condition as Windows: sign/notarize only on tag push and not dry-run.

### Workflow Structure Changes

Recommended:
- Keep signing inside each platform build job so artifacts are already signed when uploaded.
- Ensure checksums are computed after signing/notarization/stapling.

### Docs + Checklist Reconciliation

After implementation:
- Update `docs/release_signing.md` to:
  - list actual secret names used in workflow
  - list verification commands (macOS `spctl`, Windows `signtool verify`)
- Update `docs/release_checklist.md` to add:
  - “verify release artifacts are signed/notarized” step
- Consider adding a CI-only “signing dry run” mode that asserts steps are skipped when secrets absent.

### Rollback Plan

If signing breaks releases:
- Temporarily disable signing steps by gating on a repo variable:
  - `ENABLE_SIGNING=true` default off until ready
- Or revert the workflow changes; artifacts remain unsigned but builds remain functional.

## Execution Order (Recommended)

1. Implement Pro payload generator (Part A) behind `--out` and `--max-chars`.
2. Decide `DR-0007` and set the default output location.
3. Implement Windows signing steps (lowest ambiguity).
4. Implement macOS signing + notarization, choosing the macOS packaging format.
5. Update docs/checklists and re-run:
   - `python3 scripts/update_docs_index_checklists.py`
   - any relevant CI workflow dry run via `workflow_dispatch` with `dry_run=true`
