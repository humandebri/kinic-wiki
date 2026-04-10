CREATE TABLE fs_nodes_next (
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

INSERT INTO fs_nodes_next (
    path,
    kind,
    content,
    created_at,
    updated_at,
    etag,
    deleted_at,
    metadata_json
)
SELECT
    path,
    kind,
    content,
    created_at,
    updated_at,
    etag,
    deleted_at,
    metadata_json
FROM fs_nodes
ORDER BY path ASC;

DROP TABLE fs_nodes;

ALTER TABLE fs_nodes_next RENAME TO fs_nodes;

DROP TABLE fs_nodes_fts;

CREATE VIRTUAL TABLE fs_nodes_fts USING fts5(
    content,
    content='fs_nodes',
    content_rowid='id'
);

INSERT INTO fs_nodes_fts (rowid, content)
SELECT id, content
FROM fs_nodes
WHERE deleted_at IS NULL;
