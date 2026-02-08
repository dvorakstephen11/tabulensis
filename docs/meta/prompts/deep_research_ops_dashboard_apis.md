Today is {{RUN_DATE}} (local) / {{RUN_DATETIME}}.

You are an AI research agent with web browsing. Your task is to answer:

**Question:** How much of my Fastmail, Cloudflare, Resend, and Stripe info can be read programmatically so that I can have a single dashboard I look at?

## Project Context (Repo-Specific)
This question is for operating Tabulensis (https://tabulensis.com), a small-team (often 1 operator) software business.

Known vendor usage in the repo:
- **Cloudflare**
  - Static pages are deployed on Cloudflare (marketing/docs in `public/`, web demo in `web/`).
  - A production licensing backend is intended to run as a **Cloudflare Worker + D1** in `tabulensis-api/`.
    - Worker code: `tabulensis-api/src/index.ts`
    - Worker config: `tabulensis-api/wrangler.jsonc` (D1 binding `DB`, observability enabled)
    - D1 migrations: `tabulensis-api/migrations/*.sql` (creates `licenses`, `activations`, `checkout_sessions`, `stripe_events`)
  - Production URLs referenced in repo/runbooks:
    - Website: `https://tabulensis.com`
    - Licensing API (Worker): `https://license.tabulensis.com`
    - Runbooks: `STRIPE_WORKER_NEXT_STEPS.md`, `docs/licensing_service.md`
  - Static pages call the licensing API directly (browser `fetch()`):
    - `public/download/index.html` -> `POST https://license.tabulensis.com/api/checkout/start`
    - `public/download/success/index.html` -> `GET https://license.tabulensis.com/api/checkout/status?session_id=...`
    - `public/support/billing/index.html` -> `POST https://license.tabulensis.com/portal/session`
    - `public/support/resend/index.html` -> `POST https://license.tabulensis.com/license/resend`
- **Stripe**
  - Used for checkout + billing portal + webhooks in both:
    - `tabulensis-api/src/index.ts` (Worker)
    - `license_service/src/main.rs` (Rust reference/local server)
  - Worker stores a minimal internal ledger in D1 tables:
    - `licenses`, `activations`, `checkout_sessions`, `stripe_events`
  - Stripe webhook events currently handled in Worker + Rust reference:
    - `checkout.session.completed`
    - `invoice.paid`
    - `invoice.payment_failed`
    - `customer.subscription.updated`
    - `customer.subscription.deleted`
- **Resend**
  - Used for transactional "license delivery" emails (configured in `tabulensis-api/src/index.ts`).
  - Setup notes exist in `RESEND_SETUP_CHECKLIST.md`.
  - Static support page calls the licensing API:
    - `public/support/resend/index.html` -> `POST https://license.tabulensis.com/license/resend`
- **Fastmail**
  - Support email is `support@tabulensis.com` and is managed by Fastmail (see `REPO_STATE.md`).
  - No programmatic Fastmail inbox integration exists in this repo yet; the dashboard question is about what is possible.
  - Ops note: `docs/operations.md` references support intake via `support@tabulensis.com`.

Constraints:
- Prefer **read-only** where possible and **least-privilege** tokens.
- Assume the dashboard is **internal/operator-only** (not customer-facing).
- Prefer high-ROI, low-maintenance solutions; avoid heavy infra unless justified.
- Do not ask me for secrets. Use placeholders and describe exactly what permissions/scopes are needed.

## Research Tasks (What You Must Produce)
1. **Per-service API surface inventory**
   - For each of Fastmail, Cloudflare, Resend, Stripe:
     - What official APIs exist (REST/GraphQL/JMAP/SDKs)?
     - What data can be read programmatically (account, billing, usage, analytics, logs/events, configuration)?
     - What is *not* available or is materially limited (plan restrictions, retention, missing endpoints)?
     - Auth model(s): API tokens vs OAuth, scopes/permissions, read-only options, required headers, base URLs.
     - Rate limits and practical considerations (pagination, time windows, webhooks vs polling).

2. **Dashboard feasibility answer**
   - Summarize what percentage/portion of "operator-relevant" information is realistically obtainable via APIs for each vendor.
   - Identify where "programmatic read" is best implemented as:
     - Polling the vendor API
     - Consuming webhooks (push)
     - Reading an internal mirror/ledger (for example D1 tables populated by webhooks)

3. **Recommended operator dashboard spec (MVP)**
   - Propose a small set of widgets/metrics that matter for this repo's reality:
     - Support inbox signals (Fastmail): unread count, SLA/age buckets, top threads, etc.
     - Cloudflare signals: site traffic, errors, DNS health, Worker health, deploy status, WAF/security events as available.
     - Resend signals: sent count, delivery failures/bounces/complaints, suppression list, domain verification status (as available).
     - Stripe signals: MRR/ARR, active subscriptions, trial starts, failed payments, disputes/chargebacks, payout status.
   - For each widget, map it to the **exact data source** and API call(s) required.

4. **Security + privacy guidance**
   - Recommend a safe data-handling posture (PII minimization, retention, audit logging).
   - Identify the highest-risk data (email bodies; Stripe customer data) and how to avoid over-collecting.

5. **Implementation approach options (architecture)**
   - Provide 2-3 concrete architecture options and tradeoffs, for example:
     - Cloudflare Worker Cron Trigger pulls metrics -> stores summary in D1 -> static dashboard reads from it
     - Webhooks (Stripe + Resend + Cloudflare where possible) populate a small DB; polling fills gaps
     - A local-only dashboard (runs on operator machine) that pulls directly from vendor APIs
   - For each option: complexity, ongoing maintenance, cost, and how to secure tokens.

## Output Format (Markdown)
- ## Executive Answer (table)
  - Columns: Service | What You Can Read | What You *Can't* (or Limits) | Best Mechanism (poll/webhook/internal) | Auth & Scopes | Notes
- ## Fastmail (JMAP/IMAP/etc)
- ## Cloudflare (API/GraphQL/Logpush/Analytics/etc)
- ## Resend (API/Webhooks/Events/etc)
- ## Stripe (API/Webhooks/Reports/etc)
- ## Proposed MVP Dashboard (widgets + data sources)
- ## Recommended Next Steps (ordered; <2 hours first)
- ## Appendix: Links (official docs first)

## Citation Requirements
- Use official documentation as primary sources and include direct links.
- If plan tier/pricing affects availability (e.g., logs/analytics), cite the plan docs and include the date you accessed them.
- Do not guess. If something is unclear in docs, say so and propose how to verify (test call, dashboard setting, support ticket).
