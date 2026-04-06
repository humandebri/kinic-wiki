// Where: crates/wiki_search/src/search.rs
// What: Projection writer and lexical search wrapper over ic_hybrid_engine.
// Why: Reuse the existing retrieval core without letting app code depend on engine-specific types.
use std::path::{Path, PathBuf};

use ic_hybrid_engine::{HybridEngine, HybridQueryFilters, HybridQueryRequest, IndexedDocument};
use wiki_types::{
    LexicalSearchRequest, SearchDocKind, SearchHit, SearchProjectionDoc, SearchProjectionWriter,
};

const LEXICAL_VECTOR_DISABLED: u32 = 0;
const DEFAULT_KINDS: &[SearchDocKind] = &[SearchDocKind::WikiSection, SearchDocKind::IndexPage];

pub struct WikiSearch {
    database_path: PathBuf,
}

impl WikiSearch {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn run_migrations(&self) -> Result<(), String> {
        let mut engine = self.open_engine()?;
        engine.migrate().map_err(|error| error.to_string())?;
        if engine
            .consistency_report()
            .map_err(|error| error.to_string())?
            .configured_dimension
            .is_none()
        {
            engine
                .configure_vector_dimension(1)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn lexical_search(&self, request: LexicalSearchRequest) -> Result<Vec<SearchHit>, String> {
        let engine = self.open_engine()?;
        let has_query_text = !request.query_text.trim().is_empty();
        let kinds = if request.kinds.is_empty() {
            DEFAULT_KINDS.iter().map(|kind| kind.as_str().to_string()).collect()
        } else {
            request
                .kinds
                .iter()
                .map(|kind| kind.as_str().to_string())
                .collect()
        };
        let results = engine
            .hybrid_query(&HybridQueryRequest {
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
            })
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

    fn open_engine(&self) -> Result<HybridEngine, String> {
        HybridEngine::open(&self.database_path).map_err(|error| error.to_string())
    }
}

impl SearchProjectionWriter for WikiSearch {
    fn upsert_docs(&self, docs: &[SearchProjectionDoc]) -> Result<(), String> {
        let engine = self.open_engine()?;
        for doc in docs {
            engine
                .upsert_document_by_external_id(&IndexedDocument {
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
                })
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn delete_docs_by_external_ids(&self, ids: &[String]) -> Result<(), String> {
        let engine = self.open_engine()?;
        for id in ids {
            engine
                .delete_document_by_external_id(id)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn delete_docs_by_prefix(&self, prefix: &str) -> Result<usize, String> {
        let engine = self.open_engine()?;
        engine
            .delete_documents_by_prefix(prefix)
            .map_err(|error| error.to_string())
    }
}
