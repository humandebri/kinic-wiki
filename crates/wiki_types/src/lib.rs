// Where: crates/wiki_types/src/lib.rs
// What: Shared wiki domain types and cross-crate contracts.
// Why: Keep the store, search, and runtime crates aligned on one schema and API surface.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WikiPageType {
    Entity,
    Concept,
    Overview,
    Comparison,
    QueryNote,
    SourceSummary,
}

impl WikiPageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Concept => "concept",
            Self::Overview => "overview",
            Self::Comparison => "comparison",
            Self::QueryNote => "query_note",
            Self::SourceSummary => "source_summary",
        }
    }

    pub fn group_label(&self) -> &'static str {
        match self {
            Self::Entity => "Entities",
            Self::Concept => "Concepts",
            Self::Overview => "Overviews",
            Self::Comparison => "Comparisons",
            Self::QueryNote => "Query Notes",
            Self::SourceSummary => "Source Summaries",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "entity" => Some(Self::Entity),
            "concept" => Some(Self::Concept),
            "overview" => Some(Self::Overview),
            "comparison" => Some(Self::Comparison),
            "query_note" => Some(Self::QueryNote),
            "source_summary" => Some(Self::SourceSummary),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchDocKind {
    IndexPage,
    WikiSection,
    SystemLog,
}

impl SearchDocKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IndexPage => "index_page",
            Self::WikiSection => "wiki_section",
            Self::SystemLog => "system_log",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "index_page" => Some(Self::IndexPage),
            "wiki_section" => Some(Self::WikiSection),
            "system_log" => Some(Self::SystemLog),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: String,
    pub slug: String,
    pub page_type: WikiPageType,
    pub title: String,
    pub current_revision_id: Option<String>,
    pub summary_1line: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiRevision {
    pub id: String,
    pub page_id: String,
    pub revision_no: i64,
    pub markdown: String,
    pub change_reason: String,
    pub author_type: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiSection {
    pub id: String,
    pub page_id: String,
    pub revision_id: String,
    pub section_path: String,
    pub ordinal: i64,
    pub heading: Option<String>,
    pub text: String,
    pub content_hash: String,
    pub is_current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionCitationInput {
    pub source_id: String,
    pub chunk_id: Option<String>,
    pub evidence_kind: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPage {
    pub slug: String,
    pub markdown: String,
    pub updated_at: i64,
    pub etag: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchProjectionDoc {
    pub external_id: String,
    pub kind: SearchDocKind,
    pub page_id: Option<String>,
    pub revision_id: Option<String>,
    pub section_path: Option<String>,
    pub title: String,
    pub snippet: String,
    pub citation: String,
    pub content: String,
    pub section: Option<String>,
    pub tags: Vec<String>,
    pub updated_at: i64,
}

pub trait SearchProjectionWriter {
    fn upsert_docs(&self, docs: &[SearchProjectionDoc]) -> Result<(), String>;
    fn delete_docs_by_external_ids(&self, ids: &[String]) -> Result<(), String>;
    fn delete_docs_by_prefix(&self, prefix: &str) -> Result<usize, String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePageInput {
    pub slug: String,
    pub page_type: WikiPageType,
    pub title: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPageRevisionInput {
    pub page_id: String,
    pub expected_current_revision_id: Option<String>,
    pub title: String,
    pub markdown: String,
    pub change_reason: String,
    pub author_type: String,
    pub citations: Vec<RevisionCitationInput>,
    pub tags: Vec<String>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPageRevisionOutput {
    pub revision_id: String,
    pub revision_no: i64,
    pub section_count: u32,
    pub unchanged_section_count: u32,
    pub upserted_projection_ids: Vec<String>,
    pub deleted_projection_ids: Vec<String>,
    pub rendered_system_pages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalSearchRequest {
    pub query_text: String,
    pub kinds: Vec<SearchDocKind>,
    pub section: Option<String>,
    pub tags: Vec<String>,
    pub top_k: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub external_id: String,
    pub kind: SearchDocKind,
    pub title: String,
    pub snippet: String,
    pub citation: String,
    pub section: Option<String>,
    pub tags: Vec<String>,
    pub score_bits: u32,
    pub match_reasons: Vec<String>,
}

impl SearchHit {
    pub fn score(&self) -> f32 {
        f32::from_bits(self.score_bits)
    }

    pub fn new(
        external_id: String,
        kind: SearchDocKind,
        title: String,
        snippet: String,
        citation: String,
        section: Option<String>,
        tags: Vec<String>,
        score: f32,
        match_reasons: Vec<String>,
    ) -> Self {
        Self {
            external_id,
            kind,
            title,
            snippet,
            citation,
            section,
            tags,
            score_bits: score.to_bits(),
            match_reasons,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSectionView {
    pub section_path: String,
    pub heading: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageBundle {
    pub page: WikiPage,
    pub revision: WikiRevision,
    pub sections: Vec<PageSectionView>,
}
