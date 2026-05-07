CREATE TABLE fs_links (
    source_path TEXT NOT NULL,
    target_path TEXT NOT NULL,
    raw_href TEXT NOT NULL,
    link_text TEXT NOT NULL,
    link_kind TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (source_path, target_path, raw_href)
);

CREATE INDEX fs_links_target_path_idx
ON fs_links (target_path, source_path);

CREATE INDEX fs_links_source_path_idx
ON fs_links (source_path, target_path);
