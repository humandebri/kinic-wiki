// Where: crates/wiki_store/src/schema.rs
// What: Schema setup for the wiki source-of-truth tables.
// Why: Revision state, citations, and rendered system pages must exist independently from the search engine.
use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS wiki_pages (
            id TEXT PRIMARY KEY,
            slug TEXT NOT NULL UNIQUE,
            page_type TEXT NOT NULL,
            title TEXT NOT NULL,
            current_revision_id TEXT,
            summary_1line TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS wiki_revisions (
            id TEXT PRIMARY KEY,
            page_id TEXT NOT NULL,
            revision_no INTEGER NOT NULL,
            markdown TEXT NOT NULL,
            change_reason TEXT NOT NULL,
            author_type TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(page_id, revision_no)
        );

        CREATE TABLE IF NOT EXISTS wiki_sections (
            id TEXT PRIMARY KEY,
            page_id TEXT NOT NULL,
            revision_id TEXT NOT NULL,
            section_path TEXT NOT NULL,
            ordinal INTEGER NOT NULL,
            heading TEXT,
            text TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            is_current INTEGER NOT NULL,
            UNIQUE(page_id, revision_id, section_path, ordinal)
        );

        CREATE TABLE IF NOT EXISTS revision_citations (
            id TEXT PRIMARY KEY,
            revision_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            chunk_id TEXT,
            evidence_kind TEXT NOT NULL,
            note TEXT
        );

        CREATE TABLE IF NOT EXISTS log_events (
            id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            title TEXT NOT NULL,
            body_markdown TEXT NOT NULL,
            related_page_id TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS system_pages (
            slug TEXT PRIMARY KEY,
            markdown TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            etag TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_wiki_pages_slug ON wiki_pages(slug);
        CREATE INDEX IF NOT EXISTS idx_wiki_revisions_page_revision_no
            ON wiki_revisions(page_id, revision_no);
        CREATE INDEX IF NOT EXISTS idx_wiki_sections_page_current_ordinal
            ON wiki_sections(page_id, is_current, ordinal);
        CREATE INDEX IF NOT EXISTS idx_log_events_created_at
            ON log_events(created_at DESC);
        ",
    )
    .map_err(|error| error.to_string())
}
