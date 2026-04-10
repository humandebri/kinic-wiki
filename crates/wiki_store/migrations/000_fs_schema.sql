CREATE TABLE fs_nodes (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    etag TEXT NOT NULL,
    deleted_at INTEGER,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE VIRTUAL TABLE fs_nodes_fts USING fts5(
    content,
    content='fs_nodes',
    content_rowid='id'
);

INSERT INTO fs_nodes_fts (rowid, content)
SELECT id, content
FROM fs_nodes
WHERE deleted_at IS NULL;

CREATE TABLE fs_change_log (
    revision INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    deleted_at INTEGER,
    change_kind TEXT NOT NULL
        CHECK (change_kind IN ('upsert', 'tombstone', 'path_removal'))
);

INSERT INTO fs_change_log (path, deleted_at, change_kind)
SELECT
    path,
    deleted_at,
    CASE
        WHEN deleted_at IS NULL THEN 'upsert'
        ELSE 'tombstone'
    END
FROM fs_nodes
ORDER BY path ASC;
