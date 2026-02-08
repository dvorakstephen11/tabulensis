# Operational Essentials

- Daily operator routine + automation goals: `meta_methodology.md`.
- Store Stripe secrets and webhook signing secret in a secrets manager.
- Monitor `license_service` logs for webhook failures and activation errors.
- Chargeback or abuse: set license status to canceled and reset activations via
  `POST /license/reset` (requires `LICENSE_ADMIN_TOKEN`).
- Support intake: support@tabulensis.com with a 1 business day SLA.
