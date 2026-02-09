-- OpStore schema v2
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS diff_runs (
  diff_id TEXT PRIMARY KEY,
  old_path TEXT NOT NULL,
  new_path TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  config_json TEXT,
  engine_version TEXT,
  app_version TEXT,
  schema_version INTEGER NOT NULL,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  trusted INTEGER NOT NULL DEFAULT 0,
  complete INTEGER NOT NULL DEFAULT 0,
  op_count INTEGER NOT NULL DEFAULT 0,
  warnings_count INTEGER NOT NULL DEFAULT 0,
  added_count INTEGER NOT NULL DEFAULT 0,
  removed_count INTEGER NOT NULL DEFAULT 0,
  modified_count INTEGER NOT NULL DEFAULT 0,
  moved_count INTEGER NOT NULL DEFAULT 0,
  strings_json TEXT
);

CREATE TABLE IF NOT EXISTS diff_warnings (
  diff_id TEXT NOT NULL,
  idx INTEGER NOT NULL,
  text TEXT NOT NULL,
  PRIMARY KEY (diff_id, idx),
  FOREIGN KEY (diff_id) REFERENCES diff_runs(diff_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS diff_ops (
  diff_id TEXT NOT NULL,
  op_idx INTEGER NOT NULL,
  kind TEXT NOT NULL,
  sheet_id INTEGER,
  row INTEGER,
  col INTEGER,
  row_end INTEGER,
  col_end INTEGER,
  move_id TEXT,
  payload_json TEXT NOT NULL,
  PRIMARY KEY (diff_id, op_idx),
  FOREIGN KEY (diff_id) REFERENCES diff_runs(diff_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_diff_ops_sheet ON diff_ops(diff_id, sheet_id, op_idx);
CREATE INDEX IF NOT EXISTS idx_diff_ops_kind ON diff_ops(diff_id, kind);
CREATE INDEX IF NOT EXISTS idx_diff_ops_cell ON diff_ops(diff_id, sheet_id, row, col);

CREATE TABLE IF NOT EXISTS diff_sheets (
  diff_id TEXT NOT NULL,
  sheet_id INTEGER NOT NULL,
  sheet_name TEXT NOT NULL,
  op_count INTEGER NOT NULL,
  added_count INTEGER NOT NULL,
  removed_count INTEGER NOT NULL,
  modified_count INTEGER NOT NULL,
  moved_count INTEGER NOT NULL,
  PRIMARY KEY (diff_id, sheet_id),
  FOREIGN KEY (diff_id) REFERENCES diff_runs(diff_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS batch_runs (
  batch_id TEXT PRIMARY KEY,
  old_root TEXT NOT NULL,
  new_root TEXT NOT NULL,
  strategy TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  status TEXT NOT NULL,
  item_count INTEGER NOT NULL,
  completed_count INTEGER NOT NULL,
  error TEXT
);

CREATE TABLE IF NOT EXISTS batch_items (
  batch_id TEXT NOT NULL,
  item_id INTEGER NOT NULL,
  old_path TEXT,
  new_path TEXT,
  status TEXT NOT NULL,
  diff_id TEXT,
  error TEXT,
  op_count INTEGER,
  warnings_count INTEGER,
  duration_ms INTEGER,
  PRIMARY KEY (batch_id, item_id),
  FOREIGN KEY (batch_id) REFERENCES batch_runs(batch_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workbook_indexes (
  index_id TEXT PRIMARY KEY,
  path TEXT NOT NULL,
  mtime INTEGER NOT NULL,
  size INTEGER NOT NULL,
  side TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS cell_docs USING fts5(index_id, sheet, addr, kind, text);

-- Diff domains (Iteration 2+): lets the UI pick the right navigator/viewer without guessing.
CREATE TABLE IF NOT EXISTS diff_domains (
  diff_id TEXT PRIMARY KEY,
  domain TEXT NOT NULL,
  FOREIGN KEY (diff_id) REFERENCES diff_runs(diff_id) ON DELETE CASCADE
);

-- PBIP/PBIR/TMDL (Iteration 2): document-level diffs and (optional) extracted entities.
CREATE TABLE IF NOT EXISTS diff_pbip_docs (
  diff_id TEXT NOT NULL,
  path TEXT NOT NULL,
  doc_type TEXT NOT NULL,
  change_kind TEXT NOT NULL,
  impact_hint TEXT,
  normalization_profile TEXT NOT NULL,
  normalization_applied TEXT NOT NULL,
  old_hash INTEGER,
  new_hash INTEGER,
  old_error TEXT,
  new_error TEXT,
  old_text TEXT,
  new_text TEXT,
  PRIMARY KEY (diff_id, path),
  FOREIGN KEY (diff_id) REFERENCES diff_runs(diff_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_diff_pbip_docs ON diff_pbip_docs(diff_id, path);

CREATE TABLE IF NOT EXISTS diff_pbip_entities (
  diff_id TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  doc_path TEXT NOT NULL,
  entity_kind TEXT NOT NULL,
  label TEXT NOT NULL,
  change_kind TEXT NOT NULL,
  pointer TEXT,
  impact_hint TEXT,
  old_hash INTEGER,
  new_hash INTEGER,
  old_text TEXT,
  new_text TEXT,
  PRIMARY KEY (diff_id, entity_id),
  FOREIGN KEY (diff_id) REFERENCES diff_runs(diff_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_diff_pbip_entities ON diff_pbip_entities(diff_id, doc_path);
