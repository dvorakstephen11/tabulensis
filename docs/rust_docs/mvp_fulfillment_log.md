# MVP Fulfillment Log

This file tracks decisions and progress against [mvp_checklist.md](mvp_checklist.md). Each checklist item is followed by the current decision plus context/suggestions.

## 2026-01-22 — Branding and deployment decisions

- [x] Product name finalized
  - Answer: Tabulensis.
- [x] Canonical domain + download URL
  - Answer: `tabulensis.com` with downloads at `https://tabulensis.com/download`.
- [x] Hosting + deployment approach
  - Answer: static pages deployed via Cloudflare (marketing/docs + web demo).
- [x] Support email provider
  - Answer: Fastmail manages `support@tabulensis.com`.
- [x] CLI binary name
  - Answer: `tabulensis`.

## 2026-01-12 — MVP definition decisions (Section 1)

- [x] Write a 1–2 sentence promise
  - Answer (draft): "Quickly and accurately compare large, real-world Excel files on any OS."
  - Suggestions:
    - Avoid "any size" unless you're confident; consider "large" / "real-world" workbooks and document limits explicitly.
    - Consider naming the outcome (a clear diff you can review) + the differentiator (Git-friendly / cross-platform).
    - Example alternatives to iterate from:
      - "See exactly what changed between two Excel workbooks—fast, accurate, and reviewable (CLI + desktop on Windows/macOS/Linux)."
      - "Turn Excel workbook edits into a meaningful diff you can review and version (Git-friendly output)."

- [x] Pick the primary delivery vehicle for the MVP
  - Answer: Both (CLI + Desktop)
  - Suggestions:
    - This is viable, but it usually means "two packaging + support surfaces"; keep the feature surface identical and treat the desktop app as a UI over the same engine.

- [x] Decide what the web demo is
  - Answer: Marketing/demo only; intended as a funnel into the fuller product.
  - Suggestions:
    - Be explicit on the page about what the demo is/isn't ("try it locally in your browser; nothing is uploaded"; "for full features, install the app/CLI").

- [x] Define the MVP scope boundary (explicit "not in v1" list)
  - Answer: Excel diff + Git integration, CLI + Desktop.
  - Suggestions:
    - Write a short "Not in v1" list so you can say no quickly (e.g., PBIX/PBIT, cloud sync/hosting, enterprise SSO, collaboration features).

- [x] Define supported input formats + clear "unsupported" behavior
  - Answer: `.xls`, `.xlsm`, `.xlsx`, `.xlsb`.
  - Suggestions:
    - Also explicitly document common non-goals/edge cases (password-protected/encrypted files, corrupted files, external links, very large sheets) and what the tool does (error vs partial diff).

- [x] Define the support promise for MVP
  - Answer: "Best-effort email support within 2 business days, no SLA."

- [x] Decide pricing model (pick one for MVP; can change later)
  - Answer: Yearly subscription, $80/year with a free trial (how long should the free trial last?).

- [x] Decide license unit
  - Answer: Per user, with a 2-device activation limit.
  - Enforcing a 2-device limit (practical MVP approach):
    - Define a stable `device_id` per install (e.g., hash of an OS-provided machine identifier); avoid sending raw hardware IDs.
    - On activation, send `{license_key, device_id}` to your licensing backend; store the active device list for that license.
    - Reject activation when `active_devices >= 2` and show a clear next step: deactivate an old device (self-serve) or "replace device".
    - Support deactivation from both surfaces (CLI + desktop) and ideally a tiny web "Manage devices" page.
    - For offline use, issue a signed activation token bound to `device_id` with an expiry (e.g., revalidate every N days) so users aren't bricked offline.
    - Expect churn events (OS reinstall / VM / hardware changes) and plan one escape hatch (self-serve reset, or limited automated resets per month).

- [x] Define "launch success" for the first 30 days
  - Answer: 200 website views; 1+ web demo usage events; 1+ free trial activations; 1+ successful diffs.

- [] Map the funnel on one page:
  - Answer: Landing page → download/install → first successful diff → (optional) upgrade → renewal/update
  
- [x] stand up a basic website you control
  - * [x] Decide the canonical domain + URL structure (`tabulensis.com`)
  - * [x] Choose hosting + deployment approach (Cloudflare static site)
  - * [x] Point DNS to the host and verify HTTPS/TLS works
  - * [ ] Set up `www` and apex redirects + a basic 404 page
  - * [x] Create placeholder pages/URLs you can link to from the product: `/download`, `/docs`, `/support`, `/privacy`, `/terms`
  - * [x] Set up a support email on the domain (Fastmail; `support@tabulensis.com`)
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
- []
  - Answer:
  
