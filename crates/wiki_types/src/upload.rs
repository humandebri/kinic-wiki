// Where: crates/wiki_types/src/upload.rs
// What: Upload-layer DTOs for assembling large sources before final persistence.
// Why: Large source bodies may need chunked transport, but the source of truth remains one body per source.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginSourceUploadInput {
    pub source_type: String,
    pub title: Option<String>,
    pub canonical_uri: Option<String>,
    pub sha256: String,
    pub mime_type: Option<String>,
    pub imported_at: i64,
    pub metadata_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendSourceChunkInput {
    pub upload_id: String,
    pub chunk_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceUploadStatus {
    pub upload_id: String,
    pub chunk_count: u32,
    pub byte_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizeSourceUploadInput {
    pub upload_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizeSourceUploadOutput {
    pub source_id: String,
    pub chunk_count: u32,
}
