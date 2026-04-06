CREATE TABLE source_uploads (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    title TEXT,
    canonical_uri TEXT,
    sha256 TEXT NOT NULL,
    mime_type TEXT,
    imported_at INTEGER NOT NULL,
    metadata_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE source_upload_chunks (
    upload_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    PRIMARY KEY(upload_id, ordinal)
);

CREATE INDEX idx_source_upload_chunks_upload_id ON source_upload_chunks(upload_id, ordinal);
