# Decision Register

This file tracks operator decisions surfaced by strategy / checklist audits.

Conventions:
- One decision per section.
- Decisions start as `Status: pending` and must not be "implicitly decided" via downstream edits.
- When chosen, append the decision date and rationale + evidence.

## DR-0001: Commit Policy For `docs/meta/logs/**` + `docs/meta/results/**`

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` has open items to decide whether logs/results are committed.
- This decision impacts operator workflow, privacy posture, and how automation references artifacts.

Options:
- A) Commit logs/results to git (default recommendation: *lean yes*, with strict redaction policy).
  - Pros: reproducible, searchable, discoverable, supports automation + cross-machine work.
  - Cons: higher risk of accidentally committing secrets/personal data; requires hygiene.
- B) Keep logs/results local-only (gitignored).
  - Pros: lower risk of accidental disclosure; lighter repo churn.
  - Cons: breaks audit trail; makes automation less reliable; harder to collaborate/transfer.
- C) Hybrid: commit structure + summaries; keep raw vendor outputs local-only.
  - Pros: reduces sensitive surface while preserving core audit trail.
  - Cons: operational complexity; easy to drift into "missing receipts".

Default recommendation:
- Commit logs/results to git, but enforce redaction discipline.

Chosen option:
- Commit `docs/meta/logs/**` and `docs/meta/results/**` to git.

Evidence:
- Repo already treats these as canonical operator inputs/outputs (`docs/meta/logs/README.md`, `docs/meta/results/README.md`, `docs/index.md` links).
- Overnight agent runbook assumes committed ops journal outputs under `docs/meta/logs/ops/` (on `overnight/ops-journal`).
- `.gitignore` does not exclude `docs/meta/logs/**` or `docs/meta/results/**`.

Follow-ups:
- Keep `docs/meta/logs/README.md` / `docs/meta/results/README.md` explicit about "no secrets/PII".
- Add/maintain a short "Privacy + redaction" warning in `docs/meta/README.md`.

Follow-up tasks:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` decision items (exact lines to be filled during checklist triage).

## DR-0002: Commit Policy For `docs/meta/today.md`

Status: chosen (2026-02-09)

Context:
- `meta_methodology.md` references `docs/meta/today.md` for daily scratchpad.
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` includes a decision gate for whether `docs/meta/today.md` is committed.

Options:
- A) Commit `docs/meta/today.md` (append-only daily plan scratchpad).
- B) Keep `docs/meta/today.md` local-only (gitignored), but commit daily logs under `docs/meta/logs/daily/`.
- C) Replace `docs/meta/today.md` with a generated daily plan artifact (committed) and keep scratch local-only.

Default recommendation:
- Keep the durable history in daily logs; keep today planning scratch local-only.

Chosen option:
- Keep `docs/meta/today.md` local-only (gitignored).
- Commit a template at `docs/meta/today.example.md`.

Evidence:
- `docs/meta/today.md` is overwritten frequently and is not durable history; durable artifacts live in `docs/meta/logs/daily/`.
- Checklist index generation now respects `.gitignore` (see `scripts/update_docs_index_checklists.py`), so local-only `docs/meta/today.md` will not pollute `docs/index.md`.

Follow-up tasks:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` decision items (exact lines to be filled during checklist triage).

## DR-0003: Deep Research Results Location (Canonical Home)

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks where deep research results live.
- `docs/meta/results/README.md` describes deep research captures under `docs/meta/results/`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate around deep research location + naming).

Options:
- A) Canonical home is `docs/meta/results/` (committed), with strict redaction and append-only rules.
- B) Canonical home is local-only (gitignored), with only short pointers/summaries committed to logs.
- C) Hybrid: commit syntheses only; keep raw A/B runs local-only.

Chosen option:
- Canonical directory: `docs/meta/results/` (flat).

Evidence:
- `scripts/deep_research_prompt.py` writes results under `docs/meta/results/` and does not implement a grouped `docs/meta/results/deep_research/` output mode.
- `docs/meta/results/README.md` is written assuming `docs/meta/results/` is the default destination.

Notes:
- Commit vs local-only is governed by `DR-0001` (this decision is about *path layout*, not git policy).

## DR-0004: Deep Research Capture Filename Scheme

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide a canonical filename scheme for deep research captures.
- `docs/meta/logs/README.md` and `docs/meta/results/README.md` already describe a scheme (`YYYY-MM-DD_<prompt>_a.md` / `_b.md` / `_synthesis.md`).

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/logs/README.md` (naming conventions).
- `docs/meta/results/README.md` (naming scheme + recommended structure).

Options:
- A) Keep the existing scheme from docs.
- B) Switch to timestamped scheme (e.g., `YYYY-MM-DD_HHMM_<prompt>_a.md`) to avoid same-day collisions.

Chosen option:
- Timestamped scheme (match tooling): `YYYY-MM-DD_HHMMSS_<topic>[_a|_b|_synthesis].md`.

Evidence:
- `scripts/deep_research_prompt.py` creates files with a `%Y-%m-%d_%H%M%S` prefix for both `--new-result` and `--new-a-b`.

## DR-0005: TTS Approach (Local vs Cloud) + Output Directory

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide a TTS path (cloud vs local) and document it.
- Repo already includes a local-first TTS script: `scripts/tts_generate.py`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/audio/README.md` (operator guidance).
- `scripts/tts_generate.py` (repo reality).

Options:
- A) Local TTS (OS/CLI backends) via `scripts/tts_generate.py`.
- B) Cloud TTS (pick provider + credentials + redaction rules) with outputs stored locally.

Chosen option:
- Local-first TTS via `scripts/tts_generate.py` (backend auto-detect; no cloud provider configured).
- Default output directory: `docs/meta/audio/` (timestamped filenames; overrideable via `--out`).

Evidence:
- `scripts/tts_generate.py` supports local backends (`powershell(_wsl)`, `say`, `espeak`) and writes to `docs/meta/audio/` by default.
- `docs/meta/audio/README.md` documents local usage and output location.
- `.gitignore` excludes `docs/meta/audio/*.wav` and `docs/meta/audio/*.mp3` (audio outputs are local-only; README is committed).

## DR-0006: Canonical Inputs To Daily Planning

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide canonical inputs.
- `docs/meta/logs/README.md` already provides a suggested order of inputs.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/logs/README.md` ("Canonical Inputs To Daily Planning").

Options:
- A) Adopt the existing list from `docs/meta/logs/README.md` as canonical.
- B) Modify the list (add/remove inputs) and update docs + automation accordingly.

Chosen option:
- Adopt the existing list from `docs/meta/logs/README.md` as canonical.

Evidence:
- `docs/meta/logs/README.md` already enumerates canonical daily planning inputs.
- `scripts/generate_daily_plan_context.py` implements a concrete bundle consistent with this.

## DR-0007: Context Payload Output Location

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks where context payload outputs live.
- `docs/meta/prompts/pro_context_payload.md` defines a spec; tooling/output path must match.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/prompts/pro_context_payload.md`.

Options:
- A) Store generated payloads under `docs/meta/results/` (committed).
- B) Store under `tmp/` (local-only), commit only spec and hashes.

Chosen option:
- Store generated payloads under `tmp/` (local-only).
  - Default path: `tmp/pro_context_payload.md` (override via generator `--out`).

Evidence:
- `tmp/` is already gitignored (safe default for large payloads and anything that might include vendor snapshots later).
- Payloads will embed checklists verbatim (many `- [ ]` lines); keeping them out of `docs/meta/results/` avoids polluting the auto-indexed checklist surface (`docs/index.md`).
- Decision recorded here and referenced from `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`.

## DR-0008: Automation Run Output Directory (Single Canonical Path)

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide a single output dir for automated runs and document it.
- Overnight agent already uses `tmp/overnight_agent/` for raw runtime + a committed ops journal on a dedicated branch.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate references `docs/meta/automation/README.md`).
- `docs/meta/automation/overnight_agent_runbook.md` ("Where Outputs Go").

Options:
- A) Standardize on `tmp/<automation_name>/` for raw runtime and `docs/meta/logs/ops/` for sanitized reports.
- B) Store all automation artifacts under `docs/meta/results/` (committed) with strict redaction.

Chosen option:
- Standardize on `tmp/<automation_name>/` for raw runtime and `docs/meta/logs/ops/` for sanitized reports.

Evidence:
- `docs/meta/automation/overnight_agent_runbook.md` documents raw artifacts under `tmp/overnight_agent/` and ops journal reports under `docs/meta/logs/ops/` on branch `overnight/ops-journal`.
- `docs/meta/automation/overnight_agent.yaml` config uses `tmp/overnight_agent/` paths for runtime state.

## DR-0009: Automation Run Filename Scheme

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` suggests `YYYY-MM-DD_HHMM_<task>_<agent>.md`.
- Need consistent naming for per-run reports/questions.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/automation/overnight_agent_runbook.md` (run_id-based filenames + daily questions file).

Options:
- A) Adopt timestamp + slug scheme for human scan.
- B) Use opaque `run_id` scheme everywhere, with an index mapping to tasks.

Chosen option:
- Adopt a timestamp + slug scheme for human scan:
  - `YYYY-MM-DD_HHMMSS_<task>_<agent>.md`
  - `<task>` / `<agent>` are ASCII `lower_snake_case`.
  - Each report must include a run header block inside the file (run tag, timestamps, git commit, environment, and any internal `run_id`).

Evidence:
- Documented in `docs/meta/automation/README.md` ("Output Locations (Canonical)").

## DR-0010: Where Automation Runs (Local vs Remote)

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide whether automation runs locally vs other execution environment.
- Impacts secrets, filesystem access, uptime, and safety model.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).

Options:
- A) Local workstation only.
- B) Dedicated always-on machine (mini PC / server).
- C) Cloud VM / hosted runner (higher complexity; secrets posture).

Chosen option:
- Local workstation only.

Rationale:
- Minimizes secrets and access surface area while this automation is still evolving.
- Matches current tooling assumptions (local filesystem paths, local `tmp/`, local git worktrees, optional Codex CLI usage).
- Avoids operational overhead (provisioning, patching, remote alerts) until scheduling + guardrails stabilize.

Evidence:
- Overnight agent stores raw runtime state under `tmp/overnight_agent/` and uses local git worktrees by default (see `docs/meta/automation/overnight_agent_runbook.md`).
- Automation guardrails assume "do not proceed if required local tools are missing" and a human-in-the-loop safety model (`docs/meta/automation/guardrails.md`).

Follow-ups:
- If/when you need always-on reliability, revisit this decision and consider `B` (dedicated machine) before `C` (cloud).

## DR-0011: Midnight Run Scheduler

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide scheduler and record it.
- Could be cron/systemd/launchd/Task Scheduler depending on OS.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate; see also `docs/meta/automation/README.md`).

Options:
- A) `cron` (Linux/macOS).
- B) `launchd` (macOS).
- C) Windows Task Scheduler.
- D) Cross-platform Python "always-on" loop.

Chosen option:
- Windows Task Scheduler (invoking WSL).

Rationale:
- Matches the current operator environment (Windows + WSL) while keeping automation "local workstation only" (`DR-0010`).
- More reliable than WSL cron for "run at midnight" because the scheduler lives on Windows.
- Keeps the automation start mechanism simple and explicit (no always-on daemon yet).

Evidence:
- Scheduler example doc: `docs/meta/automation/task_scheduler_example.md`.

Follow-ups:
- If you move to macOS-only or Linux-only operations, revisit and switch to `launchd`/`cron` as appropriate.

## DR-0012: Canonical UTM Scheme (Marketing)

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide a canonical UTM scheme and document it.
- This belongs under `docs/meta/marketing_templates/`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate references `docs/meta/marketing_templates/README.md`).

Options:
- A) Define a minimal UTM schema (`utm_source`, `utm_medium`, `utm_campaign`, optional `utm_content`).
- B) Keep ad hoc.

Chosen option:
- Define a minimal UTM schema:
  - Required: `utm_source`, `utm_medium`, `utm_campaign`
  - Optional: `utm_content`
  - Value rule: ASCII `[A-Za-z0-9_-]` (no spaces), short + stable.

Evidence:
- Documented in `docs/meta/marketing_templates/README.md` ("Canonical UTM Scheme").
- Implemented generator: `scripts/utm.py`.
- Metrics file created: `docs/meta/logs/marketing/metrics.csv`.

## DR-0013: Minimum Demo-Video Format

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide minimum demo-video format.
- Checklist also requests that this decision be recorded in `docs/meta/marketing_templates/README.md`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/marketing_templates/README.md`

Options:
- A) 30-60 second "hook" demo (screen recording) with captions; publish to landing + YouTube.
- B) 2-3 minute "walkthrough" demo; publish to docs + YouTube.

Chosen option:
- A) 30-60 second "hook" demo (screen recording) with captions; publish to landing + YouTube.

Rationale:
- Easier to make and ship consistently (low activation energy).
- Maximizes iteration speed while you are still refining the product story and ideal "wow" workflow.
- Works well as a landing-page asset and as a social clip.

Evidence:
- Recorded in `docs/meta/marketing_templates/README.md` ("Minimum Demo Video Format (Chosen)").

Follow-ups:
- Create `DEMO_VIDEO_CHECKLIST.md` and add it to `docs/index.md` (see `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` section 8.6).

## DR-0014: Fastmail Access Method

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks to decide Fastmail access method (API/IMAP/webhook).
- Impacts the "ops dashboard" vision in `meta_methodology.md`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).
- `docs/meta/prompts/deep_research_ops_dashboard_apis.md` (prompt to research vendor APIs).

Options:
- A) Use Fastmail JMAP API (if available) or IMAP for read-only access.
- B) Keep manual-only initially; revisit later.

Chosen option:
- B) Keep manual-only initially; revisit later.

Rationale:
- Avoids adding a new integration + secrets/credential handling while support volume is still low.
- Avoids accidental over-collection of email content (PII risk) during early automation work.
- Keeps the "ops dashboard" effort scoped to other vendors first (Stripe/Resend/Cloudflare), where APIs/webhooks are already part of the stack.

Follow-ups:
- Still create `scripts/fastmail_export_instructions.md` (manual steps) and a lightweight `docs/meta/automation/fastmail_synthesis.md` spec when you are ready to formalize the workflow.
- Revisit once support inbox volume or SLA needs justify automation, using `docs/meta/prompts/deep_research_ops_dashboard_apis.md`.

## DR-0015: Second OpenAI Pro Account

Status: chosen (2026-02-09)

Context:
- `meta_methodology.md` suggests a second Pro account may be needed for limits.
- Checklist asks to decide and record in `docs/meta/automation/model_accounts.md`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate; see `docs/meta/automation/model_accounts.md`).
- `meta_methodology.md` (mentions second Pro account).

Options:
- A) Create second account now; document usage policy.
- B) Defer until limits materially block progress.

Chosen option:
- B) Defer until limits materially block progress.

Rationale:
- Keep operations simple until this becomes a real constraint.
- Avoid additional account/credential management and accidental cross-use policy mistakes.

Evidence:
- Recorded in `docs/meta/automation/model_accounts.md` ("Decisions").

Follow-ups:
- When you first hit a limit that blocks an overnight run or a deep research session, record:
  - date
  - what was blocked
  - mitigation taken
- If repeated blockers occur, revisit and implement option A with a clear usage split policy.

## DR-0016: Security Tools For Vulnerability Discovery Workflow

Status: chosen (2026-02-09)

Context:
- `meta_methodology.md` calls out AI-driven vulnerability discovery.
- Checklist asks to decide tools and record in `docs/meta/automation/guardrails.md`.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).

Options:
- A) Start with basic dependency audit + fuzzing + SAST, and automate weekly.
- B) Add more specialized tooling later.

Chosen option:
- Start with basic dependency audit + secret scanning + minimal SAST:
  - Rust dependency audit: `cargo audit` (RustSec)
  - npm dependency audit: `npm audit --omit=dev` (under `tabulensis-api/`)
  - Secret scanning: `gitleaks detect`
  - Minimal SAST: `semgrep --config auto`

Evidence:
- Guardrails updated: `docs/meta/automation/guardrails.md` ("Security Tooling (Chosen)").
- Operator checklist created: `SECURITY_DAILY_CHECKLIST.md`.
- Wrapper report script created: `scripts/security_audit.sh`.

## DR-0017: Stale Checklist Threshold

Status: chosen (2026-02-09)

Context:
- Checklist asks to decide "stale checklist" threshold (N days) and record in `meta_methodology.md` definition-of-done.

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate).

Options:
- A) N=7 days.
- B) N=14 days.
- C) N=30 days.

Chosen option:
- C) N=30 days.

Rationale:
- Keeps noise low given the size/variety of checklists in this repo.
- Aligns with a monthly "freshness" expectation: if a checklist has not changed in a month, it is probably drifting or blocked.

Evidence:
- Recorded in `meta_methodology.md` ("Methodology DoD (Measurable)").

Follow-ups:
- Implement `scripts/methodology_audit.py` to operationalize this threshold (see `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` section 15).

## DR-0018: Automation Worktree Isolation (Current vs Dedicated Worktrees)

Status: chosen (2026-02-09)

Context:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` asks whether automation should run in the current worktree or in dedicated git worktrees.
- This impacts safety (scope control, avoiding accidental mutation/capture of unrelated changes).

Source pointers:
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (section 6.3 "Worktree Isolation").
- `docs/meta/automation/overnight_agent_runbook.md` (already uses worktrees).

Options:
- A) Run automation in the current worktree (fast, risky).
- B) Run automation in dedicated git worktrees (safer).

Chosen option:
- B) Run automation in dedicated git worktrees.

Rationale:
- Aligns with the repo's non-negotiable automation safety model: never mutate the primary working tree.
- Reduces "blast radius" when automation runs while you are actively working.

Evidence:
- Overnight agent already uses dedicated worktrees (`docs/meta/automation/overnight_agent.yaml` `repo.worktree_root`).
- Helper script added: `scripts/agent_worktree.sh`.

Follow-ups:
- Use `scripts/agent_worktree.sh <name>` to create/ensure `../excel_diff_worktrees/<name>` worktrees for manual agent sessions.

## DR-0019: Download Artifact Hosting (Where `tabulensis.com/download` Points)

Status: chosen (2026-02-09)

Context:
- `public/download/index.html` needs stable, correct links to downloadable CLI artifacts.
- `STRIPE_WORKER_NEXT_STEPS.md` calls out "Decide hosting" for release artifacts.

Source pointers:
- `public/download/index.html`
- `.github/workflows/release.yml` (release artifacts are published to GitHub Releases)
- `STRIPE_WORKER_NEXT_STEPS.md` (section "Downloads")

Options:
- A) GitHub Releases (link `tabulensis.com/download` to GitHub release assets).
- B) Cloudflare R2 public bucket (upload from CI; serve from your own domain).
- C) Cloudflare Pages static assets (bake artifacts into Pages; generally awkward for large binaries).

Chosen option:
- A) GitHub Releases.

Rationale:
- Lowest operational overhead: CI already creates a GitHub Release with attached artifacts.
- Avoids introducing new cloud credentials and bucket policies until the download funnel is stable.

Implementation notes:
- Use stable "latest" asset names (in addition to versioned assets) so `tabulensis.com/download` links never need to change.

## DR-0020: CI Signing Key Handling (Windows Authenticode)

Status: chosen (2026-02-09)

Context:
- You will eventually wire Windows Authenticode signing into `.github/workflows/release.yml`.
- This decision determines how the signing certificate/private key is stored and how signing is gated.

Source pointers:
- `docs/release_signing.md`
- `.github/workflows/release.yml`
- `tabulensis_launch_to_dos_from_our_chat.md` (Signing/notarization section)

Options:
- 1) GitHub-hosted runners + GitHub Environment secrets/approvals (store PFX as base64 + password; restrict via required reviewers; sign only on tags).
- 2) Self-hosted Windows runner + cert in OS store / hardware token.
- 3) Azure Artifact Signing (Trusted Signing) via GitHub OIDC (no PFX/private-key handling in CI).
- 4) Other signing service (SignPath / DigiCert KeyLocker / etc).

Chosen option:
- 3) Azure Artifact Signing (Trusted Signing) via GitHub OIDC.

Rationale:
- Avoids handling an exportable private key (`.pfx`) in CI.
- Keeps infra minimal (still GitHub-hosted runners) while improving key custody.
- Fits the current release workflow model (tag-triggered GitHub Actions) and remains compatible with approval gating.

Implementation notes:
- Prefer an environment (example name: `release-signing`) with required reviewers.
- Configure Azure:
  - Create an Azure Artifact Signing (Trusted Signing) account + certificate profile.
  - Assign the signing role (certificate-profile signer) to the GitHub OIDC identity you will use in Actions.
- Gate signing steps on both:
  - running on a tag (`refs/tags/v*`), and
  - `dry_run != true`.
- In the signing job, request `permissions: id-token: write` and sign via the official action, then verify with:
  - `signtool verify /pa /v <exe>`
- Keep any “PFX in secrets” flow as an explicit fallback only (not the default).
