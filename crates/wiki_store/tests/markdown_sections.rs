use std::sync::Mutex;

use rusqlite::Connection;
use tempfile::tempdir;
use wiki_store::WikiStore;
use wiki_types::{
    CommitPageRevisionInput, CreatePageInput, SearchProjectionDoc, SearchProjectionWriter,
    WikiPageType,
};

#[derive(Default)]
struct RecordingSearch {
    upserts: Mutex<Vec<String>>,
    deletes: Mutex<Vec<String>>,
}

impl SearchProjectionWriter for RecordingSearch {
    fn upsert_docs(&self, docs: &[SearchProjectionDoc]) -> Result<(), String> {
        self.upserts
            .lock()
            .expect("mutex should lock")
            .extend(docs.iter().map(|doc| doc.external_id.clone()));
        Ok(())
    }

    fn delete_docs_by_external_ids(&self, ids: &[String]) -> Result<(), String> {
        self.deletes
            .lock()
            .expect("mutex should lock")
            .extend(ids.iter().cloned());
        Ok(())
    }

    fn delete_docs_by_prefix(&self, _prefix: &str) -> Result<usize, String> {
        Ok(0)
    }
}

#[test]
fn migrations_create_required_tables() {
    let dir = tempdir().expect("temp dir should exist");
    let store = WikiStore::new(dir.path().join("wiki.sqlite3"));
    store.run_migrations().expect("migrations should succeed");

    let conn = Connection::open(store.database_path()).expect("db should open");
    for table in [
        "wiki_pages",
        "wiki_revisions",
        "wiki_sections",
        "revision_citations",
        "log_events",
        "system_pages",
    ] {
        let exists = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get::<_, i64>(0),
            )
            .expect("table should exist");
        assert_eq!(exists, 1);
    }
}

#[test]
fn commit_revision_reports_unchanged_and_deleted_sections() {
    let dir = tempdir().expect("temp dir should exist");
    let store = WikiStore::new(dir.path().join("wiki.sqlite3"));
    let search = RecordingSearch::default();
    store.run_migrations().expect("migrations should succeed");
    let page_id = store
        .create_page(CreatePageInput {
            slug: "diff-page".to_string(),
            page_type: WikiPageType::Overview,
            title: "Diff Page".to_string(),
            created_at: 1_700_000_000,
        })
        .expect("page should create");

    let first = store
        .commit_page_revision(
            &search,
            CommitPageRevisionInput {
                page_id: page_id.clone(),
                expected_current_revision_id: None,
                title: "Diff Page".to_string(),
                markdown: "# Keep\n\nsame\n\n# Remove\n\nold".to_string(),
                change_reason: "first".to_string(),
                author_type: "test".to_string(),
                citations: Vec::new(),
                tags: Vec::new(),
                updated_at: 1_700_000_001,
            },
        )
        .expect("first revision should commit");

    let second = store
        .commit_page_revision(
            &search,
            CommitPageRevisionInput {
                page_id,
                expected_current_revision_id: Some(first.revision_id),
                title: "Diff Page".to_string(),
                markdown: "# Keep\n\nsame\n\n# Add\n\nnew".to_string(),
                change_reason: "second".to_string(),
                author_type: "test".to_string(),
                citations: Vec::new(),
                tags: Vec::new(),
                updated_at: 1_700_000_002,
            },
        )
        .expect("second revision should commit");

    assert_eq!(second.unchanged_section_count, 1);
    assert!(
        second
            .deleted_projection_ids
            .iter()
            .any(|id| id.ends_with(":section:remove"))
    );
}

#[test]
fn current_page_bundle_uses_intro_and_duplicate_section_paths() {
    let dir = tempdir().expect("temp dir should exist");
    let store = WikiStore::new(dir.path().join("wiki.sqlite3"));
    let search = RecordingSearch::default();
    store.run_migrations().expect("migrations should succeed");
    store.create_page(CreatePageInput {
        slug: "shape".to_string(),
        page_type: WikiPageType::Concept,
        title: "Shape".to_string(),
        created_at: 1_700_000_000,
    })
    .and_then(|page_id| {
        store.commit_page_revision(
            &search,
            CommitPageRevisionInput {
                page_id,
                expected_current_revision_id: None,
                title: "Shape".to_string(),
                markdown: "Lead\n\n# Root\n\n## Child\n\none\n\n## Child\n\ntwo".to_string(),
                change_reason: "shape".to_string(),
                author_type: "test".to_string(),
                citations: Vec::new(),
                tags: Vec::new(),
                updated_at: 1_700_000_001,
            },
        )
    })
    .expect("revision should commit");

    let page = store
        .get_page_by_slug("shape")
        .expect("page lookup should succeed")
        .expect("page should exist");
    let paths = page
        .sections
        .iter()
        .map(|section| section.section_path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"__intro__"));
    assert!(paths.contains(&"root/child"));
    assert!(paths.contains(&"root/child-2"));
}
