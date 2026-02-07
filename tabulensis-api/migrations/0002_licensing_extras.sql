-- Extend schema for Tabulensis licensing flows:
-- - Store plaintext license key + email for portal + resend flows
-- - Store checkout session mapping for success page lookup
-- - Store device labels for nicer UX

ALTER TABLE licenses ADD COLUMN license_key TEXT;
ALTER TABLE licenses ADD COLUMN email TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_licenses_license_key ON licenses(license_key);
CREATE INDEX IF NOT EXISTS idx_licenses_email ON licenses(email);

ALTER TABLE activations ADD COLUMN device_label TEXT;

CREATE TABLE IF NOT EXISTS checkout_sessions (
  session_id TEXT PRIMARY KEY,
  license_id TEXT,
  license_key TEXT NOT NULL,
  email TEXT,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_checkout_sessions_license_key ON checkout_sessions(license_key);
CREATE INDEX IF NOT EXISTS idx_checkout_sessions_email ON checkout_sessions(email);

