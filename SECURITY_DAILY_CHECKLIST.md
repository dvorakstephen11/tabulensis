# Security Daily/Weekly Checklist

This is a minimal, repeatable security routine for Tabulensis.

Goal: catch high-likelihood issues (dependency CVEs, leaked secrets, obvious static-analysis findings) without creating a high-maintenance security program.

## Quick Run (Recommended)

Generate an ops report (append-only):

```bash
bash scripts/security_audit.sh
```

This writes/updates:
- `docs/meta/logs/ops/YYYY-MM-DD_security_audit.md`

## Checklist

Run weekly (or before cutting a release):

- [ ] Run `bash scripts/security_audit.sh` and skim the report.
- [ ] If any "RED" findings:
  - [ ] Patch or upgrade the dependency/tooling.
  - [ ] Add a short note to the most recent daily log (`docs/meta/logs/daily/YYYY-MM-DD.md`) with what changed and why.
- [ ] If any "YELLOW" (tool missing / error):
  - [ ] Install the missing tool and re-run.
  - [ ] If you intentionally defer, record why in the report.

## Chosen Tooling (Authoritative)

Decision: `docs/meta/results/decision_register.md` (`DR-0016`).

- Rust dependency audit: `cargo audit` (RustSec)
- npm dependency audit: `npm audit` (under `tabulensis-api/` when present)
- Secret scanning: `gitleaks`
- Minimal SAST: `semgrep --config auto`

## Redaction Rules

- Do not commit secrets, tokens, API keys, webhook secrets, or customer PII.
- If a tool output includes sensitive info, commit only a sanitized summary and keep raw output local-only.

