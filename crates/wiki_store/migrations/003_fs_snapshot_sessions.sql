CREATE TABLE fs_snapshot_sessions (
    session_id TEXT PRIMARY KEY,
    prefix TEXT NOT NULL,
    snapshot_revision INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE fs_snapshot_session_paths (
    session_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (session_id, ordinal),
    UNIQUE (session_id, path)
);

CREATE INDEX fs_snapshot_sessions_expires_at_idx
    ON fs_snapshot_sessions (expires_at);
