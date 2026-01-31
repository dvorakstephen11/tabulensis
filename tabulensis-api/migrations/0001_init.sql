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
