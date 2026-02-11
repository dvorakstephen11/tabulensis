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

Note (repo reality): non-sensitive vars are committed in `tabulensis-api/wrangler.jsonc` and will be applied on deploy.
You still need to set secrets (Stripe keys, signing key, etc) in Cloudflare.

Stripe:
- `STRIPE_SECRET_KEY`
- `STRIPE_WEBHOOK_SECRET`
- `STRIPE_PRICE_ID`
- `STRIPE_SUCCESS_URL=https://tabulensis.com/download/success` (var; default set in `tabulensis-api/wrangler.jsonc`)
- `STRIPE_CANCEL_URL=https://tabulensis.com/download` (var; default set in `tabulensis-api/wrangler.jsonc`)
- `STRIPE_PORTAL_RETURN_URL=https://tabulensis.com/support/billing` (var; default set in `tabulensis-api/wrangler.jsonc`)
- `STRIPE_TRIAL_DAYS=30` (var; default set in `tabulensis-api/wrangler.jsonc`)

Licensing:
- `LICENSE_SIGNING_KEY_B64` (32-byte Ed25519 seed, base64)
- `LICENSE_TOKEN_TTL_DAYS=14` (var; default set in `tabulensis-api/wrangler.jsonc`)
- `LICENSE_PAST_DUE_GRACE_DAYS=3` (var; default set in `tabulensis-api/wrangler.jsonc`)
- `LICENSE_MAX_DEVICES=2` (var; default set in `tabulensis-api/wrangler.jsonc`)
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

## 7) Fulfillment: “Resend License” Email

License emails are sent via **Resend** when configured.

Required Worker settings:
1. Secret: `RESEND_API_KEY`
2. Vars: `RESEND_FROM`, `RESEND_REPLY_TO`

Sending behavior:
1. Primary: on `checkout.session.completed` webhook (idempotency-keyed by Stripe event id).
2. On-demand: `POST /license/resend` (lookup by `email` or `license_key`).

Setup checklist: `RESEND_SETUP_CHECKLIST.md`

## 8) Downloads: Make `tabulensis.com/download` Actually Serve Binaries

The download page is a Cloudflare Pages static page:
- `https://tabulensis.com/download/`

Binary downloads are served from your domain via a Worker route:
- `https://tabulensis.com/dl/<asset>` (handled by the licensing Worker)

The Worker can serve downloads from:
- Recommended: an R2 bucket bound as `DOWNLOAD_BUCKET` (keeps the origin private).
- Alternative: an origin URL base set via `DOWNLOAD_ORIGIN_BASE_URL` (can be GitHub Releases, R2 public HTTP endpoint, S3, etc).

Checklist:
1. Decide where artifacts are built:
   - CI (recommended): `.github/workflows/release.yml` produces stable `tabulensis-latest-*` assets as workflow artifacts (and can optionally upload them to R2).
   - Local/manual (fine for MVP): put the stable `tabulensis-latest-*` files in `target/dist_latest/` in this repo.
2. Decide hosting:
   - Cloudflare R2 + public bucket
   - Cloudflare Pages static assets
   - GitHub Releases (only as an implementation detail behind the proxy, if you want)
3. Create the Worker route for downloads:
   - Route: `tabulensis.com/dl/*` (and optionally `www.tabulensis.com/dl/*`) -> `tabulensis-api`
   - Note: this is already in `tabulensis-api/wrangler.jsonc` and the root `wrangler.jsonc`.
4. R2 setup (recommended):
   - Cloudflare Dashboard: enable R2 (Wrangler commands fail with API code `10042` until you do).
   - Create R2 buckets:
     - Prod: `tabulensis-downloads`
     - Dev: `tabulensis-downloads-dev`
     - Example commands:
       - `npx wrangler r2 bucket create tabulensis-downloads`
       - `npx wrangler r2 bucket create tabulensis-downloads-dev`
   - Bind it to the Worker as `DOWNLOAD_BUCKET` (see `tabulensis-api/src/index.ts`).
     - This repo already declares the binding:
       - `tabulensis-api/wrangler.jsonc` and the root `wrangler.jsonc`
       - Prod bucket: `tabulensis-downloads`
       - Dev bucket: `tabulensis-downloads-dev`
     - Note: `r2_buckets` is not inherited into named envs; that’s why `env.dev` also has its own `r2_buckets`.
   - Upload release artifacts into that bucket with keys matching the asset filenames, for example:
     - `tabulensis-latest-windows-x86_64.exe`
     - `tabulensis-latest-windows-x86_64.zip`
     - `tabulensis-latest-macos-universal.tar.gz`
     - `tabulensis-latest-linux-x86_64.tar.gz`
     - Note: these stable filenames are what the download page links to. Locally, you typically copy/zip from a build output like `target/release-cli/tabulensis` (Linux/macOS) or `target-windows/release-cli/tabulensis.exe` (Windows) into `target/dist_latest/`.
     - Local upload helper (artifact dir is the repo-local folder containing those stable filenames):
       - `./scripts/upload_downloads_to_r2.sh tabulensis-downloads target/dist_latest`
   - Optional: automate uploads on tag releases via GitHub Actions:
     - `.github/workflows/release.yml` includes an optional “Upload latest assets to R2” step.
     - Set GitHub repo secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` (and optionally `R2_DOWNLOAD_BUCKET`).
5. Origin URL setup (if not using R2 binding):
   - Set `DOWNLOAD_ORIGIN_BASE_URL` to a base URL like `https://example.com/releases/latest/download/`.
6. Update the download page links (already done):
   - `public/download/index.html` links to `/dl/<asset>` (same-domain downloads).
7. Ensure integrity:
   - Publish SHA256 checksums (`scripts/generate_checksums.py`)
8. Optional:
   - Publish Homebrew formula + Scoop manifest at `https://tabulensis.com/download/`

## 9) Live-Mode Validation (Before Announcing)

1. Do a real LIVE checkout with a real card (small internal test).
2. Confirm:
   - webhook received
   - license transitions `pending` -> `trialing` -> `active` (at `invoice.paid`)
   - portal session opens from `/support/billing`
3. Confirm device limit enforcement (2 devices) works as expected.
