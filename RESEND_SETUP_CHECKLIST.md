# Resend Setup Checklist (Tabulensis License Delivery)

This checklist covers setting up transactional license emails for Tabulensis using **Resend**, wired into the Cloudflare Worker licensing backend in `tabulensis-api/`.

## 0) Create Your Resend Account (One-Time)

- [x] Create a Resend account: `https://resend.com`
- [x] Verify your login email address.
- [ ] Recommended: enable 2FA on your Resend account.
- [ ] (Optional) Create an Organization/Team for Tabulensis and invite the other admins/operators.
- [ ] Confirm your plan/billing is sufficient for production sending (you'll also need a verified sending domain below).

## 1) Decide Your Sending Identity

- [x] Pick a sending subdomain (recommended), e.g. `mail.tabulensis.com`.
- [x] Pick a From address, e.g. `Tabulensis <licenses@mail.tabulensis.com>`.
- [x] Pick a Reply-To, e.g. `support@tabulensis.com`.

## 2) Verify Domain In Resend

- [x] Resend dashboard: Domains -> add your domain/subdomain.
- [x] In Cloudflare DNS, add the Resend-provided records:
- [x] SPF (TXT)
- [x] DKIM (Resend-provided value)
- [ ] Recommended: add a DMARC record once SPF/DKIM are in place.
- [x] Wait until the domain shows as verified in Resend.

## 3) Create A Resend API Key

- [x] Create an API key with sending access.
- [ ] Store it in a password manager (do not commit, do not paste into logs).

## 4) Add Worker Secret (Cloudflare)

- [x] Run this command:
From `tabulensis-api/`:
```bash
XDG_CONFIG_HOME=/tmp npx wrangler secret put RESEND_API_KEY
```

If you're using the `dev` env (`wrangler dev --env dev`), also set:
```bash
XDG_CONFIG_HOME=/tmp npx wrangler secret put RESEND_API_KEY --env dev
```

Optional non-secret vars (set via Cloudflare dashboard or Wrangler vars):

- [x] `RESEND_FROM` (e.g. `Tabulensis <licenses@mail.tabulensis.com>`)
- [x] `RESEND_REPLY_TO` (e.g. `support@tabulensis.com`)

## 5) Implement Email Sending In `tabulensis-api/`

- [x] Add dependency (recommended):
- [x] `npm install resend`
- [x] Add Worker env support:
- [x] `RESEND_API_KEY` (secret)
- [x] `RESEND_FROM` (var, or hardcode)
- [x] Implement `sendLicenseEmail(to, licenseKey, ...)` using Resend send-email API (HTML + text).
- [x] Add an **Idempotency-Key** so retries don't double-send.
- [x] Recommended key: `license-email/stripe-event/<event.id>` for webhook sends.

Wire email sending into:

- [x] Stripe webhook `checkout.session.completed`:
- [x] Use Stripe-provided email (if present) to send the license key.
- [x] `POST /license/resend`:
- [x] Look up license by `email` or `license_key`.
- [x] Send to the email stored on the license record.
- [x] If email is missing, return a clear error instructing the user to contact support.

## 6) Email Content (Minimum Good License Email)

- [x] Subject: `Your Tabulensis license key`
- [x] Include the license key (copy/paste friendly)
- [x] Include download link: `https://tabulensis.com/download`
- [x] Include activation command: `tabulensis license activate <KEY>`
- [x] Include billing link: `https://tabulensis.com/support/billing`
- [x] Include support contact: `support@tabulensis.com`
- [x] Include both HTML and plain-text bodies (recommended)

## 7) Test Before Sending Real Customer Email

- [ ] Use Resend test recipients to validate behavior (deliver/bounce/complaint).
- [ ] After domain verification, send to addresses you control and confirm delivery + formatting.

## 8) Deploy + Validate End-to-End

- [ ] Deploy Worker:

```bash
cd tabulensis-api
XDG_CONFIG_HOME=/tmp npx wrangler deploy --domain license.tabulensis.com
```

- [ ] Run a Stripe TEST checkout via `https://tabulensis.com/download`.
- [ ] Confirm:
- [ ] `https://tabulensis.com/download/success?session_id=...` displays a license key
- [ ] The license email arrives and matches the displayed key
- [ ] `https://tabulensis.com/support/resend` successfully resends the key
