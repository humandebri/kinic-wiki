// Where: crates/wiki_search/src/search.rs
// What: Stateless projection writes and lexical search over a shared SQLite connection.
// Why: The wiki store and retrieval engine must share one DB transaction to stay atomic.
use rusqlite::Connection;

use ic_hybrid_engine::{
    HybridQueryFilters, HybridQueryRequest, IndexedDocument, delete_document_by_external_id_in_connection,
    delete_documents_by_prefix_in_connection, hybrid_query_connection,
    upsert_document_by_external_id_in_connection,
};
use wiki_types::{LexicalSearchRequest, SearchDocKind, SearchHit, SearchProjectionDoc};

const LEXICAL_VECTOR_DISABLED: u32 = 0;
const DEFAULT_KINDS: &[SearchDocKind] = &[SearchDocKind::WikiSection, SearchDocKind::IndexPage];

pub struct WikiSearch;

impl WikiSearch {
    pub fn upsert_docs_in_tx(conn: &Connection, docs: &[SearchProjectionDoc]) -> Result<(), String> {
        for doc in docs {
            upsert_document_by_external_id_in_connection(
                conn,
                &IndexedDocument {
                    external_id: Some(doc.external_id.clone()),
                    kind: Some(doc.kind.as_str().to_string()),
                    title: doc.title.clone(),
                    snippet: doc.snippet.clone(),
                    citation: doc.citation.clone(),
                    content: doc.content.clone(),
                    version: None,
                    section: doc.section.clone(),
                    tags: doc.tags.clone(),
                    embedding: vec![1.0],
                    updated_at: Some(doc.updated_at),
                },
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn delete_docs_by_external_ids_in_tx(
        conn: &Connection,
        ids: &[String],
    ) -> Result<(), String> {
        for id in ids {
            delete_document_by_external_id_in_connection(conn, id)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn delete_docs_by_prefix_in_tx(conn: &Connection, prefix: &str) -> Result<usize, String> {
        delete_documents_by_prefix_in_connection(conn, prefix).map_err(|error| error.to_string())
    }

    pub fn lexical_search(
        conn: &Connection,
        request: LexicalSearchRequest,
    ) -> Result<Vec<SearchHit>, String> {
        let has_query_text = !request.query_text.trim().is_empty();
        let kinds = if request.kinds.is_empty() {
            DEFAULT_KINDS
                .iter()
                .map(|kind| kind.as_str().to_string())
                .collect()
        } else {
            request
                .kinds
                .iter()
                .map(|kind| kind.as_str().to_string())
                .collect()
        };
        let results = hybrid_query_connection(
            conn,
            &HybridQueryRequest {
                query_text: request.query_text,
                query_embedding: vec![1.0],
                version: None,
                top_k: request.top_k,
                keyword_candidate_limit: None,
                vector_candidate_limit: Some(LEXICAL_VECTOR_DISABLED),
                keyword_weight: Some(1.0),
                vector_weight: Some(0.0),
                scoring_policy: None,
                filters: Some(HybridQueryFilters {
                    section: request.section,
                    tags: request.tags,
                    kinds,
                }),
            },
        )
        .map_err(|error| error.to_string())?;
        results
            .into_iter()
            .filter(|result| !has_query_text || result.breakdown.keyword_score > 0.0)
            .map(|result| {
                let kind = result
                    .document
                    .kind
                    .as_deref()
                    .and_then(SearchDocKind::from_str)
                    .ok_or_else(|| "search result is missing kind".to_string())?;
                let external_id = result
                    .document
                    .external_id
                    .ok_or_else(|| "search result is missing external_id".to_string())?;
                Ok(SearchHit::new(
                    external_id,
                    kind,
                    result.document.title,
                    result.document.snippet,
                    result.document.citation,
                    result.document.section,
                    result.document.tags,
                    result.score,
                    result.match_reasons,
                ))
            })
            .collect()
    }
}
