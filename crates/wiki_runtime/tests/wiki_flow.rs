use rusqlite::Connection;
use tempfile::tempdir;
use wiki_runtime::WikiService;
use wiki_types::{
    CommitPageRevisionInput, CreatePageInput, LexicalSearchRequest, SearchDocKind, WikiPageType,
};

#[test]
fn commit_and_search_lexical_returns_current_sections() {
    let dir = tempdir().expect("temp dir should exist");
    let db_path = dir.path().join("wiki.sqlite3");
    let service = WikiService::new(db_path);
    service.run_migrations().expect("migrations should succeed");
    let page_id = service
        .create_page(CreatePageInput {
            slug: "placeholder".to_string(),
            page_type: WikiPageType::Overview,
            title: "Placeholder".to_string(),
            created_at: 1_700_000_000,
        })
        .expect("page should create");
    let output = service
        .commit_page_revision(CommitPageRevisionInput {
            page_id,
            expected_current_revision_id: None,
            title: "Placeholder".to_string(),
            markdown: "# Title\n\nbody".to_string(),
            change_reason: "seed".to_string(),
            author_type: "test".to_string(),
            citations: Vec::new(),
            tags: Vec::new(),
            updated_at: 1_700_000_001,
        })
        .expect("revision should commit");
    assert_eq!(output.section_count, 1);
    let hits = service
        .search_lexical(LexicalSearchRequest {
            query_text: "body".to_string(),
            kinds: Vec::new(),
            section: None,
            tags: Vec::new(),
            top_k: 5,
        })
        .expect("search should succeed");
    assert!(hits.iter().any(|hit| hit.kind == SearchDocKind::WikiSection));
}

#[test]
fn revision_update_replaces_old_keywords_and_keeps_system_index_searchable() {
    let dir = tempdir().expect("temp dir should exist");
    let db_path = dir.path().join("wiki.sqlite3");
    let service = WikiService::new(db_path);
    service.run_migrations().expect("migrations should succeed");
    let page_id = service
        .create_page(CreatePageInput {
            slug: "alpha".to_string(),
            page_type: WikiPageType::Entity,
            title: "Alpha".to_string(),
            created_at: 1_700_000_000,
        })
        .expect("page should create");
    let first = service
        .commit_page_revision(CommitPageRevisionInput {
            page_id: page_id.clone(),
            expected_current_revision_id: None,
            title: "Alpha".to_string(),
            markdown: "# Alpha\n\nlegacy token".to_string(),
            change_reason: "seed".to_string(),
            author_type: "test".to_string(),
            citations: Vec::new(),
            tags: Vec::new(),
            updated_at: 1_700_000_001,
        })
        .expect("first revision should commit");
    service
        .commit_page_revision(CommitPageRevisionInput {
            page_id,
            expected_current_revision_id: Some(first.revision_id),
            title: "Alpha".to_string(),
            markdown: "# Alpha\n\nfresh token".to_string(),
            change_reason: "update".to_string(),
            author_type: "test".to_string(),
            citations: Vec::new(),
            tags: Vec::new(),
            updated_at: 1_700_000_002,
        })
        .expect("second revision should commit");

    let old_hits = service
        .search_lexical(LexicalSearchRequest {
            query_text: "legacy".to_string(),
            kinds: vec![SearchDocKind::WikiSection],
            section: None,
            tags: Vec::new(),
            top_k: 5,
        })
        .expect("old search should succeed");
    assert!(old_hits.is_empty());

    let new_hits = service
        .search_lexical(LexicalSearchRequest {
            query_text: "fresh".to_string(),
            kinds: vec![SearchDocKind::WikiSection],
            section: None,
            tags: Vec::new(),
            top_k: 5,
        })
        .expect("new search should succeed");
    assert!(!new_hits.is_empty());

    let index_hits = service
        .search_lexical(LexicalSearchRequest {
            query_text: "index".to_string(),
            kinds: vec![SearchDocKind::IndexPage],
            section: None,
            tags: Vec::new(),
            top_k: 10,
        })
        .expect("index search should succeed");
    assert!(
        index_hits
            .iter()
            .any(|hit| hit.external_id == "sys:index.md")
    );
}

#[test]
fn expected_revision_mismatch_fails() {
    let dir = tempdir().expect("temp dir should exist");
    let db_path = dir.path().join("wiki.sqlite3");
    let service = WikiService::new(db_path);
    service.run_migrations().expect("migrations should succeed");
    let page_id = service
        .create_page(CreatePageInput {
            slug: "conflict".to_string(),
            page_type: WikiPageType::Overview,
            title: "Conflict".to_string(),
            created_at: 1_700_000_000,
        })
        .expect("page should create");
    let error = service
        .commit_page_revision(CommitPageRevisionInput {
            page_id,
            expected_current_revision_id: Some("wrong".to_string()),
            title: "Conflict".to_string(),
            markdown: "# Conflict\n\nbody".to_string(),
            change_reason: "seed".to_string(),
            author_type: "test".to_string(),
            citations: Vec::new(),
            tags: Vec::new(),
            updated_at: 1_700_000_001,
        })
        .expect_err("conflicting revision should fail");
    assert!(error.contains("expected_current_revision_id"));
}

#[test]
fn projection_failure_rolls_back_revision_write() {
    let dir = tempdir().expect("temp dir should exist");
    let db_path = dir.path().join("wiki.sqlite3");
    let service = WikiService::new(db_path.clone());
    service.run_migrations().expect("migrations should succeed");
    let page_id = service
        .create_page(CreatePageInput {
            slug: "rollback".to_string(),
            page_type: WikiPageType::Overview,
            title: "Rollback".to_string(),
            created_at: 1_700_000_000,
        })
        .expect("page should create");
    let conn = Connection::open(&db_path).expect("db should open");
    conn.execute_batch(
        "
        CREATE TRIGGER fail_projection_insert
        BEFORE INSERT ON documents
        BEGIN
            SELECT RAISE(ABORT, 'projection insert failed');
        END;
        ",
    )
    .expect("trigger should create");

    let error = service
        .commit_page_revision(CommitPageRevisionInput {
            page_id: page_id.clone(),
            expected_current_revision_id: None,
            title: "Rollback".to_string(),
            markdown: "# Rollback\n\nbody".to_string(),
            change_reason: "seed".to_string(),
            author_type: "test".to_string(),
            citations: Vec::new(),
            tags: Vec::new(),
            updated_at: 1_700_000_001,
        })
        .expect_err("projection failure should abort commit");
    assert!(error.contains("projection insert failed"));

    let revision_count = conn
        .query_row(
            "SELECT COUNT(*) FROM wiki_revisions WHERE page_id = ?1",
            [&page_id],
            |row| row.get::<_, i64>(0),
        )
        .expect("revision count should query");
    let section_count = conn
        .query_row(
            "SELECT COUNT(*) FROM wiki_sections WHERE page_id = ?1",
            [&page_id],
            |row| row.get::<_, i64>(0),
        )
        .expect("section count should query");
    let document_count = conn
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get::<_, i64>(0))
        .expect("document count should query");
    assert_eq!(revision_count, 0);
    assert_eq!(section_count, 0);
    assert_eq!(document_count, 0);
}
