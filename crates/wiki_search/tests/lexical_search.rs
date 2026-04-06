use tempfile::tempdir;
use wiki_search::WikiSearch;
use wiki_types::{LexicalSearchRequest, SearchDocKind, SearchProjectionDoc, SearchProjectionWriter};

#[test]
fn lexical_search_drops_keyword_miss_fallback_hits() {
    let dir = tempdir().expect("temp dir should exist");
    let search = WikiSearch::new(dir.path().join("search.sqlite3"));
    search.run_migrations().expect("migrations should succeed");
    search
        .upsert_docs(&[SearchProjectionDoc {
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
        }])
        .expect("projection should upsert");

    let results = search
        .lexical_search(LexicalSearchRequest {
            query_text: "legacy".to_string(),
            kinds: vec![SearchDocKind::WikiSection],
            section: None,
            tags: Vec::new(),
            top_k: 5,
        })
        .expect("search should succeed");
    assert!(results.is_empty());
}
