CREATE TABLE fs_path_state (
    path TEXT PRIMARY KEY,
    last_change_revision INTEGER NOT NULL
);

INSERT INTO fs_path_state (path, last_change_revision)
SELECT path, MAX(revision) AS last_change_revision
FROM fs_change_log
GROUP BY path
ORDER BY path ASC;
