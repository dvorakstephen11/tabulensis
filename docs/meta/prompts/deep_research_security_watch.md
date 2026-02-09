<!--
Paste this into ChatGPT "Deep research" mode.
Shortcut: `python3 scripts/deep_research_prompt.py --prompt deep_research_security_watch.md`
-->

## When to Use Which Prompt

Use `deep_research_security_watch.md` when you want a stack-relevant security watch for Tabulensis (high-signal security news, supply-chain incidents, and CVEs that could affect the product, build/release pipeline, or customer trust posture); run weekly, and also run immediately after any major upstream incident. For a broader market + demand + acquisition/partnership scan, use `deep_research_market_analysis.md`; for competitor/pricing monitoring, use `deep_research_competitor_watch.md`; for distribution micro-experiments, use `deep_research_distribution_experiments.md`; and for ops dashboard API feasibility, use `deep_research_ops_dashboard_apis.md`.

Today is {{RUN_DATE}} (local) / {{RUN_DATETIME}}.

You are an AI research agent with web browsing.

## Project Context (Repo-Specific)

This research supports Tabulensis (https://tabulensis.com): a desktop app + CLI that compares Excel workbooks (.xlsx/.xlsm) and Power BI packages (.pbix/.pbit) and produces a structured diff (including Power Query / M changes).

Repo stack overview (high-level):
- Rust workspace crates: core diff engine (`core/`), CLI (`cli/`), desktop backend (`desktop/backend/`), desktop GUI via wxDragon (`desktop/wx/`), WASM bindings (`wasm/`), licensing client/server (`license_client/`, `license_service/`), UI payload (`ui_payload/`).
- Cloudflare Worker licensing backend: `tabulensis-api/` (TypeScript via Wrangler, D1 database).
- Static web UI / demo: `public/`, `web/`.
- Python fixture generator: `fixtures/` (openpyxl/lxml/jinja2/pyyaml).
- CI: GitHub Actions under `.github/workflows/` (includes third-party actions like `dtolnay/*` and `softprops/action-gh-release` plus `actions/*`).

Prioritize vulnerabilities and incidents in these areas (most likely to matter):
- Untrusted input parsing: ZIP containers (xlsx/pbix), XML parsing, VBA parsing.
- Crypto/TLS/network: license signing/verifying, webhook/auth token handling, HTTP clients.
- Storage: SQLite (bundled), DB schema migrations, local caches.
- Desktop UI: wxWidgets / WebView components.
- Build/release supply chain: Rust toolchain/cargo, GitHub Actions, npm tooling (Wrangler), Python packages.

Representative dependencies to treat as "watch list" anchors (not exhaustive):
- Rust: `zip`, `quick-xml`, `ovba`, `rusqlite` (bundled SQLite), `reqwest` (rustls), `ureq` (TLS), `ed25519-dalek`, `sha2`, `clap`, `axum`, `tokio`, `wasm-bindgen`, `windows-sys`.
- Worker (npm): `wrangler`, `typescript`, `vitest`, `@cloudflare/vitest-pool-workers`, `resend`, `tweetnacl`.
- Python (fixtures): `openpyxl`, `lxml`, `jinja2`, `pyyaml`.

Constraints:
- Assume a tiny team (often 1 operator). Prefer high-severity, high-likelihood items and crisp, actionable recommendations.
- Do not guess. If you cannot validate a fact, say so and propose a verification step.
- Prefer information from the last 14-30 days; include older items only if they are actively exploited or likely to affect us.
- Treat exploitation status and fixes as time-sensitive: always include "as of" dates and direct links.
- Do not ask me for secrets. Use placeholders and describe required scopes/permissions if relevant.

## Research Tasks (What You Must Produce)

1. **Executive security brief (as of {{RUN_DATE}})**
   - Identify the top 5-10 security items we should care about this week.
   - For each item, include:
     - What happened (1-2 sentences)
     - Who is impacted (products/versions)
     - Severity (CVSS or vendor severity)
     - Exploitation status (known exploited / likely / no evidence) with date and source
     - Fix status (patched versions, mitigations, or "no fix yet")
     - Why it matters for Tabulensis specifically

2. **CVE + advisory sweep (stack-relevant)**
   - Find recent CVEs/advisories that may affect:
     - Rust ecosystem (RustSec, GitHub Security Advisories, notable crate advisories)
     - Rust toolchain/cargo supply chain (if any incidents/vulns)
     - Cloudflare Workers / Wrangler / D1
     - SQLite (especially if "bundled" via rusqlite)
     - wxWidgets / embedded WebView components
     - Python packages used for fixtures
   - Focus especially on vulnerabilities involving:
     - ZIP/XML parsing issues (zip bombs, decompression bombs, path traversal, XXE/entity expansion)
     - RCE, auth bypass, signature verification bypass
     - TLS/crypto vulnerabilities
     - Sandbox escapes relevant to Workers / desktop WebView contexts

3. **Supply-chain incident watch (last 30-90 days)**
   - Identify notable supply-chain events across:
     - crates.io / Rust ecosystem
     - npm ecosystem (including Cloudflare tooling and popular transitive deps)
     - GitHub Actions ecosystem (action compromise, tag retargeting, malicious updates)
     - PyPI ecosystem
   - Highlight any incidents involving packages/actions we use, or close neighbors (same vendors/maintainers, similar tooling).

4. **Implications for Tabulensis (map to repo reality)**
   - For each relevant issue, map it to:
     - Impacted component(s) (repo path + dependency or vendor)
     - Risk scenario (how it could be triggered in our usage)
     - Recommended action (update/pin/mitigate/monitor/accept) and rationale
   - Produce a prioritized action list with:
     - Immediate (< 2 hours)
     - Short-term (1-3 days)
     - Longer-term (1-4 weeks)

5. **Low-maintenance monitoring plan**
   - Recommend a small set of feeds/alerts we should check/sub to, with:
     - Exact URL
     - Cadence (daily/weekly/monthly)
     - Trigger thresholds (what would cause us to take action)
   - Prefer approaches that avoid brittle scraping.

6. **Operator follow-ups (commands to run locally; do not run them)**
   - Provide a short checklist of local commands and checks to validate exposure, such as:
     - Rust: `cargo audit` (RustSec) and/or `cargo deny`
     - Node: `npm audit` under `tabulensis-api/` (if applicable)
     - GitHub Actions: identify third-party actions and whether they are pinned to immutable SHAs
   - For each follow-up, describe what a "bad" finding looks like and the next action.

## Output Format (Markdown)

- ## Executive Brief (top 10)
- ## Stack-Relevant Advisories and CVEs (table)
  - Columns: Item | Affected component | Severity | Exploited? | Fixed version / mitigation | Evidence | Sources
- ## Supply-Chain Incidents (last 30-90 days)
- ## Recommended Actions (prioritized)
  - Split into: <2 hours, 1-3 days, 1-4 weeks
- ## Monitoring Plan (feeds + cadence)
- ## Append-Only Research Log Entry (ready to paste)
  - Include: date, 5-10 bullets, and a compact "links" list.

## Citation Requirements

- Prefer primary sources:
  - Vendor advisories and official release notes
  - GitHub Security Advisories
  - RustSec advisory database
  - NVD and/or CISA KEV (when applicable)
- Every non-obvious claim must include a direct link.
- For "exploited in the wild" claims: cite a reputable source and include the date the claim was made.
- If transitive dependency exposure is unclear, say so and propose how to confirm (for example, "check Cargo.lock / npm lock and run audit tooling").
