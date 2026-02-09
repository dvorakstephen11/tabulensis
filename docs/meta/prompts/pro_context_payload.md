# Pro Context Payload Spec (`pro_context_payload.md`)

This file defines the requirements for the "Ultimate Context Payload" used with
GPT-5.x-Pro (or similar large-context models) for high-leverage planning, design
review, and operator assistance.

This is a specification document, not the generated payload itself.

## Goals

- Produce one deterministic, auditable markdown payload that can be pasted into a model.
- Ensure the payload always includes Tabulensis operating SOPs and repo guardrails.
- Include enough codebase context to answer "where/what to change" questions without
  re-scanning the repo.
- Optionally append a bounded vendor snapshot section (redacted; no secrets).

## Non-goals

- This spec does not decide where generated payload artifacts are stored. That decision
  is recorded in `docs/meta/README.md` (see `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`).
- This spec does not contain vendor credentials or raw vendor data.

## Required Contents (MUST)

The generated payload MUST include these sections, in this order:

1. Generation metadata
   - Generation timestamp (local time, ISO format).
   - Repo root path (repo-relative is ok), git branch, git commit (short SHA).
   - Generator version string (script path + git SHA of generator, if available).
   - Final payload size: character count and a token estimate (see "Hard Limits").

2. Repo guardrails (authoritative)
   - `AGENTS.md` (verbatim).

3. Canonical docs index surfaces
   - `docs/index.md` (verbatim, including the auto-indexed unfinished checklists block).

4. Operator methodology + primary checklist
   - `meta_methodology.md` (verbatim).
   - `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (verbatim, or truncated with an explicit
     marker if too large).

5. Operating docs (SOPs/runbooks/checklists)
   - The "Operating docs (SOPs, runbooks, checklists)" entries from `docs/index.md`,
     concatenated deterministically (see "Deterministic SOP Concatenation Order").
   - To avoid duplicates, if a document was already included in earlier sections (for
     example `meta_methodology.md`), it should be skipped in this section while
     preserving the canonical order for the remaining entries.

6. Recent operator logs (bounded)
   - The last N daily logs under `docs/meta/logs/daily/` (N is a generator option;
     recommended default N=3).
   - Optional: last N ops journal entries under `docs/meta/logs/ops/` (bounded).

7. Codebase context (bounded)
   - A deterministic repo file manifest (for example `git ls-files` output), excluding
     build artifacts and large generated outputs.
   - A directory tree summary rooted at `/` with key subtrees (at minimum: `core/`,
     `cli/`, `desktop/`, `scripts/`, `docs/`).
   - A deterministic set of entrypoint docs and code bundles (see "Codebase Bundle
     Guidance").

8. Vendor snapshot (optional but recommended)
   - A final section that contains vendor readings/snapshots, bounded and redacted.
   - The section MUST be present when `--vendor-snapshot` (or equivalent) is set, even
     if it only contains placeholders.

## Hard Limits (MUST ENFORCE)

Primary budget is characters (because paste/upload limits vary and tokenizers differ).

- Hard max characters: 800000
- Soft warning threshold: 700000
- Token estimate: `est_tokens = floor(chars / 4)` (rough; reported for operator
  convenience only)

Truncation rules:

- The generator MUST accept `--max-chars <int>`.
- When a section would exceed the remaining budget, truncate that section and append a
  single-line marker:
  - `... truncated (max-chars reached) ...`
- The generator MUST report per-section character counts and whether truncation occurred.

## Output Format (Single Markdown File)

The generator writes a single markdown file with stable structure.

### Header

- Start with a top-level title:
  - `# Tabulensis Pro Context Payload`
- Include a "Generation metadata" subsection as described above.

### Section convention

- Each logical section begins with a level-2 header:
  - `## <Section name>`
- When embedding file contents, each file is wrapped as:
  - `### File: <repo-relative-path>`
  - A fenced code block using 4 backticks to safely embed triple-backtick markdown from
    source files.
  - Language tag derived from extension (`markdown`, `rust`, `python`, `toml`, `yaml`,
    `json`, `csv`, `text`).

Example:

````markdown
### File: AGENTS.md
````text
<verbatim contents>
````
````

## Deterministic SOP Concatenation Order (MUST)

The payload's "Operating docs" concatenation order MUST be stable between runs.

Rules:

- The generator MUST include the "Operating docs (SOPs, runbooks, checklists)" list from
  `docs/index.md` in the order it appears in `docs/index.md`.
- If `docs/index.md` changes, the resulting payload order changes accordingly. This is
  acceptable and desired because `docs/index.md` is the canonical ordering surface.
- If a doc listed in `docs/index.md` is missing, include a stub entry that clearly
  indicates it was missing.

For reference, as of 2026-02-09 the intended order is:

1. `meta_methodology.md`
2. `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
3. `docs/meta/README.md`
4. `docs/meta/prompts/README.md`
5. `docs/meta/audio/README.md`
6. `docs/meta/automation/overnight_operator_agent_plan.md`
7. `docs/meta/automation/overnight_agent_runbook.md`
8. `docs/meta/automation/overnight_agent.yaml`
9. `docs/operations.md`
10. `docs/licensing_service.md`
11. `STRIPE_WORKER_NEXT_STEPS.md`
12. `RESEND_SETUP_CHECKLIST.md`
13. `docs/release_checklist.md`
14. `docs/release_readiness.md`
15. `docs/release_signing.md`
16. `docs/auto_update_strategy.md`
17. `docs/installer_ux.md`

Note: The generator must still include `AGENTS.md` and `docs/index.md` earlier in the
payload even though they are not in this list.

## Codebase Bundle Guidance (Recommended)

At minimum include:

- `README.md`
- `Cargo.toml` and crate `Cargo.toml` files
- Maintainer entrypoints and architecture docs:
  - `docs/maintainers/entrypoints.md`
  - `docs/maintainers/architecture.md`
  - `docs/maintainers/fixtures.md`
- Key perf/validation docs:
  - `benchmarks/README.md`
  - `docs/perf_playbook.md`

Then include deterministic code bundles (expand as needed), for example:

- Core engine: `core/src/**`, `core/tests/**` (bounded; prefer selected patterns).
- CLI entrypoints: `cli/src/**`
- Desktop backend entrypoints: `desktop/backend/src/**`
- Automation scripts: `scripts/*.py`

If the max-chars budget is tight, prefer including:

1. Entrypoint docs and repo manifest/tree
2. Core diff engine sources and tests
3. Desktop backend orchestration
4. Everything else

## Security / Redaction Requirements (MUST)

- Never include secrets, tokens, API keys, emails with sensitive content, or customer PII.
- Vendor snapshots must be sanitized. If unsure, include only a link/reference to where
  the operator can review it locally.

