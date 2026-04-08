CREATE TABLE fs_change_log (
    revision INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    deleted_at INTEGER
);

INSERT INTO fs_change_log (path, deleted_at)
SELECT path, deleted_at
FROM fs_nodes
ORDER BY path ASC;
