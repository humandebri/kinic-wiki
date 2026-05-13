CREATE TABLE worker_cursors (
  database_id TEXT NOT NULL,
  prefix TEXT NOT NULL,
  snapshot_revision TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (database_id, prefix)
);

CREATE TABLE source_jobs (
  database_id TEXT NOT NULL,
  source_path TEXT NOT NULL,
  source_etag TEXT NOT NULL,
  status TEXT NOT NULL
    CHECK (status IN ('queued', 'processing', 'completed', 'failed')),
  target_path TEXT,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (database_id, source_path)
);

CREATE INDEX source_jobs_status_idx
ON source_jobs (status, updated_at);
