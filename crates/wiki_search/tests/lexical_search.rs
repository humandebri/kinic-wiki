use ic_hybrid_engine::{configure_vector_dimension_connection, migrate_connection, register_sqlite_vec};
use rusqlite::Connection;
use tempfile::tempdir;
use wiki_search::WikiSearch;
use wiki_types::{LexicalSearchRequest, SearchDocKind, SearchProjectionDoc};

#[test]
fn lexical_search_drops_keyword_miss_fallback_hits() {
    let dir = tempdir().expect("temp dir should exist");
    let db_path = dir.path().join("search.sqlite3");
    register_sqlite_vec().expect("sqlite-vec should register");
    let mut conn = Connection::open(db_path).expect("db should open");
    migrate_connection(&mut conn).expect("migrations should succeed");
    configure_vector_dimension_connection(&conn, 1).expect("vector dimension should configure");
    WikiSearch::upsert_docs_in_tx(
        &conn,
        &[SearchProjectionDoc {
            external_id: "page:test:section:intro".to_string(),
            kind: SearchDocKind::WikiSection,
            page_id: Some("test".to_string()),
            revision_id: Some("rev".to_string()),
            section_path: Some("intro".to_string()),
            title: "Test".to_string(),
            snippet: "snippet".to_string(),
            citation: "wiki://test#intro".to_string(),
            content: "fresh token".to_string(),
            section: Some("intro".to_string()),
            tags: vec!["overview".to_string()],
            updated_at: 1,
        }],
    )
    .expect("projection should upsert");

    let results = WikiSearch::lexical_search(
        &conn,
        LexicalSearchRequest {
            query_text: "legacy".to_string(),
            kinds: vec![SearchDocKind::WikiSection],
            section: None,
            tags: Vec::new(),
            top_k: 5,
        },
    )
    .expect("search should succeed");
    assert!(results.is_empty());
}
