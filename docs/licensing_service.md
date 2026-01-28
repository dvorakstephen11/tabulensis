# Licensing Service

This repo includes a minimal licensing backend in `license_service/`.

## Quick start (mock mode)

```bash
export LICENSE_MOCK_STRIPE=1
cargo run -p license_service
```

This starts the service on `0.0.0.0:8080` by default.

## Required environment variables (Stripe mode)

- `STRIPE_SECRET_KEY`
- `STRIPE_WEBHOOK_SECRET`
- `STRIPE_PRICE_ID`
- `STRIPE_SUCCESS_URL`
- `STRIPE_CANCEL_URL`
- `STRIPE_PORTAL_RETURN_URL`
- `LICENSE_SIGNING_KEY_B64` (32-byte Ed25519 private key, base64 encoded)

A starter template lives at `license_service/.env.example`.

Admin helpers:
- `LICENSE_ADMIN_TOKEN` enables the `/license/reset` endpoint.
- Clients should set `TABULENSIS_LICENSE_PUBLIC_KEY` to the value returned by `GET /public_key`.

## Endpoints

- `POST /api/checkout/start`
- `GET /api/checkout/status?session_id=...`
- `POST /stripe/webhook`
- `POST /license/activate`
- `POST /license/deactivate`
- `POST /license/status`
- `POST /license/resend`
- `POST /license/reset` (admin token required via `X-Admin-Token`)
- `POST /portal/session`
- `GET /public_key`
- `GET /health`

## Notes

- Tokens are signed with Ed25519 and include an expiry window for offline grace.
- Device IDs are hashed client-side before submission.
- In mock mode, checkout completion is simulated and licenses are created locally.

## Local Stripe testing

```bash
stripe listen --forward-to localhost:8080/stripe/webhook
stripe trigger checkout.session.completed
```
