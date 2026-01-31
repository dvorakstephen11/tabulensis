Below is the **full Stripe + licensing checklist** (end-to-end), updated for your decisions:

* **Storage:** Cloudflare **D1**
* **Trial:** **30 days**, and **requires card up front** (Checkout collects payment method by default; you only *disable* that by setting `payment_method_collection=if_required`). ([Stripe Docs][1])

---

# Stripe + Licensing Checklist (D1 + card-up-front trial)

## 0) Architecture decisions (locked in)

* [x] **Stripe Checkout** for subscriptions
* [x] **Cloudflare Worker** (or Pages Function) as your backend API
* [x] **Cloudflare D1** for license + activation state
* [x] **Webhook-driven** state sync (Stripe → your DB)
* [x] Offline access via **signed activation tokens** (time-limited)

---

## 1) Stripe setup (Dashboard)

### 1.1 Business + payouts

* [ ] Stripe Dashboard → Settings → Business details: confirm **LLC + EIN** are correct
* [ ] Settings → Payouts/Bank accounts: ensure **Regions business checking** is set as payout destination

### 1.2 Product, price, and trial

* [ ] Products → Create product “Tabulensis”
* [ ] Add recurring price: **$80 / year**
* [ ] Confirm you will run the **30-day trial via Checkout Session creation** (`subscription_data[trial_period_days]=30`) ([Stripe Docs][1])

### 1.3 Customer Portal (strongly recommended)

This avoids building cancellation/card-update flows yourself.

* [ ] Billing → Customer portal (or Settings → Customer portal): configure features you want customers to self-serve
* [ ] Your backend will create **portal sessions** on demand (see Section 4.6) ([Stripe Docs][2])

---

## 2) Cloudflare backend project setup (Worker + D1)

> Recommended: keep your website repo as Pages; create a separate repo for API, e.g. `tabulensis-api`.

### 2.1 Initialize Worker project

* [ ] `wrangler init tabulensis-api` (choose TypeScript or JS)
* [ ] Set a modern `compatibility_date`
* [ ] (Recommended) enable Node compat for easier crypto + Stripe SDK usage:

  * [ ] `nodejs_compat` compatibility flag (so you can use `node:crypto`) ([Cloudflare Docs][3])

### 2.2 Create D1 databases

* [ ] Create prod DB:

  * [ ] `wrangler d1 create tabulensis_licensing_prod`
* [ ] Create dev DB (or use preview DB):

  * [ ] `wrangler d1 create tabulensis_licensing_dev`

### 2.3 Bind D1 in `wrangler.jsonc`

* [ ] Add a `d1_databases` binding so your Worker can access it via `env.<BINDING_NAME>` ([Cloudflare Docs][4])
  (Also include `preview_database_id` so dev/testing is isolated.) ([Cloudflare Docs][5])

### 2.4 Migrations

* [ ] Use D1 migrations in `migrations/` (default) ([Cloudflare Docs][5])
* [ ] Create an initial migration:

  * [ ] `wrangler d1 migrations create tabulensis_licensing_prod init`
* [ ] Apply migrations:

  * [ ] `wrangler d1 migrations apply tabulensis_licensing_prod`
  * [ ] `wrangler d1 migrations apply tabulensis_licensing_dev`

(Cloudflare documents the migrations folder + applied migration tracking behavior.) ([Cloudflare Docs][5])

---

## 3) D1 schema (what to create in the init migration)

### 3.1 Tables (minimum robust set)

* [ ] `licenses`

  * license identity + status + Stripe linkage + device limit
* [ ] `activations`

  * device activations with 2-device enforcement
* [ ] `stripe_events`

  * webhook idempotency (so retries don’t create duplicates)
* [ ] `license_keys`

  * store only **hashes** of license keys (never plaintext)

### 3.2 Suggested schema (SQLite / D1)

Put this in your initial migration:

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS licenses (
  id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,

  status TEXT NOT NULL, -- trialing | active | past_due | canceled | revoked

  stripe_customer_id TEXT,
  stripe_subscription_id TEXT UNIQUE,

  trial_end INTEGER,
  current_period_end INTEGER,

  max_devices INTEGER NOT NULL DEFAULT 2
);

CREATE TABLE IF NOT EXISTS license_keys (
  license_id TEXT PRIMARY KEY REFERENCES licenses(id) ON DELETE CASCADE,
  key_hash TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS activations (
  id TEXT PRIMARY KEY,
  license_id TEXT NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,

  device_id_hash TEXT NOT NULL,

  activated_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  revoked_at INTEGER,

  UNIQUE(license_id, device_id_hash)
);

CREATE INDEX IF NOT EXISTS idx_activations_license_id ON activations(license_id);
CREATE INDEX IF NOT EXISTS idx_activations_device_id_hash ON activations(device_id_hash);

CREATE TABLE IF NOT EXISTS stripe_events (
  id TEXT PRIMARY KEY, -- Stripe event.id
  type TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  processed_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_stripe_events_type ON stripe_events(type);
```

---

## 4) Backend endpoints (Worker routes)

### 4.1 Secrets + config (DO THIS EARLY)

Never put secrets in `wrangler.jsonc` vars; use secrets. ([Cloudflare Docs][6])

* [ ] `STRIPE_SECRET_KEY` (test + live per environment)
* [ ] `STRIPE_WEBHOOK_SECRET` (test + live per environment)
* [ ] `STRIPE_PRICE_ID_YEARLY`
* [ ] `APP_ORIGIN` ([https://tabulensis.com](https://tabulensis.com))
* [ ] `LICENSE_SIGNING_PRIVATE_KEY` (see Section 5)
* [ ] `LICENSE_SIGNING_KEY_ID` (optional but helpful)
* [ ] `LICENSE_TOKEN_TTL_SECONDS` (e.g. 1209600 for 14 days)

### 4.2 `POST /api/stripe/checkout/start`

Purpose: create a Stripe Checkout Session (subscription + trial).

* [ ] Input: optional `{ email }` (you can also let Checkout collect it)
* [ ] Create Checkout session with:

  * `mode=subscription`
  * `line_items[0][price]=STRIPE_PRICE_ID_YEARLY`
  * `line_items[0][quantity]=1`
  * `subscription_data[trial_period_days]=30` ([Stripe Docs][1])
  * (Explicit) `payment_method_collection=always` (card up front) ([Stripe Docs][7])
  * `success_url=https://tabulensis.com/success?session_id={CHECKOUT_SESSION_ID}`
  * `cancel_url=https://tabulensis.com/download?canceled=1`

### 4.3 `GET /api/stripe/checkout/session?session_id=...` (optional but useful)

Purpose: on your success page, exchange session_id for your “activation instructions”.

* [ ] Returns: “license key displayed?” / “email sent?” / “next steps”.

### 4.4 `POST /api/stripe/webhook`

Purpose: Stripe calls this; you verify signature and update your DB.

* [ ] Verify webhook signatures using the `Stripe-Signature` header + endpoint secret, **using the raw request body** (Stripe warns that body manipulation breaks verification). ([Stripe Docs][8])
* [ ] Implement idempotency:

  * [ ] Insert `event.id` into `stripe_events` first; if it already exists, return 200.

### 4.5 Webhook events to subscribe to (test + live)

Minimum robust set:

* [ ] `checkout.session.completed`
* [ ] `invoice.paid`
* [ ] `invoice.payment_failed`
* [ ] `customer.subscription.updated`
* [ ] `customer.subscription.deleted`

Stripe’s subscription webhook guidance explains handling renewals and failures via invoice events and subscription status changes. ([Stripe Docs][9])

### 4.6 `POST /api/stripe/customer-portal/session`

Purpose: “Manage billing” button (cancel/update card).

* [ ] Input: identify the customer (email lookup or license key)
* [ ] Create a Stripe Billing Portal session and return its URL ([Stripe Docs][2])

---

## 5) License issuance + lifecycle (Stripe → D1)

### 5.1 On `checkout.session.completed`

* [ ] Extract:

  * `customer` id
  * `subscription` id
* [ ] Create `licenses` row:

  * `status = trialing` (usually)
  * store Stripe ids
* [ ] Generate a license key:

  * [ ] cryptographically random (at least 128 bits)
  * [ ] store only `hash(key)` in `license_keys.key_hash`
* [ ] Deliver license key:

  * [ ] Show on success page
  * [ ] Email (recommended)

### 5.2 On `invoice.paid`

* [ ] Set license `status=active`
* [ ] Update `current_period_end` from the subscription/invoice data

### 5.3 On `invoice.payment_failed`

* [ ] Set license `status=past_due` (and decide behavior in app: grace vs lock)
  Stripe notes payment failures can be temporary or final; your app should react via these events. ([Stripe Docs][9])

### 5.4 On `customer.subscription.deleted`

* [ ] Set license `status=canceled` (or `revoked`)
* [ ] Optionally mark existing activations as revoked on next refresh

### 5.5 Optional: trial reminders

Even with card up front, reminders reduce churn:

* [ ] Listen to `customer.subscription.trial_will_end`
* [ ] Email a “trial ending soon” message
  Stripe’s Checkout free-trial docs describe reminder approaches and mention trial compliance requirements. ([Stripe Docs][1])

---

## 6) Activation + enforcement (what stops “download and use free”)

### 6.1 Device identity

* [ ] Compute a stable `device_id` locally (no raw hardware IDs exposed)
* [ ] Hash it before sending to server (`device_id_hash`)
* [ ] Persist device_id locally so it survives reboots

### 6.2 `POST /api/license/activate`

Input: `{ license_key, device_id }`
Server logic:

* [ ] Hash `license_key` and look up license
* [ ] Confirm license status is `trialing` or `active`
* [ ] Enforce 2-device limit:

  * [ ] Count `activations` where `revoked_at IS NULL`
  * [ ] If device already activated: allow + update `last_seen_at`
  * [ ] Else if count >= 2: reject with “device limit reached” + next steps
  * [ ] Else insert activation row

Return:

* [ ] A **signed activation token** (offline-capable) containing:

  * `license_id`
  * `device_id_hash`
  * `status`
  * `exp` (expiry)
  * `iat` (issued at)
  * `kid` (key id)

### 6.3 `POST /api/license/deactivate`

Input: `{ license_key, device_id }`

* [ ] Set `revoked_at` for that activation
* [ ] Return success

### 6.4 Token signing (robust offline licensing)

Goal: app can verify locally without any secret.

* [ ] Use asymmetric signing:

  * Worker stores **private key** as a secret
  * App ships **public key**
* [ ] Use `node:crypto` in Workers (supported with `nodejs_compat`) for signing/verification primitives. ([Cloudflare Docs][3])
* [ ] Token TTL (offline grace): 7–14 days recommended
* [ ] App refreshes token periodically and on expiration

### 6.5 App-side enforcement points

* [ ] CLI:

  * [ ] `tabulensis license activate <KEY>`
  * [ ] `tabulensis license status`
  * [ ] `tabulensis license deactivate`
  * [ ] Block all real diff operations unless token verifies + not expired
* [ ] Desktop:

  * [ ] License screen: enter key, activate/deactivate, show expiry + devices used
  * [ ] Block diff run unless token verifies + not expired

### 6.6 Past-due policy (make a decision and enforce consistently)

* [ ] Decide how `past_due` behaves:

  * Option A: allow until token expires, then lock until paid
  * Option B: shorten token TTL immediately on `past_due`
* [ ] Implement the chosen behavior in `/license/activate` and in app UI messaging

---

## 7) Local development + environments (test vs live)

### 7.1 Local secrets

* [ ] Put local secrets in `.dev.vars` or `.env` (do not commit) ([Cloudflare Docs][10])

### 7.2 Environments

* [ ] Create `dev` and `prod` environments in Wrangler and define env-specific secrets/vars (Cloudflare notes secrets are environment-scoped). ([Cloudflare Docs][11])
* [ ] Use Stripe **test mode** keys + webhook secret in dev
* [ ] Use Stripe **live mode** keys + webhook secret in prod

### 7.3 Webhook testing

* [ ] Use Stripe CLI to forward webhooks to your local Worker dev server
* [ ] Confirm signature verification and idempotency behave correctly (Stripe will retry webhooks; your endpoint must be resilient). ([Stripe Docs][8])

---

## 8) Website wiring (minimal)

* [ ] `/download` has “Start trial” button → calls your backend `/stripe/checkout/start`
* [ ] `/success` page:

  * [ ] reads `session_id` and shows next steps
  * [ ] displays license key and/or indicates “we emailed it”
* [ ] “Manage billing” button:

  * [ ] calls `/stripe/customer-portal/session` and redirects user to Stripe portal ([Stripe Docs][2])

---

## 9) Go-live cutover checklist

* [ ] Stripe live product/price created (or ensure live IDs exist)
* [ ] Live webhook endpoint configured with the same event list (Stripe has separate test/live webhook configs)
* [ ] Worker deployed with prod secrets using `wrangler secret put` (and environment-specific secrets if applicable) ([Cloudflare Docs][6])
* [ ] D1 prod database migrated and verified
* [ ] End-to-end test in live mode with a real card (small charge at trial end won’t happen immediately, but session + license issuance should)

---

## 10) “Unlocked” by your decisions (specific defaults you should now adopt)

### Because you chose D1

* [ ] Use D1 migrations + `env.DB.prepare(...).bind(...).run()` style queries ([Cloudflare Docs][12])
* [ ] Store Stripe event IDs (`stripe_events`) to handle webhook retries safely
* [ ] Use unique constraints on `stripe_subscription_id` and `(license_id, device_id_hash)` to prevent duplicates

### Because trial requires card up front

* [ ] Do **not** set `payment_method_collection=if_required`
* [ ] Either omit `payment_method_collection` (default collects payment method) or explicitly set `payment_method_collection=always` ([Stripe Docs][1])
* [ ] You can skip the “missing payment method” trial end behavior entirely, because it’s only relevant when you *don’t* collect payment method up front ([Stripe Docs][1])

---

If you want, I can take the above and turn it into a **single, executable implementation plan** (exact D1 migrations, exact Worker route contracts, and a minimal Rust client module API for CLI + desktop). To do that cleanly, tell me one thing: do you want the API hosted as **a dedicated Worker** (recommended) or as **Pages Functions** inside your website project? Both can bind to D1. ([Cloudflare Docs][13])

[1]: https://docs.stripe.com/payments/checkout/free-trials?utm_source=chatgpt.com "Configure free trials | Stripe Documentation"
[2]: https://docs.stripe.com/customer-management/integrate-customer-portal?utm_source=chatgpt.com "Integrate the customer portal with the API | Stripe Documentation"
[3]: https://developers.cloudflare.com/workers/runtime-apis/nodejs/crypto/?utm_source=chatgpt.com "crypto · Cloudflare Workers docs"
[4]: https://developers.cloudflare.com/d1/get-started/?utm_source=chatgpt.com "Getting started · Cloudflare D1 docs"
[5]: https://developers.cloudflare.com/d1/reference/migrations/?utm_source=chatgpt.com "Migrations · Cloudflare D1 docs"
[6]: https://developers.cloudflare.com/workers/configuration/secrets/?utm_source=chatgpt.com "Secrets · Cloudflare Workers docs"
[7]: https://docs.stripe.com/api/checkout/sessions/create?utm_source=chatgpt.com "Create a Checkout Session | Stripe API Reference"
[8]: https://docs.stripe.com/webhooks/test?utm_source=chatgpt.com "Receive Stripe events in your webhook endpoint | Stripe Documentation"
[9]: https://docs.stripe.com/billing/subscriptions/webhooks?utm_source=chatgpt.com "Using webhooks with subscriptions | Stripe Documentation"
[10]: https://developers.cloudflare.com/workers/development-testing/environment-variables/?utm_source=chatgpt.com "Environment variables and secrets · Cloudflare Workers docs"
[11]: https://developers.cloudflare.com/workers/wrangler/environments/?utm_source=chatgpt.com "Environments · Cloudflare Workers docs"
[12]: https://developers.cloudflare.com/d1/worker-api/d1-database/?utm_source=chatgpt.com "D1 Database · Cloudflare D1 docs"
[13]: https://developers.cloudflare.com/pages/functions/bindings/?utm_source=chatgpt.com "Bindings · Cloudflare Pages docs"
