# Tabulensis — Launch To‑Dos (from our chat)

> Scope: Only the checklist/to‑do items we discussed in this conversation. Excludes any content from your uploaded MVP checklist docs and excludes the legal document text you already implemented.

## 0) One-page goal
A stranger can **pay → receive a license → download → activate → run diffs**, and you can **revoke/renew** automatically.

---

## 1) Banking + bookkeeping setup
- [ ] Decide timeline for entity formation (sole prop now vs LLC soon)
- [ ] Open a dedicated account for product finances
  - [ ] If pre‑LLC: separate personal checking dedicated to Tabulensis ops
  - [ ] Post‑LLC: business checking under the LLC
- [ ] Get a business credit card (recommended) and run nearly all expenses through it
- [ ] (Optional) Create a tax reserve savings account and auto‑transfer a percentage of revenue
- [ ] Pick bookkeeping system (even simple at first) and track by product using categories/classes

---

## 2) Stripe integration (before binaries are available)
### 2.1 Stripe account + payouts
- [ ] Create Stripe account
- [ ] Complete business details and any identity verification
- [ ] Add Regions bank account as payout destination

### 2.2 Product, price, trial
- [ ] Create “Tabulensis” product in Stripe
- [ ] Create recurring price (e.g., $80/year)
- [ ] Configure 30‑day trial behavior (trial in subscription checkout)

### 2.3 Choose integration style
- [ ] Use Stripe Checkout (fastest for static sites)
- [ ] Add a small backend (Cloudflare Worker recommended) to:
  - [ ] Create Checkout sessions
  - [ ] Receive Stripe webhooks
  - [ ] Issue and manage licenses

### 2.4 Backend endpoints
- [ ] Implement `POST /api/checkout/start`
  - Creates Stripe Checkout Session (subscription + 30‑day trial)
  - Returns Checkout URL/session id
- [ ] Implement `POST /stripe/webhook`
  - Verifies webhook signature
  - Updates license state

### 2.5 Webhook events to handle
- [ ] `checkout.session.completed` (create customer + license, attach Stripe IDs)
- [ ] `invoice.paid` (mark paid/active)
- [ ] `invoice.payment_failed` (mark past_due and tighten grace if desired)
- [ ] `customer.subscription.updated` (track status changes)
- [ ] `customer.subscription.deleted` (revoke)

### 2.6 Test end-to-end
- [ ] Use Stripe test mode for the full purchase/trial lifecycle
- [ ] Use Stripe CLI to forward webhooks locally during development
- [ ] Confirm the loop: checkout → webhook → license issued → app activates

### 2.7 Go live checklist
- [ ] Create live keys and live webhook endpoint
- [ ] Deploy Worker with live secrets
- [ ] Flip the website “Buy” flow to live mode only when fulfillment is real

---

## 3) License enforcement (how you stop “download and use free”)
### 3.1 Decide trial policy
- [ ] Recommended: trial starts only through Stripe checkout (no local/offline-only trial loophole)

### 3.2 Data model (minimum)
- [ ] License record: `license_key`, Stripe customer/subscription ids, `status`, trial end, period end, `max_devices=2`
- [ ] Activation record: `license_key`, `device_id`, activated_at, last_seen_at, revoked_at

### 3.3 Licensing API
- [ ] `POST /license/activate`:
  - Validates license status (trialing/active)
  - Enforces 2‑device max
  - Returns a signed activation token bound to `{license_key, device_id}`
- [ ] `POST /license/deactivate`:
  - Frees a device slot
- [ ] (Optional) `GET /license/status`:
  - Helpful for UX and debugging

### 3.4 Device identity
- [ ] Generate a stable `device_id` locally
- [ ] Hash/derive identifiers to avoid transmitting raw hardware ids
- [ ] Provide a path to replace devices (deactivate, or support-assisted reset)

### 3.5 Token + offline grace
- [ ] Use a signed token with an expiry (recommended offline validity: 7–14 days)
- [ ] Store token in OS-appropriate config/app data directory
- [ ] On expiry: attempt refresh; if cannot refresh and expired → restrict paid operations

### 3.6 Enforce in both products
- [ ] CLI:
  - [ ] Add `tabulensis license activate <KEY>`
  - [ ] Add `tabulensis license status`
  - [ ] Add `tabulensis license deactivate`
  - [ ] Before any real diff: require valid token (else exit non-zero with clear message)
- [ ] Desktop:
  - [ ] Add Settings/About → License page (enter key, activate/deactivate, show status)
  - [ ] Before running diffs: require valid token (else show clear CTA)

### 3.7 Stripe → license state sync
- [ ] In webhook handler, move license between: trialing → active → past_due → canceled
- [ ] Define what “past_due” means in-app (e.g., shorten grace or disable paid ops)

### 3.8 Minimal customer support ops
- [ ] Ability to resend license key
- [ ] Ability to reset activations (support tool or admin endpoint)

---

## 4) Website flow (purchase → download → activate)
- [ ] Ensure `/download` clearly provides:
  - [ ] Buy/Start trial button → Stripe Checkout
  - [ ] Post-checkout “success” page that explains activation steps
  - [ ] Links to legal pages (already done)
- [ ] Decide how license key is delivered:
  - [ ] Show on success page
  - [ ] Email delivery (recommended)
  - [ ] Both
- [ ] Add “Resend license” flow (even if email-only support at first)
- [ ] Add cancellation instructions (support contact or portal link)

---

## 5) Release artifacts + distribution
### 5.1 Artifacts
- [ ] Produce Windows, macOS, and Linux build outputs
- [ ] Package per platform (zip/tar.gz/installer as appropriate)
- [ ] Publish checksums for each artifact
- [ ] Maintain versioning and a changelog
- [ ] Include `LICENSE.txt` (proprietary notice) and `THIRD_PARTY_NOTICES` in every release bundle

### 5.2 Hosting downloads
- [x] Decide where artifacts live (Cloudflare R2, GitHub Releases, etc.) (Chosen: GitHub Releases; see `docs/meta/results/decision_register.md` `DR-0019`)
- [x] Ensure `/download` points to the correct versions (implemented via stable `releases/latest/download/tabulensis-latest-*` links)

---

## 6) Signing/notarization (recommended; not strictly required to automate)
### macOS
- [ ] Apple Developer Program membership
- [ ] Developer ID signing
- [ ] Notarization in CI + stapling

### Windows
- [ ] Azure Artifact Signing (Trusted Signing) account + certificate profile configured
- [x] Decide key handling strategy for CI (Chosen: Azure Artifact Signing via GitHub OIDC + Environment approvals; see `docs/meta/results/decision_register.md` `DR-0020`)

---

## 7) Operational essentials (lightweight but real)
- [ ] Secrets management: keep Stripe secrets + webhook signing secret out of git
- [ ] Logging/monitoring for webhook failures and activation errors
- [ ] Basic chargeback/abuse handling policy (what triggers revocation)
- [ ] Support intake process (single email + canned responses is fine)

---

## 8) Critical path to first legit sale (minimal)
- [ ] Stripe Checkout live + payouts configured
- [ ] Webhooks wired and verified
- [ ] License issuance + activation working
- [ ] CLI + Desktop enforce license before running diffs
- [ ] Download page + post-checkout instructions live
- [ ] At least one platform release artifact available (Windows is often the first)

---

## 9) After launch (high ROI improvements)
- [ ] Self-serve customer portal (cancel, invoice, update card)
- [ ] Self-serve deactivate/reset device slots
- [ ] Auto-updater strategy
- [ ] Better installer UX (MSI/DMG/PKG)
- [ ] Code signing/notarization if not done pre-launch
