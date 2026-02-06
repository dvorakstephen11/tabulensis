# Stripe + Licensing (Cloudflare Worker/D1) Next Steps

This checklist assumes:
- Website is hosted at `https://tabulensis.com`
- Licensing API is hosted at `https://license.tabulensis.com`
- Backend code lives in `tabulensis-api/` (Cloudflare Worker + D1)

## 1) Cloudflare: Deploy the Worker

1. Ensure the Worker is deployed from `tabulensis-api/`.
2. Bind your custom domain:
   - `license.tabulensis.com` -> the Worker.
3. Confirm the Worker responds:
   - `GET https://license.tabulensis.com/health` returns `{ "status": "ok" }`

## 2) Cloudflare: D1 Migrations

1. Apply D1 migrations for both environments:
   - `tabulensis-api/migrations/0001_init.sql`
   - `tabulensis-api/migrations/0002_licensing_extras.sql`
2. Sanity check the DB has required tables:
   - `licenses`, `activations`, `checkout_sessions`, `stripe_events`

## 3) Cloudflare: Set Secrets/Vars

Set these in Worker env (prod + dev separately):

Stripe:
- `STRIPE_SECRET_KEY`
- `STRIPE_WEBHOOK_SECRET`
- `STRIPE_PRICE_ID`
- `STRIPE_SUCCESS_URL=https://tabulensis.com/download/success`
- `STRIPE_CANCEL_URL=https://tabulensis.com/download`
- `STRIPE_PORTAL_RETURN_URL=https://tabulensis.com/support/billing`
- `STRIPE_TRIAL_DAYS=30`

Licensing:
- `LICENSE_SIGNING_KEY_B64` (32-byte Ed25519 seed, base64)
- `LICENSE_TOKEN_TTL_DAYS=14`
- `LICENSE_PAST_DUE_GRACE_DAYS=3`
- `LICENSE_MAX_DEVICES=2`
- `LICENSE_ADMIN_TOKEN` (optional; enables `/license/reset`)

Dev helpers:
- `LICENSE_MOCK_STRIPE=1` (dev only; simulates Stripe without API calls)

## 4) Stripe Dashboard Setup

1. Create Product + Price (annual):
   - Record `price_id` and set `STRIPE_PRICE_ID`.
2. Enable/configure Customer Portal features you want.
3. Create a webhook endpoint (LIVE + TEST separately) pointing to:
   - `https://license.tabulensis.com/stripe/webhook`
4. Subscribe webhook endpoint to:
   - `checkout.session.completed`
   - `invoice.paid`
   - `invoice.payment_failed`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`

## 5) Website Wiring Verification

The static pages call the licensing API directly:
- Checkout start: `https://license.tabulensis.com/api/checkout/start`
- Checkout status: `https://license.tabulensis.com/api/checkout/status?session_id=...`
- Resend: `https://license.tabulensis.com/license/resend`
- Billing portal: `https://license.tabulensis.com/portal/session`

Validate in browser:
1. Go to `https://tabulensis.com/download`
2. Click “Start trial / Buy”
3. After Stripe checkout completes:
   - You land on `https://tabulensis.com/download/success?session_id=...`
   - The page polls `/api/checkout/status` until it shows a license key and status is `trialing`/`active`

## 6) CLI/Desktop Licensing End-to-End

1. Public key discovery:
   - `tabulensis` fetches `GET https://license.tabulensis.com/public_key` on first run and caches it.
2. Activation:
   - `tabulensis license activate <KEY>`
3. Status:
   - `tabulensis license status`
4. Deactivation:
   - `tabulensis license deactivate`

## 7) Fulfillment: “Resend License” Email (Not Implemented Yet)

Right now `POST /license/resend` returns `{ "status": "queued" }` but does not send email.

Decide and implement:
1. Email provider (Resend/Postmark/SES/etc).
2. Template: key + activation instructions + billing portal link.
3. When to send:
   - On `checkout.session.completed` webhook (primary)
   - On-demand via `/license/resend`

## 8) Downloads: Make `tabulensis.com/download` Actually Serve Binaries

You still need a release publishing path that puts artifacts at:
- `https://tabulensis.com/download/...` (Windows/macOS/Linux, plus checksums)

Checklist:
1. Decide where artifacts are built:
   - CI (recommended) producing `target/dist/*` via `scripts/package_cli_*.py`
2. Decide hosting:
   - Cloudflare R2 + public bucket
   - Cloudflare Pages static assets
   - GitHub Releases mirrored to `tabulensis.com/download`
3. Update `public/download/index.html` “Windows/macOS/Linux release” links to real artifact URLs.
4. Ensure integrity:
   - Publish SHA256 checksums (`scripts/generate_checksums.py`)
5. Optional:
   - Publish Homebrew formula + Scoop manifest at `https://tabulensis.com/download/`

## 9) Live-Mode Validation (Before Announcing)

1. Do a real LIVE checkout with a real card (small internal test).
2. Confirm:
   - webhook received
   - license transitions `pending` -> `trialing` -> `active` (at `invoice.paid`)
   - portal session opens from `/support/billing`
3. Confirm device limit enforcement (2 devices) works as expected.

